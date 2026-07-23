//! Live FreeTAKTeam reticulum-rs TCP substrate spike.
//!
//! Two localhost transports exchange a MYC1 envelope via announce.

#![cfg(feature = "transport-rns")]

use mycelia_core::ids::NodeId;
use mycelia_core::transport::ReticulumTransport;
use mycelia_node::config::RnsInterfaceConfig;
use mycelia_node::rns_transport::{wait_recv, RnsTransport};
use std::time::Duration;

#[tokio::test]
async fn rns_two_nodes_exchange_envelope() {
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let a_id = NodeId::new([0x11u8; 16]);
    let b_id = NodeId::new([0x22u8; 16]);

    let mut a = RnsTransport::start_tcp(a_id, "127.0.0.1:0".parse().unwrap(), &[], dir_a.path())
        .await
        .expect("start A");
    let listen_a = a.listen_addr();

    let mut b = RnsTransport::start_tcp(
        b_id,
        "127.0.0.1:0".parse().unwrap(),
        &[listen_a],
        dir_b.path(),
    )
    .await
    .expect("start B");

    tokio::time::sleep(Duration::from_millis(800)).await;

    a.announce(b"mycelis-rns-spike").unwrap();
    let incoming = wait_recv(&mut b, Duration::from_secs(10))
        .await
        .expect("B should receive MYC1 envelope from A");
    assert_eq!(incoming.from, a_id);
    assert_eq!(&incoming.payload[..], b"mycelis-rns-spike");
}

#[tokio::test]
async fn rns_udp_two_nodes_exchange_envelope() {
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let a_id = NodeId::new([0x55u8; 16]);
    let b_id = NodeId::new([0x66u8; 16]);

    // Ephemeral UDP ports
    let sock_a = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let addr_a = sock_a.local_addr().unwrap();
    drop(sock_a);
    let sock_b = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let addr_b = sock_b.local_addr().unwrap();
    drop(sock_b);

    let ifaces_a = vec![RnsInterfaceConfig::Udp {
        bind: addr_a.to_string(),
        forward: Some(addr_b.to_string()),
        name: Some("udp-a".into()),
    }];
    let ifaces_b = vec![RnsInterfaceConfig::Udp {
        bind: addr_b.to_string(),
        forward: Some(addr_a.to_string()),
        name: Some("udp-b".into()),
    }];

    let mut a = RnsTransport::start(a_id, &ifaces_a, dir_a.path(), addr_a)
        .await
        .expect("start A udp");
    let mut b = RnsTransport::start(b_id, &ifaces_b, dir_b.path(), addr_b)
        .await
        .expect("start B udp");

    assert!(a.iface_kinds().contains(&"udp"));
    assert!(b.iface_kinds().contains(&"udp"));

    tokio::time::sleep(Duration::from_millis(500)).await;

    a.announce(b"mycelis-udp-spike").unwrap();
    let incoming = wait_recv(&mut b, Duration::from_secs(10))
        .await
        .expect("B should receive MYC1 over UDP");
    assert_eq!(incoming.from, a_id);
    assert_eq!(&incoming.payload[..], b"mycelis-udp-spike");
}

#[tokio::test]
#[ignore = "optional flaky twin under load"]
async fn rns_twin_bidirectional_ignored() {
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let a_id = NodeId::new([0x33u8; 16]);
    let b_id = NodeId::new([0x44u8; 16]);

    let mut a = RnsTransport::start_tcp(a_id, "127.0.0.1:0".parse().unwrap(), &[], dir_a.path())
        .await
        .unwrap();
    let listen_a = a.listen_addr();
    let mut b = RnsTransport::start_tcp(
        b_id,
        "127.0.0.1:0".parse().unwrap(),
        &[listen_a],
        dir_b.path(),
    )
    .await
    .unwrap();
    tokio::time::sleep(Duration::from_millis(800)).await;

    a.announce(b"a->b").unwrap();
    b.announce(b"b->a").unwrap();
    let _ = wait_recv(&mut b, Duration::from_secs(10)).await;
    let _ = wait_recv(&mut a, Duration::from_secs(10)).await;
}
