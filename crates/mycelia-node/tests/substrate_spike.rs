//! Substrate spike (G2): two mock-transport nodes exchange ServiceAnnounce.
//!
//! FreeTAKTeam `reticulum-rs` host adapter is tracked as follow-on; mock hub
//! validates Mycelia control-plane over a ReticulumTransport implementation.

use mycelia_core::ids::{NodeId, ServiceId};
use mycelia_core::message::ControlMessage;
use mycelia_core::registry::{ServiceRecord, Visibility, SERVICE_RECORD_VERSION};
use mycelia_core::transport::ReticulumTransport;
use mycelia_node::mock_transport::{MockHub, MockTransport};

#[test]
fn substrate_spike_service_announce() {
    let hub = MockHub::new();
    let a_id = NodeId::new([0xau8; 16]);
    let b_id = NodeId::new([0xbu8; 16]);
    let mut a = MockTransport::new(a_id, hub.clone());
    let mut b = MockTransport::new(b_id, hub);

    let mut name = heapless::String::new();
    name.push_str("spike").unwrap();
    let record = ServiceRecord {
        version: SERVICE_RECORD_VERSION,
        service_id: ServiceId::new([1u8; 16]),
        name,
        owner: a_id,
        visibility: Visibility::Public,
        audience: [0u8; 16],
        endpoint: a_id,
        meta: heapless::Vec::new(),
        expires_at: 99999,
        sig: [0u8; 64],
    };
    let msg = ControlMessage::ServiceAnnounce { record };
    let mut buf = [0u8; 1024];
    let n = msg.encode_frame(&mut buf).unwrap();
    a.announce(&buf[..n]).unwrap();

    let incoming = b.poll_recv().unwrap().expect("b should receive announce");
    assert_eq!(incoming.from, a_id);
    let decoded = ControlMessage::decode_frame(&incoming.payload).unwrap();
    match decoded {
        ControlMessage::ServiceAnnounce { record } => {
            assert_eq!(record.name.as_str(), "spike");
        }
        _ => panic!("expected ServiceAnnounce"),
    }
}
