# galfus-target

`galfus-target` specifies the low-level target call ABI and capability provider interface, decoupling the virtual machine from host-specific system calls.

## Responsibilities

- **TargetCall**: Explicit low-level system call commands (e.g., standard input/output).
- **TargetCapabilityProvider**: Trait to run custom system call providers, allowing virtualization or sandboxing of the virtual machine environment.
- **NativeTarget**: Desktop/server implementation backed by standard input and output.
- **WebTarget**: In-memory implementation for playground and wasm use, capturing writes and returning EOF for reads.
- **DefaultTargetCapabilityProvider**: Backwards-compatible alias for `NativeTarget`.
