# Galfus Script - Architecture Reference

## Table of Contents

1. [Identity](#1-identity)
2. [Architecture model](#2-architecture-model)
3. [Execution model](#3-execution-model)
4. [WorkspaceGraph](#4-workspacegraph)
5. [ModuleGraph](#5-modulegraph)
6. [SemanticGraph](#6-semanticgraph)
7. [Graph layers](#7-graph-layers)
8. [Graph phases](#8-graph-phases)
9. [Stable identities](#9-stable-identities)
10. [Export surface](#10-export-surface)
11. [External references](#11-external-references)
12. [Module resolution](#12-module-resolution)
13. [Frontend pipeline](#13-frontend-pipeline)
14. [Incremental compilation](#14-incremental-compilation)
15. [Hot reload](#15-hot-reload)
16. [Diagnostics](#16-diagnostics)
17. [Lowering to MIR](#17-lowering-to-mir)
18. [MIR](#18-mir)
19. [Bytecode](#19-bytecode)
20. [Galfus Module Image](#20-galfus-module-image)
21. [`.gfb`](#21-gfb)
22. [`.gfb.map`](#22-gfbmap)
23. [VM core](#23-vm-core)
24. [Runner manager](#24-runner-manager)
25. [JIT policy](#25-jit-policy)
26. [Function model](#26-function-model)
27. [Synthetic functions](#27-synthetic-functions)
28. [Struct initialization](#28-struct-initialization)
29. [Module system](#29-module-system)
30. [Import backends](#30-import-backends)
31. [C-ABI integration](#31-c-abi-integration)
32. [WASM integration](#32-wasm-integration)
33. [Builtin modules](#33-builtin-modules)
34. [Owner Graph Runtime](#34-owner-graph-runtime)
35. [Root blocks](#35-root-blocks)
36. [Affected Graph Release](#36-affected-graph-release)
37. [Weak references](#37-weak-references)
38. [Copy model](#38-copy-model)
39. [Runtime arrays and variadics](#39-runtime-arrays-and-variadics)
40. [Debug architecture](#40-debug-architecture)
41. [Development mode](#41-development-mode)
42. [Release mode](#42-release-mode)
43. [Browser/WASM mode](#43-browserwasm-mode)
44. [Architecture summary](#44-architecture-summary)

---

## 1. Identity

Galfus Script is a typed VM scripting language with deterministic ownership and configurable aggressive JIT.

Short definition:

```txt
Galfus is a typed VM scripting language
with deterministic Owner Graph runtime,
module-local semantic graphs,
medium-level bytecode,
multi-backend modules,
and hybrid/eager JIT.
```

The language is not centered around native AOT compilation.

The core model is:

```txt
source -> WorkspaceGraph -> ModuleGraph -> MIR -> Module Image -> VM -> execute
```

The main design principles are:

```txt
typed VM
deterministic runtime
module-local SemanticGraph
incremental frontend
stable export surfaces
stable external references
source-level debugging
medium-level bytecode
multi-backend imports
hybrid JIT by default
eager JIT in release
WASM-capable VM
```

---

## 2. Architecture model

Galfus uses a graph-centered frontend.

There is no standalone frontend tree that must be copied into a separate high-level IR before semantic analysis.

The frontend builds and updates graphs.

```txt
WorkspaceGraph
  +-- ModuleGraph(main)
  +-- ModuleGraph(user)
  +-- ModuleGraph(engine)
  +-- ModuleGraph(math)
  `-- dependency edges
```

Each `ModuleGraph` owns the complete semantic state for one module.

This includes:

```txt
parsed syntax nodes
resolved names
resolved imports
resolved types
function signatures
struct fields
choice variants
enum discriminants
anchor functions
constraints
satisfies declarations
decorators
default values
null safety metadata
ownership metadata
source spans
debug links
export surface
external references
lowering metadata
```

Conceptually, modules work like spreadsheet sheets.

A module can use an exported symbol from another module without copying the other module's internal graph.

```txt
Sheet: main.gfs
  uses user::create("Ana")

Sheet: user.gfs
  exports fn create(name: String): User
```

The `main` module stores a stable external reference to `user::create`.

It does not point directly to arbitrary internal nodes of `user`.

---

## 3. Execution model

Galfus source code is not executed directly.

It is compiled into a typed module image that the VM can execute.

```txt
.gfs source
  -> lexer
  -> parser
  -> ModuleGraph syntax layer
  -> resolver layers
  -> type layer
  -> semantic layers
  -> MIR
  -> bytecode
  -> Galfus Module Image
  -> VM
  -> execution backend
```

The execution backend can be:

```txt
interpreter
JIT compiled function
builtin compiled function
native C-ABI function
WASM core function
WASM component function
```

The VM does not know source syntax.

It executes typed bytecode and runtime metadata from a module image.

---

## 4. WorkspaceGraph

`WorkspaceGraph` is the project-level graph.

It owns the module dependency network.

Responsibilities:

```txt
track source modules
track binary modules
track builtin modules
track native modules
track WASM modules
resolve package/module paths
own ModuleGraph instances
track dependency edges
track reverse dependency edges
track invalidation state
track graph versions
track cache entries
coordinate incremental compilation
coordinate hot reload
```

Conceptual shape:

```txt
WorkspaceGraph {
  modules: Arena<ModuleGraph>,
  packages: PackageRegistry,
  dependencies: DependencyGraph,
  reverse_dependencies: ReverseDependencyGraph,
  cache: WorkspaceCache,
  invalidation_queue: InvalidationQueue,
}
```

The workspace does not flatten all modules into one global semantic graph.

Each module remains isolated behind its export surface.

---

## 5. ModuleGraph

`ModuleGraph` is the semantic graph of a single module.

It is the frontend's main product.

A module graph owns:

```txt
module identity
source identity
module version
syntax nodes
symbol table
type table
function table
struct table
enum table
choice table
constraint table
anchor table
decorator metadata
ownership metadata
import table
export surface
external references
diagnostics
debug/source mapping data
lowering cache
```

Conceptual shape:

```txt
ModuleGraph {
  module_id: ModuleId,
  source_id: SourceId,
  version: ModuleVersion,

  nodes: Arena<Node>,
  symbols: Arena<Symbol>,
  types: Arena<Type>,
  functions: Arena<Function>,
  structs: Arena<StructDef>,
  enums: Arena<EnumDef>,
  choices: Arena<ChoiceDef>,
  constraints: Arena<ConstraintDef>,

  syntax: SyntaxLayer,
  names: NameLayer,
  types_layer: TypeLayer,
  semantics: SemanticLayer,
  ownership: OwnershipLayer,
  imports: ImportLayer,
  exports: ExportSurface,
  external_refs: ExternalRefTable,
  lowering: LoweringLayer,
  debug: DebugLayer,
  diagnostics: DiagnosticSet,
}
```

The `ModuleGraph` is not a runtime object.

It is a compiler/tooling object.

The VM receives a `Galfus Module Image`, not a `ModuleGraph`.

---

## 6. SemanticGraph

`SemanticGraph` is the logical content of a `ModuleGraph`.

It combines parsed syntax, resolved names, type information, semantic checks, and source-level metadata.

The term `SemanticGraph` refers to the module-local graph after it has enough information to be lowered.

A `SemanticGraph` contains source-level intent.

Example source:

```galfus
var user = User {
  name: "Renato",
}
```

The semantic graph keeps this as a struct initialization concept:

```txt
StructInitNode {
  struct: StructId(User),
  fields: {
    FieldId(name): StringLiteral("Renato"),
    FieldId(age): DefaultMarker,
  },
  result_type: TypeId(User),
  ownership: CreatesOwner,
  source_span: Span(...),
}
```

It does not immediately erase this into a low-level call.

That happens during lowering to MIR.

---

## 7. Graph layers

A `ModuleGraph` is one graph with multiple layers.

The layers prevent the graph from becoming an unstructured object with every possible field on every node.

### Syntax layer

Stores parsed source structure.

```txt
node kind
children
tokens
source spans
literal data
operator data
block structure
```

### Name layer

Stores symbol resolution.

```txt
local bindings
module bindings
import bindings
resolved symbols
shadowing rules
visibility rules
anchor lookup results
```

### Type layer

Stores type information.

```txt
expression types
binding types
function signatures
generic substitutions
coercions
cast metadata
union active possibilities
nullability information
```

### Semantic layer

Stores language-level meaning.

```txt
struct field mapping
struct defaults
choice payload mapping
enum discriminants
constraint satisfaction
decorator application order
parameter default metadata
variadic collector metadata
range metadata
array metadata
```

### Ownership layer

Stores ownership/runtime safety metadata.

```txt
creates owner
shares owner handle
requires local root
requires temporary root
creates strong edge
removes strong edge
creates weak edge
may trigger release
```

### Import layer

Stores imports and backend resolution.

```txt
source module import
binary module import
builtin module import
native dynamic library import
WASM core import
WASM component import
```

### Export layer

Stores the public surface of the module.

```txt
exported constants
exported variables
exported functions
exported structs
exported enums
exported choices
exported constraints
exported type aliases
exported submodules
```

### Lowering layer

Stores data needed by MIR generation.

```txt
synthetic initializer refs
synthetic constructor refs
decorator wrapper refs
copy thunk refs
cast thunk refs
variadic collector refs
node-to-MIR links
```

### Debug layer

Stores source-level mapping.

```txt
source spans
original names
scope metadata
local variable metadata
inferred type display
diagnostic anchors
breakpoint anchors
watch expression anchors
stack trace anchors
```

---

## 8. Graph phases

A `ModuleGraph` evolves through phases.

The phases are compiler invariants, not separate full copies of the graph.

```txt
ParsedGraph
  -> ImportedGraph
  -> ResolvedGraph
  -> TypedGraph
  -> CheckedGraph
  -> LowerableGraph
```

### ParsedGraph

The syntax layer is available.

Names may still be unresolved.

Types may still be unknown.

### ImportedGraph

Imports are known and linked to module candidates.

External modules may still be loading or checking.

### ResolvedGraph

Names resolve to symbols.

Imported symbols resolve to external references.

Anchor calls resolve to function candidates.

### TypedGraph

Expressions, bindings, functions, fields, choices, and enums have type information.

Generic substitutions and cast metadata are available.

### CheckedGraph

Semantic rules have been validated.

This includes:

```txt
constraint rules
satisfies rules
decorator rules
default parameter rules
null safety rules
weak reference rules
ownership preparation rules
visibility rules
```

### LowerableGraph

The graph can be lowered to MIR.

Synthetic functions and lowering metadata have been assigned.

---

## 9. Stable identities

Incremental compilation depends on stable identity.

Internal nodes can change when the source changes.

External modules must not depend on unstable internal node identifiers.

Internal references use local ids:

```txt
NodeId
SymbolId
TypeId
FunctionId
StructId
EnumId
ChoiceId
ConstraintId
```

Cross-module references use stable exported ids:

```txt
GlobalExportRef {
  module_id: ModuleId,
  export_id: ExportId,
  export_kind: ExportKind,
  surface_hash: Hash,
}
```

Rules:

```txt
within a module, local ids are allowed
between modules, ExportId is required
external modules must not point to private NodeId values
export identity must survive internal body edits when possible
signature changes create a new export surface hash
```

Bad cross-module reference:

```txt
main::CallExpr -> user::NodeId(182)
```

Good cross-module reference:

```txt
main::CallExpr -> GlobalExportRef(user, export create)
```

---

## 10. Export surface

The export surface is the stable public facade of a module.

Other modules depend on the export surface, not on internal graph details.

A module export surface contains:

```txt
exported symbol names
export kinds
function signatures
type signatures
struct public fields
choice variants
enum discriminants
constraint signatures
type alias expansions where needed
anchor signatures
ABI-facing metadata
visibility metadata
surface hashes
```

Example:

```galfus
export struct User {
  id: int64,
  name: String,
}

export fn create(name: String): User {
  return User {
    id: 1,
    name,
  }
}
```

Export surface:

```txt
ModuleExports(user) {
  struct User {
    id: int64,
    name: String,
  }

  fn create(String): User
}
```

If only the body of `create` changes, the export surface may remain stable.

If the signature of `create` changes, dependent modules must be invalidated.

---

## 11. External references

External references connect one module graph to another module's export surface.

They also represent builtin, native, binary, and WASM imports.

```txt
ExternalRef {
  source_module: ModuleId,
  target_module: ModuleId,
  export_id: ExportId,
  export_kind: ExportKind,
  backend_kind: BackendKind,
  expected_signature_hash: Hash,
  resolved_signature_hash: Hash,
}
```

Backend kinds:

```txt
GalfusSource
GalfusBinary
Builtin
NativeCAbi
WasmCore
WasmComponent
```

Example source:

```galfus
import user from "./user"

var u = user::create("Ana")
```

External ref:

```txt
ExternalRef {
  source_module: main,
  target_module: user,
  export: create,
  export_kind: Function,
  backend_kind: GalfusSource,
  expected_signature: fn(String): User,
}
```

External references are the basis for incremental invalidation.

---

## 12. Module resolution

Modules are namespaces and compilation units.

The same import syntax can resolve to different backend kinds.

```galfus
import string from "string"
import collections from "collections"
import game from "./game"
import physics from "./libphysics"
import fast_math from "./fast_math.wasm"
import image from "./image_component.wasm"
import engine from "engine"
```

Search targets:

```txt
Galfus source module      .gfs
Galfus binary module      .gfb
builtin module
native dynamic library    .dll / .so / .dylib
WASM core module          .wasm
WASM component            .wasm component
```

Resolution produces module records in the workspace graph.

```txt
ModuleRecord {
  module_id,
  module_name,
  backend_kind,
  source_location,
  export_surface,
  load_state,
}
```

Source modules own a `ModuleGraph`.

Binary, builtin, native, and WASM modules expose importable surfaces without requiring a source graph.

---

## 13. Frontend pipeline

The frontend builds and updates module graphs.

```txt
source text
  -> lexer
  -> parser
  -> syntax layer update
  -> import resolution
  -> name resolution
  -> type checking
  -> semantic checking
  -> ownership metadata preparation
  -> export surface generation
  -> external reference validation
  -> lowering metadata preparation
```

The frontend is responsible for:

```txt
lexing
parsing
module resolution
import resolution
name resolution
type checking
constraint checking
satisfies checking
decorator checking
null safety checking
weak reference checking
ownership metadata preparation
diagnostic generation
export surface generation
incremental invalidation
```

The frontend product is a lowerable `ModuleGraph`.

---

## 14. Incremental compilation

Incremental compilation operates on module graphs and export surfaces.

Each module has hashes by layer.

```txt
SourceHash
SyntaxHash
ImportHash
NameHash
TypeHash
SemanticHash
OwnershipHash
ExportSurfaceHash
BodyHash
LoweringHash
MirHash
BytecodeHash
DebugMapHash
```

Invalidation rules:

```txt
source text changed
  -> update syntax layer

syntax changed but exports unchanged
  -> recheck local module
  -> rebuild affected MIR functions
  -> patch module image

export surface changed
  -> invalidate dependent external refs
  -> re-resolve affected dependents
  -> re-typecheck affected dependents

ABI surface changed
  -> rebuild ABI adapters
  -> invalidate native/WASM call sites if needed
```

The workspace tracks reverse dependencies.

```txt
user changed
  -> find modules that import user
  -> compare old export surface with new export surface
  -> invalidate only affected dependents
```

The ideal case:

```txt
function body changed
export surface unchanged
  -> rebuild local MIR/bytecode only
  -> keep dependents valid
```

---

## 15. Hot reload

Hot reload patches running or reloadable module images when safe.

Development mode uses the module graph to determine the smallest safe reload unit.

Hot reload flow:

```txt
file changed
  -> update ModuleGraph
  -> rebuild changed functions
  -> rebuild changed debug map ranges
  -> compare export surface
  -> patch module image
  -> notify VM loader
  -> update function entrypoints
```

Safe hot reload cases:

```txt
function body changed
private helper changed
private constant changed
private synthetic lowering changed
non-exported implementation detail changed
```

Potentially unsafe hot reload cases:

```txt
exported function signature changed
exported struct layout changed
exported choice payload changed
exported enum discriminant changed
ownership semantics changed
ABI signature changed
native/WASM boundary changed
```

Unsafe reload can fall back to full module reload or full program restart depending on runner policy.

The VM must support replacing function records and invalidating stale JIT entrypoints.

---

## 16. Diagnostics

Diagnostics are graph anchored.

A diagnostic points to a source span and a graph node.

```txt
Diagnostic {
  code,
  severity,
  message,
  primary_span,
  related_spans,
  node_id,
  phase,
}
```

Diagnostics can be produced by:

```txt
parser
import resolver
name resolver
type checker
constraint checker
decorator checker
null safety checker
ownership checker
lowering validator
```

Because graph nodes survive incremental updates when possible, diagnostics can be updated without recomputing the full workspace.

---

## 17. Lowering to MIR

Lowering reads a lowerable `ModuleGraph` and produces MIR.

The graph preserves source-level intent.

MIR expresses VM-level operations.

Example source:

```galfus
var user = User {
  name: "Renato",
}
```

Semantic graph node:

```txt
StructInitNode {
  struct: User,
  fields: {
    name: "Renato",
    age: DefaultMarker,
  },
}
```

MIR conceptual form:

```txt
r0 = const_string "Renato"
r1 = default
r2 = call User::__init(r0, r1)
store_local user, r2
```

Lowering is responsible for:

```txt
turning source concepts into explicit operations
creating synthetic function calls
creating temporary roots
creating local root operations
creating strong edge operations
creating weak edge operations
expanding default parameters
expanding variadic collectors
applying decorator wrappers
lowering struct initialization
lowering choice construction
lowering enum discriminants
lowering casts
lowering ranges
lowering arrays
preserving debug links
```

---

## 18. MIR

MIR means Medium-level Intermediate Representation.

MIR is close to VM execution.

It is the main input to bytecode generation.

MIR makes these things explicit:

```txt
local roots
temporary roots
field writes
strong edges
weak edges
calls
casts
choice construction
enum discriminants
struct initialization
decorator application
range construction
array construction
variadic collection
copy thunks
cast thunks
```

MIR should not become the source-level semantic graph.

The distinction is:

```txt
ModuleGraph = source meaning and tooling state
MIR         = execution-oriented intermediate form
```

---

## 19. Bytecode

Bytecode is the VM instruction stream generated from MIR.

It is typed through module metadata.

Bytecode contains low-level executable instructions such as:

```txt
load constant
load local
store local
call function
call external
jump
branch
construct owner
write field
write strong edge
write weak edge
cast
return
leave root block
```

The bytecode is stored inside a `Galfus Module Image`.

The VM executes bytecode through the interpreter or uses it as input/context for JIT compilation.

---

## 20. Galfus Module Image

A Galfus Module Image is the VM-loadable representation of a module.

It can live:

```txt
in RAM
in disk cache
inside a package
inside an executable
inside a browser session
downloaded from the network
```

It contains:

```txt
bytecode
type table
function table
constant table
module table
import table
export table
struct layouts
choice layouts
enum discriminants
ownership metadata
debug links
ABI metadata
JIT metadata
```

The module image is the real execution unit.

The VM loads module images, not source graphs.

---

## 21. `.gfb`

`.gfb` means Galfus Binary.

It is the disk form of a Galfus Module Image.

```txt
main.gfb
```

A `.gfb` contains what the VM needs to execute:

```txt
header
format version
module name
module identity
export surface hash
type table
function table
import table
export table
constant table
bytecode
struct layouts
choice layouts
enum discriminants
ownership metadata
ABI metadata
JIT metadata
optional compact debug references
```

A `.gfb` is not necessarily native machine code.

It is a portable binary module for the Galfus VM.

---

## 22. `.gfb.map`

`.gfb.map` is the debug map for a `.gfb`.

```txt
main.gfb
main.gfb.map
```

It contains:

```txt
source spans
node -> source mapping
MIR -> source mapping
bytecode -> source mapping
synthetic function -> source mapping
original names
local variable metadata
scope metadata
inferred types
breakpoint data
watch data
stack trace data
external reference display data
```

It allows the debugger to show source-level information instead of internal VM details.

Example internal function:

```txt
User::__init
```

Can be shown to the user as:

```galfus
User {
  name: "Renato",
}
```

The debug map can be generated from the `ModuleGraph` debug layer and MIR lowering links.

---

## 23. VM core

The VM core executes Galfus Module Images.

It is designed to be host-agnostic.

Responsibilities:

```txt
load module image
link imports
execute bytecode
manage call frames
manage locals
manage temporaries
manage root blocks
perform casts
call functions
dispatch module calls
run Owner Graph Runtime
perform Affected Graph Release
support debug hooks
support JIT entrypoints
support native/WASM calls
support hot reload patch points
invalidate stale JIT entrypoints
```

The VM does not need to know source syntax.

It executes typed bytecode and runtime metadata.

---

## 24. Runner manager

The runner manager is the orchestration layer around the VM and frontend.

It handles:

```txt
CLI
watch mode
REPL
project loading
package resolution
module resolution
source graph construction
incremental graph updates
.gfb cache
.gfb.map cache
lint/check server
debug server
hot reload coordination
JIT policy selection
VM loader
host bridge
```

Conceptually:

```txt
galfus-run-manager
  +-- workspace graph
  +-- module resolver
  +-- frontend graph builder
  +-- incremental compiler
  +-- cache manager
  +-- lint/check server
  +-- debug server
  +-- VM loader
  +-- hot reload manager
  `-- host bridge
```

The VM core remains smaller and more portable.

---

## 25. JIT policy

Galfus supports configurable JIT modes.

```txt
--jit=off
--jit=lazy
--jit=hybrid
--jit=eager
```

Default general mode:

```txt
--jit=hybrid
```

Release mode:

```txt
--release => --jit=eager
```

### `off`

No native JIT.

Useful for:

```txt
debug
deterministic testing
WASM/browser VM
tooling
fallback mode
```

### `lazy`

Starts interpreted and compiles hot functions later.

```txt
interpret first
count calls/loops
detect hot paths
JIT hot functions
```

### `hybrid`

Default mode.

```txt
interpret cold functions
JIT small/obvious functions early
JIT hot paths lazily
preserve good debug behavior
preserve hot reload behavior
```

### `eager`

Default in release.

```txt
load module
compile eligible functions immediately
execute through compiled entrypoints
keep interpreter as fallback
```

Hot reload must invalidate or replace affected JIT entrypoints.

---

## 26. Function model

Every function has a VM-level function record.

```txt
GalfusFunction {
  module_id
  function_id
  name
  signature_id
  backend_kind
  bytecode_offset
  jit_entrypoint
  native_entrypoint
  wasm_entrypoint
  debug_ref
  version
}
```

Possible backend kinds:

```txt
InterpretedBytecode
JitCompiled
BuiltinCompiled
NativeCAbi
WasmCore
WasmComponent
```

Function calls use the function table.

The same source-level call syntax can reach different backends.

Hot reload may replace the function body while keeping a stable function identity when the signature remains compatible.

---

## 27. Synthetic functions

Synthetic functions are internal compiler-generated functions.

They are hidden from the user.

Examples:

```txt
struct initializers
choice constructors
decorator wrappers
field default initializers
module initializers
copy thunks
cast thunks
variadic argument collectors
ABI adapters
WASM adapters
```

They do not appear in:

```txt
normal autocomplete
normal documentation
public exports
ordinary reflection
standard stack traces
```

Debug tools can reveal them in compiler-internal mode.

The debug map maps synthetic functions back to source expressions.

---

## 28. Struct initialization

Each `struct` has one internal synthetic initializer.

Example source:

```galfus
struct User {
  name: String,
  age: int32 = 0,
}

var user = User {
  name: "Renato",
}
```

The semantic graph stores this as a struct initialization node with a default marker.

Conceptual lowering:

```txt
User::__init(
  name = "Renato",
  age = default
)
```

The `default` marker is internal.

It is:

```txt
not null
not public
not constructible by users
not part of normal type space
```

The synthetic initializer:

```txt
allocates the owner
applies field defaults
applies field decorators
validates required fields
registers strong edges
returns the owner handle
```

In release mode, `User::__init` is a strong candidate for eager JIT.

---

## 29. Module system

Modules are namespaces and compilation units.

A module can export:

```txt
const
var
fn
struct
enum
choice
constraint
type alias
submodule reexport
```

Example:

```galfus
export const version = 1

export struct User {
  name: String,
}

export fn createUser(name: String): User {
  return User {
    name,
  }
}
```

Imports are resolved by the runner manager and represented in the workspace graph.

The same import syntax can load different backend kinds.

---

## 30. Import backends

An import may resolve to:

```txt
Galfus source module      .gfs
Galfus binary module      .gfb
builtin module
native dynamic library    .dll / .so / .dylib
WASM core module          .wasm
WASM component            .wasm component
```

Examples:

```galfus
import string from "string"
import collections from "collections"
import game from "./game"
import physics from "./libphysics"
import fast_math from "./fast_math.wasm"
import image from "./image_component.wasm"
import engine from "engine"
```

To user code, all of them look like modules.

```galfus
string::trim(text)
physics::step(world, dt)
fast_math::add(a, b)
engine::spawn(entity)
```

In the graph, each imported symbol is represented by an `ExternalRef`.

---

## 31. C-ABI integration

Galfus can load native dynamic libraries.

```txt
Windows -> .dll
Linux   -> .so
macOS   -> .dylib
```

Example:

```galfus
import physics from "./libphysics"
```

Execution path:

```txt
Galfus call
  -> ABI adapter
  -> C-ABI
  -> native dynamic library
```

User code does not manipulate raw pointers.

Native bindings should map into safe Galfus-facing types:

```txt
int32
float32
bool
String
Buffer<uint8>
ABI-safe struct
opaque handle
Result
```

Native resources are represented as typed handles, not raw pointers.

Native imports expose an export surface and ABI metadata to the workspace graph.

---

## 32. WASM integration

Galfus supports two WASM integration modes.

### WASM core module

A WASM core module can be imported:

```galfus
import fast_math from "./fast_math.wasm"
```

Execution path:

```txt
Galfus VM
  -> WASM runtime
  -> WASM export
```

This supports modules compiled from:

```txt
Rust
C
Zig
AssemblyScript
other WASM-producing languages
```

### WASM Component

A WASM component can also be imported:

```galfus
import image from "./image_component.wasm"
```

WASM Component supports richer interface types:

```txt
records
variants
strings
lists
resources
imports
exports
```

This is useful for strongly typed integration with Rust and other component-capable toolchains.

WASM imports expose an export surface to the workspace graph.

---

## 33. Builtin modules

Builtin modules are standard modules shipped with the runtime.

Examples:

```txt
string
integer
float
math
reflect
collections
matrix
colors
serialize
```

Builtin modules may be compiled.

Example:

```galfus
string::trim(text)
math::sqrt(9.0)
collections::list::push(users, user)
```

These calls may execute as:

```txt
builtin compiled function
JIT intrinsic
VM bytecode
```

Builtin compiled functions must still respect Galfus semantics:

```txt
type safety
null safety
owner graph
strong edges
weak edges
root blocks
```

Builtin modules expose typed export surfaces to the workspace graph.

---

## 34. Owner Graph Runtime

Complex values are represented internally by owner handles.

Complex values include:

```txt
String
struct
array
tuple
List
Map
Set
Buffer
function
closure
union
choice with payload
```

Conceptual owner metadata:

```txt
OwnerMetaPointer {
  id
  generation
  type_id
  state: alive | releasing | released

  root_count
  strong_in_count

  strong_edges
  weak_edges

  payload
}
```

The Owner Graph Runtime is responsible for deterministic lifetime management.

Users do not manually destroy values.

---

## 35. Root blocks

A root block is a runtime/language unit that tracks local roots.

Example:

```galfus
{
  var a = Node { name: "A" }
  var b = Node { name: "B" }

  a.next = b
  b.next = a
}
```

At block exit:

```txt
leave_root_block(block)
```

The runtime:

```txt
removes local roots from the block
marks affected owners as candidates
checks affected graph fragments
destroys unreachable owner graphs
```

Roots can be:

```txt
local root
temporary root
module/global root
closure root
runner root
debugger root
```

Anything that must not be destroyed must be reachable through a root.

---

## 36. Affected Graph Release

Galfus does not perform global periodic heap tracing.

Instead, it uses Affected Graph Release.

When roots or strong edges are removed, affected owners become candidates.

The runtime analyzes only the affected graph fragments.

High-level algorithm:

```txt
1. root or strong edge is removed
2. affected owner is marked as candidate
3. candidate graph fragment is expanded through strong edges
4. nodes reachable from external roots are marked alive
5. unmarked nodes are unreachable
6. unreachable nodes are destroyed deterministically
```

This allows strong cycles to be collected without scanning the whole heap.

Example cycle:

```txt
a -> b
b -> a
```

If no external root reaches `a` or `b`, the whole cycle is released.

---

## 37. Weak references

`weak` references do not keep values alive.

Example:

```galfus
struct CacheEntry {
  weak resource: Resource | null = null,
}
```

Weak references must be nullable.

Invalid:

```galfus
weak resource: Resource
```

Weak load behavior:

```txt
target alive     -> T
target releasing -> null
target released  -> null
```

Weak collections:

```txt
WeakVec<T>
WeakMap<K, V>
WeakSet<T>
```

Weak handles can use owner id + generation internally.

---

## 38. Copy model

Complex assignment shares the owner handle.

```galfus
var a = User { name: "Ana" }
var b = a
```

`a` and `b` point to the same owner.

No implicit deep copy occurs.

No implicit shallow copy occurs.

Shallow copy:

```galfus
var b = User {
  ...a,
}
```

Deep copy:

```galfus
var c = copy a
```

Deep copy preserves:

```txt
cycles
topology
internal sharing
```

---

## 39. Runtime arrays and variadics

Galfus has two array forms.

Compile-time-sized array:

```galfus
var a: [int32; 3] = [1, 2, 3]
```

Runtime-sized fixed array:

```galfus
var size = 2
var a = buffer::array<int32>(size)
// type: [int32]
```

`[T; N]` means:

```txt
fixed array with compile-time-known length N
```

`[T]` means:

```txt
fixed array with runtime-known length n
```

Both are fixed after construction.

They are not `List<T>`.

A variadic function receives its arguments through a runtime-sized internal array:

```galfus
fn summarize(...values: [int32]): int32 {
  return values.length
}
```

Inside the function:

```txt
values: [int32]
```

The semantic graph represents the variadic parameter as a collector.

Lowering creates the internal runtime-sized array.

---

## 40. Debug architecture

Debugging uses `.gfb.map` or an in-memory `GalfusDebugMap`.

The debug map translates internal execution details back to source-level concepts.

It maps:

```txt
source node -> source span
source node -> semantic entity
semantic entity -> MIR operation
MIR operation -> bytecode offset
bytecode offset -> source span
synthetic function -> source expression
local slot -> variable name
runtime type -> source type
call frame -> source function
external ref -> import path/symbol
```

Example internal function:

```txt
User::__init
```

Can be shown as:

```galfus
User {
  name: "Renato",
}
```

The debugger should hide synthetic details by default.

Compiler-internal mode can reveal synthetic functions, MIR operations, and graph node ids.

---

## 41. Development mode

Default development command:

```txt
galfus run main.gfs
```

Default JIT policy:

```txt
--jit=hybrid
```

Development mode prioritizes:

```txt
fast edit-run cycle
good diagnostics
hot reload
module graph reuse
export surface comparison
source-level stack traces
incremental checking
in-memory debug maps
hybrid JIT
```

Typical dev flow:

```txt
source in memory
  -> ModuleGraph update
  -> affected graph layers rechecked
  -> changed functions lowered to MIR
  -> module image patched in memory
  -> debug map patched in memory
  -> VM hot reload
  -> hybrid execution
```

The `.gfb` file does not need to be written to disk.

---

## 42. Release mode

Release mode uses eager JIT by default.

```txt
galfus run --release main.gfs
```

Equivalent policy:

```txt
--jit=eager
```

Release mode prioritizes:

```txt
lower runtime overhead
compiled function entrypoints
compiled synthetic initializers
compiled builtin wrappers
compact module images
less debug overhead
no hot reload requirement
```

Release flow:

```txt
source or .gfb
  -> module graph if source is used
  -> module image
  -> eager JIT eligible functions
  -> execute
```

The interpreter remains available as fallback.

---

## 43. Browser/WASM mode

The Galfus VM can be compiled to WASM and run in the browser.

Use cases:

```txt
online playground
interactive documentation
browser demos
online tests
web apps without writing application logic in JavaScript
```

Flow:

```txt
.gfs or .gfb
  -> Galfus VM compiled to WASM
  -> interpret Galfus bytecode in browser
  -> execute
```

There may still be minimal JavaScript glue for loading WASM and connecting browser APIs.

Application logic can be written in Galfus.

Browser mode does not need native JIT.

It can use:

```txt
interpreter
quickening
builtin WASM implementations
```

The frontend may also run in browser mode for playgrounds by building in-memory module graphs.

---

## 44. Architecture summary

```txt
.gfs source
  -> Lexer / Parser
  -> ModuleGraph syntax layer
  -> Import resolution
  -> Name resolution
  -> Type checking
  -> Semantic checking
  -> Ownership metadata preparation
  -> Export surface generation
  -> External reference validation
  -> Lowering metadata
  -> MIR
  -> Bytecode
  -> Galfus Module Image
     +-- bytecode
     +-- metadata
     +-- debug links
  -> VM Loader
  -> JIT Policy
     +-- default: hybrid
     `-- release: eager
  -> Execution
     +-- interpreted bytecode
     +-- JIT compiled functions
     +-- builtin compiled modules
     +-- native C-ABI dylibs
     +-- WASM core modules
     `-- WASM components
```

Workspace architecture:

```txt
WorkspaceGraph
  +-- ModuleGraph(main)
  |     +-- syntax layer
  |     +-- name layer
  |     +-- type layer
  |     +-- semantic layer
  |     +-- ownership layer
  |     +-- export surface
  |     +-- external refs
  |     `-- debug/lowering links
  |
  +-- ModuleGraph(user)
  |     `-- export surface
  |
  +-- BuiltinModule(string)
  |     `-- export surface
  |
  +-- NativeModule(physics)
  |     `-- ABI export surface
  |
  `-- WasmModule(fast_math)
        `-- WASM export surface
```

In memory:

```txt
WorkspaceGraph
ModuleGraph
GalfusModuleImage
GalfusDebugMap
```

On disk:

```txt
main.gfb
main.gfb.map
module graph cache, optional
```

Core principles:

```txt
typed VM
module-local SemanticGraph
no global frontend object
stable export surfaces
stable cross-module external refs
incremental graph updates
hot reload through module image patching
MIR remains execution-oriented
VM remains source-agnostic
deterministic Owner Graph runtime
affected graph release
no global periodic heap tracing
multi-backend imports
hybrid JIT by default
eager JIT in release
WASM VM for browser execution
```
