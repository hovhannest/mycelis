//! Mycelia protocol core — `no_std` capable.
//!
//! Wire format: little-endian, version byte prefix. See `docs/wire-format.md`.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::module_name_repetitions)]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod domain;
pub mod ids;
pub mod message;
pub mod peer;
pub mod pow;
pub mod registry;
pub mod transport;
pub mod wire;

#[cfg(feature = "crypto")]
pub mod attestation;

pub use ids::{CommunityId, DomainId, NodeId, ServiceId, HASH_LEN};
pub use message::{ControlMessage, MAX_FRAME_PAYLOAD};
pub use peer::{PeerCache, PeerInfo, MAX_PEX_ENTRIES};
pub use registry::{ServiceRecord, Visibility};
pub use transport::{Incoming, ReticulumTransport, TransportError};

#[cfg(feature = "crypto")]
pub use attestation::{Attestation, AttestationError, Capability, Scope};
