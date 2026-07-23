//! Simple proof-of-work stamp for announce/registry writes (Phase 5.1).

use crate::wire::{DecodeError, Decoder, Encoder};

#[cfg(feature = "crypto")]
use sha2::{Digest, Sha256};

pub const POW_VERSION: u8 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowStamp {
    pub version: u8,
    pub difficulty: u8,
    pub nonce: u64,
}

impl PowStamp {
    pub fn encode(&self, enc: &mut Encoder<'_>) -> Result<(), DecodeError> {
        enc.write_u8(self.version)?;
        enc.write_u8(self.difficulty)?;
        enc.write_u64(self.nonce)?;
        Ok(())
    }

    pub fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
        let version = dec.read_u8()?;
        if version != POW_VERSION {
            return Err(DecodeError::InvalidVersion);
        }
        Ok(Self {
            version,
            difficulty: dec.read_u8()?,
            nonce: dec.read_u64()?,
        })
    }
}

/// Leading zero bits required in SHA-256(payload || nonce_le).
#[cfg(feature = "crypto")]
pub fn verify_pow(payload: &[u8], stamp: &PowStamp) -> bool {
    let mut hasher = Sha256::new();
    hasher.update(payload);
    hasher.update(stamp.nonce.to_le_bytes());
    let digest = hasher.finalize();
    leading_zero_bits(&digest) >= stamp.difficulty as u32
}

#[cfg(feature = "crypto")]
pub fn mine_pow(payload: &[u8], difficulty: u8) -> PowStamp {
    let mut nonce = 0u64;
    loop {
        let stamp = PowStamp {
            version: POW_VERSION,
            difficulty,
            nonce,
        };
        if verify_pow(payload, &stamp) {
            return stamp;
        }
        nonce = nonce.wrapping_add(1);
    }
}

pub fn leading_zero_bits(bytes: &[u8]) -> u32 {
    let mut bits = 0u32;
    for b in bytes {
        if *b == 0 {
            bits += 8;
        } else {
            bits += b.leading_zeros();
            break;
        }
    }
    bits
}

#[cfg(all(test, feature = "crypto"))]
mod tests {
    use super::*;

    #[test]
    fn mine_and_verify() {
        let payload = b"mycelis-announce";
        let stamp = mine_pow(payload, 8);
        assert!(verify_pow(payload, &stamp));
        let bad = PowStamp {
            difficulty: 8,
            nonce: stamp.nonce ^ 1,
            version: POW_VERSION,
        };
        assert!(!verify_pow(payload, &bad));
    }
}
