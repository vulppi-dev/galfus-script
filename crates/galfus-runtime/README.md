# galfus-runtime

`galfus-runtime` defines the execution environment, module registration, loading boundaries, and concurrency (logical thread) tracking.

## Responsibilities

- **ModuleRegistry**: Tracks active loaded modules and their references in the runtime.
- **RuntimeLoader**: Manages loading of `ModuleImage` formats into the registry.
- **LogicalThread**: Represents a virtual thread of execution, state, and concurrency tracking.
- **Runtime Execution Context**: Aggregates registries and the optional host providers supplied for an execution.

The runtime does not select a target or construct platform adapters. It passes
the optional providers received from `galfus-workspace` to the VM only when an
entry is executed.
