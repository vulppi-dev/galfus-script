# galfus-runtime

`galfus-runtime` defines entrypoint validation and execution orchestration over
a borrowed `BytecodeGraph`.

## Responsibilities

- **Entrypoint Execution**: Validates and invokes exported module entries.
- **Runtime Context**: Is created from a borrowed `BytecodeGraph` and optional host providers, then passes both to the VM for one execution.
- **Host Integration**: Receives `Providers` from an embedding host or workspace and routes capability requests to the host platform.

The runtime does not copy, rebuild, or deduplicate the `BytecodeGraph`.
Current VM state includes a global-slot vector and initialization flag for each
module, plus the heap and call frames.

When available, `ExecutionMetadata` maps a function instruction offset to its
source span. Panics always retain the module ID, function index, and instruction
offset; formatting enriches them with the optional source span.
