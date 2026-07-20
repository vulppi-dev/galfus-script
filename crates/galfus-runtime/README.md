# galfus-runtime

`galfus-runtime` defines the execution environment, module state, and execution context mapping over the `BytecodeGraph`.

## Responsibilities

- **Runtime Execution State**: Maintains global variables, module initialization status, and the runtime heap.
- **Runtime Context**: Aggregates the `BytecodeGraph` and the optional host providers supplied for an execution.
- **Host Integration**: Receives `Providers` from the workspace and routes capability requests to the host platform.

The runtime does not copy, rebuild, or deduplicate the `BytecodeGraph`. It merely maintains the live execution state associated with it.
