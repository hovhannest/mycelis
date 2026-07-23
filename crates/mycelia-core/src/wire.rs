//! Little-endian binary codec helpers. Wire version is owned by higher-level types.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    Truncated,
    Overflow,
    InvalidVersion,
    InvalidEnum,
    InvalidLength,
    TrailingBytes,
}

#[cfg(feature = "std")]
impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated => write!(f, "truncated input"),
            Self::Overflow => write!(f, "buffer overflow"),
            Self::InvalidVersion => write!(f, "invalid version"),
            Self::InvalidEnum => write!(f, "invalid enum discriminant"),
            Self::InvalidLength => write!(f, "invalid length"),
            Self::TrailingBytes => write!(f, "trailing bytes"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

pub struct Encoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> Encoder<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn write_u8(&mut self, v: u8) -> Result<(), DecodeError> {
        if self.pos >= self.buf.len() {
            return Err(DecodeError::Overflow);
        }
        self.buf[self.pos] = v;
        self.pos += 1;
        Ok(())
    }

    pub fn write_u16(&mut self, v: u16) -> Result<(), DecodeError> {
        self.write_bytes(&v.to_le_bytes())
    }

    pub fn write_u32(&mut self, v: u32) -> Result<(), DecodeError> {
        self.write_bytes(&v.to_le_bytes())
    }

    pub fn write_u64(&mut self, v: u64) -> Result<(), DecodeError> {
        self.write_bytes(&v.to_le_bytes())
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), DecodeError> {
        if self.pos + bytes.len() > self.buf.len() {
            return Err(DecodeError::Overflow);
        }
        self.buf[self.pos..self.pos + bytes.len()].copy_from_slice(bytes);
        self.pos += bytes.len();
        Ok(())
    }

    /// Length-prefixed blob (u16 LE length).
    pub fn write_blob(&mut self, bytes: &[u8]) -> Result<(), DecodeError> {
        if bytes.len() > u16::MAX as usize {
            return Err(DecodeError::InvalidLength);
        }
        self.write_u16(bytes.len() as u16)?;
        self.write_bytes(bytes)
    }
}

pub struct Decoder<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Decoder<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    pub fn read_u8(&mut self) -> Result<u8, DecodeError> {
        if self.pos >= self.buf.len() {
            return Err(DecodeError::Truncated);
        }
        let v = self.buf[self.pos];
        self.pos += 1;
        Ok(v)
    }

    pub fn read_u16(&mut self) -> Result<u16, DecodeError> {
        let mut b = [0u8; 2];
        self.read_exact(&mut b)?;
        Ok(u16::from_le_bytes(b))
    }

    pub fn read_u32(&mut self) -> Result<u32, DecodeError> {
        let mut b = [0u8; 4];
        self.read_exact(&mut b)?;
        Ok(u32::from_le_bytes(b))
    }

    pub fn read_u64(&mut self) -> Result<u64, DecodeError> {
        let mut b = [0u8; 8];
        self.read_exact(&mut b)?;
        Ok(u64::from_le_bytes(b))
    }

    pub fn read_exact(&mut self, out: &mut [u8]) -> Result<(), DecodeError> {
        if self.pos + out.len() > self.buf.len() {
            return Err(DecodeError::Truncated);
        }
        out.copy_from_slice(&self.buf[self.pos..self.pos + out.len()]);
        self.pos += out.len();
        Ok(())
    }

    pub fn read_blob<'b>(&'b mut self) -> Result<&'a [u8], DecodeError>
    where
        'a: 'b,
    {
        let len = self.read_u16()? as usize;
        if self.pos + len > self.buf.len() {
            return Err(DecodeError::Truncated);
        }
        let slice = &self.buf[self.pos..self.pos + len];
        self.pos += len;
        Ok(slice)
    }

    pub fn finish(self) -> Result<(), DecodeError> {
        if self.pos != self.buf.len() {
            Err(DecodeError::TrailingBytes)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u64_roundtrip() {
        let mut buf = [0u8; 16];
        let pos = {
            let mut enc = Encoder::new(&mut buf);
            enc.write_u64(0x0123_4567_89ab_cdef).unwrap();
            enc.position()
        };
        let mut dec = Decoder::new(&buf[..pos]);
        assert_eq!(dec.read_u64().unwrap(), 0x0123_4567_89ab_cdef);
    }

    #[test]
    fn truncated_fails() {
        let mut dec = Decoder::new(&[1, 2]);
        assert_eq!(dec.read_u32(), Err(DecodeError::Truncated));
    }
}
