//! Gateway start via config + SOCKS CONNECT to loopback echo (Phase 7).

#![cfg(feature = "gateway")]

use mycelia_node::config::NodeConfig;
use mycelia_node::control::{control_call, ControlRequest};
use mycelia_node::mock_transport::MockHub;
use mycelia_node::runtime::NodeRuntime;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::test]
async fn gateway_socks_echo_via_config() {
    let hub = MockHub::new();
    let dir = tempfile::tempdir().unwrap();
    let gw_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway_bind = gw_listener.local_addr().unwrap();
    drop(gw_listener);

    let cfg = NodeConfig {
        data_dir: dir.path().to_path_buf(),
        enable_mdns: false,
        enable_gateway: true,
        gateway_bind,
        pow_difficulty: 0,
        transport: "mock".into(),
        ..Default::default()
    };

    // Pre-create state with GATEWAY attestation before start.
    {
        let mut st = mycelia_node::state::NodeState::bootstrap_with_pow(dir.path(), 0).unwrap();
        st.grant_self_gateway();
    }

    let rt = NodeRuntime::start(cfg, Some(hub)).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let status = control_call(rt.control_addr(), &ControlRequest::GatewayStatus)
        .await
        .unwrap();
    assert!(status.ok);
    assert_eq!(status.body["enabled"], true);

    // Echo server
    let echo = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let echo_addr = echo.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut s, _) = echo.accept().await.unwrap();
        let mut buf = [0u8; 16];
        let n = s.read(&mut buf).await.unwrap();
        s.write_all(&buf[..n]).await.unwrap();
    });

    let mut c = TcpStream::connect(gateway_bind).await.unwrap();
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

    rt.shutdown();
}
