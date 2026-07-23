//! Localhost HTTP JSON control plane for CLI (Windows-portable).

use crate::discovery::wrap_pow;
use crate::runtime::NodeHandle;
use crate::state::domain_scope;
use mycelia_core::ids::{CommunityId, DomainId, NodeId};
use mycelia_core::registry::Visibility;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum ControlRequest {
    Status,
    DomainsList,
    DomainsCreate {
        name: String,
    },
    CommunitiesList,
    CommunitiesCreate {
        name: String,
    },
    CommunitiesInvite {
        community_hex: String,
        subject_hex: String,
    },
    ServicesList,
    ServicesAdvertise {
        name: String,
        domain_hex: Option<String>,
        visibility: String,
    },
    PeersList,
    Invite {
        domain_hex: String,
        subject_hex: String,
    },
    GatewayStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ControlResponse {
    pub ok: bool,
    pub error: Option<String>,
    pub body: serde_json::Value,
}

pub async fn serve_control(bind: SocketAddr, handle: NodeHandle) -> anyhow::Result<SocketAddr> {
    let listener = TcpListener::bind(bind).await?;
    let local = listener.local_addr()?;
    let handle = Arc::new(handle);
    tokio::spawn(async move {
        loop {
            let Ok((socket, _)) = listener.accept().await else {
                break;
            };
            let h = Arc::clone(&handle);
            tokio::spawn(async move {
                let _ = handle_conn(socket, h).await;
            });
        }
    });
    Ok(local)
}

async fn handle_conn(mut socket: TcpStream, handle: Arc<NodeHandle>) -> anyhow::Result<()> {
    let mut buf = vec![0u8; 65536];
    let n = socket.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }
    let req: ControlRequest = serde_json::from_slice(&buf[..n])?;
    let resp = dispatch(&handle, req).await;
    let out = serde_json::to_vec(&resp)?;
    socket.write_all(&out).await?;
    Ok(())
}

async fn dispatch(handle: &NodeHandle, req: ControlRequest) -> ControlResponse {
    match req {
        ControlRequest::Status => {
            let st = handle.state.lock().await;
            ControlResponse {
                ok: true,
                error: None,
                body: serde_json::json!({
                    "node_id": hex::encode(st.node_id.as_bytes()),
                    "uptime_secs": st.now().saturating_sub(st.started_at),
                    "peers": st.peer_cache.len(),
                    "listen": handle.listen_addr.to_string(),
                    "interfaces": handle.iface_kinds,
                }),
            }
        }
        ControlRequest::DomainsList => {
            let st = handle.state.lock().await;
            let mut domains = vec![];
            for a in &st.attestations {
                if let Some(d) = a.domain_id() {
                    domains.push(hex::encode(d.as_bytes()));
                }
            }
            domains.sort();
            domains.dedup();
            ControlResponse {
                ok: true,
                error: None,
                body: serde_json::json!({ "domains": domains }),
            }
        }
        ControlRequest::DomainsCreate { name } => {
            let mut st = handle.state.lock().await;
            let d = st.create_domain(name.as_bytes());
            ControlResponse {
                ok: true,
                error: None,
                body: serde_json::json!({ "domain": hex::encode(d.as_bytes()) }),
            }
        }
        ControlRequest::CommunitiesList => {
            let st = handle.state.lock().await;
            let communities: Vec<_> = st
                .list_communities()
                .into_iter()
                .map(|c| hex::encode(c.as_bytes()))
                .collect();
            ControlResponse {
                ok: true,
                error: None,
                body: serde_json::json!({ "communities": communities }),
            }
        }
        ControlRequest::CommunitiesCreate { name } => {
            let mut st = handle.state.lock().await;
            let c = st.create_community(name.as_bytes());
            ControlResponse {
                ok: true,
                error: None,
                body: serde_json::json!({ "community": hex::encode(c.as_bytes()) }),
            }
        }
        ControlRequest::CommunitiesInvite {
            community_hex,
            subject_hex,
        } => {
            let mut st = handle.state.lock().await;
            let community = match parse_hex16(&community_hex) {
                Ok(b) => CommunityId::new(b),
                Err(e) => {
                    return ControlResponse {
                        ok: false,
                        error: Some(e),
                        body: serde_json::Value::Null,
                    }
                }
            };
            let subject = match parse_hex16(&subject_hex) {
                Ok(b) => NodeId::new(b),
                Err(e) => {
                    return ControlResponse {
                        ok: false,
                        error: Some(e),
                        body: serde_json::Value::Null,
                    }
                }
            };
            let att = st.invite_community_member(community, subject);
            let msg = mycelia_core::message::ControlMessage::Invite {
                from: st.node_id,
                domain: *community.as_bytes(),
                attestation: att,
            };
            let mut buf = [0u8; 1024];
            if let Ok(n) = msg.encode_frame(&mut buf) {
                let mut t = handle.transport.lock().await;
                let _ = t.send(&subject, &buf[..n]);
                let _ = t.announce(&buf[..n]);
            }
            ControlResponse {
                ok: true,
                error: None,
                body: serde_json::json!({ "invited": subject_hex }),
            }
        }
        ControlRequest::ServicesList => {
            let st = handle.state.lock().await;
            let viewer = st.node_id;
            let list = st.list_services_for(viewer);
            let services: Vec<_> = list
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "name": r.name.as_str(),
                        "owner": hex::encode(r.owner.as_bytes()),
                        "visibility": format!("{:?}", r.visibility),
                    })
                })
                .collect();
            ControlResponse {
                ok: true,
                error: None,
                body: serde_json::json!({ "services": services }),
            }
        }
        ControlRequest::ServicesAdvertise {
            name,
            domain_hex,
            visibility,
        } => {
            let mut st = handle.state.lock().await;
            let vis = match visibility.as_str() {
                "public" => Visibility::Public,
                "domain" => Visibility::Domain,
                "community" => Visibility::Community,
                "invitation" => Visibility::Invitation,
                "hidden" => Visibility::Hidden,
                _ => Visibility::Domain,
            };
            let audience = if let Some(h) = domain_hex {
                match parse_hex16(&h) {
                    Ok(b) => b,
                    Err(e) => {
                        return ControlResponse {
                            ok: false,
                            error: Some(e),
                            body: serde_json::Value::Null,
                        }
                    }
                }
            } else {
                [0u8; 16]
            };
            let rec = st.advertise_service(&name, vis, audience);
            let msg = mycelia_core::message::ControlMessage::ServiceAnnounce {
                record: rec.clone(),
            };
            let mut buf = [0u8; 1024];
            if let Ok(n) = msg.encode_frame(&mut buf) {
                let framed = wrap_pow(&buf[..n], st.pow_difficulty);
                let mut t = handle.transport.lock().await;
                let _ = t.announce(&framed);
            }
            ControlResponse {
                ok: true,
                error: None,
                body: serde_json::json!({ "name": rec.name.as_str() }),
            }
        }
        ControlRequest::PeersList => {
            let st = handle.state.lock().await;
            let peers: Vec<_> = st
                .peer_cache
                .iter()
                .map(|p| hex::encode(p.node_id.as_bytes()))
                .collect();
            ControlResponse {
                ok: true,
                error: None,
                body: serde_json::json!({ "peers": peers }),
            }
        }
        ControlRequest::Invite {
            domain_hex,
            subject_hex,
        } => {
            let mut st = handle.state.lock().await;
            let domain = match parse_hex16(&domain_hex) {
                Ok(b) => DomainId::new(b),
                Err(e) => {
                    return ControlResponse {
                        ok: false,
                        error: Some(e),
                        body: serde_json::Value::Null,
                    }
                }
            };
            let subject = match parse_hex16(&subject_hex) {
                Ok(b) => NodeId::new(b),
                Err(e) => {
                    return ControlResponse {
                        ok: false,
                        error: Some(e),
                        body: serde_json::Value::Null,
                    }
                }
            };
            let att = st.invite_member(domain, subject);
            let msg = mycelia_core::message::ControlMessage::Invite {
                from: st.node_id,
                domain: domain_scope(domain),
                attestation: att,
            };
            let mut buf = [0u8; 1024];
            if let Ok(n) = msg.encode_frame(&mut buf) {
                let mut t = handle.transport.lock().await;
                let _ = t.send(&subject, &buf[..n]);
                let _ = t.announce(&buf[..n]);
            }
            ControlResponse {
                ok: true,
                error: None,
                body: serde_json::json!({ "invited": subject_hex }),
            }
        }
        ControlRequest::GatewayStatus => {
            let st = handle.state.lock().await;
            ControlResponse {
                ok: true,
                error: None,
                body: serde_json::json!({
                    "enabled": st.gateway_enabled,
                    "bind": st.gateway_bind.map(|a| a.to_string()),
                    "has_capability": st.has_gateway_capability(),
                }),
            }
        }
    }
}

fn parse_hex16(s: &str) -> Result<[u8; 16], String> {
    let bytes = hex::decode(s).map_err(|e| e.to_string())?;
    if bytes.len() != 16 {
        return Err("expected 16 bytes hex".into());
    }
    let mut a = [0u8; 16];
    a.copy_from_slice(&bytes);
    Ok(a)
}

pub async fn control_call(
    addr: SocketAddr,
    req: &ControlRequest,
) -> anyhow::Result<ControlResponse> {
    let mut stream = TcpStream::connect(addr).await?;
    let body = serde_json::to_vec(req)?;
    stream.write_all(&body).await?;
    stream.shutdown().await?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await?;
    Ok(serde_json::from_slice(&buf)?)
}
