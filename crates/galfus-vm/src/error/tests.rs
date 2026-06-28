use super::*;

#[test]
fn test_vm_panic_display() {
    let panic = VmPanic {
        error: VmError::DivisionByZero,
        stack_trace: vec![
            StackFrameInfo {
                function_name: "foo".to_string(),
                pc: 12,
            },
            StackFrameInfo {
                function_name: "main".to_string(),
                pc: 4,
            },
        ],
    };
    let display_str = format!("{}", panic);
    assert!(display_str.contains("VM Panic: Division by zero"));
    assert!(display_str.contains("  #0: foo (at PC 12)"));
    assert!(display_str.contains("  #1: main (at PC 4)"));
}
