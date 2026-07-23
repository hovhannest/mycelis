//! Disposable Internet locator: maps Mycelia/RNS destination → reachability hints.
//!
//! Key namespace: `/mycelis/v1/peer/<rns_dest_hex>`
//! PeerID is transport plumbing; RNS identity in the record value is authoritative.

use futures::StreamExt;
use libp2p::kad::{
    store::{MemoryStore, RecordStore},
    Behaviour as KadBehaviour, Quorum, Record, RecordKey,
};
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{identity, noise, tcp, yamux, Multiaddr, PeerId, Swarm, SwarmBuilder};
use std::time::Duration;
use thiserror::Error;

pub const KEY_PREFIX: &str = "/mycelis/v1/peer/";

#[derive(Debug, Error)]
pub enum DhtError {
    #[error("swarm: {0}")]
    Swarm(String),
    #[error("timeout")]
    Timeout,
    #[error("not found")]
    NotFound,
}

#[derive(NetworkBehaviour)]
struct Behaviour {
    kad: KadBehaviour<MemoryStore>,
    identify: libp2p::identify::Behaviour,
}

pub struct LocatorNode {
    pub peer_id: PeerId,
    swarm: Swarm<Behaviour>,
}

impl LocatorNode {
    pub fn new() -> Result<Self, DhtError> {
        let local_key = identity::Keypair::generate_ed25519();
        let peer_id = local_key.public().to_peer_id();
        let store = MemoryStore::new(peer_id);
        let mut kad = KadBehaviour::new(peer_id, store);
        kad.set_mode(Some(libp2p::kad::Mode::Server));
        let identify = libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
            "/mycelis/1.0.0".into(),
            local_key.public(),
        ));
        let behaviour = Behaviour { kad, identify };
        let swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| DhtError::Swarm(e.to_string()))?
            .with_behaviour(|_| Ok(behaviour))
            .map_err(|e| DhtError::Swarm(e.to_string()))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(30)))
            .build();
        Ok(Self { peer_id, swarm })
    }

    pub fn listen_on(&mut self, addr: Multiaddr) -> Result<(), DhtError> {
        self.swarm
            .listen_on(addr)
            .map_err(|e| DhtError::Swarm(e.to_string()))?;
        Ok(())
    }

    /// Convenience: listen on `127.0.0.1` ephemeral TCP port.
    pub fn listen_local_ephemeral(&mut self) -> Result<(), DhtError> {
        use std::str::FromStr;
        self.listen_on(
            Multiaddr::from_str("/ip4/127.0.0.1/tcp/0")
                .map_err(|e| DhtError::Swarm(e.to_string()))?,
        )
    }

    pub fn dial(&mut self, addr: Multiaddr) -> Result<(), DhtError> {
        self.swarm
            .dial(addr)
            .map_err(|e| DhtError::Swarm(e.to_string()))?;
        Ok(())
    }

    pub fn record_key(rns_dest_hex: &str) -> RecordKey {
        RecordKey::new(&(format!("{KEY_PREFIX}{rns_dest_hex}")))
    }

    /// Store a record in the local MemoryStore (no network quorum).
    pub fn put_local(&mut self, rns_dest_hex: &str, value: Vec<u8>) -> Result<(), DhtError> {
        let key = Self::record_key(rns_dest_hex);
        let record = Record {
            key,
            value,
            publisher: Some(self.peer_id),
            expires: None,
        };
        self.swarm
            .behaviour_mut()
            .kad
            .store_mut()
            .put(record)
            .map_err(|e| DhtError::Swarm(e.to_string()))
    }

    /// Publish via Kad (requires peers for quorum); also stores locally.
    pub fn put_record(&mut self, rns_dest_hex: &str, value: Vec<u8>) -> Result<(), DhtError> {
        let key = Self::record_key(rns_dest_hex);
        let record = Record {
            key,
            value,
            publisher: Some(self.peer_id),
            expires: None,
        };
        self.swarm
            .behaviour_mut()
            .kad
            .put_record(record, Quorum::One)
            .map_err(|e| DhtError::Swarm(e.to_string()))?;
        Ok(())
    }

    pub fn get_local(&mut self, rns_dest_hex: &str) -> Result<Vec<u8>, DhtError> {
        let key = Self::record_key(rns_dest_hex);
        self.swarm
            .behaviour_mut()
            .kad
            .store_mut()
            .get(&key)
            .map(|r| r.value.clone())
            .ok_or(DhtError::NotFound)
    }

    pub fn get(&mut self, rns_dest_hex: &str) {
        let key = Self::record_key(rns_dest_hex);
        self.swarm.behaviour_mut().kad.get_record(key);
    }

    pub async fn drive_until_put_ok(&mut self, timeout: Duration) -> Result<(), DhtError> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if left.is_zero() {
                return Err(DhtError::Timeout);
            }
            match tokio::time::timeout(left, self.swarm.select_next_some()).await {
                Ok(SwarmEvent::Behaviour(BehaviourEvent::Kad(
                    libp2p::kad::Event::OutboundQueryProgressed { result, .. },
                ))) => {
                    if let libp2p::kad::QueryResult::PutRecord(Ok(_)) = result {
                        return Ok(());
                    }
                }
                Ok(_) => {}
                Err(_) => return Err(DhtError::Timeout),
            }
        }
    }

    pub async fn drive_until_get(&mut self, timeout: Duration) -> Result<Vec<u8>, DhtError> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if left.is_zero() {
                return Err(DhtError::Timeout);
            }
            match tokio::time::timeout(left, self.swarm.select_next_some()).await {
                Ok(SwarmEvent::Behaviour(BehaviourEvent::Kad(
                    libp2p::kad::Event::OutboundQueryProgressed { result, .. },
                ))) => {
                    if let libp2p::kad::QueryResult::GetRecord(Ok(ok)) = result {
                        use libp2p::kad::GetRecordOk;
                        if let GetRecordOk::FoundRecord(rec) = ok {
                            return Ok(rec.record.value);
                        }
                    }
                }
                Ok(_) => {}
                Err(_) => return Err(DhtError::Timeout),
            }
        }
    }

    /// Wait until the swarm reports at least one listen address.
    pub async fn wait_listening(&mut self, timeout: Duration) -> Result<Multiaddr, DhtError> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if left.is_zero() {
                return Err(DhtError::Timeout);
            }
            match tokio::time::timeout(left, self.swarm.select_next_some()).await {
                Ok(SwarmEvent::NewListenAddr { address, .. }) => return Ok(address),
                Ok(_) => {}
                Err(_) => return Err(DhtError::Timeout),
            }
        }
    }

    /// Background event pump (keeps Kad alive).
    pub async fn run_forever(mut self) {
        loop {
            let _ = self.swarm.select_next_some().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn local_put_get() {
        let mut a = LocatorNode::new().unwrap();
        a.listen_on(Multiaddr::from_str("/ip4/127.0.0.1/tcp/0").unwrap())
            .unwrap();
        // Wait for listen addr
        loop {
            if let SwarmEvent::NewListenAddr { address, .. } = a.swarm.select_next_some().await {
                let _ = address;
                break;
            }
        }
        let dest = "00112233445566778899aabbccddeeff";
        a.put_local(dest, b"tcp:127.0.0.1:1234".to_vec()).unwrap();
        let val = a.get_local(dest).unwrap();
        assert_eq!(val, b"tcp:127.0.0.1:1234");
        assert_eq!(
            LocatorNode::record_key(dest).to_vec(),
            format!("{KEY_PREFIX}{dest}").into_bytes()
        );
    }
}
