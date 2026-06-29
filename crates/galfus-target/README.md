# galfus-target

`galfus-target` specifies the low-level target call ABI and capability provider interface, decoupling the virtual machine from host-specific system calls.

## Responsibilities

- **TargetCall**: Explicit low-level system call commands (e.g., standard input/output).
- **TargetCapabilityProvider**: Trait to run custom system call providers, allowing virtualization or sandboxing of the virtual machine environment.
- **DefaultTargetCapabilityProvider**: A concrete implementation forwarding calls to standard system I/O streams.
