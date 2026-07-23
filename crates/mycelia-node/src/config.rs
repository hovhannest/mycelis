use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub data_dir: PathBuf,
    pub listen: SocketAddr,
    pub control_bind: SocketAddr,
    pub static_peers: Vec<SocketAddr>,
    pub enable_mdns: bool,
    pub enable_dht: bool,
    pub enable_gateway: bool,
    pub pow_difficulty: u8,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from(".mycelis"),
            listen: "127.0.0.1:0".parse().unwrap(),
            control_bind: "127.0.0.1:0".parse().unwrap(),
            static_peers: vec![],
            enable_mdns: true,
            enable_dht: false,
            enable_gateway: false,
            pow_difficulty: 8,
        }
    }
}

impl NodeConfig {
    pub fn from_toml_str(s: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(s)
    }

    pub fn load_or_default(path: &std::path::Path) -> anyhow::Result<Self> {
        if path.exists() {
            let s = std::fs::read_to_string(path)?;
            Ok(Self::from_toml_str(&s)?)
        } else {
            Ok(Self::default())
        }
    }
}
