//! Scoped service registry records.

use crate::ids::{CommunityId, DomainId, NodeId, ServiceId, HASH_LEN};
use crate::wire::{DecodeError, Decoder, Encoder};

#[cfg(feature = "crypto")]
use crate::attestation::{Attestation, Scope};

pub const SERVICE_RECORD_VERSION: u8 = 1;
pub const MAX_SERVICE_NAME: usize = 32;
pub const MAX_ENDPOINT_META: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Visibility {
    Public = 1,
    Community = 2,
    Domain = 3,
    Invitation = 4,
    Hidden = 5,
}

impl Visibility {
    fn from_u8(v: u8) -> Result<Self, DecodeError> {
        match v {
            1 => Ok(Self::Public),
            2 => Ok(Self::Community),
            3 => Ok(Self::Domain),
            4 => Ok(Self::Invitation),
            5 => Ok(Self::Hidden),
            _ => Err(DecodeError::InvalidEnum),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRecord {
    pub version: u8,
    pub service_id: ServiceId,
    pub name: heapless::String<MAX_SERVICE_NAME>,
    pub owner: NodeId,
    pub visibility: Visibility,
    /// Audience scope id (domain/community) or zeros for public; for invitation = invitee node.
    pub audience: [u8; HASH_LEN],
    pub endpoint: NodeId,
    pub meta: heapless::Vec<u8, MAX_ENDPOINT_META>,
    pub expires_at: u64,
    pub sig: [u8; 64],
}

impl ServiceRecord {
    pub fn is_visible_to(&self, viewer: NodeId, viewer_atts: &[AttestationView]) -> bool {
        match self.visibility {
            Visibility::Public => true,
            Visibility::Hidden => false,
            Visibility::Domain => viewer_atts.iter().any(|a| {
                a.scope == ScopeView::Domain && a.scope_id == self.audience && a.subject == viewer
            }),
            Visibility::Community => viewer_atts.iter().any(|a| {
                a.scope == ScopeView::Community
                    && a.scope_id == self.audience
                    && a.subject == viewer
            }),
            Visibility::Invitation => self.audience == *viewer.as_bytes(),
        }
    }

    pub fn encode_to(&self, buf: &mut [u8]) -> Result<usize, DecodeError> {
        let mut enc = Encoder::new(buf);
        enc.write_u8(self.version)?;
        self.service_id.encode(&mut enc)?;
        enc.write_blob(self.name.as_bytes())?;
        self.owner.encode(&mut enc)?;
        enc.write_u8(self.visibility as u8)?;
        enc.write_bytes(&self.audience)?;
        self.endpoint.encode(&mut enc)?;
        enc.write_blob(&self.meta)?;
        enc.write_u64(self.expires_at)?;
        enc.write_bytes(&self.sig)?;
        Ok(enc.position())
    }

    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        let mut dec = Decoder::new(buf);
        let version = dec.read_u8()?;
        if version != SERVICE_RECORD_VERSION {
            return Err(DecodeError::InvalidVersion);
        }
        let service_id = ServiceId::decode(&mut dec)?;
        let name_bytes = dec.read_blob()?;
        let mut name = heapless::String::new();
        name.push_str(core::str::from_utf8(name_bytes).map_err(|_| DecodeError::InvalidEnum)?)
            .map_err(|_| DecodeError::Overflow)?;
        let owner = NodeId::decode(&mut dec)?;
        let visibility = Visibility::from_u8(dec.read_u8()?)?;
        let mut audience = [0u8; HASH_LEN];
        dec.read_exact(&mut audience)?;
        let endpoint = NodeId::decode(&mut dec)?;
        let meta_bytes = dec.read_blob()?;
        let mut meta = heapless::Vec::new();
        meta.extend_from_slice(meta_bytes)
            .map_err(|_| DecodeError::Overflow)?;
        let expires_at = dec.read_u64()?;
        let mut sig = [0u8; 64];
        dec.read_exact(&mut sig)?;
        Ok(Self {
            version,
            service_id,
            name,
            owner,
            visibility,
            audience,
            endpoint,
            meta,
            expires_at,
            sig,
        })
    }
}

/// Lightweight view for visibility checks without full crypto module coupling on leaf listing.
#[derive(Debug, Clone, Copy)]
pub struct AttestationView {
    pub subject: NodeId,
    pub scope: ScopeView,
    pub scope_id: [u8; HASH_LEN],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeView {
    Domain,
    Community,
}

#[cfg(feature = "crypto")]
impl From<&Attestation> for AttestationView {
    fn from(a: &Attestation) -> Self {
        Self {
            subject: a.subject,
            scope: match a.scope {
                Scope::Domain => ScopeView::Domain,
                Scope::Community => ScopeView::Community,
            },
            scope_id: a.scope_id,
        }
    }
}

#[cfg(feature = "alloc")]
#[derive(Debug, Default, Clone)]
pub struct ServiceRegistry {
    records: alloc::vec::Vec<ServiceRecord>,
}

#[cfg(feature = "alloc")]
impl ServiceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&mut self, record: ServiceRecord) {
        if let Some(pos) = self
            .records
            .iter()
            .position(|r| r.service_id == record.service_id)
        {
            self.records[pos] = record;
        } else {
            self.records.push(record);
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &ServiceRecord> {
        self.records.iter()
    }

    pub fn list_visible(
        &self,
        viewer: NodeId,
        viewer_atts: &[AttestationView],
        now: u64,
    ) -> alloc::vec::Vec<&ServiceRecord> {
        self.records
            .iter()
            .filter(|r| r.expires_at >= now && r.is_visible_to(viewer, viewer_atts))
            .collect()
    }
}

pub fn domain_audience(id: DomainId) -> [u8; HASH_LEN] {
    *id.as_bytes()
}

pub fn community_audience(id: CommunityId) -> [u8; HASH_LEN] {
    *id.as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record(vis: Visibility, audience: [u8; HASH_LEN]) -> ServiceRecord {
        let mut name = heapless::String::new();
        name.push_str("printer").unwrap();
        ServiceRecord {
            version: SERVICE_RECORD_VERSION,
            service_id: ServiceId::new([1u8; 16]),
            name,
            owner: NodeId::new([2u8; 16]),
            visibility: vis,
            audience,
            endpoint: NodeId::new([3u8; 16]),
            meta: heapless::Vec::new(),
            expires_at: 9999,
            sig: [0u8; 64],
        }
    }

    #[test]
    fn visibility_matrix() {
        let viewer = NodeId::new([9u8; 16]);
        let domain = [5u8; 16];
        let public = sample_record(Visibility::Public, [0u8; 16]);
        assert!(public.is_visible_to(viewer, &[]));

        let hidden = sample_record(Visibility::Hidden, domain);
        assert!(!hidden.is_visible_to(viewer, &[]));

        let domain_svc = sample_record(Visibility::Domain, domain);
        assert!(!domain_svc.is_visible_to(viewer, &[]));
        let att = AttestationView {
            subject: viewer,
            scope: ScopeView::Domain,
            scope_id: domain,
        };
        assert!(domain_svc.is_visible_to(viewer, &[att]));

        let invite = sample_record(Visibility::Invitation, *viewer.as_bytes());
        assert!(invite.is_visible_to(viewer, &[]));
        assert!(!invite.is_visible_to(NodeId::new([8u8; 16]), &[]));
    }

    #[test]
    fn encode_decode() {
        let rec = sample_record(Visibility::Public, [0u8; 16]);
        let mut buf = [0u8; 256];
        let n = rec.encode_to(&mut buf).unwrap();
        let out = ServiceRecord::decode(&buf[..n]).unwrap();
        assert_eq!(out.name.as_str(), "printer");
        assert_eq!(out.visibility, Visibility::Public);
    }
}
