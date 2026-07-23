//! Spawn FreeTAKTeam Reticulum interfaces from [`RnsInterfaceConfig`].

use crate::config::RnsInterfaceConfig;
use reticulum_rs::iface::i2p::I2pInterface;
use reticulum_rs::iface::kiss::{KissInterface, KissTcpClientInterface};
use reticulum_rs::iface::lora::{LoraConfig, LoraInterface};
use reticulum_rs::iface::meshtastic::{spawn_meshtastic, MeshtasticInterfaceConfig};
use reticulum_rs::iface::pipe::PipeInterface;
use reticulum_rs::iface::rnode_multi::RNodeMultiInterface;
use reticulum_rs::iface::serial::SerialInterface;
use reticulum_rs::iface::tcp_client::TcpClient;
use reticulum_rs::iface::tcp_server::TcpServer;
use reticulum_rs::iface::udp::UdpInterface;
use reticulum_rs::iface::weave::WeaveInterface;
use reticulum_rs::iface::InterfaceManager;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Result of attaching configured interfaces.
#[derive(Debug, Clone)]
pub struct SpawnReport {
    /// Primary TCP listen address when a `tcp_server` was configured (resolved if port was 0).
    pub listen_addr: Option<SocketAddr>,
    pub kinds: Vec<&'static str>,
}

/// Attach every configured interface to `mgr`.
pub async fn spawn_all(
    mgr: Arc<Mutex<InterfaceManager>>,
    ifaces: &[RnsInterfaceConfig],
) -> anyhow::Result<SpawnReport> {
    let mut listen_addr: Option<SocketAddr> = None;
    let mut kinds = Vec::new();

    for iface in ifaces {
        kinds.push(iface.kind_name());
        match iface {
            RnsInterfaceConfig::TcpServer { bind, .. } => {
                let addr = resolve_tcp_bind(bind)?;
                if listen_addr.is_none() {
                    listen_addr = Some(addr);
                }
                mgr.lock()
                    .await
                    .spawn(TcpServer::new(addr.to_string(), mgr.clone()), TcpServer::spawn);
            }
            RnsInterfaceConfig::TcpClient { target, .. } => {
                mgr.lock()
                    .await
                    .spawn(TcpClient::new(target.clone()), TcpClient::spawn);
            }
            RnsInterfaceConfig::Udp { bind, forward, .. } => {
                let bind_addr = resolve_udp_bind(bind)?;
                mgr.lock().await.spawn(
                    UdpInterface::new(bind_addr.to_string(), forward.clone()),
                    UdpInterface::spawn,
                );
            }
            RnsInterfaceConfig::Serial { device, baud, .. } => {
                mgr.lock()
                    .await
                    .spawn(SerialInterface::new(device.clone(), *baud), SerialInterface::spawn);
            }
            RnsInterfaceConfig::Kiss { device, baud, .. } => {
                mgr.lock()
                    .await
                    .spawn(KissInterface::new(device.clone(), *baud), KissInterface::spawn);
            }
            RnsInterfaceConfig::KissTcpClient { target, .. } => {
                mgr.lock().await.spawn(
                    KissTcpClientInterface::new(target.clone()),
                    KissTcpClientInterface::spawn,
                );
            }
            RnsInterfaceConfig::Lora {
                device,
                baud,
                region,
                ..
            } => {
                let cfg = lora_config_for_region(region)?;
                if let Some(rest) = device.strip_prefix("tcp://") {
                    mgr.lock()
                        .await
                        .spawn(LoraInterface::new_tcp(rest.to_string(), cfg), LoraInterface::spawn);
                } else {
                    mgr.lock().await.spawn(
                        LoraInterface::new(device.clone(), *baud, cfg),
                        LoraInterface::spawn,
                    );
                }
            }
            RnsInterfaceConfig::RnodeMulti { device, .. } => {
                if let Some(rest) = device.strip_prefix("tcp://") {
                    mgr.lock().await.spawn(
                        RNodeMultiInterface::new_tcp(rest.to_string(), mgr.clone()),
                        RNodeMultiInterface::spawn,
                    );
                } else {
                    mgr.lock().await.spawn(
                        RNodeMultiInterface::new(device.clone(), mgr.clone()),
                        RNodeMultiInterface::spawn,
                    );
                }
            }
            RnsInterfaceConfig::Pipe { command, .. } => {
                mgr.lock()
                    .await
                    .spawn(PipeInterface::new(command.clone()), PipeInterface::spawn);
            }
            RnsInterfaceConfig::I2p {
                name,
                sam,
                peers,
                connectable,
            } => {
                mgr.lock().await.spawn(
                    I2pInterface::new(name.clone(), mgr.clone())
                        .with_sam_endpoint(sam.clone())
                        .with_peers(peers.clone())
                        .with_connectable(*connectable),
                    I2pInterface::spawn,
                );
            }
            RnsInterfaceConfig::Weave { device, .. } => {
                mgr.lock().await.spawn(
                    WeaveInterface::new(device.clone(), mgr.clone()),
                    WeaveInterface::spawn,
                );
            }
            RnsInterfaceConfig::Meshtastic { name } => {
                let mut guard = mgr.lock().await;
                let _ = spawn_meshtastic(
                    &mut *guard,
                    name.clone(),
                    MeshtasticInterfaceConfig::default(),
                );
            }
            RnsInterfaceConfig::Local { path, .. } => {
                spawn_local_server(&mgr, path).await?;
            }
            RnsInterfaceConfig::LocalClient { path, .. } => {
                spawn_local_client(&mgr, path).await?;
            }
            RnsInterfaceConfig::ReticulumBle { peripheral_id, name } => {
                spawn_rnode_ble(&mgr, name.as_deref().unwrap_or("rnode-ble"), peripheral_id)
                    .await?;
            }
            RnsInterfaceConfig::Vrn76KissBle { peripheral_id, name } => {
                spawn_vrn76_ble(&mgr, name.as_deref().unwrap_or("vrn76"), peripheral_id).await?;
            }
        }
    }

    Ok(SpawnReport { listen_addr, kinds })
}

fn lora_config_for_region(region: &str) -> anyhow::Result<LoraConfig> {
    match region.to_ascii_uppercase().as_str() {
        "US915" | "US" => Ok(LoraConfig::us915_default()),
        other => match LoraConfig::for_region(other) {
            Ok(Some(cfg)) => Ok(cfg),
            Ok(None) => anyhow::bail!("unknown lora region '{other}'"),
            Err(e) => anyhow::bail!("lora region '{other}': {e}"),
        },
    }
}

fn resolve_tcp_bind(bind: &str) -> anyhow::Result<SocketAddr> {
    let addr: SocketAddr = bind
        .parse()
        .map_err(|e| anyhow::anyhow!("tcp_server bind '{bind}': {e}"))?;
    if addr.port() != 0 {
        return Ok(addr);
    }
    let listener = std::net::TcpListener::bind(addr)?;
    let resolved = listener.local_addr()?;
    drop(listener);
    Ok(resolved)
}

fn resolve_udp_bind(bind: &str) -> anyhow::Result<SocketAddr> {
    let addr: SocketAddr = bind
        .parse()
        .map_err(|e| anyhow::anyhow!("udp bind '{bind}': {e}"))?;
    if addr.port() != 0 {
        return Ok(addr);
    }
    let sock = std::net::UdpSocket::bind(addr)?;
    let resolved = sock.local_addr()?;
    drop(sock);
    Ok(resolved)
}

#[cfg(unix)]
async fn spawn_local_server(
    mgr: &Arc<Mutex<InterfaceManager>>,
    path: &str,
) -> anyhow::Result<()> {
    use reticulum_rs::iface::local::LocalUnixServer;
    mgr.lock()
        .await
        .spawn(LocalUnixServer::new(path, mgr.clone()), LocalUnixServer::spawn);
    Ok(())
}

#[cfg(not(unix))]
async fn spawn_local_server(
    _mgr: &Arc<Mutex<InterfaceManager>>,
    _path: &str,
) -> anyhow::Result<()> {
    anyhow::bail!("interface type 'local' requires Unix (shared-instance sockets)")
}

#[cfg(unix)]
async fn spawn_local_client(
    mgr: &Arc<Mutex<InterfaceManager>>,
    path: &str,
) -> anyhow::Result<()> {
    use reticulum_rs::iface::local::{LocalUnixClient, LocalUnixEndpoint};
    mgr.lock().await.spawn(
        LocalUnixClient::new_connect(LocalUnixEndpoint::filesystem(path)),
        LocalUnixClient::spawn,
    );
    Ok(())
}

#[cfg(not(unix))]
async fn spawn_local_client(
    _mgr: &Arc<Mutex<InterfaceManager>>,
    _path: &str,
) -> anyhow::Result<()> {
    anyhow::bail!("interface type 'local_client' requires Unix")
}

#[cfg(feature = "iface-ble")]
async fn spawn_rnode_ble(
    mgr: &Arc<Mutex<InterfaceManager>>,
    label: &str,
    peripheral_id: &str,
) -> anyhow::Result<()> {
    use reticulum_rs::iface::rnode_ble::{
        NativeRnodeBleKissInterface, NativeRnodeBleSettings, RnodeBleKissConfig,
    };
    mgr.lock().await.spawn(
        NativeRnodeBleKissInterface::new(
            label,
            NativeRnodeBleSettings::for_peripheral(peripheral_id),
            RnodeBleKissConfig::default(),
        ),
        NativeRnodeBleKissInterface::spawn,
    );
    Ok(())
}

#[cfg(not(feature = "iface-ble"))]
async fn spawn_rnode_ble(
    _mgr: &Arc<Mutex<InterfaceManager>>,
    _label: &str,
    _peripheral_id: &str,
) -> anyhow::Result<()> {
    anyhow::bail!(
        "interface type 'reticulum_ble' requires cargo feature `iface-ble` \
         (enables reticulum-rs-transport rnode-ble)"
    )
}

#[cfg(feature = "iface-ble")]
async fn spawn_vrn76_ble(
    mgr: &Arc<Mutex<InterfaceManager>>,
    label: &str,
    peripheral_id: &str,
) -> anyhow::Result<()> {
    use reticulum_rs::iface::vrn76_kiss_ble::{
        NativeVrn76BleSettings, NativeVrn76KissBleInterface, Vrn76KissBleConfig,
    };
    mgr.lock().await.spawn(
        NativeVrn76KissBleInterface::new(
            label,
            NativeVrn76BleSettings::for_peripheral(peripheral_id),
            Vrn76KissBleConfig::default(),
        ),
        NativeVrn76KissBleInterface::spawn,
    );
    Ok(())
}

#[cfg(not(feature = "iface-ble"))]
async fn spawn_vrn76_ble(
    _mgr: &Arc<Mutex<InterfaceManager>>,
    _label: &str,
    _peripheral_id: &str,
) -> anyhow::Result<()> {
    anyhow::bail!(
        "interface type 'vrn76_kiss_ble' requires cargo feature `iface-ble`"
    )
}
