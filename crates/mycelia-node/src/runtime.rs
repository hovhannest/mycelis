use crate::config::NodeConfig;
use crate::discovery::{handle_control_bytes, DiscoveryManager};
use crate::mock_transport::{MockHub, MockTransport};
use crate::state::NodeState;
use mycelia_core::transport::ReticulumTransport;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

#[derive(Clone)]
pub struct NodeHandle {
    pub state: Arc<Mutex<NodeState>>,
    pub transport: Arc<Mutex<dyn ReticulumTransport + Send>>,
    pub control_addr: std::net::SocketAddr,
    pub listen_addr: std::net::SocketAddr,
    pub iface_kinds: Vec<String>,
}

pub struct NodeRuntime {
    pub handle: NodeHandle,
    shutdown: Option<oneshot::Sender<()>>,
}

impl NodeRuntime {
    pub async fn start(mut cfg: NodeConfig, hub: Option<MockHub>) -> anyhow::Result<Self> {
        cfg.apply_env_overrides();
        if cfg.data_dir.as_os_str().is_empty() {
            cfg.data_dir = PathBuf::from(".mycelis");
        }
        std::fs::create_dir_all(&cfg.data_dir)?;
        let state = NodeState::bootstrap_with_pow(&cfg.data_dir, cfg.pow_difficulty)?;

        let use_mock = hub.is_some()
            || cfg.wants_mock_transport()
            || !cfg!(feature = "transport-rns");

        let mut iface_kinds: Vec<String> = Vec::new();

        let (transport, listen_addr): (
            Arc<Mutex<dyn ReticulumTransport + Send>>,
            std::net::SocketAddr,
        ) = if use_mock {
            iface_kinds.push("mock".into());
            let hub = hub.unwrap_or_default();
            let listen = tokio::net::TcpListener::bind(cfg.listen).await?;
            let listen_addr = listen.local_addr()?;
            // Placeholder accept loop (mock path has no RNS TCP server).
            tokio::spawn(async move {
                loop {
                    let Ok((mut s, _)) = listen.accept().await else {
                        break;
                    };
                    tokio::spawn(async move {
                        use tokio::io::AsyncReadExt;
                        let mut buf = [0u8; 1];
                        let _ = s.read(&mut buf).await;
                    });
                }
            });
            (
                Arc::new(Mutex::new(MockTransport::new(state.node_id, hub))),
                listen_addr,
            )
        } else {
            #[cfg(feature = "transport-rns")]
            {
                let ifaces = cfg.effective_interfaces();
                iface_kinds = ifaces.iter().map(|i| i.kind_name().to_string()).collect();
                let rns = crate::rns_transport::RnsTransport::start(
                    state.node_id,
                    &ifaces,
                    &cfg.data_dir,
                    cfg.listen,
                )
                .await?;
                let listen_addr = rns.listen_addr();
                (Arc::new(Mutex::new(rns)), listen_addr)
            }
            #[cfg(not(feature = "transport-rns"))]
            {
                unreachable!("transport-rns disabled but use_mock was false");
            }
        };

        let state = Arc::new(Mutex::new(state));
        let discovery = DiscoveryManager::new(&cfg)?;
        {
            let st = state.lock().await;
            let _ = discovery.register_mdns(st.node_id, listen_addr);
        }

        DiscoveryManager::dial_static_peers(
            Arc::clone(&state),
            Arc::clone(&transport),
            &cfg.static_peers,
        )
        .await;

        // DHT cold-start + publish (feature + config gated).
        #[cfg(feature = "discovery-dht")]
        if cfg.enable_dht {
            let peer_keys: Vec<String> = {
                let st = state.lock().await;
                st.peer_cache
                    .iter()
                    .map(|p| hex::encode(p.node_id.as_bytes()))
                    .collect()
            };
            let nid = {
                let st = state.lock().await;
                st.node_id
            };
            if let Err(e) =
                crate::discovery::spawn_dht_locator(nid, listen_addr, peer_keys).await
            {
                tracing::warn!("DHT locator failed to start: {e:#}");
            }
        }

        // Gateway (feature + config + GATEWAY attestation).
        #[cfg(feature = "gateway")]
        if cfg.enable_gateway {
            let mut st = state.lock().await;
            if !st.has_gateway_capability() {
                tracing::warn!(
                    "enable_gateway set but no GATEWAY attestation; gateway not started"
                );
            } else {
                let bind = cfg.gateway_bind;
                st.gateway_enabled = true;
                st.gateway_bind = Some(bind);
                let gw = mycelia_gateway::SocksGateway::new(bind);
                tokio::spawn(async move {
                    if let Err(e) = gw.serve().await {
                        tracing::error!("gateway exited: {e:#}");
                    }
                });
            }
        }

        let (stop_tx, mut stop_rx) = oneshot::channel::<()>();
        let poll_state = Arc::clone(&state);
        let poll_transport = Arc::clone(&transport);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = &mut stop_rx => break,
                    _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {
                        let mut messages = Vec::new();
                        {
                            let mut t = poll_transport.lock().await;
                            while let Ok(Some(incoming)) = t.poll_recv() {
                                messages.push(incoming);
                            }
                        }
                        if !messages.is_empty() {
                            let mut st = poll_state.lock().await;
                            for incoming in messages {
                                handle_control_bytes(&mut st, incoming.from, &incoming.payload);
                            }
                        }
                    }
                }
            }
        });

        let placeholder = NodeHandle {
            state: Arc::clone(&state),
            transport: Arc::clone(&transport),
            control_addr: cfg.control_bind,
            listen_addr,
            iface_kinds: iface_kinds.clone(),
        };
        let control_addr = crate::control::serve_control(cfg.control_bind, placeholder).await?;

        let lock = cfg.data_dir.join("control.addr");
        std::fs::write(&lock, control_addr.to_string())?;

        Ok(Self {
            handle: NodeHandle {
                state,
                transport,
                control_addr,
                listen_addr,
                iface_kinds,
            },
            shutdown: Some(stop_tx),
        })
    }

    pub fn control_addr(&self) -> std::net::SocketAddr {
        self.handle.control_addr
    }

    pub fn shutdown(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}
