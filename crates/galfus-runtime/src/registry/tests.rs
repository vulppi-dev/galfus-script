use super::*;
use galfus_vm::thread::ThreadState;

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

#[test]
fn registry_keeps_the_mailbox_and_key_while_a_thread_is_running() {
    let id = ThreadId::from_executor(1).expect("non-zero thread ID");
    let mut thread = VirtualThread::new();
    thread.key = Some("worker".to_string());
    let mut registry = ThreadRegistry::new();

    registry.register(id, thread);
    let mailbox = registry.get_mailbox(id).expect("mailbox is registered");
    let _running_thread = registry.take(id).expect("thread is available to run");

    mailbox.lock().unwrap().push_back((7, VmValue::Int32(42)));

    assert!(registry.contains(id));
    assert_eq!(registry.lookup_key("worker"), Some(id));
    assert_eq!(registry.state(id), Some(ThreadState::Created));
    assert_eq!(registry.get_mailbox(id).unwrap().lock().unwrap().len(), 1);
}

#[test]
fn registry_tracks_state_after_the_thread_body_is_taken() {
    let id = ThreadId::from_executor(1).expect("non-zero thread ID");
    let mut registry = ThreadRegistry::new();

    registry.register(id, VirtualThread::new());
    assert!(registry.mark_running(id));
    let _running_thread = registry.take(id).expect("thread is available to run");
    assert!(registry.mark_exited(id, 7));

    assert_eq!(registry.state(id), Some(ThreadState::Exited(7)));
}
