//! Peer cache and peer-exchange structures.

use crate::ids::NodeId;
use crate::wire::{DecodeError, Decoder, Encoder};

/// Default max PEX entries on leaf / constrained nodes.
pub const MAX_PEX_ENTRIES: usize = 32;
pub const MAX_INTERFACE_HINT: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerInfo {
    pub node_id: NodeId,
    pub last_seen: u64,
    pub interface_hint: heapless::Vec<u8, MAX_INTERFACE_HINT>,
}

impl PeerInfo {
    pub fn encode(&self, enc: &mut Encoder<'_>) -> Result<(), DecodeError> {
        self.node_id.encode(enc)?;
        enc.write_u64(self.last_seen)?;
        enc.write_blob(&self.interface_hint)?;
        Ok(())
    }

    pub fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let node_id = NodeId::decode(dec)?;
        let last_seen = dec.read_u64()?;
        let hint = dec.read_blob()?;
        let mut interface_hint = heapless::Vec::new();
        interface_hint
            .extend_from_slice(hint)
            .map_err(|_| DecodeError::Overflow)?;
        Ok(Self {
            node_id,
            last_seen,
            interface_hint,
        })
    }
}

#[derive(Debug, Clone)]
pub struct PeerCache<const N: usize = MAX_PEX_ENTRIES> {
    peers: heapless::Vec<PeerInfo, N>,
    ttl_secs: u64,
}

impl<const N: usize> PeerCache<N> {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            peers: heapless::Vec::new(),
            ttl_secs,
        }
    }

    pub fn len(&self) -> usize {
        self.peers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.peers.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &PeerInfo> {
        self.peers.iter()
    }

    pub fn upsert(&mut self, peer: PeerInfo) {
        if let Some(existing) = self.peers.iter_mut().find(|p| p.node_id == peer.node_id) {
            if peer.last_seen >= existing.last_seen {
                *existing = peer;
            }
            return;
        }
        if self.peers.len() < N {
            let _ = self.peers.push(peer);
        } else {
            // Replace oldest.
            if let Some((idx, _)) = self
                .peers
                .iter()
                .enumerate()
                .min_by_key(|(_, p)| p.last_seen)
            {
                self.peers[idx] = peer;
            }
        }
    }

    pub fn merge_pex(&mut self, incoming: &[PeerInfo]) {
        for p in incoming {
            self.upsert(p.clone());
        }
    }

    pub fn expire(&mut self, now: u64) {
        let ttl = self.ttl_secs;
        self.peers
            .retain(|p| now.saturating_sub(p.last_seen) <= ttl);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_dedup_expire() {
        let mut cache = PeerCache::<8>::new(100);
        let a = PeerInfo {
            node_id: NodeId::new([1u8; 16]),
            last_seen: 50,
            interface_hint: heapless::Vec::new(),
        };
        let a2 = PeerInfo {
            node_id: NodeId::new([1u8; 16]),
            last_seen: 80,
            interface_hint: heapless::Vec::new(),
        };
        let b = PeerInfo {
            node_id: NodeId::new([2u8; 16]),
            last_seen: 10,
            interface_hint: heapless::Vec::new(),
        };
        cache.upsert(a);
        cache.merge_pex(&[a2, b]);
        assert_eq!(cache.len(), 2);
        assert_eq!(
            cache
                .iter()
                .find(|p| p.node_id.0[0] == 1)
                .unwrap()
                .last_seen,
            80
        );
        cache.expire(200);
        assert_eq!(cache.len(), 0);
    }
}
