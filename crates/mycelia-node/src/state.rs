use ed25519_dalek::{SigningKey, VerifyingKey};
use mycelia_core::attestation::{
    generate_signing_key, node_id_from_verifying_key, Attestation, Capability, Scope,
};
use mycelia_core::domain::{
    apply_attestation, apply_community_attestation, CommunityStore, DomainStore,
};
use mycelia_core::ids::{CommunityId, DomainId, NodeId, ServiceId};
use mycelia_core::peer::{PeerCache, PeerInfo};
use mycelia_core::registry::{
    domain_audience, AttestationView, ServiceRecord, ServiceRegistry, Visibility,
    SERVICE_RECORD_VERSION,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct NodeState {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
    pub node_id: NodeId,
    pub domains: DomainStore,
    pub communities: CommunityStore,
    pub registry: ServiceRegistry,
    pub peer_cache: PeerCache<64>,
    pub attestations: Vec<Attestation>,
    pub started_at: u64,
    pub data_dir: PathBuf,
    pub pow_difficulty: u8,
    pub gateway_enabled: bool,
    pub gateway_bind: Option<std::net::SocketAddr>,
}

#[derive(Serialize, Deserialize)]
struct HexBlob {
    hex: String,
}

impl NodeState {
    pub fn bootstrap(data_dir: &Path) -> anyhow::Result<Self> {
        Self::bootstrap_with_pow(data_dir, 8)
    }

    pub fn bootstrap_with_pow(data_dir: &Path, pow_difficulty: u8) -> anyhow::Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let key_path = data_dir.join("identity.key");
        let signing_key = if key_path.exists() {
            let bytes = std::fs::read(&key_path)?;
            if bytes.len() != 32 {
                anyhow::bail!("invalid identity key length");
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            SigningKey::from_bytes(&arr)
        } else {
            let sk = generate_signing_key();
            std::fs::write(&key_path, sk.to_bytes())?;
            sk
        };
        let verifying_key = signing_key.verifying_key();
        let node_id = node_id_from_verifying_key(&verifying_key);
        let mut state = Self {
            signing_key,
            verifying_key,
            node_id,
            domains: DomainStore::new(),
            communities: CommunityStore::new(),
            registry: ServiceRegistry::new(),
            peer_cache: PeerCache::new(86_400),
            attestations: vec![],
            started_at: now_secs(),
            data_dir: data_dir.to_path_buf(),
            pow_difficulty,
            gateway_enabled: false,
            gateway_bind: None,
        };
        state.load_persisted()?;
        Ok(state)
    }

    fn load_persisted(&mut self) -> anyhow::Result<()> {
        let att_path = self.data_dir.join("attestations.json");
        if att_path.exists() {
            let raw = std::fs::read_to_string(&att_path)?;
            let blobs: Vec<HexBlob> = serde_json::from_str(&raw).unwrap_or_default();
            for b in blobs {
                if let Ok(bytes) = hex::decode(&b.hex) {
                    if let Ok(att) = Attestation::decode(&bytes) {
                        self.accept_attestation_no_persist(att);
                    }
                }
            }
        }
        let reg_path = self.data_dir.join("registry.json");
        if reg_path.exists() {
            let raw = std::fs::read_to_string(&reg_path)?;
            let blobs: Vec<HexBlob> = serde_json::from_str(&raw).unwrap_or_default();
            for b in blobs {
                if let Ok(bytes) = hex::decode(&b.hex) {
                    if let Ok(rec) = ServiceRecord::decode(&bytes) {
                        self.registry.upsert(rec);
                    }
                }
            }
        }
        let peer_path = self.data_dir.join("peer_cache.json");
        if peer_path.exists() {
            let raw = std::fs::read_to_string(&peer_path)?;
            let blobs: Vec<HexBlob> = serde_json::from_str(&raw).unwrap_or_default();
            for b in blobs {
                if let Ok(bytes) = hex::decode(&b.hex) {
                    let mut dec = mycelia_core::wire::Decoder::new(&bytes);
                    if let Ok(peer) = PeerInfo::decode(&mut dec) {
                        self.peer_cache.upsert(peer);
                    }
                }
            }
        }
        Ok(())
    }

    pub fn persist(&self) -> anyhow::Result<()> {
        let mut att_blobs = Vec::new();
        for a in &self.attestations {
            let mut buf = [0u8; 256];
            if let Ok(n) = a.encode_to(&mut buf) {
                att_blobs.push(HexBlob {
                    hex: hex::encode(&buf[..n]),
                });
            }
        }
        std::fs::write(
            self.data_dir.join("attestations.json"),
            serde_json::to_string_pretty(&att_blobs)?,
        )?;

        let mut reg_blobs = Vec::new();
        for r in self.registry.iter() {
            let mut buf = [0u8; 512];
            if let Ok(n) = r.encode_to(&mut buf) {
                reg_blobs.push(HexBlob {
                    hex: hex::encode(&buf[..n]),
                });
            }
        }
        std::fs::write(
            self.data_dir.join("registry.json"),
            serde_json::to_string_pretty(&reg_blobs)?,
        )?;

        let mut peer_blobs = Vec::new();
        for p in self.peer_cache.iter() {
            let mut buf = [0u8; 128];
            let n = {
                let mut enc = mycelia_core::wire::Encoder::new(&mut buf);
                if p.encode(&mut enc).is_err() {
                    continue;
                }
                enc.position()
            };
            peer_blobs.push(HexBlob {
                hex: hex::encode(&buf[..n]),
            });
        }
        std::fs::write(
            self.data_dir.join("peer_cache.json"),
            serde_json::to_string_pretty(&peer_blobs)?,
        )?;
        Ok(())
    }

    pub fn now(&self) -> u64 {
        now_secs()
    }

    pub fn create_domain(&mut self, name_seed: &[u8]) -> DomainId {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(self.node_id.as_bytes());
        h.update(name_seed);
        let d = h.finalize();
        let mut id = [0u8; 16];
        id.copy_from_slice(&d[..16]);
        let domain = DomainId::new(id);
        let att = Attestation::issue(
            &self.signing_key,
            self.node_id,
            self.node_id,
            Scope::Domain,
            id,
            Capability::OWNER
                .union(Capability::MEMBER)
                .union(Capability::ADVERTISE),
            0,
            u64::MAX,
            None,
        )
        .expect("issue owner attestation");
        apply_attestation(&mut self.domains, &att);
        self.attestations.push(att);
        let _ = self.persist();
        domain
    }

    pub fn create_community(&mut self, name_seed: &[u8]) -> CommunityId {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(b"community:");
        h.update(self.node_id.as_bytes());
        h.update(name_seed);
        let d = h.finalize();
        let mut id = [0u8; 16];
        id.copy_from_slice(&d[..16]);
        let community = CommunityId::new(id);
        let att = Attestation::issue(
            &self.signing_key,
            self.node_id,
            self.node_id,
            Scope::Community,
            id,
            Capability::OWNER
                .union(Capability::MEMBER)
                .union(Capability::ADVERTISE),
            0,
            u64::MAX,
            None,
        )
        .expect("issue community owner attestation");
        apply_community_attestation(&mut self.communities, &att);
        self.attestations.push(att);
        let _ = self.persist();
        community
    }

    pub fn invite_member(&mut self, domain: DomainId, subject: NodeId) -> Attestation {
        let att = Attestation::issue(
            &self.signing_key,
            self.node_id,
            subject,
            Scope::Domain,
            *domain.as_bytes(),
            Capability::MEMBER.union(Capability::ADVERTISE),
            0,
            u64::MAX,
            None,
        )
        .expect("issue member attestation");
        apply_attestation(&mut self.domains, &att);
        self.attestations.push(att.clone());
        let _ = self.persist();
        att
    }

    pub fn invite_community_member(
        &mut self,
        community: CommunityId,
        subject: NodeId,
    ) -> Attestation {
        let att = Attestation::issue(
            &self.signing_key,
            self.node_id,
            subject,
            Scope::Community,
            *community.as_bytes(),
            Capability::MEMBER.union(Capability::ADVERTISE),
            0,
            u64::MAX,
            None,
        )
        .expect("issue community member attestation");
        apply_community_attestation(&mut self.communities, &att);
        self.attestations.push(att.clone());
        let _ = self.persist();
        att
    }

    /// Issue a self GATEWAY capability attestation (for enabling SOCKS gateway).
    pub fn grant_self_gateway(&mut self) -> Attestation {
        let scope_id = *self.node_id.as_bytes();
        let att = Attestation::issue(
            &self.signing_key,
            self.node_id,
            self.node_id,
            Scope::Domain,
            scope_id,
            Capability::GATEWAY.union(Capability::MEMBER),
            0,
            u64::MAX,
            None,
        )
        .expect("issue gateway attestation");
        self.accept_attestation(att.clone());
        att
    }

    pub fn has_gateway_capability(&self) -> bool {
        self.attestations.iter().any(|a| {
            a.subject == self.node_id && a.caps.contains(Capability::GATEWAY)
        })
    }

    pub fn accept_attestation(&mut self, att: Attestation) {
        self.accept_attestation_no_persist(att);
        let _ = self.persist();
    }

    fn accept_attestation_no_persist(&mut self, att: Attestation) {
        match att.scope {
            Scope::Domain => apply_attestation(&mut self.domains, &att),
            Scope::Community => apply_community_attestation(&mut self.communities, &att),
        }
        if !self.attestations.iter().any(|a| a.sig == att.sig) {
            self.attestations.push(att);
        }
    }

    pub fn advertise_service(
        &mut self,
        name: &str,
        visibility: Visibility,
        audience: [u8; 16],
    ) -> ServiceRecord {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(self.node_id.as_bytes());
        h.update(name.as_bytes());
        let d = h.finalize();
        let mut sid = [0u8; 16];
        sid.copy_from_slice(&d[..16]);
        let mut sname = heapless::String::new();
        let _ = sname.push_str(name);
        let mut record = ServiceRecord {
            version: SERVICE_RECORD_VERSION,
            service_id: ServiceId::new(sid),
            name: sname,
            owner: self.node_id,
            visibility,
            audience,
            endpoint: self.node_id,
            meta: heapless::Vec::new(),
            expires_at: self.now().saturating_add(86_400),
            sig: [0u8; 64],
        };
        let mut body = [0u8; 512];
        let mut tmp = record.clone();
        tmp.sig = [0u8; 64];
        let n = tmp.encode_to(&mut body).unwrap_or(0);
        use ed25519_dalek::Signer;
        let sig = self.signing_key.sign(&body[..n]);
        record.sig = sig.to_bytes();
        self.registry.upsert(record.clone());
        let _ = self.persist();
        record
    }

    pub fn attestation_views(&self) -> Vec<AttestationView> {
        self.attestations
            .iter()
            .map(AttestationView::from)
            .collect()
    }

    pub fn list_services_for(&self, viewer: NodeId) -> Vec<&ServiceRecord> {
        let views = self.attestation_views();
        let atts = if viewer == self.node_id {
            views
        } else {
            self.attestations
                .iter()
                .filter(|a| a.subject == viewer)
                .map(AttestationView::from)
                .collect()
        };
        self.registry.list_visible(viewer, &atts, self.now())
    }

    pub fn list_communities(&self) -> Vec<CommunityId> {
        let mut ids = Vec::new();
        for a in &self.attestations {
            if let Some(c) = a.community_id() {
                if !ids.contains(&c) {
                    ids.push(c);
                }
            }
        }
        ids
    }
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub use mycelia_core::registry::domain_audience as audience_domain;

pub fn domain_scope(domain: DomainId) -> [u8; 16] {
    domain_audience(domain)
}
