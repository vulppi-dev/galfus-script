# Galfus Script - Development Project Reference

This document defines the standard development project layout for Galfus Script.

It complements the syntax and base architecture references by specifying how a project is organized on disk, how `galfus.toml` describes a module, how local aliases and public exports work, how module discovery is performed, and how external payloads are attached through Galfus proxy files.

The goal is a compact, deterministic, explicit project model.

---

## Table of Contents

1. [Identity](#1-identity)
2. [Core Principles](#2-core-principles)
3. [Standard Project Layout](#3-standard-project-layout)
4. [Root Directories](#4-root-directories)
5. [`galfus.toml`](#5-galfustoml)
6. [`[module]`](#6-module)
7. [`[paths]`](#7-paths)
8. [`[exports]`](#8-exports)
9. [`[alias]`](#9-alias)
10. [Import Address Model](#10-import-address-model)
11. [Module Discovery](#11-module-discovery)
12. [Artifact Roles](#12-artifact-roles)
13. [Galfus Proxy Files (`.gfp`)](#13-galfus-proxy-files-gfp)
14. [Galfus Map Files (`.gfm`)](#14-galfus-map-files-gfm)
15. [Application Modules](#15-application-modules)
16. [Library Modules](#16-library-modules)
17. [Workspace / Monorepo Layout](#17-workspace--monorepo-layout)
18. [Deterministic Resolution Rules](#18-deterministic-resolution-rules)
19. [Build and Bundle Behavior](#19-build-and-bundle-behavior)
20. [Examples](#20-examples)
21. [Validation Rules](#21-validation-rules)
22. [Non-Goals](#22-non-goals)
23. [Summary](#23-summary)

---

## 1. Identity

A Galfus development project is a deterministic graph of Galfus modules.

A project is not described as a package-first structure. It is described as a module-first structure.

The central configuration file is:

```txt
galfus.toml
```

The standard root directories are:

```txt
src/
modules/
build/
```

A standard project therefore starts as:

```txt
my-module/
  galfus.toml
  src/
  modules/
  build/
```

The main design rule is:

```txt
Configuration is explicit.
There is no automatic central source file.
There is no implicit index file.
There is no implicit mod file.
```

Galfus does not use automatic `index`, `mod`, `main`, or directory-entry conventions to decide module identity.

Entrypoints and exports are declared explicitly in `galfus.toml`.

---

## 2. Core Principles

```txt
module-first project model
explicit entrypoints
explicit exports
explicit local aliases
deterministic filesystem discovery
TOML configuration
no automatic index/mod/root file convention
no arbitrary import loaders
imports resolve only to Galfus module descriptors
external binaries are payloads, not import targets
compact release bundles
optional debug/tooling maps
```

A Galfus project avoids implicit loader behavior similar to Node.js, Deno, Bun, or bundler ecosystems.

This means Galfus imports do not directly load arbitrary file formats.

Valid import targets are always Galfus module descriptors:

```txt
.gfs  Galfus source
.gfb  Galfus binary
.gfp  Galfus proxy for external payloads
```

External payloads are not direct import targets:

```txt
.wasm
.dll
.so
.dylib
```

External payloads are reachable only through `.gfp` files.

---

## 3. Standard Project Layout

A simple application project:

```txt
my-app/
  galfus.toml
  src/
    root.gfs
  modules/
  build/
```

A library project:

```txt
my-lib/
  galfus.toml
  src/
    root.gfs
    math/
      root.gfs
    vectors/
      root.gfs
  modules/
  build/
```

A project with external payloads:

```txt
my-app/
  galfus.toml
  src/
    root.gfs
  modules/
    physics/
      physics.gfp
      libphysics.so
    fast_math/
      fast_math.gfp
      fast_math.wasm
    collections/
      collections.gfb
      collections.gfm
  build/
```

The resolver discovers Galfus module descriptors.

It does not discover native or WASM payloads directly.

---

## 4. Root Directories

### `src/`

`src/` contains project-owned Galfus source files.

```txt
src/
  root.gfs
  user.gfs
  math/
    root.gfs
  vectors/
    vec2.gfs
    vec3.gfs
```

No file inside `src/` is special by filename alone.

A source file becomes an entry module only if referenced by:

```txt
[module].entry
[exports]
[alias]
a relative import
another resolved module
```

### `modules/`

`modules/` contains attached dependencies and local module artifacts.

It may contain:

```txt
.gfs
.gfb
.gfm
.gfp
.wasm
.dll
.so
.dylib
```

However, only these are module discovery candidates:

```txt
.gfs
.gfb
.gfp
```

`.gfm` is a sidecar map for `.gfb`.

External payloads are referenced by `.gfp` files.

### `build/`

`build/` contains generated files.

```txt
build/
  cache/
  debug/
  release/
  bundle/
  logs/
```

`build/` must be disposable.

A project must remain reproducible after deleting `build/`, assuming all source files, module descriptors, dependency artifacts, lockfiles, and configured external payloads still exist.

---

## 5. `galfus.toml`

`galfus.toml` is the canonical project configuration file.

TOML is used because it stays readable as configuration grows and avoids the visual noise of large JSON files.

A minimal application configuration:

```toml
[module]
name = "my-module"
version = "0.0.1"
target = "app"
entry = "src/root.gfs"
```

A fuller configuration:

```toml
[module]
name = "my-module"
version = "0.0.1"
target = "app"
organization = "vulppi"
entry = "src/root.gfs"

[paths]
src = "src"
modules = "modules"
build = "build"

[exports]
"my/math" = "src/math/root.gfs"
"my/vectors" = "src/vectors/root.gfs"

[alias]
math = "src/math/root.gfs"
vectors = "src/vectors/root.gfs"
internal = "src/internal"
```

The configuration avoids the term `package`.

Galfus projects are described as modules because compilation, resolution, imports, exports, and bundling are all graph operations over modules.

---

## 6. `[module]`

The `[module]` table defines the identity and target kind of the project module.

```toml
[module]
name = "my-module"
version = "0.0.1"
target = "app"
organization = "vulppi"
entry = "src/root.gfs"
```

Required fields:

```txt
name
version
target
```

Optional fields:

```txt
organization
entry, depending on target
```

### `name`

The module name is the stable public identity of the module.

Recommended form:

```txt
kebab-case
```

Examples:

```toml
name = "my-module"
name = "math-core"
name = "game-runtime"
```

### `version`

The module version identifies the published or local module version.

Recommended form:

```txt
semver
```

Example:

```toml
version = "0.0.1"
```

### `target`

Initial target kinds:

```txt
app
lib
```

Future target kinds may include:

```txt
proxy
tool
test
```

Target validation:

```txt
target = "app"
  requires [module].entry

target = "lib"
  requires [module].entry or at least one [exports] entry
```

### `organization`

`organization` is optional.

When present, public import addresses use the `@organization/module` form.

Example:

```toml
organization = "vulppi"
name = "my-module"
```

Public address root:

```txt
@vulppi/my-module
```

When absent, public addresses use the module name directly:

```txt
my-module
```

### `entry`

`entry` points to a Galfus source module descriptor.

Example:

```toml
entry = "src/root.gfs"
```

For applications, `entry` is required.

For libraries, `entry` is optional if explicit exports exist.

There is no implicit fallback to `src/index.gfs`, `src/mod.gfs`, `src/main.gfs`, or any equivalent automatic root file.

---

## 7. `[paths]`

`[paths]` defines project-local directory names.

```toml
[paths]
src = "src"
modules = "modules"
build = "build"
```

Default values:

```txt
src     = "src"
modules = "modules"
build   = "build"
```

If omitted, the defaults are used.

Paths must be relative to the directory that contains `galfus.toml`, unless explicitly defined otherwise by a future workspace policy.

The standard directory names are recommended for normal projects.

---

## 8. `[exports]`

`[exports]` defines public module export addresses.

Exports are explicit.

There is no export glob by default.

Example:

```toml
[exports]
"my/math" = "src/math/root.gfs"
"my/vectors" = "src/vectors/root.gfs"
```

This means:

```txt
Public export address: my/math
Source module:         src/math/root.gfs

Public export address: my/vectors
Source module:         src/vectors/root.gfs
```

When the module has an organization and module name:

```toml
[module]
organization = "vulppi"
name = "my-module"
```

The public import addresses become:

```txt
@vulppi/my-module/my/math
@vulppi/my-module/my/vectors
```

Exports are module addresses, not namespaces.

This distinction is important.

```txt
Export path:
  public graph address used by import resolution

Namespace:
  internal naming/symbol structure owned by the imported module itself
```

A consumer chooses the local binding name in the import statement:

```galfus
import vectors from "@vulppi/my-module/my/vectors"
```

Then the consumer uses that local binding:

```galfus
vectors::Vec3
vectors::length(v)
```

The name `vectors` in code is the local import binding, not the export path namespace.

---

## 9. `[alias]`

`[alias]` defines local import shortcuts.

Alias keys do not include `$` in `galfus.toml`.

Example:

```toml
[alias]
math = "src/math/root.gfs"
vectors = "src/vectors/root.gfs"
internal = "src/internal"
```

The `$` prefix appears only in Galfus import strings.

```galfus
import math from "$math"
import vectors from "$vectors"
import internal from "$internal"
```

Resolution rule:

```txt
"$math"     -> [alias].math
"$vectors"  -> [alias].vectors
"$internal" -> [alias].internal
```

Aliases are local to the current project module.

They are not public exports.

They do not affect dependency consumers.

Aliases exist to make project-internal imports explicit and stable without relying on long relative paths.

Alias values may point to:

```txt
Galfus source files
Galfus binary modules
Galfus proxy files
Galfus module directories, if directory alias resolution is enabled
```

Physical candidates are still restricted to:

```txt
.gfs
.gfb
.gfp
```

---

## 10. Import Address Model

A Galfus import string is a module address.

It is not a generic file loader instruction.

Supported address forms:

```txt
$alias
@organization/module/export/path
module/export/path
./relative/path
../relative/path
builtin-name
```

Examples:

```galfus
import math from "$math"
import vectors from "@vulppi/my-module/my/vectors"
import localUser from "./user"
import string from "string"
```

### `$alias`

Alias imports resolve through `[alias]`.

```galfus
import math from "$math"
```

Looks up:

```toml
[alias]
math = "src/math/root.gfs"
```

### `@organization/module/export/path`

Organized public imports start with `@`.

```galfus
import vectors from "@vulppi/my-module/my/vectors"
```

Address structure:

```txt
@vulppi/my-module/my/vectors
│       │         └─ export path
│       └─ module name
└─ organization
```

### `module/export/path`

Unorganized public imports do not use `@`.

```galfus
import math from "math-core/math/vector"
```

Address structure:

```txt
math-core/math/vector
│         └─ export path
└─ module name
```

### `./relative/path` and `../relative/path`

Relative imports resolve from the importing source module location.

```galfus
import user from "./user"
import utils from "../utils/root"
```

The resolver applies Galfus module candidate discovery only:

```txt
./user.gfs
./user.gfb
./user.gfp
```

It does not try arbitrary extensions.

### Builtins

Builtin imports are Galfus module surfaces provided by the runtime/toolchain.

Example:

```galfus
import string from "string"
```

Builtins are still resolved as Galfus module surfaces, not arbitrary host loaders.

---

## 11. Module Discovery

The module resolver discovers only Galfus module descriptors.

Candidate extensions:

```txt
.gfs
.gfb
.gfp
```

Sidecar extension:

```txt
.gfm
```

External payload extensions:

```txt
.wasm
.dll
.so
.dylib
```

Rules:

```txt
.gfs/.gfb/.gfp are module candidates.
.gfm is an optional sidecar for .gfb.
.wasm/.dll/.so/.dylib are not module candidates.
.wasm/.dll/.so/.dylib are reachable only through .gfp.
```

The resolver must not behave like a loader pipeline.

Invalid imports:

```galfus
import image from "./image.wasm"
import physics from "./libphysics.so"
import data from "./data.json"
import config from "./config.toml"
```

Valid alternatives:

```galfus
import image from "./image"
import physics from "./physics"
```

Where discovery finds:

```txt
image.gfp
physics.gfp
```

And those proxy files point to their external payloads.

---

## 12. Artifact Roles

Galfus project artifacts have separate responsibilities.

```txt
.gfs = source
.gfb = compact Galfus execution artifact
.gfm = Galfus map for debug/tooling/source reconstruction
.gfp = human-readable proxy for external payload binding
.wasm/.dll/.so/.dylib = external payloads referenced by .gfp
```

### `.gfs`

`.gfs` files contain Galfus Script source.

They are parsed by the frontend and lowered through the Galfus compilation pipeline.

### `.gfb`

`.gfb` means Galfus Binary.

It is the compact executable artifact loaded by the Galfus VM.

It should contain execution-relevant compact data such as:

```txt
bytecode
constant pool
function table
type table
layout table
import slots
export slots
ownership metadata
module init data
minimal runtime metadata
```

It is not a human-readable format.

### `.gfm`

`.gfm` means Galfus Map.

It reconstructs the human/tooling/debug view of a `.gfb`.

It is optional for execution.

### `.gfp`

`.gfp` means Galfus Proxy.

It is a human-readable binding descriptor for external payloads.

It exposes a Galfus-facing module surface and describes how that surface binds to a WASM or native C-ABI payload.

### External payloads

External payloads include:

```txt
.wasm
.dll
.so
.dylib
```

They are not imported directly by Galfus source.

They are bound through `.gfp` files and converted into adapter descriptors during build/bundle resolution.

---

## 13. Galfus Proxy Files (`.gfp`)

A `.gfp` file describes an external module as a Galfus module surface.

It is human-readable.

It is declarative.

It should be deterministic.

It should not execute scripts during resolution.

Conceptual native proxy:

```txt
module physics from native {
  payload "./libphysics.so"
  hash "..."

  type World = opaque

  fn createWorld(): World
    symbol "physics_create_world"

  fn step(world: World, dt: float32): null
    symbol "physics_step"

  fn destroyWorld(world: World): null
    symbol "physics_destroy_world"
}
```

Conceptual WASM proxy:

```txt
module fast_math from wasm {
  payload "./fast_math.wasm"
  hash "..."

  fn add(a: int32, b: int32): int32
    export "add"
}
```

A `.gfp` should describe:

```txt
module name
backend kind
payload path
payload hash
Galfus-facing exports
ABI mapping
ownership/resource policy
string/buffer policy
struct/layout policy, if needed
error/result mapping
platform constraints, for native payloads
adapter requirements
```

Valid backend kinds may include:

```txt
wasm_core
wasm_component
native_c_abi
host_bridge
```

A `.gfp` is not a debug map.

It is the deterministic bridge between an external payload and the Galfus module graph.

---

## 14. Galfus Map Files (`.gfm`)

A `.gfm` file is an optional sidecar for `.gfb`.

When present next to a `.gfb`, it may be attached for:

```txt
IDE metadata
source reconstruction
stack traces
profiling labels
debugger breakpoints
watch expressions
human-readable graph output
symbolication
```

Discovery rule:

```txt
if candidate == module.gfb:
  load module.gfb
  if module.gfm exists:
    attach module.gfm for tooling/debug
```

A `.gfm` is not required for execution.

A release bundle may omit `.gfm` by default.

A debug build or private symbolication build may keep `.gfm`.

The `.gfm` should not be used as the binding descriptor for external payloads.

External binding belongs to `.gfp`.

---

## 15. Application Modules

An application module uses:

```toml
[module]
target = "app"
entry = "src/root.gfs"
```

`entry` is required.

Example:

```toml
[module]
name = "my-game"
version = "0.0.1"
target = "app"
organization = "vulppi"
entry = "src/root.gfs"

[alias]
engine = "modules/engine/engine.gfb"
math = "src/math/root.gfs"
```

Application modules may also expose explicit exports, but they do not need to.

Example application with public exports:

```toml
[module]
name = "my-game"
version = "0.0.1"
target = "app"
organization = "vulppi"
entry = "src/root.gfs"

[exports]
"tools/map-editor" = "src/tools/map_editor/root.gfs"
```

This is useful for applications that also expose tool modules, plugin surfaces, or development-only modules.

---

## 16. Library Modules

A library module uses:

```toml
[module]
target = "lib"
```

A library must define at least one of:

```txt
[module].entry
[exports]
```

Library with entry:

```toml
[module]
name = "math-core"
version = "0.0.1"
target = "lib"
organization = "vulppi"
entry = "src/root.gfs"
```

External import address:

```txt
@vulppi/math-core
```

Library with explicit exports:

```toml
[module]
name = "math-core"
version = "0.0.1"
target = "lib"
organization = "vulppi"

[exports]
"scalar" = "src/scalar/root.gfs"
"vector" = "src/vector/root.gfs"
"matrix" = "src/matrix/root.gfs"
```

External import addresses:

```txt
@vulppi/math-core/scalar
@vulppi/math-core/vector
@vulppi/math-core/matrix
```

Library with both entry and exports:

```toml
[module]
name = "math-core"
version = "0.0.1"
target = "lib"
organization = "vulppi"
entry = "src/root.gfs"

[exports]
"vector" = "src/vector/root.gfs"
"matrix" = "src/matrix/root.gfs"
```

External import addresses:

```txt
@vulppi/math-core
@vulppi/math-core/vector
@vulppi/math-core/matrix
```

---

## 17. Workspace / Monorepo Layout

A workspace is a root project that groups multiple Galfus modules.

Recommended layout:

```txt
my-workspace/
  galfus.toml
  workspace/
    engine/
      galfus.toml
      src/
      modules/
      build/
    game/
      galfus.toml
      src/
      modules/
      build/
    editor/
      galfus.toml
      src/
      modules/
      build/
  modules/
  build/
```

Root workspace configuration:

```toml
[workspace]
members = [
  "workspace/engine",
  "workspace/game",
  "workspace/editor",
]
```

Each workspace member has its own `galfus.toml`.

Each member remains a Galfus module project.

Workspace members should be resolved before external dependencies when a matching module identity exists.

Recommended precedence:

```txt
1. local aliases
2. relative imports
3. workspace members
4. local modules directory
5. registry/cache dependencies
6. builtins
```

The exact package registry and lockfile format are outside this document.

---

## 18. Deterministic Resolution Rules

Import resolution should be deterministic and order-independent after canonicalization.

Recommended top-level import address classification:

```txt
1. starts with "$"
   -> local alias

2. starts with "@"
   -> organized module address

3. starts with "./" or "../"
   -> relative Galfus module path

4. otherwise
   -> unorganized module address, workspace module, dependency module, or builtin
```

Physical candidate extensions:

```txt
.gfs
.gfb
.gfp
```

Recommended candidate priority for local development:

```txt
1. .gfs
2. .gfb
3. .gfp
```

Recommended candidate priority for published dependency resolution:

```txt
1. .gfb
2. .gfp
3. .gfs, only if source dependencies are enabled
```

A project or workspace policy may override candidate priority, but the priority must be explicit and deterministic.

Directory alias resolution, if enabled, must also be deterministic:

```txt
alias = "src/vectors"

"$vectors/vec2" -> src/vectors/vec2.gfs
"$vectors/vec3" -> src/vectors/vec3.gfs
```

No implicit root file is used for directory aliases unless explicitly configured.

Invalid implicit behavior:

```txt
"$vectors" -> src/vectors/index.gfs
"$vectors" -> src/vectors/mod.gfs
"$vectors" -> src/vectors/root.gfs
```

Valid explicit behavior:

```toml
[alias]
vectors = "src/vectors/root.gfs"
```

---

## 19. Build and Bundle Behavior

During development, the manager may use:

```txt
.gfs source
.gfb dependencies
.gfm sidecars for tooling/debug
.gfp proxy files
external payloads referenced by .gfp
```

Build flow:

```txt
.gfs/.gfb/.gfp discovery
  -> module records
  -> export surfaces
  -> external references
  -> adapter descriptors from .gfp
  -> validation
  -> bytecode / module image generation
  -> .gfb
  -> optional .gfm
```

Bundle flow:

```txt
entry module
  -> dependency discovery
  -> BundleGraph
  -> reachability analysis
  -> tree-shaking
  -> unused runtime slice removal
  -> unused adapter removal
  -> minimal bridge manifest
  -> compact .gfb bundle
  -> selected runtime slices
  -> selected external payloads
  -> signature / integrity metadata
```

Final release bundles do not require `.gfm` or `.gfp` by default.

Reason:

```txt
.gfp has already been compiled into adapter descriptors and a minimal bridge manifest.
.gfm is only needed for debug/tooling/source reconstruction.
```

A final bundle may retain debug maps only when explicitly requested.

Examples:

```txt
debug build:
  app.gfb
  app.gfm

release build:
  app.gfb

release executable bundle:
  app executable
  compact VM
  compact .gfb bundle
  selected runtime slices
  selected external payloads
  minimal bridge manifest
  signature
```

---

## 20. Examples

### Simple app

```txt
hello/
  galfus.toml
  src/
    root.gfs
  modules/
  build/
```

```toml
[module]
name = "hello"
version = "0.0.1"
target = "app"
entry = "src/root.gfs"
```

`src/root.gfs`:

```galfus
import string from "string"

fn main(): null {
  return
}
```

### App with aliases

```toml
[module]
name = "my-game"
version = "0.0.1"
target = "app"
organization = "vulppi"
entry = "src/root.gfs"

[alias]
math = "src/math/root.gfs"
player = "src/player/root.gfs"
```

Usage:

```galfus
import math from "$math"
import player from "$player"
```

### Library with explicit exports

```toml
[module]
name = "math-core"
version = "0.0.1"
target = "lib"
organization = "vulppi"

[exports]
"scalar" = "src/scalar/root.gfs"
"vector" = "src/vector/root.gfs"
"matrix" = "src/matrix/root.gfs"
```

Consumer:

```galfus
import vector from "@vulppi/math-core/vector"
```

### Project using a WASM payload

```txt
image-app/
  galfus.toml
  src/
    root.gfs
  modules/
    image/
      image.gfp
      image.wasm
  build/
```

`galfus.toml`:

```toml
[module]
name = "image-app"
version = "0.0.1"
target = "app"
entry = "src/root.gfs"

[alias]
image = "modules/image/image.gfp"
```

Usage:

```galfus
import image from "$image"
```

Invalid:

```galfus
import image from "./modules/image/image.wasm"
```

### Project using a native payload

```txt
physics-app/
  galfus.toml
  src/
    root.gfs
  modules/
    physics/
      physics.gfp
      libphysics.so
  build/
```

`galfus.toml`:

```toml
[module]
name = "physics-app"
version = "0.0.1"
target = "app"
entry = "src/root.gfs"

[alias]
physics = "modules/physics/physics.gfp"
```

Usage:

```galfus
import physics from "$physics"
```

Invalid:

```galfus
import physics from "./modules/physics/libphysics.so"
```

---

## 21. Validation Rules

Recommended validation rules:

```txt
[module].name is required.
[module].version is required.
[module].target is required.
[module].target must be known.
[module].target = "app" requires [module].entry.
[module].target = "lib" requires [module].entry or at least one [exports] entry.
[module].organization, if present, must be a valid organization identifier.
[module].entry, if present, must resolve to a Galfus module descriptor.
[exports] keys must be explicit public export paths.
[exports] values must resolve to Galfus module descriptors.
[exports] must not use glob by default.
[alias] keys must not include "$".
[alias] values must resolve to Galfus module descriptors or configured module directories.
Import strings that start with "$" must resolve through [alias].
Import strings that start with "@" must follow @organization/module/path format.
External payload files must not be direct import targets.
.gfm must not be used as a module candidate.
.gfp must be declarative and deterministic.
Final release bundles should not require .gfm or .gfp unless explicitly configured.
```

Canonicalization rules should include:

```txt
normalize path separators
reject ambiguous relative paths
reject unresolved symlink behavior unless policy-defined
sort expanded sets lexicographically when expansion is allowed
hash canonical module surfaces
hash external payloads referenced by .gfp
hash minimal bridge manifests in final bundles
```

---

## 22. Non-Goals

This development project standard does not define:

```txt
package registry protocol
lockfile format
network dependency fetching
semantic version solving
credential storage
native installer layout
IDE protocol details
final .gfp grammar
final binary encoding
final signature scheme
```

It also explicitly avoids:

```txt
automatic index/mod/root file discovery
arbitrary extension loaders
importing JSON/TOML/images directly
importing WASM/native libraries directly
export globbing by default
namespace generation from export paths
```

---

## 23. Summary

```txt
Galfus project identity is module-first.

Standard project:
  galfus.toml
  src/
  modules/
  build/

Configuration:
  [module] defines identity and target.
  [paths] defines standard directories.
  [exports] defines explicit public module addresses.
  [alias] defines local import shortcuts without the "$" prefix.

Imports:
  "$name" resolves through [alias].
  "@org/module/path" resolves through organized module identity.
  "module/path" resolves through unorganized module identity, workspace, dependency, or builtin lookup.
  "./path" and "../path" resolve relatively.

Discovery:
  only .gfs, .gfb, and .gfp are module candidates.
  .gfm is an optional sidecar for .gfb.
  .wasm, .dll, .so, and .dylib are payloads only, referenced by .gfp.

Artifacts:
  .gfs = source
  .gfb = compact Galfus execution artifact
  .gfm = debug/tooling/source reconstruction map
  .gfp = human-readable external payload proxy

Release:
  final bundles are consolidated, tree-shaken, slice-minimized,
  bridge-manifest-minimized, validated, and signed.
  .gfm and .gfp are not required in the final bundle by default.
```
