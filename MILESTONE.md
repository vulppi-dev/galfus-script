# Galfus Script Milestones

This document tracks the development milestones for Galfus Script.

Galfus Script is currently focused on the MVP: validating the language frontend, lowering valid `.gfs` programs into an executable representation, serializing it as `.gfb`, loading it into the VM, and executing it with the required ownership model.

The MVP is not a product-distribution milestone. It does not include package publishing, registry support, adapters, JIT, debugger integration, IDE tooling, multi-target packaging, or Galfus Engine integration.

## Table of Contents

- [Current Status](#current-status)
- [MVP Pipeline](#mvp-pipeline)
- [Frontend Closure Plan](#frontend-closure-plan)
- [Frontend Reaudit Closure Plan](#frontend-reaudit-closure-plan)
- [Milestone 0 — Project Foundation](#milestone-0--project-foundation)
- [Milestone 1 — Workspace Graph](#milestone-1--workspace-graph)
- [Milestone 2 — Lexer and Parser](#milestone-2--lexer-and-parser)
- [Milestone 3 — Local Resolver](#milestone-3--local-resolver)
- [Milestone 4 — Full Resolver](#milestone-4--full-resolver)
- [Milestone 5 — Type Checker](#milestone-5--type-checker)
- [Milestone 6 — Semantic Checker](#milestone-6--semantic-checker)
- [Milestone 7 — Ownership Checker](#milestone-7--ownership-checker)
- [Milestone 8 — MIR](#milestone-8--mir)
- [Milestone 9 — Bytecode](#milestone-9--bytecode)
- [Milestone 10 — Galfus Module Image](#milestone-10--galfus-module-image)
- [Milestone 11 — `.gfb`](#milestone-11--gfb)
- [Milestone 12 — VM Core](#milestone-12--vm-core)
- [Milestone 13 — Owner Graph Core Runtime](#milestone-13--owner-graph-core-runtime)
- [Milestone 14 — Local MVP Runner](#milestone-14--local-mvp-runner)
- [Milestone 15 — MVP Validation Suite](#milestone-15--mvp-validation-suite)
- [MVP Success Criteria](#mvp-success-criteria)
- [Out of MVP](#out-of-mvp)
- [After MVP](#after-mvp)

## Current Status

Current phase:

```txt
MIR / lowering
```

Completed:

```txt
lexer
parser
local resolver
type checker
semantic checker
ownership checker
```

Active:

```txt
MIR
bytecode
Module Image
```

Frontend MVP status:

```txt
closed for local .gfs validation
```

Not usable yet:

```txt
MIR
bytecode
.gfb
VM execution
```

## MVP Pipeline

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

## Milestone 0 — Project Foundation

Goal: keep the repository organized enough for MVP development and public inspection.

- [x] Rust workspace exists
- [x] Core crate structure exists
- [x] `docs/` folder exists
- [x] Syntax reference exists
- [x] Semantic reference exists
- [x] Architecture reference exists
- [x] Workspace reference exists
- [x] MVP reference exists
- [x] Adapters surface reference exists
- [x] `examples/` folder exists
- [x] Syntax demonstration project exists
- [ ] Public README finalized
- [x] `MILESTONE.md` added
- [ ] Repository description added
- [ ] Repository topics added
- [x] License file confirmed
- [x] Contribution stance documented

## Milestone 1 — Workspace Graph

Goal: build the local project/module model used by the frontend.

- [ ] Find project root through `galfus.toml`
- [x] Parse minimal `galfus.toml`
- [x] Validate `[module]`
- [x] Validate module `name`
- [x] Validate module `target`
- [x] Validate app `entry`
- [x] Validate local library exports
- [x] Resolve local `src/` files
- [x] Resolve local relative imports from project graph
- [x] Resolve named imports from project graph
- [ ] Resolve local aliases, if included in MVP
- [x] Build local module records
- [x] Build local module graph
- [x] Detect missing source files
- [x] Detect invalid import targets
- [ ] Detect ambiguous local imports
- [ ] Preserve case-sensitive paths
- [x] Produce workspace diagnostics

## Milestone 2 — Lexer and Parser

Goal: parse the complete accepted Galfus syntax surface.

Status: completed as current frontend foundation.

- [x] Source files
- [x] Comments
- [x] Identifiers
- [x] Paths
- [x] Imports
- [x] Named imports
- [x] Exports
- [x] `var`
- [x] `const`
- [x] Primitive types
- [x] Integer literals
- [x] Binary literals
- [x] Octal literals
- [x] Hex literals
- [x] Float literals
- [x] Boolean literals
- [x] `null`
- [x] String literals as UTF-8 byte arrays
- [x] Array literals
- [x] Array spread
- [x] Array types
- [x] Fixed-size array types
- [x] Indexing
- [x] Negative indexing syntax
- [x] Tuple types
- [x] Tuple expressions
- [x] Grouped expressions
- [x] Grouped types
- [x] Struct declarations
- [x] Struct fields
- [x] Struct field defaults
- [x] Const fields
- [x] Struct literals
- [x] `new(Type)` typed struct literals
- [x] `new` inferred struct literals
- [x] Struct literal shorthand
- [x] Inferred struct literals
- [x] Struct expansion
- [x] Struct literal spread
- [x] Enums
- [x] Enum discriminants
- [x] Enum base type
- [x] Enum value access
- [x] Choices
- [x] Choice payloads
- [x] Generic choices
- [x] Type aliases
- [x] Named types
- [x] Path types
- [x] Generic types
- [x] Union types
- [x] Function types
- [x] Casts
- [x] Arithmetic operators
- [x] Comparison operators
- [x] Boolean operators
- [x] Bitwise operators
- [x] Null fallback
- [x] Fallback assignment
- [x] Assignment operators
- [x] Member access
- [x] Null-safe member access
- [x] Functions
- [x] Stamped functions
- [x] Default parameters
- [x] Rest parameters
- [x] Trailing arguments
- [x] Arrow functions
- [x] Anchor functions
- [x] Generics
- [x] Constraints
- [x] `satisfies`
- [x] Decorators
- [x] Destructuring
- [x] Ranges
- [x] `if` / `else`
- [x] `for in`
- [x] `while`
- [x] `loop`
- [x] `break`
- [x] `continue`
- [x] `return`
- [x] `match`
- [x] `instanceof`
- [x] Weak fields
- [x] Parser diagnostics
- [x] Parser recovery
- [x] Missing comma recovery hardening

## Milestone 3 — Local Resolver

Goal: resolve local source-level symbols and paths enough to support current frontend validation.

Status: completed as local resolver foundation.

- [x] Build local symbol tables
- [x] Register private top-level symbols
- [x] Register exported top-level symbols
- [x] Resolve local paths
- [x] Resolve local type paths
- [x] Resolve local callable paths
- [x] Resolve local anchor function paths
- [x] Resolve enum variant paths
- [x] Resolve choice constructor paths
- [x] Detect duplicate local symbols
- [x] Detect missing local symbols
- [x] Produce local resolver diagnostics

## Milestone 4 — Full Resolver

Goal: expand resolution from local source validation into module-aware semantic resolution.

- [x] Resolve imported module bindings
- [x] Resolve named imports
- [x] Resolve workspace graph imports
- [x] Resolve export surfaces
- [x] Resolve type paths across modules
- [x] Resolve callable paths across modules
- [x] Resolve anchor function paths across modules
- [x] Resolve enum variant paths across modules
- [x] Resolve choice constructor paths across modules
- [x] Detect private symbol access from imports
- [x] Detect invalid export references
- [x] Handle import cycles without infinite loading
- [x] Preserve module-local semantic boundaries
- [x] Produce full resolver diagnostics

## Milestone 5 — Type Checker

Goal: validate all core type rules and materialize inferred types before lowering.

- [x] Primitive scalar typing
- [x] Default integer literal typing
- [x] Default float literal typing
- [x] Boolean literal typing
- [x] Null typing
- [x] String literal typing as `[uint8]`
- [x] Array literal typing
- [x] Fixed-size array typing
- [x] Runtime-sized array typing
- [x] Tuple typing
- [x] Struct literal typing
- [x] Struct field compatibility
- [x] Struct field defaults
- [x] Const field validation
- [x] Enum typing
- [x] Enum base type validation
- [x] Choice construction typing
- [x] Choice payload typing
- [x] Type alias preservation
- [x] Type alias assignability
- [x] Union type normalization
- [x] Union assignment compatibility
- [x] Nullability validation
- [x] Weak field nullability validation
- [x] Weak field ownership metadata prep
- [x] Function signature typing
- [x] Function return typing
- [x] Function stamp recursion validation
- [x] Call argument typing
- [x] Default parameter typing
- [x] Rest parameter typing
- [x] Arrow function typing
- [x] Anchor function typing
- [x] Generic declaration typing
- [x] Generic instantiation typing
- [x] Constraint validation
- [x] `satisfies` validation
- [x] Cast validation
- [x] Operator typing
- [x] Match expression typing
- [x] `instanceof` branch typing
- [x] Type narrowing
- [x] Destructuring typing
- [x] Range typing
- [x] Iterator / iterable constraint typing
- [x] Type checker diagnostics

## Milestone 6 — Semantic Checker

Goal: validate language behavior beyond raw type compatibility.

- [x] Top-level initialization semantics
- [x] Local binding initialization
- [x] Mutable binding reassignment
- [x] Immutable binding reassignment rejection
- [x] Export surface validation
- [x] Import binding semantics
- [x] Function return path validation
- [x] Bare return validation
- [x] Remaining stamped function lowering restrictions
- [x] Anchor call semantics
- [x] No implicit anchor write-back
- [x] Struct expansion semantics
- [x] Struct literal spread semantics
- [x] Shallow copy semantics
- [x] Explicit deep copy semantics placeholder
- [x] Enum symbol preservation
- [x] Choice exhaustiveness checks
- [x] Match fallback behavior
- [x] `instanceof` narrowing behavior
- [x] Decorator target validation
- [x] Decorator order validation
- [x] Destructuring semantics
- [x] Range semantics
- [x] Loop control validation
- [x] Break / continue target validation
- [x] Module initialization cycle validation
- [x] Runtime panic condition modeling
- [x] Semantic diagnostics

## Milestone 7 — Ownership Checker

Goal: validate ownership metadata before `.gfb` generation.

- [x] Model anchors
- [x] Model edges
- [x] Model weak observers
- [x] Validate weak fields
- [x] Validate captured values
- [x] Validate closure anchors
- [x] Validate module state anchors
- [x] Validate block-local anchors
- [x] Validate temporaries
- [x] Validate ownership cycles
- [x] Validate release eligibility
- [x] Prepare ownership metadata
- [x] Prepare anchor metadata
- [x] Prepare edge metadata
- [x] Prepare weak metadata
- [x] Ownership diagnostics

## Milestone 8 — MIR

Goal: lower validated semantic graphs into a typed mid-level representation.

- [ ] MIR module representation
- [ ] MIR function representation
- [ ] MIR block representation
- [ ] MIR local representation
- [ ] MIR temporary representation
- [ ] MIR instruction representation
- [ ] Materialize inferred types in MIR
- [ ] Lower constants
- [ ] Lower variables
- [ ] Lower assignments
- [ ] Lower arithmetic
- [ ] Lower comparisons
- [ ] Lower boolean operations
- [ ] Lower null fallback
- [ ] Lower casts
- [ ] Lower control flow
- [ ] Lower loops
- [ ] Lower `match`
- [ ] Lower `instanceof`
- [ ] Lower function calls
- [ ] Lower anchor calls
- [ ] Lower struct literals
- [ ] Lower struct field access
- [ ] Lower tuples
- [ ] Lower arrays
- [ ] Lower choices
- [ ] Lower enums
- [ ] Lower module initialization
- [ ] Lower ownership metadata references
- [ ] MIR validation
- [ ] MIR diagnostics

## Milestone 9 — Bytecode

Goal: lower MIR into bytecode executable by the MVP VM.

- [ ] Define MVP bytecode format
- [ ] Define instruction encoding
- [ ] Define constant encoding
- [ ] Define function table references
- [ ] Define type table references
- [ ] Define layout table references
- [ ] Define local slots
- [ ] Define temporary slots
- [ ] Emit constants
- [ ] Emit local load/store
- [ ] Emit module state load/store
- [ ] Emit arithmetic instructions
- [ ] Emit comparison instructions
- [ ] Emit boolean instructions
- [ ] Emit cast instructions
- [ ] Emit jumps
- [ ] Emit branches
- [ ] Emit calls
- [ ] Emit returns
- [ ] Emit struct operations
- [ ] Emit tuple operations
- [ ] Emit array operations
- [ ] Emit choice operations
- [ ] Emit enum operations
- [ ] Emit module init instructions
- [ ] Emit panic instruction or panic path
- [ ] Bytecode validation
- [ ] Bytecode diagnostics

## Milestone 10 — Galfus Module Image

Goal: build the minimal runtime-facing executable image.

- [ ] Define Module Image structure
- [ ] Build bytecode section
- [ ] Build constant pool
- [ ] Build function table
- [ ] Build type table
- [ ] Build layout table
- [ ] Build import slots
- [ ] Build export slots
- [ ] Build module init data
- [ ] Build ownership metadata
- [ ] Build anchor metadata
- [ ] Build edge metadata
- [ ] Build weak metadata
- [ ] Build minimal runtime metadata
- [ ] Build integrity metadata placeholder
- [ ] Ensure no frontend-only data is included
- [ ] Validate Module Image before serialization

## Milestone 11 — `.gfb`

Goal: serialize and load Galfus Binary artifacts.

- [ ] Define `.gfb` header
- [ ] Define format version
- [ ] Define runtime ABI version
- [ ] Define section table
- [ ] Define body size
- [ ] Define body hash or checksum strategy
- [ ] Serialize Module Image to `.gfb`
- [ ] Read `.gfb` header
- [ ] Validate `.gfb` format
- [ ] Validate `.gfb` version
- [ ] Validate `.gfb` integrity metadata
- [ ] Deserialize `.gfb` into Module Image
- [ ] Report `.gfb` loader diagnostics
- [ ] Add `.gfb` golden tests

## Milestone 12 — VM Core

Goal: execute `.gfb` through a minimal interpreted VM.

- [ ] VM runtime structure
- [ ] Module Image loading
- [ ] Module Image validation
- [ ] Entrypoint lookup
- [ ] Bytecode dispatch
- [ ] Call frames
- [ ] Locals
- [ ] Temporaries
- [ ] Function calls
- [ ] Returns
- [ ] Cast execution
- [ ] Control flow execution
- [ ] Struct execution support
- [ ] Tuple execution support
- [ ] Array execution support
- [ ] Choice execution support
- [ ] Enum execution support
- [ ] Module initialization
- [ ] Minimal panic handling
- [ ] VM diagnostics
- [ ] VM execution tests

## Milestone 13 — Owner Graph Core Runtime

Goal: execute the Galfus ownership model at runtime.

- [ ] Runtime anchor representation
- [ ] Runtime edge representation
- [ ] Runtime weak observer representation
- [ ] Value lifetime tracking
- [ ] Deterministic release
- [ ] Cycle-safe release
- [ ] Weak invalidation
- [ ] Module state roots
- [ ] Call frame roots
- [ ] Local roots
- [ ] Temporary roots
- [ ] Closure roots
- [ ] Owner graph tests
- [ ] Runtime ownership panic paths

## Milestone 14 — Local MVP Runner

Goal: provide a local developer command that proves the MVP pipeline.

- [ ] Load local `galfus.toml`
- [ ] Load app entrypoint
- [ ] Build workspace graph
- [ ] Run frontend
- [ ] Run semantic validation
- [ ] Run ownership validation
- [ ] Build MIR
- [ ] Build bytecode
- [ ] Build Module Image
- [ ] Write `.gfb`
- [ ] Load `.gfb`
- [ ] Execute in VM
- [ ] Report diagnostics
- [ ] Exit with correct status code

## Milestone 15 — MVP Validation Suite

Goal: prove the complete language surface through local `.gfs` programs.

- [x] Primitive values and casts
- [x] Arrays and negative indexing
- [x] String literals as `[uint8]`
- [x] Tuples
- [x] Structs
- [x] Struct defaults
- [x] Const fields
- [x] Enums and enum casts
- [x] Choices and match
- [x] Unions and null narrowing
- [x] `instanceof` expressions
- [x] Functions
- [x] Stamped functions
- [x] Anchor functions on structs
- [x] Generics
- [x] Constraints
- [x] `satisfies`
- [x] Decorators
- [x] Destructuring
- [x] Ranges
- [x] `for in` with iterator / iterable constraints
- [x] Weak fields
- [x] Ownership validation
- [x] Module imports
- [x] Module exports
- [ ] `.gfb` serialization
- [ ] VM execution
- [ ] Panic behavior

## MVP Success Criteria

The MVP is complete when:

- [x] The compiler parses the full accepted syntax
- [x] The compiler rejects invalid syntax with useful diagnostics
- [x] The resolver builds correct module-local semantic graphs
- [x] The type checker validates all core type rules
- [x] The semantic checker validates current language semantics
- [x] The ownership checker validates anchors, edges, and weak fields
- [ ] The compiler lowers valid programs into MIR
- [ ] The compiler lowers MIR into a Galfus Module Image
- [ ] The compiler serializes the Module Image into `.gfb`
- [ ] The VM loads and validates `.gfb`
- [ ] The VM executes bytecode correctly
- [ ] The Owner Graph Core releases values deterministically
- [ ] Runtime failures produce panic
- [x] Local imports and exports work
- [ ] No excluded ecosystem feature is required to run MVP programs

## Out of MVP

The following are intentionally excluded from the MVP:

- [ ] Package registry
- [ ] Published dependencies
- [ ] Dependency cache model
- [ ] `galfus.lock`
- [ ] Publishing system
- [ ] Version resolution for published modules
- [ ] `.gfp`
- [ ] `.gfm`
- [ ] Native adapters
- [ ] WASM adapters
- [ ] Mobile adapters
- [ ] Embedded adapters
- [ ] External payload bridges
- [ ] C ABI bridge
- [ ] Runtime compiler module
- [ ] Runtime compilation
- [ ] Hot reload
- [ ] JIT
- [ ] Quickening
- [ ] Runtime profiles beyond minimal execution
- [ ] Debug hooks
- [ ] Breakpoints
- [ ] Debug trace
- [ ] Owner Graph Extra
- [ ] Source reconstruction
- [ ] IDE autocomplete metadata
- [ ] Rich `.gfm` diagnostics
- [ ] Server sandbox policy
- [ ] Multi-tenant execution controls
- [ ] Desktop executable packaging
- [ ] Web/WASM package generation
- [ ] Android target
- [ ] iOS target
- [ ] Embedded target
- [ ] Galfus Engine integration

## After MVP

Possible post-MVP directions:

- [ ] Standard modules
- [ ] Runtime profiles
- [ ] Debug maps
- [ ] `.gfp` adapter model
- [ ] WASM runtime
- [ ] C ABI integration
- [ ] Native adapter integration
- [ ] Embedded runtime experiments
- [ ] OTA-oriented `.gfb` workflows
- [ ] Sandbox configuration
- [ ] REPL
- [ ] Playground
- [ ] Documentation site
- [ ] GitHub Discussions launch
- [ ] Galfus Engine modularization
