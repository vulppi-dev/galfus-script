use super::*;

#[test]
fn test_default_capability_provider_write() {
    let mut provider = NativeTarget;
    let res = provider.invoke(TargetCall::Write(b"hello"));
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), TargetResult::Success);
}
