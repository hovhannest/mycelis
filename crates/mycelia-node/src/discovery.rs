//! Discovery providers: peer cache / PEX, static peers, mDNS, optional DHT.

use crate::config::NodeConfig;
use crate::state::NodeState;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use mycelia_core::ids::NodeId;
use mycelia_core::message::ControlMessage;
use mycelia_core::peer::PeerInfo;
use mycelia_core::pow::{mine_pow, verify_pow, PowStamp};
use mycelia_core::transport::ReticulumTransport;
use mycelia_core::wire::{Decoder, Encoder};
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
        let mut st = state.lock().await;
        let now = st.now();
        for addr in peers {
            let mut hint = heapless::Vec::new();
            let s = addr.to_string();
            let _ = hint.extend_from_slice(s.as_bytes());
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
        let _ = st.persist();
    }
}

/// Prepend a PoW stamp to a control frame (used for ServiceAnnounce).
/// Wire: `MPW1` + PowStamp + frame.
pub const POW_WRAP_MAGIC: &[u8; 4] = b"MPW1";

pub fn wrap_pow(frame: &[u8], difficulty: u8) -> Vec<u8> {
    if difficulty == 0 {
        return frame.to_vec();
    }
    let stamp = mine_pow(frame, difficulty);
    let mut hdr = [0u8; 16];
    let hdr_len = {
        let mut enc = Encoder::new(&mut hdr);
        stamp.encode(&mut enc).expect("pow header fits");
        enc.position()
    };
    let mut out = Vec::with_capacity(4 + hdr_len + frame.len());
    out.extend_from_slice(POW_WRAP_MAGIC);
    out.extend_from_slice(&hdr[..hdr_len]);
    out.extend_from_slice(frame);
    out
}

fn try_split_pow(bytes: &[u8]) -> Option<(PowStamp, &[u8])> {
    if bytes.len() < 4 + 10 || &bytes[..4] != POW_WRAP_MAGIC {
        return None;
    }
    let mut dec = Decoder::new(&bytes[4..]);
    let stamp = PowStamp::decode(&mut dec).ok()?;
    let rest = &bytes[4 + dec.position()..];
    if rest.is_empty() {
        return None;
    }
    Some((stamp, rest))
}

/// Process an incoming control frame into node state.
pub fn handle_control_bytes(state: &mut NodeState, from: NodeId, bytes: &[u8]) {
    // Prefer PoW-wrapped frames; fall back to raw for non-announce traffic.
    let (frame, pow_ok) = if let Some((stamp, rest)) = try_split_pow(bytes) {
        let ok = stamp.difficulty >= state.pow_difficulty && verify_pow(rest, &stamp);
        (rest, ok)
    } else {
        (bytes, false)
    };

    let Ok(msg) = ControlMessage::decode_frame(frame) else {
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
            let _ = state.persist();
        }
        ControlMessage::ServiceAnnounce { record } => {
            if state.pow_difficulty > 0 && !pow_ok {
                tracing::debug!("rejecting ServiceAnnounce without valid PoW");
                return;
            }
            state.registry.upsert(record);
            state.peer_cache.upsert(PeerInfo {
                node_id: from,
                last_seen: state.now(),
                interface_hint: heapless::Vec::new(),
            });
            let _ = state.persist();
        }
        ControlMessage::PeerExchange { peers } => {
            state.peer_cache.merge_pex(&peers);
            state.peer_cache.upsert(PeerInfo {
                node_id: from,
                last_seen: state.now(),
                interface_hint: heapless::Vec::new(),
            });
            let _ = state.persist();
        }
        ControlMessage::ServiceQuery { from: qfrom, .. } => {
            let views = state.attestation_views();
            let visible = state.registry.list_visible(qfrom, &views, state.now());
            let mut records = heapless::Vec::new();
            for r in visible.into_iter().take(4) {
                let _ = records.push(r.clone());
            }
            let _ = ControlMessage::ServiceQueryResponse { records };
        }
        ControlMessage::ServiceQueryResponse { records } => {
            for r in records {
                state.registry.upsert(r);
            }
            let _ = state.persist();
        }
        ControlMessage::PolicyUpdate { .. } => {}
    }
}

/// DHT publish hook — active with `discovery-dht` feature.
pub async fn maybe_publish_dht(rns_dest_hex: &str, hint: &[u8]) {
    #[cfg(feature = "discovery-dht")]
    {
        tracing::debug!(
            "DHT publish {rns_dest_hex} hint_len={}",
            hint.len()
        );
        // Actual LocatorNode publish is driven by `spawn_dht_locator` in runtime.
        let _ = (rns_dest_hex, hint);
    }
    #[cfg(not(feature = "discovery-dht"))]
    {
        let _ = (rns_dest_hex, hint);
    }
}

/// Spawn disposable Internet locator; publish listen hint; query known peers.
#[cfg(feature = "discovery-dht")]
pub async fn spawn_dht_locator(
    node_id: NodeId,
    listen: SocketAddr,
    peer_keys: Vec<String>,
) -> anyhow::Result<()> {
    use mycelia_dht::LocatorNode;
    use std::time::Duration;

    let mut locator = LocatorNode::new()?;
    locator.listen_local_ephemeral()?;
    let _ = locator.wait_listening(Duration::from_secs(3)).await?;

    let key = hex::encode(node_id.as_bytes());
    let hint = format!("tcp:{listen}").into_bytes();
    locator.put_local(&key, hint.clone())?;
    let _ = locator.put_record(&key, hint.clone());
    maybe_publish_dht(&key, &hint).await;

    for peer_hex in &peer_keys {
        match locator.get_local(peer_hex) {
            Ok(val) => {
                tracing::info!(
                    "DHT cold-start hit for {peer_hex}: {}",
                    String::from_utf8_lossy(&val)
                );
            }
            Err(_) => {
                locator.get(peer_hex);
            }
        }
    }

    tokio::spawn(async move {
        locator.run_forever().await;
    });
    Ok(())
}
