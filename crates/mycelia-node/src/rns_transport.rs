//! FreeTAKTeam `reticulum-rs` adapter implementing [`ReticulumTransport`].
//!
//! Control payloads ride in RNS announce `app_data` using the frozen MYC1 envelope
//! (see `docs/wire-format.md` / `docs/substrate-notes.md`).
//! Carrier interfaces are attached via [`crate::rns_ifaces`].

use crate::config::RnsInterfaceConfig;
use crate::rns_ifaces::{spawn_all, SpawnReport};
use mycelia_core::ids::NodeId;
use mycelia_core::transport::{Incoming, ReticulumTransport, TransportError};
use rand_core::OsRng;
use reticulum_rs::transport::destination::DestinationName;
use reticulum_rs::transport::identity::PrivateIdentity;
use reticulum_rs::transport::transport::{Transport, TransportConfig};
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

/// Magic + from(16) + to(16) prefix before Mycelia control payload.
pub const MYC1_MAGIC: &[u8; 4] = b"MYC1";
pub const MYC1_HEADER_LEN: usize = 4 + 16 + 16;

#[derive(Debug)]
enum Cmd {
    Deliver { envelope: Vec<u8> },
}

/// Live Reticulum transport for Mycelia control frames.
pub struct RnsTransport {
    node_id: NodeId,
    cmd_tx: mpsc::UnboundedSender<Cmd>,
    inbox: Arc<Mutex<VecDeque<Incoming>>>,
    listen_addr: SocketAddr,
    iface_kinds: Vec<&'static str>,
}

impl RnsTransport {
    /// Spawn a background driver owning `reticulum_rs::Transport`.
    ///
    /// Identity is loaded from / written to `data_dir/rns.identity` (hex).
    /// Interfaces come from `ifaces` (see [`NodeConfig::effective_interfaces`](crate::config::NodeConfig::effective_interfaces)).
    /// Must be called from a Tokio runtime.
    pub async fn start(
        node_id: NodeId,
        ifaces: &[RnsInterfaceConfig],
        data_dir: &Path,
        fallback_listen: SocketAddr,
    ) -> anyhow::Result<Self> {
        let ifaces = ifaces.to_vec();
        let data_dir = data_dir.to_path_buf();
        let identity = load_or_create_identity(&data_dir)?;

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let inbox: Arc<Mutex<VecDeque<Incoming>>> = Arc::new(Mutex::new(VecDeque::new()));
        let inbox_driver = Arc::clone(&inbox);
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<anyhow::Result<SpawnReport>>();

        tokio::spawn(async move {
            let result = driver_main(
                node_id,
                identity,
                ifaces,
                cmd_rx,
                inbox_driver,
                ready_tx,
            )
            .await;
            if let Err(e) = result {
                tracing::error!("rns transport driver exited: {e:#}");
            }
        });

        let report = ready_rx
            .await
            .map_err(|_| anyhow::anyhow!("rns transport driver dropped before ready"))??;

        let listen_addr = report.listen_addr.unwrap_or(fallback_listen);

        Ok(Self {
            node_id,
            cmd_tx,
            inbox,
            listen_addr,
            iface_kinds: report.kinds,
        })
    }

    /// Convenience: synthesize TCP listen + peer clients (legacy tests / examples).
    pub async fn start_tcp(
        node_id: NodeId,
        listen: SocketAddr,
        static_peers: &[SocketAddr],
        data_dir: &Path,
    ) -> anyhow::Result<Self> {
        let mut ifaces = vec![RnsInterfaceConfig::TcpServer {
            bind: listen.to_string(),
            name: Some("listen".into()),
        }];
        for peer in static_peers {
            ifaces.push(RnsInterfaceConfig::TcpClient {
                target: peer.to_string(),
                name: None,
            });
        }
        Self::start(node_id, &ifaces, data_dir, listen).await
    }

    pub fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    pub fn iface_kinds(&self) -> &[&'static str] {
        &self.iface_kinds
    }
}

impl ReticulumTransport for RnsTransport {
    fn identity(&self) -> NodeId {
        self.node_id
    }

    fn send(&mut self, dest: &NodeId, bytes: &[u8]) -> Result<(), TransportError> {
        let envelope = encode_envelope(self.node_id, Some(*dest), bytes);
        self.cmd_tx
            .send(Cmd::Deliver { envelope })
            .map_err(|_| TransportError::SendFailed)
    }

    fn poll_recv(&mut self) -> Result<Option<Incoming>, TransportError> {
        let mut q = self.inbox.lock().map_err(|_| TransportError::RecvFailed)?;
        Ok(q.pop_front())
    }

    fn announce(&mut self, app_data: &[u8]) -> Result<(), TransportError> {
        let envelope = encode_envelope(self.node_id, None, app_data);
        self.cmd_tx
            .send(Cmd::Deliver { envelope })
            .map_err(|_| TransportError::SendFailed)
    }
}

/// Build MYC1 envelope. `to = None` → 16 zero bytes (broadcast).
pub fn encode_envelope(from: NodeId, to: Option<NodeId>, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(MYC1_HEADER_LEN + payload.len());
    out.extend_from_slice(MYC1_MAGIC);
    out.extend_from_slice(from.as_bytes());
    match to {
        Some(t) => out.extend_from_slice(t.as_bytes()),
        None => out.extend_from_slice(&[0u8; 16]),
    }
    out.extend_from_slice(payload);
    out
}

/// Parse MYC1 envelope. Returns `None` if magic/length invalid.
pub fn decode_envelope(bytes: &[u8]) -> Option<(NodeId, NodeId, &[u8])> {
    if bytes.len() < MYC1_HEADER_LEN || &bytes[..4] != MYC1_MAGIC {
        return None;
    }
    let mut from = [0u8; 16];
    from.copy_from_slice(&bytes[4..20]);
    let mut to = [0u8; 16];
    to.copy_from_slice(&bytes[20..36]);
    Some((NodeId::new(from), NodeId::new(to), &bytes[36..]))
}

fn directed_for_us(to: NodeId, self_id: NodeId) -> bool {
    to.as_bytes() == &[0u8; 16] || to == self_id
}

fn load_or_create_identity(data_dir: &Path) -> anyhow::Result<PrivateIdentity> {
    std::fs::create_dir_all(data_dir)?;
    let path = data_dir.join("rns.identity");
    if path.exists() {
        let hex = std::fs::read_to_string(&path)?;
        let id = PrivateIdentity::new_from_hex_string(hex.trim())
            .map_err(|e| anyhow::anyhow!("rns.identity: {e:?}"))?;
        Ok(id)
    } else {
        let id = PrivateIdentity::new_from_rand(OsRng);
        std::fs::write(&path, id.to_hex_string())?;
        Ok(id)
    }
}

async fn driver_main(
    node_id: NodeId,
    identity: PrivateIdentity,
    ifaces: Vec<RnsInterfaceConfig>,
    mut cmd_rx: mpsc::UnboundedReceiver<Cmd>,
    inbox: Arc<Mutex<VecDeque<Incoming>>>,
    ready_tx: tokio::sync::oneshot::Sender<anyhow::Result<SpawnReport>>,
) -> anyhow::Result<()> {
    let cfg = TransportConfig::new("mycelis", &identity, true);
    let mut tp = Transport::new(cfg);
    let mgr = tp.iface_manager();

    let report = match spawn_all(mgr, &ifaces).await {
        Ok(r) => r,
        Err(e) => {
            let _ = ready_tx.send(Err(anyhow::anyhow!("{e:#}")));
            return Err(e);
        }
    };

    // Allow accept/connect to settle briefly.
    tokio::time::sleep(Duration::from_millis(150)).await;

    let dest = tp
        .add_destination(identity.clone(), DestinationName::new("mycelis", "v1"))
        .await;

    let mut ann_rx = tp.recv_announces().await;
    let mut data_rx = tp.received_data_events();

    let _ = ready_tx.send(Ok(report));

    loop {
        tokio::select! {
            cmd = cmd_rx.recv() => {
                match cmd {
                    Some(Cmd::Deliver { envelope }) => {
                        tp.send_announce(&dest, Some(&envelope)).await;
                    }
                    None => break,
                }
            }
            ev = ann_rx.recv() => {
                match ev {
                    Ok(announce) => {
                        let app = announce.app_data.as_slice();
                        push_if_envelope(&inbox, node_id, app);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            ev = data_rx.recv() => {
                match ev {
                    Ok(data) => {
                        push_if_envelope(&inbox, node_id, data.data.as_slice());
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
    Ok(())
}

fn push_if_envelope(inbox: &Arc<Mutex<VecDeque<Incoming>>>, self_id: NodeId, bytes: &[u8]) {
    let Some((from, to, payload)) = decode_envelope(bytes) else {
        return;
    };
    if from == self_id {
        return;
    }
    if !directed_for_us(to, self_id) {
        return;
    }
    let mut v = heapless::Vec::new();
    if v.extend_from_slice(payload).is_err() {
        return;
    }
    if let Ok(mut q) = inbox.lock() {
        q.push_back(Incoming {
            from,
            payload: v,
        });
    }
}

/// Helper for tests / examples: wait until inbox has a message or timeout.
pub async fn wait_recv(transport: &mut RnsTransport, timeout: Duration) -> Option<Incoming> {
    let deadline = tokio::time::Instant::now() + timeout;
    loop {
        if let Ok(Some(msg)) = transport.poll_recv() {
            return Some(msg);
        }
        if tokio::time::Instant::now() >= deadline {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
