//! In-process mock transport for tests and substrate spike without full RNS daemon.

use mycelia_core::ids::NodeId;
use mycelia_core::transport::{Incoming, ReticulumTransport, TransportError};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
struct HubInner {
    /// dest -> queue
    queues: std::collections::HashMap<[u8; 16], VecDeque<(NodeId, Vec<u8>)>>,
    announces: Vec<(NodeId, Vec<u8>)>,
}

/// Shared hub connecting mock transports (simulates mesh).
#[derive(Clone, Default)]
pub struct MockHub {
    inner: Arc<Mutex<HubInner>>,
}

impl MockHub {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn take_announces(&self) -> Vec<(NodeId, Vec<u8>)> {
        let mut g = self.inner.lock().unwrap();
        std::mem::take(&mut g.announces)
    }
}

pub struct MockTransport {
    id: NodeId,
    hub: MockHub,
}

impl MockTransport {
    pub fn new(id: NodeId, hub: MockHub) -> Self {
        {
            let mut g = hub.inner.lock().unwrap();
            g.queues.entry(id.0).or_default();
        }
        Self { id, hub }
    }
}

impl ReticulumTransport for MockTransport {
    fn identity(&self) -> NodeId {
        self.id
    }

    fn send(&mut self, dest: &NodeId, bytes: &[u8]) -> Result<(), TransportError> {
        let mut g = self.hub.inner.lock().unwrap();
        let q = g.queues.entry(dest.0).or_default();
        q.push_back((self.id, bytes.to_vec()));
        Ok(())
    }

    fn poll_recv(&mut self) -> Result<Option<Incoming>, TransportError> {
        let mut g = self.hub.inner.lock().unwrap();
        let q = g.queues.entry(self.id.0).or_default();
        if let Some((from, payload)) = q.pop_front() {
            let mut v = heapless::Vec::new();
            v.extend_from_slice(&payload)
                .map_err(|_| TransportError::BufferTooSmall)?;
            Ok(Some(Incoming { from, payload: v }))
        } else {
            Ok(None)
        }
    }

    fn announce(&mut self, app_data: &[u8]) -> Result<(), TransportError> {
        let mut g = self.hub.inner.lock().unwrap();
        g.announces.push((self.id, app_data.to_vec()));
        // Flood announce payload to all other queues as a broadcast message.
        let others: Vec<[u8; 16]> = g
            .queues
            .keys()
            .copied()
            .filter(|k| *k != self.id.0)
            .collect();
        for k in others {
            g.queues
                .get_mut(&k)
                .unwrap()
                .push_back((self.id, app_data.to_vec()));
        }
        Ok(())
    }
}
