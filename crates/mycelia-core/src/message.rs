//! Mycelia-native control-plane messages (not LXMF in MVP).

use crate::ids::NodeId;
use crate::peer::{PeerInfo, MAX_PEX_ENTRIES};
use crate::wire::{DecodeError, Decoder, Encoder};

#[cfg(feature = "crypto")]
use crate::attestation::Attestation;

use crate::registry::ServiceRecord;

pub const FRAME_VERSION: u8 = 1;
/// Max control-plane payload (leaf-friendly).
pub const MAX_FRAME_PAYLOAD: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    Invite = 1,
    AttestationAnnounce = 2,
    ServiceAnnounce = 3,
    ServiceQuery = 4,
    ServiceQueryResponse = 5,
    PeerExchange = 6,
    PolicyUpdate = 7,
}

impl MessageType {
    fn from_u8(v: u8) -> Result<Self, DecodeError> {
        match v {
            1 => Ok(Self::Invite),
            2 => Ok(Self::AttestationAnnounce),
            3 => Ok(Self::ServiceAnnounce),
            4 => Ok(Self::ServiceQuery),
            5 => Ok(Self::ServiceQueryResponse),
            6 => Ok(Self::PeerExchange),
            7 => Ok(Self::PolicyUpdate),
            _ => Err(DecodeError::InvalidEnum),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)] // control frames stay inline for no_std / LoRa budgets
pub enum ControlMessage {
    Invite {
        from: NodeId,
        domain: [u8; 16],
        #[cfg(feature = "crypto")]
        attestation: Attestation,
        #[cfg(not(feature = "crypto"))]
        attestation_bytes: heapless::Vec<u8, MAX_ATTESTATION_BYTES_FALLBACK>,
    },
    AttestationAnnounce {
        #[cfg(feature = "crypto")]
        attestation: Attestation,
        #[cfg(not(feature = "crypto"))]
        attestation_bytes: heapless::Vec<u8, MAX_ATTESTATION_BYTES_FALLBACK>,
    },
    ServiceAnnounce {
        record: ServiceRecord,
    },
    ServiceQuery {
        from: NodeId,
        filter_domain: Option<[u8; 16]>,
    },
    ServiceQueryResponse {
        records: heapless::Vec<ServiceRecord, 4>,
    },
    PeerExchange {
        peers: heapless::Vec<PeerInfo, MAX_PEX_ENTRIES>,
    },
    /// Stub for future policy distribution.
    PolicyUpdate {
        epoch: u64,
        payload: heapless::Vec<u8, 64>,
    },
}

#[cfg(not(feature = "crypto"))]
const MAX_ATTESTATION_BYTES_FALLBACK: usize = 256;

impl ControlMessage {
    pub fn message_type(&self) -> MessageType {
        match self {
            Self::Invite { .. } => MessageType::Invite,
            Self::AttestationAnnounce { .. } => MessageType::AttestationAnnounce,
            Self::ServiceAnnounce { .. } => MessageType::ServiceAnnounce,
            Self::ServiceQuery { .. } => MessageType::ServiceQuery,
            Self::ServiceQueryResponse { .. } => MessageType::ServiceQueryResponse,
            Self::PeerExchange { .. } => MessageType::PeerExchange,
            Self::PolicyUpdate { .. } => MessageType::PolicyUpdate,
        }
    }

    /// Length-prefixed frame: version | type | u16 len | payload
    pub fn encode_frame(&self, buf: &mut [u8]) -> Result<usize, DecodeError> {
        let mut payload = [0u8; MAX_FRAME_PAYLOAD];
        let plen = self.encode_payload(&mut payload)?;
        if plen > u16::MAX as usize {
            return Err(DecodeError::InvalidLength);
        }
        let mut enc = Encoder::new(buf);
        enc.write_u8(FRAME_VERSION)?;
        enc.write_u8(self.message_type() as u8)?;
        enc.write_u16(plen as u16)?;
        enc.write_bytes(&payload[..plen])?;
        Ok(enc.position())
    }

    pub fn decode_frame(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut dec = Decoder::new(buf);
        let version = dec.read_u8()?;
        if version != FRAME_VERSION {
            return Err(DecodeError::InvalidVersion);
        }
        let ty = MessageType::from_u8(dec.read_u8()?)?;
        let plen = dec.read_u16()? as usize;
        if plen > MAX_FRAME_PAYLOAD {
            return Err(DecodeError::InvalidLength);
        }
        let mut payload = [0u8; MAX_FRAME_PAYLOAD];
        if plen > 0 {
            dec.read_exact(&mut payload[..plen])?;
        }
        Self::decode_payload(ty, &payload[..plen])
    }

    fn encode_payload(&self, buf: &mut [u8]) -> Result<usize, DecodeError> {
        let mut enc = Encoder::new(buf);
        match self {
            #[cfg(feature = "crypto")]
            Self::Invite {
                from,
                domain,
                attestation,
            } => {
                from.encode(&mut enc)?;
                enc.write_bytes(domain)?;
                let mut abuf = [0u8; 256];
                let n = attestation
                    .encode_to(&mut abuf)
                    .map_err(|_| DecodeError::Overflow)?;
                enc.write_blob(&abuf[..n])?;
            }
            #[cfg(feature = "crypto")]
            Self::AttestationAnnounce { attestation } => {
                let mut abuf = [0u8; 256];
                let n = attestation
                    .encode_to(&mut abuf)
                    .map_err(|_| DecodeError::Overflow)?;
                enc.write_blob(&abuf[..n])?;
            }
            Self::ServiceAnnounce { record } => {
                let mut rbuf = [0u8; 512];
                let n = record.encode_to(&mut rbuf)?;
                enc.write_blob(&rbuf[..n])?;
            }
            Self::ServiceQuery {
                from,
                filter_domain,
            } => {
                from.encode(&mut enc)?;
                match filter_domain {
                    Some(d) => {
                        enc.write_u8(1)?;
                        enc.write_bytes(d)?;
                    }
                    None => enc.write_u8(0)?,
                }
            }
            Self::ServiceQueryResponse { records } => {
                enc.write_u8(records.len() as u8)?;
                for r in records {
                    let mut rbuf = [0u8; 512];
                    let n = r.encode_to(&mut rbuf)?;
                    enc.write_blob(&rbuf[..n])?;
                }
            }
            Self::PeerExchange { peers } => {
                enc.write_u8(peers.len() as u8)?;
                for p in peers {
                    p.encode(&mut enc)?;
                }
            }
            Self::PolicyUpdate { epoch, payload } => {
                enc.write_u64(*epoch)?;
                enc.write_blob(payload)?;
            }
            #[cfg(not(feature = "crypto"))]
            _ => return Err(DecodeError::InvalidEnum),
        }
        Ok(enc.position())
    }

    fn decode_payload(ty: MessageType, buf: &[u8]) -> Result<Self, DecodeError> {
        let mut dec = Decoder::new(buf);
        match ty {
            #[cfg(feature = "crypto")]
            MessageType::Invite => {
                let from = NodeId::decode(&mut dec)?;
                let mut domain = [0u8; 16];
                dec.read_exact(&mut domain)?;
                let blob = dec.read_blob()?;
                let attestation =
                    Attestation::decode(blob).map_err(|_| DecodeError::InvalidEnum)?;
                Ok(Self::Invite {
                    from,
                    domain,
                    attestation,
                })
            }
            #[cfg(feature = "crypto")]
            MessageType::AttestationAnnounce => {
                let blob = dec.read_blob()?;
                let attestation =
                    Attestation::decode(blob).map_err(|_| DecodeError::InvalidEnum)?;
                Ok(Self::AttestationAnnounce { attestation })
            }
            MessageType::ServiceAnnounce => {
                let blob = dec.read_blob()?;
                let record = ServiceRecord::decode(blob)?;
                Ok(Self::ServiceAnnounce { record })
            }
            MessageType::ServiceQuery => {
                let from = NodeId::decode(&mut dec)?;
                let flag = dec.read_u8()?;
                let filter_domain = if flag == 1 {
                    let mut d = [0u8; 16];
                    dec.read_exact(&mut d)?;
                    Some(d)
                } else {
                    None
                };
                Ok(Self::ServiceQuery {
                    from,
                    filter_domain,
                })
            }
            MessageType::ServiceQueryResponse => {
                let n = dec.read_u8()? as usize;
                let mut records = heapless::Vec::new();
                for _ in 0..n {
                    let blob = dec.read_blob()?;
                    let r = ServiceRecord::decode(blob)?;
                    records.push(r).map_err(|_| DecodeError::Overflow)?;
                }
                Ok(Self::ServiceQueryResponse { records })
            }
            MessageType::PeerExchange => {
                let n = dec.read_u8()? as usize;
                let mut peers = heapless::Vec::new();
                for _ in 0..n {
                    peers
                        .push(PeerInfo::decode(&mut dec)?)
                        .map_err(|_| DecodeError::Overflow)?;
                }
                Ok(Self::PeerExchange { peers })
            }
            MessageType::PolicyUpdate => {
                let epoch = dec.read_u64()?;
                let blob = dec.read_blob()?;
                let mut payload = heapless::Vec::new();
                payload
                    .extend_from_slice(blob)
                    .map_err(|_| DecodeError::Overflow)?;
                Ok(Self::PolicyUpdate { epoch, payload })
            }
            #[cfg(not(feature = "crypto"))]
            MessageType::Invite | MessageType::AttestationAnnounce => Err(DecodeError::InvalidEnum),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{ServiceRecord, Visibility, SERVICE_RECORD_VERSION};

    #[test]
    fn reject_oversized_length() {
        let mut buf = [0u8; 8];
        buf[0] = FRAME_VERSION;
        buf[1] = MessageType::PolicyUpdate as u8;
        // claim huge length
        buf[2] = 0xff;
        buf[3] = 0xff;
        assert!(ControlMessage::decode_frame(&buf).is_err());
    }

    #[test]
    fn service_announce_roundtrip() {
        let mut name = heapless::String::new();
        name.push_str("cam").unwrap();
        let record = ServiceRecord {
            version: SERVICE_RECORD_VERSION,
            service_id: crate::ids::ServiceId::new([1u8; 16]),
            name,
            owner: NodeId::new([2u8; 16]),
            visibility: Visibility::Public,
            audience: [0u8; 16],
            endpoint: NodeId::new([3u8; 16]),
            meta: heapless::Vec::new(),
            expires_at: 100,
            sig: [9u8; 64],
        };
        let msg = ControlMessage::ServiceAnnounce { record };
        let mut buf = [0u8; MAX_FRAME_PAYLOAD];
        let n = msg.encode_frame(&mut buf).unwrap();
        let out = ControlMessage::decode_frame(&buf[..n]).unwrap();
        match out {
            ControlMessage::ServiceAnnounce { record } => {
                assert_eq!(record.name.as_str(), "cam");
            }
            _ => panic!("wrong type"),
        }
    }

    #[test]
    fn truncated_fails() {
        assert!(ControlMessage::decode_frame(&[1, 3]).is_err());
    }
}
