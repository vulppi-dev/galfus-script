use super::*;

#[test]
fn test_std_io_source_not_empty() {
    assert!(!STD_IO_SOURCE.is_empty());
    assert!(STD_IO_SOURCE.contains("print"));
}
