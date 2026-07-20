# galfus-runner

`galfus-runner` provides the high-level orchestration pipeline to build, link, and execute Galfus applications and workspaces.

## Responsibilities

- **Compilation Orchestrator**: Coordinates frontend resolution, MIR building, lowering, bytecode validation, and serialization.
- **Linker**: Aggregates separate compiled workspace modules into a single `BytecodeModule` by resolving cross-module references and rewriting indices.
- **VM Execution Runner**: Passes the compiled in-memory `BytecodeModule` into the virtual machine and executes program entry points.
