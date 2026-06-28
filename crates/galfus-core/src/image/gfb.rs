use super::ModuleImage;
use std::error::Error;
use std::fmt;
use std::io;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub enum GfbError {
    InvalidMagic,
    UnsupportedVersion(u32),
    CorruptedPayload { expected: u32, actual: u32 },
    Serialization(postcard::Error),
    Deserialization(postcard::Error),
    Io(io::Error),
}

impl fmt::Display for GfbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMagic => write!(f, "Invalid magic bytes in GFB file"),
            Self::UnsupportedVersion(v) => write!(f, "Unsupported GFB file version: {}", v),
            Self::CorruptedPayload { expected, actual } => {
                write!(
                    f,
                    "GFB payload corruption detected (expected checksum: {:x}, actual: {:x})",
                    expected, actual
                )
            }
            Self::Serialization(e) => write!(f, "GFB serialization failed: {}", e),
            Self::Deserialization(e) => write!(f, "GFB deserialization failed: {}", e),
            Self::Io(e) => write!(f, "GFB IO error: {}", e),
        }
    }
}

impl Error for GfbError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Serialization(e) => Some(e),
            Self::Deserialization(e) => Some(e),
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

fn calculate_fnv1a(bytes: &[u8]) -> u32 {
    let mut hash = 2166136261u32;
    for &byte in bytes {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    hash
}

pub fn serialize_to_gfb(image: &ModuleImage) -> Result<Vec<u8>, GfbError> {
    let payload = postcard::to_allocvec(image).map_err(GfbError::Serialization)?;
    let checksum = calculate_fnv1a(&payload);

    let mut out = Vec::with_capacity(16 + payload.len());
    out.extend_from_slice(b"GFB\x00");
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&checksum.to_le_bytes());
    out.extend_from_slice(&payload);

    Ok(out)
}

pub fn deserialize_from_gfb(bytes: &[u8]) -> Result<ModuleImage, GfbError> {
    if bytes.len() < 16 {
        return Err(GfbError::InvalidMagic);
    }

    let magic = &bytes[0..4];
    if magic != b"GFB\x00" {
        return Err(GfbError::InvalidMagic);
    }

    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if version != 1 {
        return Err(GfbError::UnsupportedVersion(version));
    }

    let payload_len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let expected_checksum = u32::from_le_bytes(bytes[12..16].try_into().unwrap());

    if bytes.len() != 16 + payload_len {
        return Err(GfbError::Io(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "GFB file length mismatch",
        )));
    }

    let payload = &bytes[16..];
    let actual_checksum = calculate_fnv1a(payload);
    if actual_checksum != expected_checksum {
        return Err(GfbError::CorruptedPayload {
            expected: expected_checksum,
            actual: actual_checksum,
        });
    }

    postcard::from_bytes(payload).map_err(GfbError::Deserialization)
}
