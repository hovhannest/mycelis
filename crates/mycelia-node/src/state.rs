use ed25519_dalek::{SigningKey, VerifyingKey};
use mycelia_core::attestation::{
    generate_signing_key, node_id_from_verifying_key, Attestation, Capability, Scope,
};
use mycelia_core::domain::{apply_attestation, CommunityStore, DomainStore};
use mycelia_core::ids::{DomainId, NodeId, ServiceId};
use mycelia_core::peer::PeerCache;
use mycelia_core::registry::{
    domain_audience, AttestationView, ServiceRecord, ServiceRegistry, Visibility,
    SERVICE_RECORD_VERSION,
};
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
}

impl NodeState {
    pub fn bootstrap(data_dir: &Path) -> anyhow::Result<Self> {
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
        Ok(Self {
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
        })
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
        domain
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
        att
    }

    pub fn accept_attestation(&mut self, att: Attestation) {
        apply_attestation(&mut self.domains, &att);
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
        // Sign body without sig field using owner key (HMAC-style: Ed25519 over encoded unsigned).
        let mut body = [0u8; 512];
        let mut tmp = record.clone();
        tmp.sig = [0u8; 64];
        let n = tmp.encode_to(&mut body).unwrap_or(0);
        use ed25519_dalek::Signer;
        let sig = self.signing_key.sign(&body[..n]);
        record.sig = sig.to_bytes();
        self.registry.upsert(record.clone());
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
        // For local queries, if viewer is self use our atts; else filter by provided atts only when remote.
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
