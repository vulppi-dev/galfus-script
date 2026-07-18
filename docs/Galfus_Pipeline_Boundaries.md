# Galfus Pipeline Boundaries

`galfus-workspace` is the stateful facade used by hosts. A host loads the
configuration and source bytes, then calls `check`, `compile`, and `run` in
that order. Hosts do not compose frontend, compiler, or runtime state.

## Source boundary

`SourceStore` belongs to `galfus-workspace`. It stores source bytes by
`ModulePath`, assigns a stable `ModuleId` on first load, and records a
`Revision` for every update. Replacing bytes for an existing path preserves
the module ID. Removing a source removes its entry and reports that stable ID
to the frontend delta.

## Frontend boundary

`FrontendSession` consumes `FrontendUpdate`: changed `FrontendSource` values,
removed module IDs, source revision, and roots. It owns the
`SemanticModuleGraph`, whose nodes are `SemanticModule` values keyed by
`ModuleId`; every node carries a `SemanticRevision`. Its report contains
diagnostics, required builtin module paths, and the set of module IDs whose
semantic output changed. The compiler must consume the semantic model rather
than source text or syntax-only output.

## Compiler boundary

`CompiledModule` is the compiler input boundary for one checked semantic
module. The compiler produces one `CompiledModuleImage` per module, preserving
its `ModuleId`, `ModulePath`, and source `SemanticRevision` alongside the
`ModuleImage`. `CompiledModuleGraph` owns those images and dependency edges;
it supports upsert and removal, so unchanged images remain cached.

## Runtime boundary

`RuntimeModuleGraph` accepts `CompiledModuleImage` values with `load` and
removes them with `unload`. It resolves image import slots by `ModuleId` and
module path, computes dependency-safe initialization order, and links the
reachable images only when executing an entry. The runtime never performs
parsing, semantic checking, or compilation.

## Host-provider boundary

`galfus-host` defines optional host contracts independently of the workspace,
runtime, and VM. A host constructs `Providers` with concrete implementations,
then passes them only to `Workspace::run`. The workspace remains the final
facade and does not expose its internal runtime or VM state.

Providers are execution-scoped. The compiler does not inspect or validate
them. The runtime passes them to the VM, which reports a runtime error only if
an instruction requires a missing provider. Consequently, running without
providers is a valid sandbox configuration for programs that do not reach
host-backed builtins.

The current `IoProvider` is synchronous and supports byte-stream reads with a
terminator and writes. It is intentionally independent of native, WASM, or
browser APIs: the CLI adapts native streams, and the playground adapts its
buffered stream to JavaScript through its WASM-facing API.

## Gate rules

`Workspace::compile` requires a successful, current `Workspace::check`.
`Workspace::run` requires a successful, current `Workspace::compile`. A
source or configuration update invalidates the later stages until the pipeline
is run again.
