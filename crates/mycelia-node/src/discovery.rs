//! Discovery providers: peer cache / PEX, static peers, mDNS, optional DHT.

use crate::config::NodeConfig;
use crate::state::NodeState;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use mycelia_core::ids::NodeId;
use mycelia_core::message::ControlMessage;
use mycelia_core::peer::PeerInfo;
use mycelia_core::transport::ReticulumTransport;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

pub const MDNS_SERVICE_TYPE: &str = "_mycelis._udp.local.";

pub struct DiscoveryManager {
    pub mdns: Option<ServiceDaemon>,
}

impl DiscoveryManager {
    pub fn new(cfg: &NodeConfig) -> anyhow::Result<Self> {
        let mdns = if cfg.enable_mdns {
            Some(ServiceDaemon::new()?)
        } else {
            None
        };
        Ok(Self { mdns })
    }

    pub fn register_mdns(&self, node_id: NodeId, listen: SocketAddr) -> anyhow::Result<()> {
        let Some(mdns) = &self.mdns else {
            return Ok(());
        };
        let instance = hex::encode(node_id.as_bytes());
        let host = format!("{instance}.local.");
        let props = [
            ("nid", instance.as_str()),
            ("port", &listen.port().to_string()),
        ];
        let info = ServiceInfo::new(
            MDNS_SERVICE_TYPE,
            &instance,
            &host,
            listen.ip(),
            listen.port(),
            &props[..],
        )?;
        mdns.register(info)?;
        Ok(())
    }

    pub async fn dial_static_peers(
        state: Arc<Mutex<NodeState>>,
        transport: Arc<Mutex<dyn ReticulumTransport + Send>>,
        peers: &[SocketAddr],
    ) {
        // Static peers in MVP mock mode: encode address into PeerInfo hints and PEX announce.
        let mut st = state.lock().await;
        let now = st.now();
        for addr in peers {
            let mut hint = heapless::Vec::new();
            let s = addr.to_string();
            let _ = hint.extend_from_slice(s.as_bytes());
            // Synthetic peer id from address hash
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(s.as_bytes());
            let d = h.finalize();
            let mut id = [0u8; 16];
            id.copy_from_slice(&d[..16]);
            st.peer_cache.upsert(PeerInfo {
                node_id: NodeId::new(id),
                last_seen: now,
                interface_hint: hint,
            });
        }
        let mut peers_vec = heapless::Vec::new();
        for p in st.peer_cache.iter().take(32) {
            let _ = peers_vec.push(p.clone());
        }
        let msg = ControlMessage::PeerExchange { peers: peers_vec };
        let mut buf = [0u8; 1024];
        if let Ok(n) = msg.encode_frame(&mut buf) {
            let mut t = transport.lock().await;
            let _ = t.announce(&buf[..n]);
        }
    }
}

/// Process an incoming control frame into node state.
pub fn handle_control_bytes(state: &mut NodeState, from: NodeId, bytes: &[u8]) {
    let Ok(msg) = ControlMessage::decode_frame(bytes) else {
        return;
    };
    match msg {
        ControlMessage::Invite { attestation, .. }
        | ControlMessage::AttestationAnnounce { attestation } => {
            state.accept_attestation(attestation);
            state.peer_cache.upsert(PeerInfo {
                node_id: from,
                last_seen: state.now(),
                interface_hint: heapless::Vec::new(),
            });
        }
        ControlMessage::ServiceAnnounce { record } => {
            state.registry.upsert(record);
            state.peer_cache.upsert(PeerInfo {
                node_id: from,
                last_seen: state.now(),
                interface_hint: heapless::Vec::new(),
            });
        }
        ControlMessage::PeerExchange { peers } => {
            state.peer_cache.merge_pex(&peers);
            state.peer_cache.upsert(PeerInfo {
                node_id: from,
                last_seen: state.now(),
                interface_hint: heapless::Vec::new(),
            });
        }
        ControlMessage::ServiceQuery { from: qfrom, .. } => {
            let views = state.attestation_views();
            let visible = state.registry.list_visible(qfrom, &views, state.now());
            let mut records = heapless::Vec::new();
            for r in visible.into_iter().take(4) {
                let _ = records.push(r.clone());
            }
            // Response is sent by runtime if needed; stash not required for in-process hub floods.
            let _ = ControlMessage::ServiceQueryResponse { records };
        }
        ControlMessage::ServiceQueryResponse { records } => {
            for r in records {
                state.registry.upsert(r);
            }
        }
        ControlMessage::PolicyUpdate { .. } => {}
    }
}

/// DHT provider hook (Task 4.2). Active only with `discovery-dht` feature.
pub async fn maybe_publish_dht(rns_dest_hex: &str, hint: &[u8]) {
    #[cfg(feature = "discovery-dht")]
    {
        let _ = (rns_dest_hex, hint);
        tracing::debug!("DHT publish requested for {rns_dest_hex}");
    }
    #[cfg(not(feature = "discovery-dht"))]
    {
        let _ = (rns_dest_hex, hint);
    }
}
