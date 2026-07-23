//! Persist peer_cache / attestations / registry across bootstrap (Phase 8).

use mycelia_core::registry::Visibility;
use mycelia_node::state::NodeState;

#[test]
fn persist_reload_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let domain = {
        let mut st = NodeState::bootstrap_with_pow(dir.path(), 0).unwrap();
        let d = st.create_domain(b"persist-home");
        st.advertise_service("printer", Visibility::Public, [0u8; 16]);
        st.peer_cache.upsert(mycelia_core::peer::PeerInfo {
            node_id: mycelia_core::ids::NodeId::new([9u8; 16]),
            last_seen: st.now(),
            interface_hint: heapless::Vec::new(),
        });
        st.persist().unwrap();
        d
    };

    let st2 = NodeState::bootstrap_with_pow(dir.path(), 0).unwrap();
    assert!(st2
        .attestations
        .iter()
        .any(|a| a.domain_id() == Some(domain)));
    assert!(st2.registry.iter().any(|r| r.name.as_str() == "printer"));
    assert!(st2
        .peer_cache
        .iter()
        .any(|p| p.node_id.as_bytes() == &[9u8; 16]));
}
