<div align="center">
  <img src="/assets/brand-effect.png" alt="Galfus" width="400" />
  
  # Galfus Script
  
  [![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE.md)
  [![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
</div>

> A small, efficient, highly modular interpreted scripting language built around typed source code, compact `.gfb` artifacts, and a deterministic VM runtime.

Galfus Script is an experimental programming language focused on validating a compact, modular, VM-first scripting model.

The current goal is not to build a full ecosystem yet. The current goal is to prove the language idea: parse and validate `.gfs` source code, build a semantic representation, lower it into an executable module image, serialize it as `.gfb`, and execute it through a minimal VM.

Galfus Script is currently in early development.

## Table of Contents

- [Status](#status)
- [Name Inspiration](#name-inspiration)
- [Motivation](#motivation)
- [Memory Philosophy](#memory-philosophy)
- [Design Goals](#design-goals)
- [Core Pipeline](#core-pipeline)
- [Artifacts](#artifacts)
- [Modularity](#modularity)
- [Example Syntax](#example-syntax)
- [Project Model](#project-model)
- [Repository Layout](#repository-layout)
- [Documentation](#documentation)
- [MVP Milestones](#mvp-milestones)
- [Relationship to Galfus Engine](#relationship-to-galfus-engine)
- [Contributing](#contributing)
- [License](#license)

## Status

**Current phase:** frontend workspace graph.

Completed frontend pieces:

```txt
lexer
parser
local resolver
```

Active focus:

```txt
workspace graph
module/project resolution
frontend validation
```

Galfus Script is not usable as an application language yet.

At this stage, examples in this repository are syntax demonstrations only. They are intended to communicate the language direction and validate parser/frontend behavior. They are not guaranteed to execute until the MVP runtime pipeline is complete.

There is no stable CLI contract, package format, runtime release, or user-facing installation flow yet.

## Name Inspiration

The name **Galfus** comes from **Galafus**, a figure from Pernambuco folklore associated with will-o'-the-wisp phenomena.

The name reflects the author's connection to Paulista, Pernambuco, and the desire to build a technical project with a local cultural identity rather than a generic technology name.

The image of a small wandering flame also fits the spirit of the language:

```txt
small
portable
modular
present where needed
lightweight by default
able to move across hosts and environments
```

Galfus Script is a technical project, but its name intentionally carries a piece of Pernambuco.

## Motivation

Galfus Script was created from a practical tension between languages and systems the author likes, and the tradeoffs that make them harder to scale, embed, or maintain.

Rust provides control, safety, and performance, but it also comes with many idioms, hidden machinery, traits, lifetimes, helper APIs, and ecosystem patterns that demand a lot of memorization.

Lua is small, simple, and excellent for embedding, but larger Lua codebases can become difficult to maintain without stronger typing, explicit contracts, and clearer module structure.

TypeScript has great ergonomics, but it inherits a lot of complexity from JavaScript: compatibility layers, runtime mismatch, large AST/tooling requirements, memory-heavy development workflows, and frequent adaptation work.

Python is versatile, but Galfus intentionally avoids indentation-defined blocks. Galfus favors explicit block structure.

Galfus Script aims for a different balance:

```txt
more structured than Lua
smaller and simpler than TypeScript tooling
less mentally heavy than Rust for scripting use cases
more explicit than Python-style indentation blocks
more controlled than traditional garbage-collected scripting runtimes
less manually burdensome than C-style memory management
```

## Memory Philosophy

Galfus Script does not aim to expose raw manual memory management to normal source code.

It also does not aim to rely on a traditional global garbage collector as the primary mental model.

Instead, Galfus explores an ownership model based on:

```txt
anchors
edges
weak observers
```

Conceptually:

```txt
anchors preserve lifetime
edges connect reachable values
weak observers do not preserve lifetime
values live while reachable from anchors through edges
```

The goal is to create a middle ground between garbage collection and direct manual memory control.

The language should give the programmer a clearer ownership model without requiring the heavy explicitness of low-level memory management in every part of the code.

## Design Goals

Galfus Script is designed to be:

```txt
small
efficient
typed
interpreted
highly modular
VM-first
explicit
deterministic
host-friendly
easy to inspect
small by default
```

The core idea is that nothing large should be mandatory.

Modules, builtins, adapters, debug tooling, runtime compiler support, and future engine integration should be included only when used or required by target policy.

## Core Pipeline

The intended MVP pipeline is:

```txt
.gfs source
  -> workspace graph
  -> source loading
  -> lexer / parser
  -> resolver
  -> type checker
  -> semantic checker
  -> ownership checker
  -> MIR
  -> bytecode
  -> Galfus Module Image
  -> .gfb
  -> VM
  -> execution
```

The `.gfb` artifact is the central executable artifact of Galfus Script.

## Artifacts

### `.gfs`

Human-authored Galfus Script source code.

```txt
src/main.gfs
```

### Galfus Module Image

The internal executable representation produced after validation and lowering.

It is not source code. It is not an AST. It is not a debug map.

It contains the minimum runtime-facing data needed by the VM.

### `.gfb`

Galfus Binary.

The serialized form of a Galfus Module Image.

The VM loads `.gfb` and executes it.

### `.gfm`

Future debug/tooling/source reconstruction map.

Not part of the MVP.

### `.gfp`

Future proxy descriptor for adapters, external payloads, native bridges, WASM bridges, and controlled host integration.

Not part of the MVP.

## Modularity

Galfus Script is designed around modules.

The long-term goal is not to ship a large mandatory runtime or standard library. Instead, Galfus should keep the runtime core small and include functionality by reachability.

This applies to:

```txt
user modules
builtin modules
adapter modules
platform modules
debug modules
future compiler module
future engine modules
```

In the future, Galfus Engine is expected to be fragmented into modules so that applications can use only the pieces they need.

## Example Syntax

The following examples are syntax demonstrations only.

They show the intended language direction, but they may not execute yet.

### Minimal entrypoint

```galfus
export fn main(): null {
  return
}
```

### Structs and anchor functions

```galfus
export struct User {
  const id: int64,
  name: [uint8],
  age: int32 = 0,
}

fn User::rename(self: User, name: [uint8]): User {
  self.name = name
  return self
}

export fn main(): null {
  var user = User {
    id: 1,
    name: "Renato",
  }

  var renamed = user::rename("Ana")

  return
}
```

### Choices and match

```galfus
choice Result<V, E> {
  Ok(V),
  Err(E),
}

fn divide(a: int32, b: int32): Result<int32, [uint8]> {
  if b == 0 {
    return Result::Err("division by zero")
  }

  return Result::Ok(a / b)
}

export fn main(): null {
  var result = divide(10, 2)

  match result {
    Result::Ok(value) => value,
    Result::Err(error) => error,
  }

  return
}
```

### Unions and `instanceof`

```galfus
fn describe(value: int32 | [uint8] | null): [uint8] {
  return instanceof value {
    int32 count => "number",
    [uint8] text => text,
    null => "missing",
  }
}
```

### Local imports

```galfus
import user from "./user"
import { Result } from "./result"

export fn main(): null {
  return
}
```

## Project Model

A minimal Galfus app is expected to look like this:

```txt
my-app/
  galfus.toml
  src/
    main.gfs
  build/
```

Example `galfus.toml`:

```toml
[module]
name = "my-app"
target = "app"
entry = "src/main.gfs"
```

A local library-style module may define exports explicitly:

```toml
[module]
name = "my-lib"
target = "lib"

[exports]
"my/user" = "src/user.gfs"
"my/result" = "src/result.gfs"
```

Galfus avoids implicit entrypoint conventions such as `main.gfs`, `index.gfs`, `mod.gfs`, or `root.gfs`.

Entrypoints, exports, aliases, and dependencies are intended to be explicit.

## Repository Layout

Current repository shape:

```txt
galfus-script/
  .cargo/
  crates/
    galfus-builtins/
    galfus-cli/
    galfus-core/
    galfus-frontend/
    galfus-host/
    galfus-ir/
    galfus-jit/
    galfus-runner/
    galfus-runtime/
    galfus-tools/
    galfus-vm/
  docs/
    Galfus_Adapters_Surface_Reference.md
    Galfus_Architecture_Reference.md
    Galfus_MVP_Reference.md
    Galfus_Semantic_Reference.md
    Galfus_Syntax_Reference.md
    Galfus_Workspace_Reference.md
  examples/
    project/
  Cargo.toml
  Cargo.lock
  deny.toml
```

Some crates are future-facing and may exist before their full MVP functionality is implemented.

## Documentation

Current reference documents:

- [`docs/Galfus_Syntax_Reference.md`](./docs/Galfus_Syntax_Reference.md)
- [`docs/Galfus_Semantic_Reference.md`](./docs/Galfus_Semantic_Reference.md)
- [`docs/Galfus_Architecture_Reference.md`](./docs/Galfus_Architecture_Reference.md)
- [`docs/Galfus_Workspace_Reference.md`](./docs/Galfus_Workspace_Reference.md)
- [`docs/Galfus_MVP_Reference.md`](./docs/Galfus_MVP_Reference.md)
- [`docs/Galfus_Adapters_Surface_Reference.md`](./docs/Galfus_Adapters_Surface_Reference.md)

Suggested reading order:

```txt
1. README.md
2. docs/Galfus_MVP_Reference.md
3. docs/Galfus_Syntax_Reference.md
4. docs/Galfus_Semantic_Reference.md
5. docs/Galfus_Workspace_Reference.md
6. docs/Galfus_Architecture_Reference.md
7. docs/Galfus_Adapters_Surface_Reference.md
```

## MVP Milestones

The MVP checklist is tracked in [`MILESTONE.md`](./MILESTONE.md).

Current completed frontend pieces:

```txt
lexer
parser
local resolver
```

Current active milestone:

```txt
workspace graph
```

The MVP is complete when Galfus Script can validate a local `.gfs` program, lower it into a Galfus Module Image, serialize it as `.gfb`, load it into the VM, and execute it with the required ownership model.

See [`MILESTONE.md`](./MILESTONE.md) for the detailed checklist.

## Relationship to Galfus Engine

Galfus Engine is part of the broader Galfus/Vulppi ecosystem, but it is currently paused while Galfus Script moves toward its MVP.

The future direction is to fragment the engine into modules so that Galfus applications can use only the engine pieces they actually need.

Galfus Script comes first because the language/runtime model should be validated before deeper engine integration.

## Contributing

Galfus Script is currently open to:

```txt
feedback
questions
design discussion
syntax discussion
architecture discussion
focused issue reports
documentation suggestions
```

Large feature contributions should wait until the MVP architecture stabilizes.

The best way to help right now is to review the language direction, discuss tradeoffs, and help identify unclear or overcomplicated parts of the design.

## License

MIT
