//! Communities create/list/invite (Phase 8).

use mycelia_node::config::NodeConfig;
use mycelia_node::control::{control_call, ControlRequest};
use mycelia_node::mock_transport::MockHub;
use mycelia_node::runtime::NodeRuntime;
use std::time::Duration;

#[tokio::test]
async fn communities_create_list_invite() {
    let hub = MockHub::new();
    let dir_a = tempfile::tempdir().unwrap();
    let dir_b = tempfile::tempdir().unwrap();
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

    let rt_a = NodeRuntime::start(cfg_a, Some(hub.clone())).await.unwrap();
    let rt_b = NodeRuntime::start(cfg_b, Some(hub)).await.unwrap();

    let resp = control_call(
        rt_a.control_addr(),
        &ControlRequest::CommunitiesCreate {
            name: "mesh".into(),
        },
    )
    .await
    .unwrap();
    assert!(resp.ok);
    let community_hex = resp.body["community"].as_str().unwrap().to_string();

    let b_id = {
        let st = rt_b.handle.state.lock().await;
        hex::encode(st.node_id.as_bytes())
    };

    let resp = control_call(
        rt_a.control_addr(),
        &ControlRequest::CommunitiesInvite {
            community_hex: community_hex.clone(),
            subject_hex: b_id,
        },
    )
    .await
    .unwrap();
    assert!(resp.ok);
    tokio::time::sleep(Duration::from_millis(200)).await;

    let resp = control_call(rt_a.control_addr(), &ControlRequest::CommunitiesList)
        .await
        .unwrap();
    assert!(resp.ok);
    let list = resp.body["communities"].as_array().unwrap();
    assert!(list.iter().any(|v| v.as_str() == Some(community_hex.as_str())));

    // B should have accepted community attestation
    {
        let st = rt_b.handle.state.lock().await;
        assert!(st
            .list_communities()
            .iter()
            .any(|c| hex::encode(c.as_bytes()) == community_hex));
    }

    rt_a.shutdown();
    rt_b.shutdown();
}
