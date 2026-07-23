//! Two in-process RNS transports linked over localhost TCP exchange a MYC1 envelope.

use mycelia_core::ids::NodeId;
use mycelia_core::transport::ReticulumTransport;
use mycelia_node::rns_transport::{wait_recv, RnsTransport};
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let dir_a = tempfile::tempdir()?;
    let dir_b = tempfile::tempdir()?;

    let a_id = NodeId::new([0xau8; 16]);
    let b_id = NodeId::new([0xbu8; 16]);

    let mut a = RnsTransport::start_tcp(a_id, "127.0.0.1:0".parse()?, &[], dir_a.path()).await?;
    let listen_a = a.listen_addr();
    println!("A listening on {listen_a} ifaces={:?}", a.iface_kinds());

    let mut b =
        RnsTransport::start_tcp(b_id, "127.0.0.1:0".parse()?, &[listen_a], dir_b.path()).await?;
    println!("B listening on {} ifaces={:?}", b.listen_addr(), b.iface_kinds());

    tokio::time::sleep(Duration::from_millis(500)).await;

    a.announce(b"ping-from-a")?;
    let got = wait_recv(&mut b, Duration::from_secs(8))
        .await
        .expect("B should receive announce envelope");
    assert_eq!(got.from, a_id);
    assert_eq!(&got.payload[..], b"ping-from-a");
    println!("B received: {}", String::from_utf8_lossy(&got.payload));

    b.send(&a_id, b"pong-from-b")?;
    let got = wait_recv(&mut a, Duration::from_secs(8))
        .await
        .expect("A should receive directed envelope");
    assert_eq!(got.from, b_id);
    assert_eq!(&got.payload[..], b"pong-from-b");
    println!("A received: {}", String::from_utf8_lossy(&got.payload));
    println!("rns_tcp_ping ok");
    Ok(())
}
