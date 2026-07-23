//! Mycelia node runtime — std profile.

pub mod config;
pub mod control;
pub mod discovery;
pub mod mock_transport;
pub mod runtime;
pub mod state;

#[cfg(feature = "transport-rns")]
pub mod rns_ifaces;
#[cfg(feature = "transport-rns")]
pub mod rns_transport;

pub use config::{NodeConfig, RnsInterfaceConfig, RnsInterfaceEntry};
pub use runtime::{NodeHandle, NodeRuntime};
pub use state::NodeState;
