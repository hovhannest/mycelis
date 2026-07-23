use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;

/// One configured Reticulum interface (reticulumd-aligned `type` tag).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RnsInterfaceConfig {
    TcpServer {
        /// `host:port` or `ip:port` bind address.
        #[serde(alias = "listen")]
        bind: String,
        #[serde(default)]
        name: Option<String>,
    },
    TcpClient {
        #[serde(alias = "target")]
        target: String,
        #[serde(default)]
        name: Option<String>,
    },
    Udp {
        bind: String,
        #[serde(default)]
        forward: Option<String>,
        #[serde(default)]
        name: Option<String>,
    },
    Serial {
        device: String,
        #[serde(default = "default_serial_baud")]
        baud: u32,
        #[serde(default)]
        name: Option<String>,
    },
    Kiss {
        device: String,
        #[serde(default = "default_kiss_baud")]
        baud: u32,
        #[serde(default)]
        name: Option<String>,
    },
    KissTcpClient {
        target: String,
        #[serde(default)]
        name: Option<String>,
    },
    Lora {
        /// Serial device path, or `tcp://host:port`.
        device: String,
        #[serde(default = "default_serial_baud")]
        baud: u32,
        #[serde(default = "default_lora_region")]
        region: String,
        #[serde(default)]
        name: Option<String>,
    },
    RnodeMulti {
        device: String,
        #[serde(default)]
        name: Option<String>,
    },
    Pipe {
        command: String,
        #[serde(default)]
        name: Option<String>,
    },
    I2p {
        #[serde(default = "default_i2p_name")]
        name: String,
        #[serde(default = "default_sam")]
        sam: String,
        #[serde(default)]
        peers: Vec<String>,
        #[serde(default)]
        connectable: bool,
    },
    Weave {
        device: String,
        #[serde(default)]
        name: Option<String>,
    },
    Meshtastic {
        #[serde(default = "default_mesh_name")]
        name: String,
    },
    /// Unix shared-instance listener (`cfg(unix)` at spawn time).
    Local {
        #[serde(default = "default_local_sock")]
        path: String,
        #[serde(default)]
        name: Option<String>,
    },
    LocalClient {
        #[serde(default = "default_local_sock")]
        path: String,
        #[serde(default)]
        name: Option<String>,
    },
    /// Requires cargo feature `iface-ble`.
    ReticulumBle {
        peripheral_id: String,
        #[serde(default)]
        name: Option<String>,
    },
    /// Requires cargo feature `iface-ble`.
    Vrn76KissBle {
        peripheral_id: String,
        #[serde(default)]
        name: Option<String>,
    },
}

fn default_serial_baud() -> u32 {
    115_200
}
fn default_kiss_baud() -> u32 {
    9_600
}
fn default_lora_region() -> String {
    "US915".into()
}
fn default_i2p_name() -> String {
    "i2p".into()
}
fn default_sam() -> String {
    "127.0.0.1:7656".into()
}
fn default_mesh_name() -> String {
    "mesh0".into()
}
fn default_local_sock() -> String {
    "/tmp/mycelis-rns.sock".into()
}
fn default_true() -> bool {
    true
}

/// TOML `[[interfaces]]` row with optional `enabled` flag.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RnsInterfaceEntry {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(flatten)]
    pub iface: RnsInterfaceConfig,
}

impl RnsInterfaceEntry {
    pub fn new(iface: RnsInterfaceConfig) -> Self {
        Self {
            enabled: true,
            iface,
        }
    }
}

impl RnsInterfaceConfig {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::TcpServer { .. } => "tcp_server",
            Self::TcpClient { .. } => "tcp_client",
            Self::Udp { .. } => "udp",
            Self::Serial { .. } => "serial",
            Self::Kiss { .. } => "kiss",
            Self::KissTcpClient { .. } => "kiss_tcp_client",
            Self::Lora { .. } => "lora",
            Self::RnodeMulti { .. } => "rnode_multi",
            Self::Pipe { .. } => "pipe",
            Self::I2p { .. } => "i2p",
            Self::Weave { .. } => "weave",
            Self::Meshtastic { .. } => "meshtastic",
            Self::Local { .. } => "local",
            Self::LocalClient { .. } => "local_client",
            Self::ReticulumBle { .. } => "reticulum_ble",
            Self::Vrn76KissBle { .. } => "vrn76_kiss_ble",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub data_dir: PathBuf,
    pub listen: SocketAddr,
    pub control_bind: SocketAddr,
    pub static_peers: Vec<SocketAddr>,
    #[serde(default = "default_transport")]
    pub transport: String,
    /// Explicit RNS interfaces. When empty, synthesized from `listen` + `static_peers`.
    #[serde(default)]
    pub interfaces: Vec<RnsInterfaceEntry>,
    pub enable_mdns: bool,
    pub enable_dht: bool,
    pub enable_gateway: bool,
    #[serde(default = "default_gateway_bind")]
    pub gateway_bind: SocketAddr,
    pub pow_difficulty: u8,
}

fn default_transport() -> String {
    "rns".into()
}

fn default_gateway_bind() -> SocketAddr {
    "127.0.0.1:1080".parse().unwrap()
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from(".mycelis"),
            listen: "127.0.0.1:0".parse().unwrap(),
            control_bind: "127.0.0.1:0".parse().unwrap(),
            static_peers: vec![],
            transport: default_transport(),
            interfaces: vec![],
            enable_mdns: true,
            enable_dht: false,
            enable_gateway: false,
            gateway_bind: default_gateway_bind(),
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

    /// Apply `MYCELIS_TRANSPORT` env override when set.
    pub fn apply_env_overrides(&mut self) {
        if let Ok(v) = std::env::var("MYCELIS_TRANSPORT") {
            if !v.is_empty() {
                self.transport = v;
            }
        }
    }

    pub fn wants_mock_transport(&self) -> bool {
        self.transport.eq_ignore_ascii_case("mock")
    }

    /// Enabled interfaces, or TCP synthesized from `listen` / `static_peers`.
    pub fn effective_interfaces(&self) -> Vec<RnsInterfaceConfig> {
        let enabled: Vec<RnsInterfaceConfig> = self
            .interfaces
            .iter()
            .filter(|e| e.enabled)
            .map(|e| e.iface.clone())
            .collect();
        if !enabled.is_empty() {
            return enabled;
        }
        let mut out = Vec::new();
        out.push(RnsInterfaceConfig::TcpServer {
            bind: self.listen.to_string(),
            name: Some("listen".into()),
        });
        for peer in &self.static_peers {
            out.push(RnsInterfaceConfig::TcpClient {
                target: peer.to_string(),
                name: None,
            });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthesize_tcp_when_interfaces_empty() {
        let cfg = NodeConfig {
            listen: "127.0.0.1:4242".parse().unwrap(),
            static_peers: vec!["10.0.0.2:4242".parse().unwrap()],
            ..Default::default()
        };
        let ifaces = cfg.effective_interfaces();
        assert_eq!(ifaces.len(), 2);
        assert!(matches!(ifaces[0], RnsInterfaceConfig::TcpServer { .. }));
        assert!(matches!(ifaces[1], RnsInterfaceConfig::TcpClient { .. }));
    }

    #[test]
    fn deserialize_interface_variants() {
        let toml = r#"
data_dir = ".mycelis"
listen = "127.0.0.1:0"
control_bind = "127.0.0.1:0"
static_peers = []
enable_mdns = false
enable_dht = false
enable_gateway = false
pow_difficulty = 8

[[interfaces]]
type = "tcp_server"
bind = "0.0.0.0:4242"

[[interfaces]]
type = "udp"
bind = "0.0.0.0:4243"
forward = "192.168.1.10:4243"

[[interfaces]]
enabled = false
type = "serial"
device = "COM3"
baud = 115200

[[interfaces]]
type = "pipe"
command = "cat"
"#;
        let cfg = NodeConfig::from_toml_str(toml).unwrap();
        assert_eq!(cfg.interfaces.len(), 4);
        let effective = cfg.effective_interfaces();
        assert_eq!(effective.len(), 3); // serial disabled
        assert_eq!(effective[0].kind_name(), "tcp_server");
        assert_eq!(effective[1].kind_name(), "udp");
        assert_eq!(effective[2].kind_name(), "pipe");
    }
}
