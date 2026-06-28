# galfus-runner

`galfus-runner` provides the high-level orchestration pipeline to build, link, and execute Galfus applications and workspaces.

## Responsibilities

- **Compilation Orchestrator**: Coordinates frontend resolution, MIR building, lowering, bytecode validation, and serialization.
- **Linker**: Aggregates separate compiled workspace modules into a single `ModuleImage` by resolving cross-module references and rewriting indices.
- **VM Execution Runner**: Loads compiled GFB images into the virtual machine and executes program entry points.
