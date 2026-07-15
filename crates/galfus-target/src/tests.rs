use super::*;

#[test]
fn default_provider_alias_points_to_native_target() {
    let _provider: DefaultTargetCapabilityProvider = NativeTarget;
}

#[test]
fn web_target_captures_writes_across_clones() {
    let target = WebTarget::new();
    let mut writer = target.clone();

    assert_eq!(
        writer.invoke(TargetCall::Write(b"hello")).unwrap(),
        TargetResult::Success
    );
    assert_eq!(
        writer.invoke(TargetCall::Write(b" world")).unwrap(),
        TargetResult::Success
    );

    assert_eq!(target.take_output(), b"hello world");
    assert_eq!(target.take_output(), b"");
}

#[test]
fn web_target_reads_eof() {
    let mut target = WebTarget::new();

    assert_eq!(
        target.invoke(TargetCall::Read).unwrap(),
        TargetResult::ReadByte(None)
    );
}
