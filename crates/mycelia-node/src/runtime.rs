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
}

pub struct NodeRuntime {
    pub handle: NodeHandle,
    shutdown: Option<oneshot::Sender<()>>,
}

impl NodeRuntime {
    pub async fn start(mut cfg: NodeConfig, hub: Option<MockHub>) -> anyhow::Result<Self> {
        if cfg.data_dir.as_os_str().is_empty() {
            cfg.data_dir = PathBuf::from(".mycelis");
        }
        std::fs::create_dir_all(&cfg.data_dir)?;
        let state = NodeState::bootstrap(&cfg.data_dir)?;
        let hub = hub.unwrap_or_default();
        let transport: Arc<Mutex<dyn ReticulumTransport + Send>> =
            Arc::new(Mutex::new(MockTransport::new(state.node_id, hub)));

        let listen = tokio::net::TcpListener::bind(cfg.listen).await?;
        let listen_addr = listen.local_addr()?;

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

        let placeholder = NodeHandle {
            state: Arc::clone(&state),
            transport: Arc::clone(&transport),
            control_addr: cfg.control_bind,
            listen_addr,
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
