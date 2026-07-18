# galfus-vm

`galfus-vm` implements the core virtual machine, register-based interpreter, call stack, heap objects representation, and ownership tracking system.

## Responsibilities

- **VirtualMachine**: Evaluates and interprets bytecode instructions, tracking execution registers and call frames.
- **Call Frame**: Manages local variables, function calls, and arguments return values.
- **Ownership Graph**: Implements deterministic resource management, tracking owners, weak links, and cycles to automatically invalidate and deallocate heap objects.
- **Panic Model**: Standard VM errors and unwinding logic.
- **Host I/O Boundary**: Resolves `std/io` through the optional `IoProvider`
  supplied for the current execution. Missing providers are reported only when
  the corresponding I/O instruction executes.
