//! Domain and community membership stores (in-memory).

#[cfg(feature = "alloc")]
use alloc::collections::BTreeMap;
#[cfg(feature = "alloc")]
use alloc::vec::Vec;

use crate::ids::{CommunityId, DomainId, NodeId};

#[cfg(feature = "crypto")]
use crate::attestation::{Attestation, Capability, Scope};

#[derive(Debug, Clone)]
pub struct Revocation {
    pub subject: NodeId,
    pub scope_id: [u8; 16],
    pub expires_at: u64,
}

#[cfg(feature = "alloc")]
#[derive(Debug, Default, Clone)]
pub struct DomainStore {
    /// domain -> members
    members: BTreeMap<DomainId, Vec<NodeId>>,
    owners: BTreeMap<DomainId, NodeId>,
    revocations: Vec<Revocation>,
}

#[cfg(feature = "alloc")]
impl DomainStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_domain(&mut self, domain: DomainId, owner: NodeId) {
        self.owners.insert(domain, owner);
        self.members.insert(domain, alloc::vec![owner]);
    }

    pub fn add_member(&mut self, domain: DomainId, member: NodeId) {
        let list = self.members.entry(domain).or_default();
        if !list.contains(&member) {
            list.push(member);
        }
    }

    pub fn revoke(&mut self, subject: NodeId, scope_id: [u8; 16], expires_at: u64) {
        if let Some(list) = self.members.get_mut(&DomainId::new(scope_id)) {
            list.retain(|n| n != &subject);
        }
        self.revocations.push(Revocation {
            subject,
            scope_id,
            expires_at,
        });
    }

    pub fn is_revoked(&self, subject: NodeId, scope_id: [u8; 16], now: u64) -> bool {
        self.revocations
            .iter()
            .any(|r| r.subject == subject && r.scope_id == scope_id && now <= r.expires_at)
    }

    pub fn members(&self, domain: DomainId) -> Option<&[NodeId]> {
        self.members.get(&domain).map(|v| v.as_slice())
    }

    pub fn owner(&self, domain: DomainId) -> Option<NodeId> {
        self.owners.get(&domain).copied()
    }

    /// Isolation: only return members if viewer is a member.
    pub fn list_members_for(&self, domain: DomainId, viewer: NodeId) -> Option<&[NodeId]> {
        let members = self.members.get(&domain)?;
        if members.contains(&viewer) {
            Some(members.as_slice())
        } else {
            None
        }
    }
}

#[cfg(feature = "alloc")]
#[derive(Debug, Default, Clone)]
pub struct CommunityStore {
    members: BTreeMap<CommunityId, Vec<NodeId>>,
}

#[cfg(feature = "alloc")]
impl CommunityStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_member(&mut self, community: CommunityId, member: NodeId) {
        let list = self.members.entry(community).or_default();
        if !list.contains(&member) {
            list.push(member);
        }
    }

    pub fn members(&self, community: CommunityId) -> Option<&[NodeId]> {
        self.members.get(&community).map(|v| v.as_slice())
    }
}

#[cfg(all(feature = "alloc", feature = "crypto"))]
pub fn apply_attestation(store: &mut DomainStore, att: &Attestation) {
    if att.scope != Scope::Domain {
        return;
    }
    let domain = DomainId::new(att.scope_id);
    if att.caps.contains(Capability::OWNER) {
        store.create_domain(domain, att.subject);
    } else {
        store.add_member(domain, att.subject);
    }
}

#[cfg(all(feature = "alloc", feature = "crypto"))]
pub fn apply_community_attestation(store: &mut CommunityStore, att: &Attestation) {
    if att.scope != Scope::Community {
        return;
    }
    store.add_member(CommunityId::new(att.scope_id), att.subject);
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use super::*;

    #[test]
    fn isolation_across_domains() {
        let mut store = DomainStore::new();
        let d_a = DomainId::new([1u8; 16]);
        let d_b = DomainId::new([2u8; 16]);
        let alice = NodeId::new([0xau8; 16]);
        let bob = NodeId::new([0xbu8; 16]);
        store.create_domain(d_a, alice);
        store.create_domain(d_b, bob);
        store.add_member(d_a, bob);

        assert!(store.list_members_for(d_a, alice).is_some());
        assert!(store.list_members_for(d_a, bob).is_some());
        assert!(store.list_members_for(d_b, alice).is_none());
        assert_eq!(store.list_members_for(d_b, bob).unwrap().len(), 1);
    }
}
