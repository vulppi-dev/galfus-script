# Galfus Architecture Reference

## Table of Contents

1. [Identity](#1-identity)
2. [Architecture Scope](#2-architecture-scope)
3. [Design Goals](#3-design-goals)
4. [Core Architecture Model](#4-core-architecture-model)
5. [Kernel, Modules, and Profiles](#5-kernel-modules-and-profiles)
6. [Compiler Module](#6-compiler-module)
7. [Source and Execution Artifacts](#7-source-and-execution-artifacts)
8. [Frontend and Compilation Pipeline](#8-frontend-and-compilation-pipeline)
9. [Galfus Module Image](#9-galfus-module-image)
10. [`.gfb` - Galfus Binary](#10-gfb---galfus-binary)
11. [`.gfm` - Galfus Map](#11-gfm---galfus-map)
12. [`.gfp` - Galfus Proxy](#12-gfp---galfus-proxy)
13. [Runtime Loader](#13-runtime-loader)
14. [VM Core](#14-vm-core)
15. [Owner Graph Core](#15-owner-graph-core)
16. [Owner Graph Extra](#16-owner-graph-extra)
17. [Data Layouts and Core Data Forms](#17-data-layouts-and-core-data-forms)
18. [Builtin Modules](#18-builtin-modules)
19. [Adapters and Host Integration](#19-adapters-and-host-integration)
20. [Bundle Model](#20-bundle-model)
21. [Tree-Shaking and Reachability](#21-tree-shaking-and-reachability)
22. [Single Distribution Unit](#22-single-distribution-unit)
23. [Runtime Profiles](#23-runtime-profiles)
24. [Sandbox Configuration](#24-sandbox-configuration)
25. [Panic Model](#25-panic-model)
26. [Debug Architecture](#26-debug-architecture)
27. [JIT and Interpreter Strategy](#27-jit-and-interpreter-strategy)
28. [Target Architecture](#28-target-architecture)
29. [Runtime Compilation](#29-runtime-compilation)
30. [Security Model](#30-security-model)
31. [Determinism Model](#31-determinism-model)
32. [Execution Flows](#32-execution-flows)
33. [Core Architecture](#33-core-architecture)
34. [Non-Goals](#34-non-goals)
35. [Architecture Summary](#35-architecture-summary)

---

## 1. Identity

Galfus is a typed VM-first language designed around a compact executable artifact, a small runtime kernel, deterministic module resolution, explicit ownership metadata, and aggressive reachability-based bundling.

Short definition:

```txt
Galfus is a typed VM-first language
with a minimal runtime kernel,
a serializable Galfus Module Image,
compact Galfus binaries,
separate debug maps,
used-only modules,
platform adapters,
and single-unit distribution.
```

The language is not centered around native AOT compilation. The primary execution model is:

```txt
.gfs source
  -> compiler pipeline
  -> Galfus Module Image
  -> .gfb serialization
  -> runtime loader
  -> VM
  -> execute
```

The primary release model is:

```txt
entry module
  -> BundleGraph
  -> reachability analysis
  -> used modules and adapters
  -> compact .gfb bundle or embedded module image
  -> target-specific single distribution unit
```

---

## 2. Architecture Scope

This document defines the structure of the Galfus compiler, runtime, VM, artifacts, bundler, adapters, targets, debug model, profiles, and execution flows.

This document does not define syntax details or semantic typing rules except where they affect architecture boundaries.

Architecture answers questions such as:

```txt
what is included in the runtime kernel
what is serialized into .gfb
what stays in .gfm
what belongs to the compiler module
what is included only when used
how adapters are selected
how bundles are emitted
how runtime profiles differ
how panic and debug information are reported
```

---

## 3. Design Goals

Galfus architecture is optimized for these goals:

```txt
small runtime
fast execution
safe execution
simple learning model
LLM-friendly project and artifact structure
deterministic builds
single-unit distribution
clear separation between tooling and runtime
```

The runtime must remain small by default. Tooling, debug data, compiler stages, rich text behavior, regular expressions, collections, adapters, reflection, and JIT components are not mandatory runtime parts.

The core rule is:

```txt
Only the runtime kernel is mandatory.
Everything else is included only when used, reached, or required by explicit policy.
```

---

## 4. Core Architecture Model

The system is divided into four architectural worlds.

```txt
Tooling world:
  source loading
  parsing
  resolution
  checking
  ownership validation
  MIR
  bytecode generation
  module image generation
  .gfb writing
  .gfm writing

Artifact world:
  .gfs
  Galfus Module Image
  .gfb
  .gfm
  .gfp
  external payload descriptors

Runtime world:
  runtime loader
  VM core
  Owner Graph Core
  used modules
  used adapters
  selected profile behavior

Platform world:
  host APIs
  native libraries
  WASM hosts
  mobile bridges
  embedded HALs
  packaging toolchains
```

Tooling produces executable artifacts. Runtime executes executable artifacts. Platform integration is done through explicit adapters and host capabilities.

---

## 5. Kernel, Modules, and Profiles

Galfus has a minimal runtime kernel:

```txt
vm_core
owner_graph_core
```

The kernel is not a module. It is the mandatory execution substrate required to load and execute a Galfus Module Image.

Everything else is a module, runtime component, adapter, debug component, or profile behavior that is included only when used or explicitly required.

Examples:

```txt
user modules
builtin modules
compiler module
platform adapter modules
collection module
regex module
rich text/string module
debug hooks
owner_graph_extra
JIT components
```

The global default is used-only inclusion. There is no need to configure common default-off behavior such as:

```txt
weak = off
jit = off
reflection = off
debug = off
bundle = single
```

Those are structural defaults. Weak support, JIT, reflection, debug hooks, and similar components enter only through reachability, profile behavior, debug build mode, or explicit policy.

---

## 6. Compiler Module

The Galfus frontend/tooling can exist as an optional module named `compiler`.

The `compiler` module may expose compilation capabilities such as:

```txt
compiler::lexer
compiler::parser
compiler::resolver
compiler::checker
compiler::ownership
compiler::mir
compiler::bytecode
compiler::image
compiler::gfb
compiler::gfm
```

A normal application does not include the `compiler` module.

The `compiler` module is included only for products that need runtime compilation or development-time tooling inside the running environment, such as:

```txt
REPL
playground
tutorial runtime
sandbox
hot reload environment
server-side controlled compilation
LLM-assisted code execution environment
```

Conceptual use:

```galfus
import compiler from "compiler"

var image = compiler::compile(source)
```

The compiler module follows the same used-only module rule as every other module.

---

## 7. Source and Execution Artifacts

Galfus uses the following artifact types.

```txt
.gfs  Galfus source file
.gfb  Galfus Binary; serialized Galfus Module Image
.gfm  Galfus Map; debug/tooling/source reconstruction map
.gfp  Galfus Proxy; descriptor for external payloads/adapters
```

Artifact responsibilities:

```txt
.gfs:
  human-authored source used by tooling

Galfus Module Image:
  in-memory executable image containing the minimum required for VM execution

.gfb:
  binary serialization of a Galfus Module Image

.gfm:
  optional debug/tooling map for source reconstruction, IDE paths, spans, names, and autocomplete

.gfp:
  proxy descriptor used to describe external payloads, host bridges, ABI, adapters, and binding metadata
```

The runtime does not require `.gfs`, `.gfm`, or `.gfp` for normal release execution.

---

## 8. Frontend and Compilation Pipeline

The normal compilation pipeline is:

```txt
.gfs source
  -> lexer/parser
  -> resolver
  -> type checker
  -> semantic checker
  -> ownership checker
  -> MIR
  -> bytecode
  -> Galfus Module Image
  -> .gfb serialization
  -> optional .gfm
```

Important boundaries:

```txt
WorkspaceGraph is tooling-only.
ModuleGraph is tooling-only.
SemanticGraph is tooling-only.
MIR is tooling-only.
Galfus Module Image is runtime-facing.
.gfb is the serialized form of the Galfus Module Image.
```

The VM receives a Galfus Module Image, normally deserialized from `.gfb`.

---

## 9. Galfus Module Image

The Galfus Module Image is the in-memory executable image containing the minimum required for VM execution.

It is not source code. It is not a frontend graph. It is not a debug map.

It contains runtime-facing data such as:

```txt
bytecode
constant pool
function table
type table
layout table
import slots
export slots
ownership metadata
anchor metadata
edge metadata
weak metadata, when used
module initialization data
adapter references
integrity-relevant metadata
```

The `.gfb` file is the binary serialization of this Module Image.

Build flow:

```txt
.gfs
  -> compiler pipeline
  -> Galfus Module Image
  -> serialize
  -> .gfb
```

Execution flow:

```txt
.gfb
  -> validate
  -> deserialize
  -> Galfus Module Image
  -> VM execute
```

---

## 10. `.gfb` - Galfus Binary

A `.gfb` is the compact binary serialization of a Galfus Module Image.

A `.gfb` should contain only what is required for execution and integrity:

```txt
format header
version
bytecode
compact constant pool
compact function table
compact type table
compact layout table
import slots
export slots
ownership metadata
anchor/edge/weak metadata
module init data
adapter references
integrity metadata
```

A `.gfb` does not contain rich development metadata. It does not contain full source, rich symbol paths, IDE autocomplete data, source reconstruction data, or debug spans beyond what is required for minimal execution diagnostics and integrity.

Release `.gfb` should remain compact.

---

## 11. `.gfm` - Galfus Map

A `.gfm` is an optional debug and tooling artifact.

It contains information such as:

```txt
source reconstruction data
source spans
module paths
symbol names
local variable names
function names
bytecode offset to source mappings
IDE autocomplete paths
diagnostic enrichment data
```

A `.gfm` is not required for release execution.

Debug builds, IDEs, REPLs, playgrounds, and development tooling may use `.gfm` to provide richer diagnostics and source-aware behavior.

Separation rule:

```txt
.gfb contains the minimum for execution and integrity.
.gfm contains debug, source reconstruction, and IDE/tooling data.
```

---

## 12. `.gfp` - Galfus Proxy

A `.gfp` describes an external payload, adapter, bridge, or host binding.

It may describe:

```txt
external payload location
ABI
symbols
adapter requirements
host capabilities
ownership/resource policy
integrity information
```

A `.gfp` is a development/build-time descriptor. In a final bundle, the `.gfp` itself is not normally required.

During bundling:

```txt
.gfp
  -> adapter descriptor
  -> bridge manifest entry
  -> embedded external payload, if used and allowed
```

The final runtime receives only the minimal adapter information and embedded payloads required by the bundle.

---

## 13. Runtime Loader

The runtime loader is responsible for converting serialized executable artifacts into runnable Module Images and connecting them inside the VM.

Responsibilities:

```txt
load .gfb
validate format and integrity
deserialize Galfus Module Image
resolve import slots inside the bundle
resolve adapter references
validate target compatibility
validate entrypoint
initialize module state
hand the Module Image to the VM
```

The runtime loader does not parse `.gfs` in normal applications. Source parsing requires the optional `compiler` module.

---

## 14. VM Core

The VM core is mandatory. It executes bytecode over Galfus Module Images.

Responsibilities:

```txt
bytecode dispatch
call frames
locals
temporaries
function calls
returns
primitive casts
control flow
module init execution
import/export slot dispatch
adapter call dispatch
panic propagation
```

The VM core does not include parser, resolver, type checker, semantic checker, ownership checker, MIR builder, `.gfb` writer, `.gfm` writer, debug hooks, JIT, or reflection by default.

---

## 15. Owner Graph Core

Owner Graph Core is mandatory. It is part of the runtime kernel and implements the memory lifetime model.

Core concepts:

```txt
anchor:
  lifetime root

edge:
  traceable data relation

weak:
  observer that does not preserve lifetime
```

A value lives while it is reachable from at least one anchor through edges. Weak observers do not preserve lifetime.

Owner Graph Core responsibilities:

```txt
manage anchors
trace edges
release affected graph fragments
handle cycles deterministically
run drop scheduling
invalidate weak observers
coordinate with VM locals, frames, closures, modules, and host roots
```

Owner Graph Core is not an optional runtime slice. It is required by Galfus memory semantics.

---

## 16. Owner Graph Extra

Owner Graph Extra is optional and used for diagnostics, debugging, validation, and development tools.

Possible features:

```txt
graph traces
ownership logs
leak reports
visual graph export
heavy runtime validation
development assertions
ownership diagnostics
```

Owner Graph Extra is not part of the minimal runtime kernel.

---

## 17. Data Layouts and Core Data Forms

The VM must be able to represent core data forms as layouts.

Core data forms:

```txt
struct
tuple
array
enum
choice
string literal as [uint8]
```

These are data/layout forms. They do not carry built-in behavior or methods.

String literals are represented as UTF-8 `[uint8]` values. There is no mandatory core `String` object.

Examples of data-only forms:

```txt
arrays:
  layout + length + elements

tuples:
  positional fixed layout

enums:
  nominal enum value + discriminant

choices:
  tagged union + payload

string literals:
  UTF-8 byte arrays
```

Rich operations belong to modules.

---

## 18. Builtin Modules

Builtin modules are provided by the Galfus distribution and are split into two tiers:

1. **`std/*` (Thin Target Standard Surface)**: Low-level modules (such as `std/io`, `std/fs`, `std/net`, `std/time`, `std/env`, `std/random`, and `std/process`) that interface directly with host capabilities.
2. **Rich Utility Modules**: Platform-agnostic, developer-friendly modules (such as `text`, `format`, `json`, `regex`, `math`, `path`, `http`, `collections`, and `crypto`) built on top of `std/*` or implementing pure algorithms.

Detailed specifications for builtins are documented in [Galfus Builtins Reference](Galfus_Builtins_Reference.md).

The rule is:

```txt
Builtin does not mean mandatory.
Builtin means provided by Galfus and included only when used.
Access to std/* requires explicit permission in configuration.
```

There is no `array` module. Arrays are core data forms. Rich collection behavior belongs to `collections`.

Regular expressions are not syntax or core runtime. They belong to a module.

Rich text or `String` behavior may be provided by a module or struct, but is not part of the kernel.

---

## 19. Adapters and Host Integration

Adapters connect Galfus modules to platform-specific capabilities.

Adapter categories:

```txt
native adapters
WASM adapters
mobile adapters
embedded adapters
server/host adapters
```

Examples:

```txt
desktop/server:
  .dll
  .so
  .dylib
  C ABI

web/WASM:
  WASM host APIs
  JavaScript host bridge

Android:
  JNI
  Kotlin bridge
  Android SDK APIs

iOS:
  Swift
  Obj-C
  C bridge
  platform frameworks

embedded:
  HAL
  static drivers
  firmware bindings
```

Adapters are selected by target, policy, and reachability. Runtime profile does not decide adapter inclusion.

Host integration exposes capabilities through adapters. It does not change Galfus language semantics.

---

## 20. Bundle Model

A bundle starts from an entry module and includes only the reachable execution graph.

The bundler constructs a BundleGraph:

```txt
entry module
  -> reachable imports
  -> reachable functions
  -> reachable types/layouts
  -> reachable builtin modules
  -> reachable adapters
  -> reachable external payloads
  -> compact bundle
```

The bundle may contain:

```txt
main .gfb
packed dependency module images
used builtin modules
used adapter descriptors
embedded external payloads, if used
manifest
integrity metadata
```

The bundle does not include unused modules, unused builtins, unused adapters, unused debug hooks, unused payloads, or tooling components unless they are reachable or explicitly required.

---

## 21. Tree-Shaking and Reachability

Galfus uses aggressive reachability-based tree-shaking.

The bundler removes:

```txt
unreachable modules
unreachable functions
unreachable types
unreachable layouts
unused builtin modules
unused adapter modules
unused external payloads
unused debug hooks
unused JIT hooks
unused owner_graph_extra
unused compiler module pieces
```

Exported does not mean included.

```txt
exported:
  available to dependents

included:
  reachable from the bundle entrypoint or required by explicit policy
```

This rule applies uniformly to user modules, builtin modules, compiler modules, tooling modules, adapter modules, and platform modules.

---

## 22. Single Distribution Unit

Every final build produces a single distribution unit.

Possible distribution units:

```txt
.gfb
executable
firmware .bin
APK
AAB
AAR
app bundle
WASM package
native archive/package
```

Single distribution unit does not always mean a single internal file.

Examples:

```txt
APK/AAB/AAR:
  one distribution unit with internal files

app bundle:
  one distribution unit with internal files

executable:
  may embed .gfb and external payloads

firmware .bin:
  may embed .gfb in flash
```

The architectural guarantee is that the output is packaged as one target-appropriate final unit.

---

## 23. Runtime Profiles

Runtime profiles define execution strategy, optimization behavior, and diagnostic level.

Profiles are incremental:

```txt
micro
fast
std
dev
```

Profile meanings:

```txt
micro:
  minimum possible runtime behavior

fast:
  micro
  + quickening
  + aggressive JIT

std:
  fast
  + balanced JIT
  + better panic messages
  + better module traces

dev:
  std
  + debug tools
  + debug trace
  + breakpoints
```

Runtime profiles do not decide:

```txt
sandbox limits
adapter selection
loaded modules
builtin module inclusion
platform policy
host capabilities
```

Those are controlled separately:

```txt
sandbox limits:
  configuration

adapters:
  target + policy + reachability

modules:
  usage + reachability

host capabilities:
  policy
```

---

## 24. Sandbox Configuration

Sandbox behavior is configuration, not a runtime profile.

By default, a Galfus program runs in a closed sandbox where access to any host-connected `std/*` standard surface is blocked. Access must be explicitly granted via permission configuration in the module or workspace descriptor.

Sandbox configuration may define:

```txt
max memory
max stack/call depth
max steps/fuel
allowed adapters
allowed host capabilities
allowed external payloads
resource limits
std/* permissions (e.g. read/write scopes, network hosts)
```

This is primarily useful for:

```txt
server multi-tenant execution
public playgrounds
REPL environments
LLM code execution sandboxes
CI validation
plugin hosts
```

Sandbox policy is orthogonal to profiles. A `micro`, `fast`, `std`, or `dev` runtime can be used with or without sandbox configuration depending on host policy and target.

---

## 25. Panic Model

Runtime failures produce `panic`.

A panic aborts the entire Galfus execution process.

A panic should identify the module trace up to the module that caused the panic.

Without debug tools, a panic reports the information available in `.gfb`, such as:

```txt
panic reason
module trace
available module/function identifiers
minimal bytecode location, if available
```

With debug tools and `.gfm`, a panic may report:

```txt
panic reason
source file
source span
function name
module path
import/module trace
local names, when available
formatted diagnostic
reconstruction context
```

Examples of runtime failures that may panic:

```txt
sandbox memory limit exceeded
stack limit exceeded
invalid external adapter response
corrupted .gfb
integrity failure
unreachable bytecode state
host capability violation
```

Operations that define valid fallback values should not panic for those cases. For example, array out-of-bounds access returns `null` by semantic rule.

---

## 26. Debug Architecture

Debug support is optional.

Debug architecture may include:

```txt
.gfm
source maps
source reconstruction data
debug hooks
trace hooks
breakpoint hooks
profiler hooks
owner_graph_extra
rich panic diagnostics
IDE paths and autocomplete metadata
```

Release execution does not require debug hooks or `.gfm`.

Debug mode enriches diagnostics without changing language semantics.

---

## 27. JIT and Interpreter Strategy

The base execution strategy is interpreter-first.

JIT-related components are optional and profile-dependent.

Components may include:

```txt
quickening
threaded interpreter
JIT hooks
JIT core
JIT backend for target architecture
```

Profile relationship:

```txt
micro:
  no required quickening or JIT

fast:
  quickening + aggressive JIT

std:
  balanced JIT and better diagnostics

dev:
  std behavior plus debug tools and trace support
```

Some platforms may restrict or disallow JIT. In those cases, the profile must degrade according to target policy while preserving valid execution through interpretation.

---

## 28. Target Architecture

Targets define packaging, platform integration, and toolchain requirements.

Examples:

```txt
desktop:
  executable or native package

server:
  executable or hosted runtime

web/WASM:
  runtime+VM WASM and .gfb payload/bundle

Android:
  APK/AAB/AAR and Android toolchain

iOS:
  app/framework and Apple platform toolchain

embedded:
  firmware .bin with static adapters and embedded .gfb
```

Targets may require external toolchains:

```txt
Android SDK/NDK
Xcode/iOS toolchain
WASM toolchain
embedded SDK/HAL
native linker
```

Target configuration does not replace reachability. It constrains what adapters, payloads, and packaging forms are legal.

---

## 29. Runtime Compilation

Runtime compilation is possible only when the `compiler` module is included.

Runtime compilation flow:

```txt
running Galfus environment
  -> compiler module receives source
  -> compiler pipeline generates Galfus Module Image
  -> Module Image is serialized or loaded directly
  -> runtime loader validates
  -> VM executes
```

Use cases:

```txt
REPL
playground
hot reload
server-side sandbox
interactive tutorial
LLM-assisted controlled code generation
```

Normal applications do not include runtime compilation by default.

Runtime compilation should be controlled by host policy, sandbox configuration, and target restrictions.

---

## 30. Security Model

Galfus security is layered.

Security mechanisms:

```txt
type checking
semantic checking
ownership checking
compact .gfb validation
integrity metadata
capability policy
adapter policy
sandbox limits
single-unit packaging
no arbitrary runtime source loading by default
```

External payloads are not arbitrary imports. They must be described by proxy/adapter metadata and included according to policy and reachability.

The runtime executes validated Module Images. It does not trust raw source or arbitrary external files by default.

---

## 31. Determinism Model

Galfus architecture favors deterministic builds and deterministic module graphs.

Determinism applies to:

```txt
module discovery
import resolution
export surfaces
BundleGraph construction
reachability analysis
adapter selection
artifact writing
integrity metadata
single-unit packaging
```

Host integration is explicit and policy-bound. There are no implicit arbitrary loaders in the runtime architecture.

---

## 32. Execution Flows

### Flow A - Source to `.gfb`

```txt
.gfs source
  -> compiler pipeline
  -> Galfus Module Image
  -> .gfb serialization
```

### Flow B - `.gfb` to Execution

```txt
.gfb
  -> validate
  -> deserialize
  -> Galfus Module Image
  -> runtime loader
  -> VM core
  -> owner_graph_core
  -> execute entrypoint
```

### Flow C - Bundle Creation

```txt
entry module
  -> BundleGraph
  -> reachability analysis
  -> used modules
  -> used adapters
  -> used external payloads
  -> tree-shaken artifact set
  -> single distribution unit
```

### Flow D - Debug Reconstruction

```txt
panic/debug event
  -> .gfb minimal identifiers
  -> .gfm source map, if available
  -> source span
  -> module trace
  -> enriched diagnostic
```

### Flow E - Runtime Compilation

```txt
running VM
  -> compiler module
  -> source input
  -> Galfus Module Image
  -> validate/load
  -> execute
```

---

## 33. Core Architecture

The core architecture should include only the minimum required execution system.

Core required parts:

```txt
vm_core
owner_graph_core
.gfb loader
Galfus Module Image deserializer
bytecode interpreter
primitive scalar support
core data layout support
minimal module init
minimal panic
single-unit bundle output
```

Core non-required parts:

```txt
compiler module in runtime
.gfm in release
regex module
rich text/string module
collection helpers beyond what is used
reflection
operator overloading
runtime parser
runtime checker
debug hooks
breakpoint hooks
owner_graph_extra
arbitrary dynamic loaders
```

---

## 34. Non-Goals

Galfus architecture does not include:

```txt
mandatory native AOT backend
mandatory LLVM backend
mandatory Cranelift backend
mandatory object file emission
mandatory native linker pipeline
standard-library bulk in the runtime kernel
operator overloading
implicit methods on core data forms
runtime parser by default
runtime checker by default
arbitrary runtime loaders by default
mandatory global GC model
```

Native/JIT/AOT-related work may exist as optional target/profile-specific systems, but they are not the architectural center.

---

## 35. Architecture Summary

Final architecture in compact form:

```txt
Kernel:
  vm_core
  owner_graph_core

Optional module:
  compiler

Artifacts:
  .gfs = source
  Galfus Module Image = minimum executable image in memory
  .gfb = serialized Module Image
  .gfm = debug/tooling/source reconstruction map
  .gfp = proxy descriptor for external payloads/adapters

Runtime:
  load .gfb
  validate
  deserialize Module Image
  execute in VM
  manage lifetime through Owner Graph Core

Modules:
  user modules
  builtin modules
  adapter modules
  platform modules
  compiler module
  all used-only

Profiles:
  micro
  fast = micro + quickening + aggressive JIT
  std = fast + balanced JIT + better panic messages and module traces
  dev = std + debug tools + debug trace + breakpoints

Not profile-controlled:
  sandbox configuration
  adapter selection
  loaded modules
  builtin reachability
  platform policy

Bundle:
  always emitted as one target-appropriate distribution unit
```
