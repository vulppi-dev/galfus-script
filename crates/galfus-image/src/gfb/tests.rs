use super::*;
use crate::ConstantPool;

#[test]
fn test_gfb_roundtrip() {
    let image = ModuleImage {
        name: "test".to_string(),
        constants: ConstantPool::default(),
        functions: Vec::new(),
        types: Vec::new(),
        struct_layouts: Vec::new(),
        choice_layouts: Vec::new(),
        imports: Vec::new(),
        exports: Vec::new(),
        init_func_idx: None,
    };

    let bytes = serialize_to_gfb(&image).unwrap();
    assert_eq!(&bytes[0..4], b"GFB\x00");

    let decoded = deserialize_from_gfb(&bytes).unwrap();
    assert_eq!(decoded.name, "test");
}

#[test]
fn test_gfb_invalid_magic() {
    let bytes = vec![0u8; 20];
    assert!(matches!(
        deserialize_from_gfb(&bytes),
        Err(GfbError::InvalidMagic)
    ));
}

#[test]
fn test_gfb_unsupported_version() {
    let mut bytes = vec![0u8; 20];
    bytes[0..4].copy_from_slice(b"GFB\x00");
    bytes[4..8].copy_from_slice(&2u32.to_le_bytes()); // Version 2
    assert!(matches!(
        deserialize_from_gfb(&bytes),
        Err(GfbError::UnsupportedVersion(2))
    ));
}

#[test]
fn test_gfb_corrupt_payload() {
    let image = ModuleImage {
        name: "test".to_string(),
        constants: ConstantPool::default(),
        functions: Vec::new(),
        types: Vec::new(),
        struct_layouts: Vec::new(),
        choice_layouts: Vec::new(),
        imports: Vec::new(),
        exports: Vec::new(),
        init_func_idx: None,
    };

    let mut bytes = serialize_to_gfb(&image).unwrap();
    // Corrupt one byte of the payload (at the end)
    let last_idx = bytes.len() - 1;
    bytes[last_idx] ^= 0xFF;

    assert!(matches!(
        deserialize_from_gfb(&bytes),
        Err(GfbError::CorruptedPayload { .. })
    ));
}
