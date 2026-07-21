use super::*;

#[test]
fn thread_ids_are_executor_owned_and_non_zero() {
    assert_eq!(ThreadId::from_executor(0), None);
    assert_ne!(ThreadId::from_executor(1), ThreadId::from_executor(2));
}

#[test]
fn registry_preserves_the_executor_assigned_identity() {
    let id = ThreadId::from_executor(42).expect("non-zero thread ID");
    let mut registry = ThreadRegistry::new();

    registry.register(id, VirtualThread::new());

    assert!(registry.get(id).is_some());
    assert_eq!(id.raw(), 42);
}
