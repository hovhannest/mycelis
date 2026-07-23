//! Leaf profile entry — must not depend on tokio/libp2p (see `scripts/check-leaf-deps.sh`).
//!
//! MCU `no_std` bring-up targets `mycelia-core` with `--no-default-features --features crypto,alloc`
//! plus FreeTAKTeam `rns-embedded-*` (Phase 5.3 / gate G1 hardware).

use mycelia_core::ids::NodeId;
use mycelia_core::peer::{PeerCache, PeerInfo};

/// Minimal leaf state.
pub struct LeafNode {
    pub id: NodeId,
    pub peers: PeerCache<16>,
}

impl LeafNode {
    pub fn new(id: NodeId) -> Self {
        Self {
            id,
            peers: PeerCache::new(3_600),
        }
    }

    pub fn note_peer(&mut self, peer: NodeId, now: u64) {
        self.peers.upsert(PeerInfo {
            node_id: peer,
            last_seen: now,
            interface_hint: heapless::Vec::new(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaf_peer_cache() {
        let mut leaf = LeafNode::new(NodeId::new([1u8; 16]));
        leaf.note_peer(NodeId::new([2u8; 16]), 10);
        assert_eq!(leaf.peers.len(), 1);
    }
}
