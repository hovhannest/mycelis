//! Compact Ed25519 membership attestations (MVP).
//!
//! Wire size budget (excl. transport): ≤ 256 bytes.

use crate::ids::{CommunityId, DomainId, NodeId, HASH_LEN};
use crate::wire::{DecodeError, Decoder, Encoder};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::OsRng;

/// Wire version for attestation records.
pub const ATTESTATION_VERSION: u8 = 1;

/// Max encoded attestation size (LoRa budget).
pub const MAX_ATTESTATION_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Scope {
    Domain = 1,
    Community = 2,
}

impl Scope {
    fn from_u8(v: u8) -> Result<Self, DecodeError> {
        match v {
            1 => Ok(Self::Domain),
            2 => Ok(Self::Community),
            _ => Err(DecodeError::InvalidEnum),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capability(pub u32);

impl Capability {
    pub const MEMBER: Self = Self(1 << 0);
    pub const OWNER: Self = Self(1 << 1);
    pub const ADVERTISE: Self = Self(1 << 2);
    pub const GATEWAY: Self = Self(1 << 3);

    pub fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestationError {
    Wire(DecodeError),
    BadSignature,
    Expired,
    NotYetValid,
    TooLarge,
    Crypto,
}

#[cfg(feature = "std")]
impl std::fmt::Display for AttestationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wire(e) => write!(f, "wire: {e:?}"),
            Self::BadSignature => write!(f, "bad signature"),
            Self::Expired => write!(f, "expired"),
            Self::NotYetValid => write!(f, "not yet valid"),
            Self::TooLarge => write!(f, "attestation exceeds size budget"),
            Self::Crypto => write!(f, "crypto error"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attestation {
    pub version: u8,
    pub issuer: NodeId,
    pub subject: NodeId,
    pub scope: Scope,
    pub scope_id: [u8; HASH_LEN],
    pub caps: Capability,
    pub not_before: u64,
    pub not_after: u64,
    /// Optional parent signature bytes for attenuation chain (64 bytes when present).
    pub parent_sig: Option<[u8; 64]>,
    pub sig: [u8; 64],
}

impl Attestation {
    pub fn domain_id(&self) -> Option<DomainId> {
        if self.scope == Scope::Domain {
            Some(DomainId::new(self.scope_id))
        } else {
            None
        }
    }

    pub fn community_id(&self) -> Option<CommunityId> {
        if self.scope == Scope::Community {
            Some(CommunityId::new(self.scope_id))
        } else {
            None
        }
    }

    fn encode_body(&self, enc: &mut Encoder<'_>) -> Result<(), DecodeError> {
        enc.write_u8(self.version)?;
        self.issuer.encode(enc)?;
        self.subject.encode(enc)?;
        enc.write_u8(self.scope as u8)?;
        enc.write_bytes(&self.scope_id)?;
        enc.write_u32(self.caps.0)?;
        enc.write_u64(self.not_before)?;
        enc.write_u64(self.not_after)?;
        match &self.parent_sig {
            Some(p) => {
                enc.write_u8(1)?;
                enc.write_bytes(p)?;
            }
            None => enc.write_u8(0)?,
        }
        Ok(())
    }

    pub fn signing_bytes(
        &self,
    ) -> Result<heapless::Vec<u8, MAX_ATTESTATION_BYTES>, AttestationError> {
        let mut buf = [0u8; MAX_ATTESTATION_BYTES];
        let mut enc = Encoder::new(&mut buf);
        self.encode_body(&mut enc).map_err(AttestationError::Wire)?;
        let n = enc.position();
        let mut out = heapless::Vec::new();
        out.extend_from_slice(&buf[..n])
            .map_err(|_| AttestationError::TooLarge)?;
        Ok(out)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn issue(
        signing_key: &SigningKey,
        issuer: NodeId,
        subject: NodeId,
        scope: Scope,
        scope_id: [u8; HASH_LEN],
        caps: Capability,
        not_before: u64,
        not_after: u64,
        parent_sig: Option<[u8; 64]>,
    ) -> Result<Self, AttestationError> {
        let mut att = Self {
            version: ATTESTATION_VERSION,
            issuer,
            subject,
            scope,
            scope_id,
            caps,
            not_before,
            not_after,
            parent_sig,
            sig: [0u8; 64],
        };
        let body = att.signing_bytes()?;
        let sig = signing_key.sign(&body);
        att.sig = sig.to_bytes();
        let encoded_len = att.encode_len()?;
        if encoded_len > MAX_ATTESTATION_BYTES {
            return Err(AttestationError::TooLarge);
        }
        Ok(att)
    }

    pub fn encode_len(&self) -> Result<usize, AttestationError> {
        let mut buf = [0u8; MAX_ATTESTATION_BYTES];
        let n = self.encode_to(&mut buf)?;
        Ok(n)
    }

    pub fn encode_to(&self, buf: &mut [u8]) -> Result<usize, AttestationError> {
        let mut enc = Encoder::new(buf);
        self.encode_body(&mut enc).map_err(AttestationError::Wire)?;
        enc.write_bytes(&self.sig).map_err(AttestationError::Wire)?;
        Ok(enc.position())
    }

    pub fn decode(buf: &[u8]) -> Result<Self, AttestationError> {
        let mut dec = Decoder::new(buf);
        let version = dec.read_u8().map_err(AttestationError::Wire)?;
        if version != ATTESTATION_VERSION {
            return Err(AttestationError::Wire(DecodeError::InvalidVersion));
        }
        let issuer = NodeId::decode(&mut dec).map_err(AttestationError::Wire)?;
        let subject = NodeId::decode(&mut dec).map_err(AttestationError::Wire)?;
        let scope = Scope::from_u8(dec.read_u8().map_err(AttestationError::Wire)?)
            .map_err(AttestationError::Wire)?;
        let mut scope_id = [0u8; HASH_LEN];
        dec.read_exact(&mut scope_id)
            .map_err(AttestationError::Wire)?;
        let caps = Capability(dec.read_u32().map_err(AttestationError::Wire)?);
        let not_before = dec.read_u64().map_err(AttestationError::Wire)?;
        let not_after = dec.read_u64().map_err(AttestationError::Wire)?;
        let parent_flag = dec.read_u8().map_err(AttestationError::Wire)?;
        let parent_sig = if parent_flag == 1 {
            let mut p = [0u8; 64];
            dec.read_exact(&mut p).map_err(AttestationError::Wire)?;
            Some(p)
        } else if parent_flag == 0 {
            None
        } else {
            return Err(AttestationError::Wire(DecodeError::InvalidEnum));
        };
        let mut sig = [0u8; 64];
        dec.read_exact(&mut sig).map_err(AttestationError::Wire)?;
        Ok(Self {
            version,
            issuer,
            subject,
            scope,
            scope_id,
            caps,
            not_before,
            not_after,
            parent_sig,
            sig,
        })
    }

    pub fn verify(&self, issuer_vk: &VerifyingKey, now: u64) -> Result<(), AttestationError> {
        if now < self.not_before {
            return Err(AttestationError::NotYetValid);
        }
        if now > self.not_after {
            return Err(AttestationError::Expired);
        }
        let body = self.signing_bytes()?;
        let sig = Signature::from_bytes(&self.sig);
        issuer_vk
            .verify(&body, &sig)
            .map_err(|_| AttestationError::BadSignature)?;
        Ok(())
    }

    /// Verify attenuated token: own sig valid AND parent_sig matches parent attestation's sig.
    pub fn verify_attenuated(
        &self,
        issuer_vk: &VerifyingKey,
        parent: &Attestation,
        now: u64,
    ) -> Result<(), AttestationError> {
        self.verify(issuer_vk, now)?;
        match &self.parent_sig {
            Some(p) if p == &parent.sig => Ok(()),
            _ => Err(AttestationError::BadSignature),
        }
    }
}

/// Derive a NodeId from a verifying key (first HASH_LEN bytes of SHA-512 of key bytes).
pub fn node_id_from_verifying_key(vk: &VerifyingKey) -> NodeId {
    use sha2::{Digest, Sha512};
    let mut hasher = Sha512::new();
    hasher.update(vk.as_bytes());
    let digest = hasher.finalize();
    let mut id = [0u8; HASH_LEN];
    id.copy_from_slice(&digest[..HASH_LEN]);
    NodeId::new(id)
}

pub fn generate_signing_key() -> SigningKey {
    SigningKey::generate(&mut OsRng)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_verify_roundtrip() {
        let sk = generate_signing_key();
        let vk = sk.verifying_key();
        let issuer = node_id_from_verifying_key(&vk);
        let subject = NodeId::new([9u8; HASH_LEN]);
        let domain = [7u8; HASH_LEN];
        let att = Attestation::issue(
            &sk,
            issuer,
            subject,
            Scope::Domain,
            domain,
            Capability::MEMBER.union(Capability::ADVERTISE),
            100,
            1_000_000,
            None,
        )
        .unwrap();
        assert!(att.encode_len().unwrap() <= MAX_ATTESTATION_BYTES);
        att.verify(&vk, 500).unwrap();
        assert_eq!(att.verify(&vk, 50), Err(AttestationError::NotYetValid));
        assert_eq!(att.verify(&vk, 2_000_000), Err(AttestationError::Expired));

        let mut buf = [0u8; MAX_ATTESTATION_BYTES];
        let n = att.encode_to(&mut buf).unwrap();
        let decoded = Attestation::decode(&buf[..n]).unwrap();
        assert_eq!(decoded, att);
        decoded.verify(&vk, 500).unwrap();
    }

    #[test]
    fn wrong_issuer_rejected() {
        let sk = generate_signing_key();
        let other = generate_signing_key();
        let vk = sk.verifying_key();
        let issuer = node_id_from_verifying_key(&vk);
        let att = Attestation::issue(
            &sk,
            issuer,
            NodeId::new([1u8; HASH_LEN]),
            Scope::Domain,
            [2u8; HASH_LEN],
            Capability::MEMBER,
            0,
            u64::MAX,
            None,
        )
        .unwrap();
        assert_eq!(
            att.verify(&other.verifying_key(), 1),
            Err(AttestationError::BadSignature)
        );
    }

    #[test]
    fn attenuated_chain() {
        let owner_sk = generate_signing_key();
        let owner_vk = owner_sk.verifying_key();
        let owner = node_id_from_verifying_key(&owner_vk);
        let domain = [3u8; HASH_LEN];
        let parent = Attestation::issue(
            &owner_sk,
            owner,
            NodeId::new([4u8; HASH_LEN]),
            Scope::Domain,
            domain,
            Capability::MEMBER.union(Capability::OWNER),
            0,
            u64::MAX,
            None,
        )
        .unwrap();
        let child = Attestation::issue(
            &owner_sk,
            owner,
            NodeId::new([5u8; HASH_LEN]),
            Scope::Domain,
            domain,
            Capability::MEMBER,
            0,
            u64::MAX,
            Some(parent.sig),
        )
        .unwrap();
        child.verify_attenuated(&owner_vk, &parent, 1).unwrap();
    }
}
