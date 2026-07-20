# galfus-runtime

`galfus-runtime` defines entrypoint validation and execution orchestration over
a borrowed `BytecodeGraph`.

## Responsibilities

- **Entrypoint Execution**: Validates and invokes exported module entries.
- **Runtime Context**: Passes the `BytecodeGraph` and optional host providers to the VM for an execution.
- **Host Integration**: Receives `Providers` from the workspace and routes capability requests to the host platform.

The runtime does not copy, rebuild, or deduplicate the `BytecodeGraph`.
Per-module runtime state is planned; current VM state includes one global-slot
vector, the heap, and call frames.
