//! Fixed-size identity and scope identifiers.

use crate::wire::{DecodeError, Decoder, Encoder};

/// Reticulum-style truncated identity / destination hash length (bytes).
pub const HASH_LEN: usize = 16;

macro_rules! id_type {
    ($name:ident, $len:expr, $tag:expr) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub [u8; $len]);

        impl $name {
            pub const LEN: usize = $len;

            pub const fn new(bytes: [u8; $len]) -> Self {
                Self(bytes)
            }

            pub fn from_slice(s: &[u8]) -> Option<Self> {
                if s.len() != $len {
                    return None;
                }
                let mut b = [0u8; $len];
                b.copy_from_slice(s);
                Some(Self(b))
            }

            pub fn as_bytes(&self) -> &[u8; $len] {
                &self.0
            }

            pub fn encode(&self, enc: &mut Encoder<'_>) -> Result<(), DecodeError> {
                enc.write_bytes(&self.0)
            }

            pub fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError> {
                let mut b = [0u8; $len];
                dec.read_exact(&mut b)?;
                Ok(Self(b))
            }
        }

        impl core::fmt::Debug for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, concat!(stringify!($name), "("))?;
                for (i, byte) in self.0.iter().enumerate() {
                    if i > 0 && i % 2 == 0 {
                        write!(f, ":")?;
                    }
                    write!(f, "{:02x}", byte)?;
                }
                write!(f, ")")
            }
        }

        #[cfg(feature = "std")]
        impl core::fmt::Display for $name {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                for byte in &self.0 {
                    write!(f, "{:02x}", byte)?;
                }
                Ok(())
            }
        }
    };
}

id_type!(NodeId, HASH_LEN, 1);
id_type!(DomainId, HASH_LEN, 2);
id_type!(CommunityId, HASH_LEN, 3);
id_type!(ServiceId, HASH_LEN, 4);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wire::{Decoder, Encoder};

    #[test]
    fn node_id_roundtrip() {
        let id = NodeId::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]);
        let mut buf = [0u8; 32];
        let mut enc = Encoder::new(&mut buf);
        id.encode(&mut enc).unwrap();
        let written = enc.position();
        let mut dec = Decoder::new(&buf[..written]);
        let out = NodeId::decode(&mut dec).unwrap();
        assert_eq!(id, out);
    }

    #[test]
    fn from_slice_rejects_wrong_len() {
        assert!(NodeId::from_slice(&[1, 2, 3]).is_none());
    }
}
