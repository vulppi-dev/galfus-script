use super::*;

#[test]
fn test_std_io_source_not_empty() {
    assert!(!STD_IO_SOURCE.is_empty());
    assert!(STD_IO_SOURCE.contains("print"));
    assert!(STD_IO_SOURCE.contains("read"));
}

#[test]
fn test_text_source_not_empty() {
    assert!(!TEXT_SOURCE.is_empty());
    assert!(TEXT_SOURCE.contains("length"));
    assert!(TEXT_SOURCE.contains("concat"));
}

#[test]
fn test_format_source_not_empty() {
    assert!(!FORMAT_SOURCE.is_empty());
    assert!(FORMAT_SOURCE.contains("stringify"));
    assert!(FORMAT_SOURCE.contains("parse"));
    assert!(FORMAT_SOURCE.contains("Result"));
}

#[test]
fn test_format_ansi_source_not_empty() {
    assert!(!FORMAT_ANSI_SOURCE.is_empty());
    assert!(FORMAT_ANSI_SOURCE.contains("Style"));
    assert!(FORMAT_ANSI_SOURCE.contains("apply"));
    assert!(FORMAT_ANSI_SOURCE.contains("red"));
}
