# galfus-jit

`galfus-jit` will implement the Just-In-Time (JIT) compilation backend for Galfus.

## Responsibilities

- **MIR-to-Native Lowering**: Translates the Medium-level Intermediate Representation (MIR) from `galfus-ir` into target-specific native machine instructions or an intermediate representation (e.g., Cranelift or LLVM).
- **Execution Engine**: Interacts with the virtual machine to dynamically compile hot paths (functions, loops) to native code.
- **Dynamic Linking**: Integrates JIT-compiled native structures and callbacks back into the VM interpreter call frames.
