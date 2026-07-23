//! MVP acceptance (Task 3.5): domain membership scopes service visibility.

use mycelia_core::message::ControlMessage;
use mycelia_node::config::NodeConfig;
use mycelia_node::control::{control_call, ControlRequest};
use mycelia_node::discovery::handle_control_bytes;
use mycelia_node::mock_transport::MockHub;
use mycelia_node::runtime::NodeRuntime;
use std::time::Duration;

#[tokio::test]
async fn e2e_domain_service_visibility() {
    let hub = MockHub::new();
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let dir_c = tempfile::tempdir().unwrap();

    let cfg_a = NodeConfig {
        data_dir: dir_a.path().to_path_buf(),
        enable_mdns: false,
        pow_difficulty: 0,
        transport: "mock".into(),
        ..Default::default()
    };
    let cfg_b = NodeConfig {
        data_dir: dir_b.path().to_path_buf(),
        enable_mdns: false,
        pow_difficulty: 0,
        transport: "mock".into(),
        ..Default::default()
    };
    let cfg_c = NodeConfig {
        data_dir: dir_c.path().to_path_buf(),
        enable_mdns: false,
        pow_difficulty: 0,
        transport: "mock".into(),
        ..Default::default()
    };

    let rt_a = NodeRuntime::start(cfg_a, Some(hub.clone())).await.unwrap();
    let rt_b = NodeRuntime::start(cfg_b, Some(hub.clone())).await.unwrap();
    let rt_c = NodeRuntime::start(cfg_c, Some(hub)).await.unwrap();

    // Create domain on A
    let resp = control_call(
        rt_a.control_addr(),
        &ControlRequest::DomainsCreate {
            name: "home".into(),
        },
    )
    .await
    .unwrap();
    assert!(resp.ok);
    let domain_hex = resp.body["domain"].as_str().unwrap().to_string();

    let b_id = {
        let st = rt_b.handle.state.lock().await;
        hex::encode(st.node_id.as_bytes())
    };

    // Invite B
    let resp = control_call(
        rt_a.control_addr(),
        &ControlRequest::Invite {
            domain_hex: domain_hex.clone(),
            subject_hex: b_id,
        },
    )
    .await
    .unwrap();
    assert!(resp.ok);

    // Allow poll loop to deliver invite
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Advertise domain-scoped service on A
    let resp = control_call(
        rt_a.control_addr(),
        &ControlRequest::ServicesAdvertise {
            name: "nas".into(),
            domain_hex: Some(domain_hex),
            visibility: "domain".into(),
        },
    )
    .await
    .unwrap();
    assert!(resp.ok);
    tokio::time::sleep(Duration::from_millis(200)).await;

    // B should see service
    {
        let st = rt_b.handle.state.lock().await;
        let viewer = st.node_id;
        let list = st.list_services_for(viewer);
        assert!(
            list.iter().any(|r| r.name.as_str() == "nas"),
            "B should see domain service"
        );
    }

    // C should not see service
    {
        let st = rt_c.handle.state.lock().await;
        let viewer = st.node_id;
        let list = st.list_services_for(viewer);
        assert!(
            list.iter().all(|r| r.name.as_str() != "nas"),
            "C must not see domain service"
        );
    }

    rt_a.shutdown();
    rt_b.shutdown();
    rt_c.shutdown();
}

#[tokio::test]
async fn pex_without_dht() {
    let hub = MockHub::new();
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
    let cfg_a = NodeConfig {
        data_dir: dir_a.path().to_path_buf(),
        enable_mdns: false,
        static_peers: vec!["127.0.0.1:9".parse().unwrap()],
        pow_difficulty: 0,
        transport: "mock".into(),
        ..Default::default()
    };
    let cfg_b = NodeConfig {
        data_dir: dir_b.path().to_path_buf(),
        enable_mdns: false,
        pow_difficulty: 0,
        transport: "mock".into(),
        ..Default::default()
    };

    // Start B first so it is registered on the hub when A announces PEX.
    let rt_b = NodeRuntime::start(cfg_b, Some(hub.clone())).await.unwrap();
    let rt_a = NodeRuntime::start(cfg_a, Some(hub)).await.unwrap();
    tokio::time::sleep(Duration::from_millis(300)).await;

    let peers_b = {
        let st = rt_b.handle.state.lock().await;
        st.peer_cache.len()
    };
    assert!(peers_b > 0, "B should learn peers via PEX/announce");

    // Force direct PEX from A identity
    {
        let st = rt_a.handle.state.lock().await;
        let mut peers = heapless::Vec::new();
        let _ = peers.push(mycelia_core::peer::PeerInfo {
            node_id: st.node_id,
            last_seen: st.now(),
            interface_hint: heapless::Vec::new(),
        });
        let msg = ControlMessage::PeerExchange { peers };
        let mut buf = [0u8; 1024];
        let n = msg.encode_frame(&mut buf).unwrap();
        let from = st.node_id;
        drop(st);
        let mut st_b = rt_b.handle.state.lock().await;
        handle_control_bytes(&mut st_b, from, &buf[..n]);
        assert!(!st_b.peer_cache.is_empty());
    }

    rt_a.shutdown();
    rt_b.shutdown();
}
