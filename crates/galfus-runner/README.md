# galfus-runner

`galfus-runner` provides the high-level orchestration pipeline to build and execute Galfus applications and workspaces.

## Responsibilities

- **Compilation Orchestrator**: Coordinates frontend resolution, MIR building, lowering, bytecode validation, and serialization.
- **Bytecode Graph Orchestrator**: Preserves compiled workspace modules in a `BytecodeGraph`; cross-module references remain import slots resolved by the runtime.
- **VM Execution Runner**: Passes the compiled in-memory `BytecodeGraph` into the virtual machine and executes program entry points.
