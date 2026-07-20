# Galfus Architecture Reference

## 1. Core Identity

Galfus Script is a typed, VM-first scripting language with a straightforward pipeline. The primary pipeline is:

```text
.gfs Source
    ↓
Frontend (SemanticGraph)
    ↓
Compiler (BytecodeGraphTransaction)
    ↓
Workspace (BytecodeGraph)
    ↓
Runtime (Execution State)
    ↓
VM (Instruction Execution)
```

**What is implemented:**
- Full compiler pipeline (parsing to bytecode)
- In-memory `BytecodeGraph` execution
- Deterministic VM and memory graph
- Optional Host Providers boundary

**What is NOT part of the current architecture (Historical or Planned):**
- **GFB (Galfus Bytecode File):** Removed. The graph exists purely in memory.
- **Bundler:** Not implemented.
- **Optimizer:** Not implemented.
- **Multithreading:** Not implemented.
- **Debugger:** Not implemented.
- **JIT Compilation:** Not implemented.

---

## 2. The Semantic Graph

The `SemanticGraph` represents the source-level meaning of the workspace.
It contains modules, symbols, typed references, and diagnostics.
The frontend processes source text and updates this graph.

---

## 3. The Bytecode Module

`BytecodeModule` is the isolated executable unit.
Each module contains its private and exported functions, globals, constants, layouts, and bytecode.
There is no global shared namespace. A variable without `export` belongs strictly to its module.

---

## 4. The Bytecode Graph

`BytecodeGraph` represents the complete executable program.
It contains multiple `BytecodeModule`s and their dependencies.
It is the only executable graph. The runtime does not rebuild or duplicate this graph.

Updates are transactional: the compiler produces a `BytecodeGraphTransaction` containing entire replaced modules. If validated, the transaction produces a new snapshot of the `BytecodeGraph`.

---

## 5. The Workspace

The `Workspace` owns the current architectural snapshots:
- Source state
- `SemanticGraph` snapshot
- `BytecodeGraph` snapshot

It manages the orchestration of the frontend, compiler, and provides an API for embedding.

---

## 6. Runtime and VM

The `Runtime` receives an `Arc<BytecodeGraph>` and optionally a set of `Providers`.
It maintains execution state (such as module globals, initialization status, and heap) but does not duplicate bytecode.

The `VM` executes bytecode instructions. It receives frames containing `ModuleId`, `FunctionId`, and `InstructionOffset`. Execution is implemented fundamentally via a `step` function that runs one instruction at a time.

---

## 7. Providers

Providers represent the boundary between Galfus and the host platform.
Capabilities (I/O, File System, Network) are injected via providers when initializing the runtime.
If a provider is not supplied, related builtin calls will fail deterministically, allowing trivial sandboxing.

---

## 8. Execution Metadata

Each `BytecodeModule` may optionally contain `ExecutionMetadata` linking bytecode back to source paths and spans.
This is used by the VM when producing structured stack traces upon panic, ensuring readable error messages without coupling the VM to the frontend.
