//! Substrate-agnostic transport trait (tech-stack §4.3).

use crate::ids::NodeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    NotConnected,
    BufferTooSmall,
    SendFailed,
    RecvFailed,
    Unsupported,
}

#[cfg(feature = "std")]
impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConnected => write!(f, "not connected"),
            Self::BufferTooSmall => write!(f, "buffer too small"),
            Self::SendFailed => write!(f, "send failed"),
            Self::RecvFailed => write!(f, "recv failed"),
            Self::Unsupported => write!(f, "unsupported"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TransportError {}

#[derive(Debug, Clone)]
pub struct Incoming {
    pub from: NodeId,
    pub payload: heapless::Vec<u8, 1024>,
}

/// Sync-friendly trait usable on leaf (poll style) and node adapters.
pub trait ReticulumTransport {
    fn identity(&self) -> NodeId;

    fn send(&mut self, dest: &NodeId, bytes: &[u8]) -> Result<(), TransportError>;

    fn poll_recv(&mut self) -> Result<Option<Incoming>, TransportError>;

    fn announce(&mut self, app_data: &[u8]) -> Result<(), TransportError>;
}
