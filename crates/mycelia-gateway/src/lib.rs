//! Minimal SOCKS5 TCP CONNECT gateway.
//!
//! Authorization: caller must present a domain attestation with GATEWAY capability
//! (checked by the node before enabling the listener). This crate implements the
//! proxy data plane only.

use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub struct SocksGateway {
    listen: SocketAddr,
}

impl SocksGateway {
    pub fn new(listen: SocketAddr) -> Self {
        Self { listen }
    }

    pub async fn serve(self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.listen).await?;
        tracing::info!("SOCKS5 gateway on {}", listener.local_addr()?);
        loop {
            let (mut client, _) = listener.accept().await?;
            tokio::spawn(async move {
                if let Err(e) = handle_socks(&mut client).await {
                    tracing::debug!("socks session ended: {e:#}");
                }
            });
        }
    }
}

async fn handle_socks(client: &mut TcpStream) -> anyhow::Result<()> {
    // greeting
    let mut hdr = [0u8; 2];
    client.read_exact(&mut hdr).await?;
    if hdr[0] != 0x05 {
        anyhow::bail!("not socks5");
    }
    let nmethods = hdr[1] as usize;
    let mut methods = vec![0u8; nmethods];
    client.read_exact(&mut methods).await?;
    // no auth
    client.write_all(&[0x05, 0x00]).await?;

    // request
    let mut req = [0u8; 4];
    client.read_exact(&mut req).await?;
    if req[0] != 0x05 || req[1] != 0x01 {
        // only CONNECT
        client
            .write_all(&[0x05, 0x07, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
            .await?;
        anyhow::bail!("unsupported command");
    }
    let atyp = req[3];
    let dest = match atyp {
        0x01 => {
            let mut ip = [0u8; 4];
            client.read_exact(&mut ip).await?;
            let mut port_b = [0u8; 2];
            client.read_exact(&mut port_b).await?;
            let port = u16::from_be_bytes(port_b);
            format!("{}.{}.{}.{}:{}", ip[0], ip[1], ip[2], ip[3], port)
        }
        0x03 => {
            let mut len = [0u8; 1];
            client.read_exact(&mut len).await?;
            let mut host = vec![0u8; len[0] as usize];
            client.read_exact(&mut host).await?;
            let mut port_b = [0u8; 2];
            client.read_exact(&mut port_b).await?;
            let port = u16::from_be_bytes(port_b);
            format!("{}:{}", String::from_utf8_lossy(&host), port)
        }
        _ => {
            client
                .write_all(&[0x05, 0x08, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
                .await?;
            anyhow::bail!("unsupported atyp");
        }
    };

    let mut upstream = TcpStream::connect(&dest).await?;
    // success
    client
        .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await?;
    tokio::io::copy_bidirectional(client, &mut upstream).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn socks_connect_echo() {
        // echo server
        let echo = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let echo_addr = echo.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut s, _) = echo.accept().await.unwrap();
            let mut buf = [0u8; 16];
            let n = s.read(&mut buf).await.unwrap();
            s.write_all(&buf[..n]).await.unwrap();
        });

        let gw = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let gw_addr = gw.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut client, _) = gw.accept().await.unwrap();
            let _ = handle_socks(&mut client).await;
        });

        let mut c = TcpStream::connect(gw_addr).await.unwrap();
        // greeting: ver=5, 1 method, noauth
        c.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
        let mut resp = [0u8; 2];
        c.read_exact(&mut resp).await.unwrap();
        assert_eq!(resp, [0x05, 0x00]);

        let ip = match echo_addr.ip() {
            std::net::IpAddr::V4(v) => v.octets(),
            _ => panic!("v4"),
        };
        let port = echo_addr.port().to_be_bytes();
        let mut req = vec![0x05, 0x01, 0x00, 0x01];
        req.extend_from_slice(&ip);
        req.extend_from_slice(&port);
        c.write_all(&req).await.unwrap();
        let mut ok = [0u8; 10];
        c.read_exact(&mut ok).await.unwrap();
        assert_eq!(ok[1], 0x00);

        c.write_all(b"ping").await.unwrap();
        let mut out = [0u8; 4];
        c.read_exact(&mut out).await.unwrap();
        assert_eq!(&out, b"ping");
    }
}
