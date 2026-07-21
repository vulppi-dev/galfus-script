use super::{ThreadState, VirtualThread};

#[test]
fn thread_state_exposes_its_lifecycle() {
    assert!(!ThreadState::Created.is_running());
    assert!(!ThreadState::Created.is_exited());
    assert_eq!(ThreadState::Created.exit_reason(), None);

    assert!(ThreadState::Running.is_running());
    assert!(!ThreadState::Running.is_exited());
    assert_eq!(ThreadState::Running.exit_reason(), None);

    assert!(!ThreadState::Exited(7).is_running());
    assert!(ThreadState::Exited(7).is_exited());
    assert_eq!(ThreadState::Exited(7).exit_reason(), Some(7));
}

#[test]
fn virtual_thread_only_allows_the_defined_state_transitions() {
    let mut thread = VirtualThread::new();

    assert!(!thread.mark_exited(1));
    assert!(thread.mark_running());
    assert!(!thread.mark_running());
    assert!(thread.mark_exited(7));
    assert!(!thread.mark_exited(8));
    assert_eq!(thread.state, ThreadState::Exited(7));
}
