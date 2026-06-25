# Galfus MVP Reference

## 1. Purpose

The Galfus MVP exists to validate the language itself, not the full ecosystem.

The MVP must prove that Galfus can take a complete `.gfs` program, validate it through the full frontend, lower it into a minimal executable representation, serialize it as `.gfb`, load it into the VM, and execute it with the required memory model.

The MVP goal is:

```txt
.gfs
  -> compiler frontend
  -> semantic validation
  -> ownership validation
  -> MIR
  -> Galfus Module Image
  -> .gfb
  -> VM
  -> execution
```

The MVP is intentionally not a product-distribution milestone. It does not need publishing, packages, adapters, registry support, multi-target bundles, debug tooling, or runtime optimization tiers.

## 2. MVP Definition

The Galfus MVP is defined as:

```txt
full language validation + minimal execution system
```

This means:

```txt
Syntax: complete
Semantics: complete
Compiler frontend: complete enough to validate all language rules
Runtime: minimal
Workspace: local and explicit
Ecosystem: excluded
```

The MVP should validate the language surface that has already been defined, while keeping the runtime, workspace, and distribution model as small as possible.

## 3. Included in the MVP

## 3.1 Full Syntax

The MVP includes the full Galfus syntax defined by the current syntax reference.

This includes, but is not limited to:

```txt
source files
comments
identifiers
imports
named imports
exports
var
const
primitive types
numeric literals
boolean literals
null literal
string literals as UTF-8 [uint8]
array literals
array spread
array types
fixed-size array types
indexing
negative indexing
tuple types
tuple expressions
grouped expressions
grouped types
struct declarations
struct field defaults
struct const fields
struct literals
struct shorthand
inferred struct literals
struct expansion
struct literal spread
enums
enum discriminants
enum base type
enum value access
enum casts
choices
generic choices
choice construction
type aliases
named types
path types
generic types
union types
function types
casts
operators
fallback
fallback assignment
member access
null-safe member access
functions
stamped functions
default parameters
rest parameters
trailing arguments
arrow functions
anchor functions for structs
generics
constraints
satisfies
decorators
destructuring
ranges
if / else
for in
while
loop
break
continue
return
match
instanceof
weak fields
```

The MVP must parse all accepted syntax and reject invalid syntax with useful diagnostics.

## 3.2 Full Semantics

The MVP includes the full Galfus semantic model defined by the current semantic reference.

This includes, but is not limited to:

```txt
module-local semantic graphs
private and exported symbols
import binding semantics
path resolution
top-level initialization
local bindings
mutable and immutable bindings
type annotations
type inference
primitive scalar semantics
null semantics
union types
union narrowing
string literal semantics as [uint8]
array semantics
fixed-size arrays
runtime-sized arrays
negative indexing
out-of-bounds indexing returning null
tuple semantics
choice payload tuple semantics
struct semantics
shallow copy
deep copy as explicit behavior
enum symbol preservation
enum casts
choice construction
choice matching
choice exhaustiveness
type alias symbol preservation
alias assignability
cast semantics
numeric cast semantics
boolean short-circuiting
null fallback
fallback assignment
function semantics
stamped function semantics
function return semantics
default parameter gaps
rest parameters
arrow function semantics
closure capture semantics
anchor functions on structs
anchor calls without implicit write-back
generic semantics
constraint semantics
satisfies semantics
decorator semantics
decorator order
destructuring semantics
range semantics
iterator and iterable constraints
if/else semantics
loop semantics
for-in semantics
match as expression
instanceof as expression
statement usage by discarding expression result
weak field semantics
anchor / edge / weak ownership model
ownership validation
module initialization
import cycles
runtime panic semantics
artifact metadata separation
```

The MVP must validate all semantic rules before `.gfb` generation.

## 3.3 Compiler Frontend

The MVP includes the full compiler frontend required to validate the language.

Required components:

```txt
lexer
parser
resolver
type checker
semantic checker
ownership checker
basic diagnostics
```

The frontend is allowed to run as a build-time tool only. It does not need to exist as a runtime `compiler` module in the MVP.

The MVP does not include runtime compilation.

The frontend owns module-local validation and typed import/export surfaces. The
runner owns the workspace graph and connects resolved surfaces to frontend
modules. Future `.gfb` and `.gfp` imports remain runner/tooling work and are not
required to close the frontend MVP.

## 3.4 Lowering Pipeline

The MVP includes the complete lowering path from validated source to executable image.

Required components:

```txt
MIR builder
ownership metadata preparation
anchor metadata preparation
edge metadata preparation
weak metadata preparation
bytecode writer
Galfus Module Image builder
.gfb serializer
```

The MVP must produce a valid Galfus Module Image before writing `.gfb`.

## 3.5 Galfus Module Image

The Galfus Module Image is the minimal in-memory executable representation required by the VM.

It contains the data the VM needs to execute a module, such as:

```txt
bytecode
constant pool
function table
type table
layout table
import slots
export slots
module init data
ownership metadata
anchor / edge / weak metadata
minimal runtime metadata
integrity metadata
```

The Module Image must not contain frontend-only data such as source text, rich diagnostics, autocomplete paths, full source spans, or source reconstruction data.

## 3.6 `.gfb`

The `.gfb` file is the serialized form of a Galfus Module Image.

MVP build flow:

```txt
.gfs
  -> compiler frontend
  -> MIR
  -> Galfus Module Image
  -> .gfb serialization
```

MVP execution flow:

```txt
.gfb
  -> validation
  -> deserialization
  -> Galfus Module Image
  -> VM execution
```

The MVP must generate and execute `.gfb` files.

## 3.7 Runtime Kernel

The MVP runtime is minimal.

Required runtime kernel:

```txt
vm_core
owner_graph_core
```

The kernel is not a module. It is the irreducible runtime required to execute `.gfb`.

## 3.8 VM Core

The MVP VM must support:

```txt
.gfb loading
Module Image validation
Module Image execution
bytecode interpretation
call frames
locals
temporaries
function calls
returns
casts
control flow
module initialization
minimal panic handling
```

The MVP VM is interpreter-first.

## 3.9 Owner Graph Core

The MVP includes `owner_graph_core` because deterministic ownership is part of the language identity.

Required ownership capabilities:

```txt
lifetime anchors
edges
weak observers
release of affected graph fragments
deterministic release
cycle-safe release
weak invalidation
```

The MVP must validate the anchor / edge / weak model at compile time and execute it at runtime.

## 3.10 Panic

Runtime failures produce `panic`.

In the MVP, panic reporting may be minimal, but it must identify the execution failure clearly enough to debug the runtime and compiler.

The final architecture defines panic as aborting the whole Galfus execution process and reporting the available module trace. The MVP may implement a minimal version of this behavior.

## 3.11 Local Workspace

The MVP supports a local, explicit workspace model.

Minimum project structure:

```txt
my-app/
  galfus.toml
  src/
    main.gfs
  build/
```

The MVP may support local multi-file projects:

```txt
my-app/
  galfus.toml
  src/
    main.gfs
    user.gfs
    result.gfs
  build/
```

The `build/` directory is disposable.

## 3.12 Minimal `galfus.toml`

For an app:

```toml
[module]
name = "my-app"
target = "app"
entry = "src/main.gfs"
```

For a local library-style project:

```toml
[module]
name = "my-lib"
target = "lib"

[exports]
"my/user" = "src/user.gfs"
"my/result" = "src/result.gfs"
```

The MVP does not need publishing metadata, registry metadata, dependency version resolution, or target packaging metadata.

## 3.13 Local Imports

The MVP should support local imports because the language needs module validation.

Examples:

```galfus
import user from "./user"
import { Result } from "./result"
```

The MVP may also support local aliases if already simple to implement:

```toml
[alias]
shared = "src/shared.gfs"
```

```galfus
import shared from "$shared"
```

The MVP does not support published dependencies.

## 4. Excluded from the MVP

The MVP excludes ecosystem, platform, optimization, and distribution features that are not required to validate the language.

## 4.1 Runtime Optimization Exclusions

Excluded:

```txt
JIT
quickening
aggressive optimization tiers
runtime profiles beyond the minimal runtime
fast profile
std profile
dev profile
```

The MVP runtime behaves like the minimal execution profile.

## 4.2 Artifact Exclusions

Excluded:

```txt
.gfp
.gfm
```

The MVP only needs:

```txt
.gfs
.gfb
```

## 4.3 Adapter Exclusions

Excluded:

```txt
platform adapters
native adapters
WASM adapters
mobile adapters
embedded adapters
external payload bridges
C ABI bridges
host capability adapters
```

The MVP runtime does not integrate with external platform APIs beyond what is needed to run tests and inspect execution results.

## 4.4 Special Module Exclusions

Excluded builtin/special modules:

```txt
math
string
text
regex
collection
compiler as runtime module
```

The language can still validate constraints, generics, decorators, choices, structs, arrays, and other features through local test modules.

For example, a local test file can define its own `Comparable<T>` constraint without requiring a published builtin module.

## 4.5 Bundle Exclusions

Excluded:

```txt
bundle command
single distribution packaging
executable packaging
firmware packaging
APK/AAB/AAR packaging
app bundle packaging
WASM package generation
native archive/package generation
```

The MVP only needs to build and execute `.gfb`.

`.gfb` and `.gfp` dependency consumption is outside the frontend closure target.
The MVP validates local `.gfs` modules through frontend-generated surfaces before
lowering.

## 4.6 Dependency and Publishing Exclusions

Excluded:

```txt
published dependencies
registry
cache dependency model
galfus.lock
publishing system
versioning system
published .gfb resolution
published .gfp resolution
```

The MVP does not publish modules.

It also does not need to consume published modules.

## 4.7 Tooling and Debug Exclusions

Excluded:

```txt
runtime compiler module
runtime compilation
hot reload
debug hooks
breakpoints
debug trace
owner_graph_extra
source reconstruction
IDE autocomplete metadata
rich .gfm diagnostics
```

The build-time compiler should still provide basic diagnostics.

## 4.8 Sandbox Exclusions

Excluded:

```txt
server sandbox policy
max memory configuration
max stack configuration
max step/fuel configuration
adapter capability policy
multi-tenant execution controls
```

Sandboxing is a runtime configuration feature for later stages, not an MVP requirement.

## 4.9 Platform Target Exclusions

Excluded:

```txt
desktop executable target
server package target
web/WASM target
Android target
iOS target
embedded target
multi-target output selection
target-specific toolchain requirements
```

The MVP may run on the developer machine, but it does not need to produce platform-specific distribution artifacts.

## 5. MVP Validation Strategy

The MVP should be validated with local `.gfs` programs that exercise the full language.

Recommended validation groups:

```txt
primitive values and casts
arrays and negative indexing
string literals as [uint8]
tuples
structs
struct defaults and const fields
enums and enum casts
choices and match
unions and null narrowing
instanceof expressions
functions
stamped functions
anchor functions on structs
generics
constraints
satisfies
decorators
destructuring
ranges
for in with iterator/iterable constraints
weak fields
ownership validation
module imports and exports
.gfb serialization
VM execution
panic behavior
```

These tests can use local modules only.

## 6. MVP Success Criteria

The MVP is successful when all of the following are true:

1. The compiler parses the full accepted syntax.
2. The compiler rejects invalid syntax with useful diagnostics.
3. The resolver builds correct module-local semantic graphs.
4. The type checker validates all core type rules.
5. The semantic checker validates all current language semantics.
6. The ownership checker validates anchors, edges, and weak fields.
7. The compiler lowers valid programs into MIR.
8. The compiler lowers MIR into a Galfus Module Image.
9. The compiler serializes the Module Image into `.gfb`.
10. The VM loads and validates `.gfb`.
11. The VM executes bytecode correctly.
12. The owner graph core releases values deterministically.
13. Runtime failures produce panic.
14. Local imports and exports work.
15. No excluded ecosystem feature is required to run MVP programs.

## 7. MVP Non-Goals

The MVP does not aim to prove:

```txt
package publishing
registry resolution
multi-platform packaging
runtime adapters
external payload integration
runtime compilation
JIT performance
debugger integration
IDE integration
sandbox hosting
standard library completeness
```

Those belong to later milestones.

## 8. Summary

The Galfus MVP is not a reduced language.

It is a reduced system around the complete language.

```txt
Language surface: complete
Semantic validation: complete
Compiler pipeline: complete
Runtime: minimal
Workspace: local only
Artifacts: .gfs and .gfb only
Ecosystem: excluded
```

The MVP should prove that Galfus works as a typed VM-first language with deterministic ownership and compact `.gfb` execution, while postponing all platform, publishing, adapter, optimization, and ecosystem concerns.
