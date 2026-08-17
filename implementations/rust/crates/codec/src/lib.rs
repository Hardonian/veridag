//! VCE-1: Veridag Canonical Encoding (spec 03-canonical-encoding).
//!
//! Deterministic, implementation-independent byte encoding. Equivalent semantic
//! objects have exactly one canonical byte representation. Decoders reject any
//! non-canonical input. No variable-length integers; big-endian fixed width.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use thiserror::Error;

/// Maximum byte-string / string length on attacker-facing paths (2^20).
pub const MAX_BYTES: usize = 1 << 20;
/// Maximum sequence length (2^16).
pub const MAX_SEQ: usize = 1 << 16;
/// Maximum nesting depth.
pub const MAX_DEPTH: usize = 16;

/// Errors returned by the VCE-1 decoder.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Input ended before a complete value was read.
    #[error("truncated input")]
    Truncated,
    /// Trailing bytes after the top-level value.
    #[error("trailing bytes")]
    TrailingBytes,
    /// A bool was not 0x00 or 0x01.
    #[error("invalid bool tag")]
    InvalidBool,
    /// An Option tag was not 0x00 or 0x01.
    #[error("invalid option tag")]
    InvalidOptionTag,
    /// An enum variant index is not defined by the protocol.
    #[error("unknown variant: {0}")]
    UnknownVariant(u8),
    /// A declared length or count exceeds the protocol limit.
    #[error("limit exceeded")]
    LimitExceeded,
    /// A declared length exceeds the remaining input.
    #[error("declared length exceeds remaining input")]
    LengthOverflow,
    /// A string field contained invalid UTF-8.
    #[error("invalid utf-8")]
    InvalidUtf8,
    /// Map keys were not strictly sorted or were duplicated.
    #[error("map keys not canonical")]
    NonCanonicalMap,
    /// Nesting depth exceeded.
    #[error("nesting too deep")]
    TooDeep,
}

/// A canonical encoder. Values are appended in field order.
#[derive(Default)]
pub struct Encoder {
    buf: Vec<u8>,
}

impl Encoder {
    /// Create an empty encoder.
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// Consume and return the encoded bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    /// Borrow the encoded bytes so far.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    /// Encode a u8.
    pub fn u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    /// Encode a u16 big-endian.
    pub fn u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    /// Encode a u32 big-endian.
    pub fn u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    /// Encode a u64 big-endian.
    pub fn u64(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    /// Encode a bool as 0x00/0x01.
    pub fn bool(&mut self, v: bool) {
        self.buf.push(u8::from(v));
    }

    /// Encode raw fixed-width bytes (no length prefix).
    pub fn fixed(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Encode a length-prefixed byte string.
    pub fn bytes(&mut self, bytes: &[u8]) {
        self.u32(bytes.len() as u32);
        self.buf.extend_from_slice(bytes);
    }

    /// Encode a UTF-8 string.
    pub fn string(&mut self, s: &str) {
        self.bytes(s.as_bytes());
    }

    /// Encode a sequence by count and a per-element closure.
    pub fn seq<T>(&mut self, items: &[T], mut f: impl FnMut(&mut Self, &T)) {
        self.u32(items.len() as u32);
        for it in items {
            f(self, it);
        }
    }

    /// Encode an Option.
    pub fn option<T>(&mut self, v: &Option<T>, mut f: impl FnMut(&mut Self, &T)) {
        match v {
            None => self.u8(0),
            Some(x) => {
                self.u8(1);
                f(self, x);
            }
        }
    }
}

/// A canonical decoder over an in-memory slice.
pub struct Decoder<'a> {
    input: &'a [u8],
    pos: usize,
    depth: usize,
}

impl<'a> Decoder<'a> {
    /// Create a decoder over `input`.
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            pos: 0,
            depth: 0,
        }
    }

    /// Remaining bytes.
    pub fn remaining(&self) -> usize {
        self.input.len() - self.pos
    }

    /// Finish decoding, rejecting trailing bytes.
    pub fn finish(self) -> Result<(), DecodeError> {
        if self.remaining() != 0 {
            return Err(DecodeError::TrailingBytes);
        }
        Ok(())
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        if self.remaining() < n {
            return Err(DecodeError::Truncated);
        }
        let s = &self.input[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    /// Decode a u8.
    pub fn u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.take(1)?[0])
    }

    /// Decode a u16 big-endian.
    pub fn u16(&mut self) -> Result<u16, DecodeError> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    /// Decode a u32 big-endian.
    pub fn u32(&mut self) -> Result<u32, DecodeError> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Decode a u64 big-endian.
    pub fn u64(&mut self) -> Result<u64, DecodeError> {
        let b = self.take(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(u64::from_be_bytes(a))
    }

    /// Decode a bool, rejecting non-{0,1} tags.
    pub fn bool(&mut self) -> Result<bool, DecodeError> {
        match self.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(DecodeError::InvalidBool),
        }
    }

    /// Decode exactly N fixed bytes.
    pub fn fixed<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let b = self.take(N)?;
        let mut a = [0u8; N];
        a.copy_from_slice(b);
        Ok(a)
    }

    /// Decode a length-prefixed byte string with a limit.
    pub fn bytes(&mut self, max: usize) -> Result<&'a [u8], DecodeError> {
        let len = self.u32()? as usize;
        if len > max {
            return Err(DecodeError::LimitExceeded);
        }
        if len > self.remaining() {
            return Err(DecodeError::LengthOverflow);
        }
        self.take(len)
    }

    /// Decode a UTF-8 string with a byte limit.
    pub fn string(&mut self, max: usize) -> Result<&'a str, DecodeError> {
        let b = self.bytes(max)?;
        core::str::from_utf8(b).map_err(|_| DecodeError::InvalidUtf8)
    }

    /// Decode a sequence into a Vec with count limit.
    pub fn seq<T>(
        &mut self,
        max: usize,
        mut f: impl FnMut(&mut Self) -> Result<T, DecodeError>,
    ) -> Result<Vec<T>, DecodeError> {
        let count = self.u32()? as usize;
        if count > max {
            return Err(DecodeError::LimitExceeded);
        }
        // Depth guard for nested structures.
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(DecodeError::TooDeep);
        }
        let mut out = Vec::with_capacity(count.min(1024));
        for _ in 0..count {
            out.push(f(self)?);
        }
        self.depth -= 1;
        Ok(out)
    }

    /// Decode an Option.
    pub fn option<T>(
        &mut self,
        mut f: impl FnMut(&mut Self) -> Result<T, DecodeError>,
    ) -> Result<Option<T>, DecodeError> {
        match self.u8()? {
            0 => Ok(None),
            1 => Ok(Some(f(self)?)),
            _ => Err(DecodeError::InvalidOptionTag),
        }
    }
}

/// Trait for types with a VCE-1 canonical encoding.
pub trait Encode {
    /// Append the canonical encoding to `enc`.
    fn encode(&self, enc: &mut Encoder);

    /// Return the canonical encoding as bytes.
    fn to_bytes(&self) -> Vec<u8> {
        let mut e = Encoder::new();
        self.encode(&mut e);
        e.into_bytes()
    }
}

/// Trait for types with a VCE-1 canonical decoder.
pub trait Decode: Sized {
    /// Decode from `dec`. Implementations must reject non-canonical input.
    fn decode(dec: &mut Decoder<'_>) -> Result<Self, DecodeError>;
}

/// Encode then decode a value, asserting canonical round-trip in tests.
#[cfg(test)]
pub fn roundtrip<T: Encode + Decode>(v: &T) -> Result<T, DecodeError> {
    let bytes = v.to_bytes();
    let mut d = Decoder::new(&bytes);
    let out = T::decode(&mut d)?;
    d.finish()?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u64_big_endian() {
        let mut e = Encoder::new();
        e.u64(0x0102_0304_0506_0708);
        assert_eq!(e.into_bytes(), vec![1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn bool_rejects_noncanonical() {
        let mut d = Decoder::new(&[2u8]);
        assert_eq!(d.bool(), Err(DecodeError::InvalidBool));
    }

    #[test]
    fn option_rejects_bad_tag() {
        let mut d = Decoder::new(&[7u8]);
        let r: Result<Option<u8>, _> = d.option(|dd| dd.u8());
        assert_eq!(r, Err(DecodeError::InvalidOptionTag));
    }

    #[test]
    fn trailing_bytes_rejected() {
        let mut d = Decoder::new(&[1u8, 2u8]);
        let _ = d.u8().unwrap();
        assert_eq!(d.finish(), Err(DecodeError::TrailingBytes));
    }

    #[test]
    fn bytes_limit_enforced_before_read() {
        // declared length 0xFFFFFFFF must be rejected as LimitExceeded (limit),
        // not by attempting to read.
        let mut d = Decoder::new(&[0xFF, 0xFF, 0xFF, 0xFF]);
        assert_eq!(d.bytes(MAX_BYTES), Err(DecodeError::LimitExceeded));
    }

    #[test]
    fn length_overflow_detected() {
        // declared length 5 but only 1 byte present.
        let mut d = Decoder::new(&[0, 0, 0, 5, 0xAA]);
        assert_eq!(d.bytes(MAX_BYTES), Err(DecodeError::LengthOverflow));
    }

    #[test]
    fn seq_count_limit() {
        let mut d = Decoder::new(&[0x00, 0x01, 0x00, 0x01]); // count 65537 > MAX_SEQ
        let r: Result<Vec<u8>, _> = d.seq(MAX_SEQ, |dd| dd.u8());
        assert_eq!(r, Err(DecodeError::LimitExceeded));
    }
}
