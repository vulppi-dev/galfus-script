# Galfus Script — Semantic Reference

This document defines the current semantic model of Galfus Script. It explains how parsed syntax is resolved, typed, validated, and lowered into execution-oriented artifacts. It does not define workspace discovery, packaging, VM internals, or runtime platform adapters except where those concepts directly affect semantic validity.

## Table of Contents

1. [Semantic scope](#1-semantic-scope)
2. [Modules and symbols](#2-modules-and-symbols)
3. [Bindings and initialization](#3-bindings-and-initialization)
4. [Primitive types](#4-primitive-types)
5. [Literals and default types](#5-literals-and-default-types)
6. [`null`](#6-null)
7. [Union types and narrowing](#7-union-types-and-narrowing)
8. [Arrays and byte strings](#8-arrays-and-byte-strings)
9. [Tuples](#9-tuples)
10. [Structs](#10-structs)
11. [Enums](#11-enums)
12. [Choices](#12-choices)
13. [Type aliases](#13-type-aliases)
14. [Casts and conversions](#14-casts-and-conversions)
15. [Operators](#15-operators)
16. [Functions](#16-functions)
17. [Stamped functions](#17-stamped-functions)
18. [Default, rest, and spread parameters](#18-default-rest-and-spread-parameters)
19. [Arrow functions and closures](#19-arrow-functions-and-closures)
20. [Anchor functions](#20-anchor-functions)
21. [Generics and constraints](#21-generics-and-constraints)
22. [Decorators](#22-decorators)
23. [Destructuring](#23-destructuring)
24. [Ranges and iteration](#24-ranges-and-iteration)
25. [Control flow](#25-control-flow)
26. [`match`](#26-match)
27. [`instanceof`](#27-instanceof)
28. [`typeof`](#28-typeof)
29. [Ownership model](#29-ownership-model)
30. [Weak fields](#30-weak-fields)
31. [Module initialization and cycles](#31-module-initialization-and-cycles)
32. [Runtime panic semantics](#32-runtime-panic-semantics)
33. [Data forms and behavior](#33-data-forms-and-behavior)
34. [Lowering and artifact metadata](#34-lowering-and-artifact-metadata)
35. [Semantic exclusions](#35-semantic-exclusions)

---

## 1. Semantic scope

The semantic layer assigns meaning to parsed syntax.

It is responsible for:

- resolving names and paths;
- building module-local semantic graphs;
- validating imports and exports after workspace resolution has identified module candidates;
- checking types;
- checking constraints;
- validating decorators;
- validating ownership rules;
- deciding type narrowing;
- deciding implicit default literal types;
- preparing compact lowering decisions for `.gfb` generation.

The semantic layer does not define the physical workspace layout, registry discovery, final bundle shape, VM instruction encoding, or platform adapter behavior.

---

## 2. Modules and symbols

### Source module as semantic unit

Each resolved `.gfs` source module is a semantic unit.

A module owns its own semantic state:

- local symbols;
- imported bindings;
- exported symbols;
- type declarations;
- function declarations;
- struct declarations;
- enum declarations;
- choice declarations;
- constraints;
- semantic diagnostics;
- ownership metadata prepared for lowering.

### Module-local semantic graph

Each module has a module-local semantic graph. Galfus does not rely on one global semantic graph for the whole workspace.

Cross-module relationships are represented through import and export surfaces.

The frontend owns local parsing, resolution, type checking, semantic validation,
ownership metadata, export surface generation, and imported surface consumption.
The runner owns workspace graph construction and decides which local or future
artifact surfaces are connected to each frontend module.

### Private symbols

Top-level symbols that are not exported are private to the module.

Private symbols may be used by other declarations inside the same module but are not visible through the module export surface.

### Exported symbols

Exported symbols define the public surface of a module.

Exporting a symbol makes it available to importing modules, subject to workspace and bundle resolution.

### Import binding semantics

A default import creates a local binding for the imported module surface:

```galfus
import user from "./user"

var created = user::create("Ana")
```

The imported binding is local to the current module.

### Named import semantics

Named imports bring specific exported symbols into the local module scope:

```galfus
import { Vec2, Vec3 } from "./vectors"
```

Only exported symbols may be named-imported.

### Path resolution

A path such as:

```galfus
a::b::c
```

is resolved by the semantic resolver according to context. It may represent an imported module path, exported symbol path, callable path, or valid type path.

The parser only recognizes the form. The semantic resolver decides the meaning.

---

## 3. Bindings and initialization

### Top-level initialization

Top-level `var` and `const` declarations are initialized as part of module initialization.

```galfus
var counter = 0
const version = 1
```

### Local bindings

Bindings declared inside blocks are local to that block.

```galfus
fn main(): null {
  var count = 0
  const limit = 10
}
```

### Mutable binding semantics

`var` creates a mutable binding.

The binding may be reassigned if the new value is compatible with the binding type and ownership rules.

### Immutable binding semantics

`const` creates an immutable binding.

The binding cannot be reassigned after initialization.

### Type annotation semantics

A type annotation defines the expected type of the binding:

```galfus
var count: int32 = 10
```

The initializer must be assignable to the annotated type.

### Type inference

A binding without an annotation infers its type from the initializer:

```galfus
var count = 10
```

Default literal typing rules apply when the initializer is an untyped literal.

---

## 4. Primitive types

Galfus core primitive scalar types are:

```galfus
bool
null

int8
int16
int32
int64

uint8
uint16
uint32
uint64

float16
float32
float64
```

The names `int`, `uint`, and `float` are primitive type families:

- `int` means `int8 | int16 | int32 | int64`;
- `uint` means `uint8 | uint16 | uint32 | uint64`;
- `float` means `float16 | float32 | float64`.

They are compact type-union names for type positions such as generic bounds.
They are not behavioral constraints and primitives do not satisfy user-defined
constraints.

There is no core dynamic top type such as `any` or `unknown`.

There is no core `String` type.

---

## 5. Literals and default types

### Boolean literals

```galfus
true
false
```

Boolean literals have type `bool`.

### Integer literals

Untyped integer literals default to `int32` when no expected type is available.

```galfus
var value = 10 // int32
```

This also applies to integer array literals:

```galfus
var values = [1, 2] // [int32]
```

If an expected type is available, the literal is checked against that expected type:

```galfus
var value: int64 = 10
var bytes: [uint8] = [65, 66]
```

### Float literals

Untyped float literals default to `float32` when no expected type is available.

```galfus
var value = 10.5 // float32
```

This also applies to float array literals:

```galfus
var values = [1.0, 2.0] // [float32]
```

If an expected type is available, the literal is checked against that expected type:

```galfus
var value: float64 = 10.5
```

### String literals

String literals produce UTF-8 byte arrays:

```galfus
var name: [uint8] = "Renato"
```

A string literal is not a `String` object. It is a `[uint8]` value whose bytes come from valid UTF-8 source text.

---

## 6. `null`

`null` represents absence.

A value may be `null` only when its type accepts `null`:

```galfus
var name: [uint8] | null = null
```

A non-nullable type cannot receive `null`:

```galfus
var name: [uint8] = null // semantic error
```

---

## 7. Union types and narrowing

### Union types

A union type represents a value that may be one of several listed types:

```galfus
[uint8] | null
int32 | int64
User | LoadError | null
```

### Union normalization

Union types are normalized by removing duplicates and preserving a canonical representation.

### Assignment compatibility

A value of type `T` may be assigned to a union that contains `T`:

```galfus
var value: int32 | null = 10
```

### Narrowing

Union values may be narrowed through:

- null checks;
- `match`;
- `instanceof`;
- control-flow analysis.

Example:

```galfus
var value: [uint8] | null = "Ana"

if value != null {
  log(value)
}
```

Inside the non-null branch, `value` is treated as `[uint8]`.

---

## 8. Arrays and byte strings

### Array semantics

Arrays are data forms. They do not provide built-in methods.

```galfus
var values = [1, 2, 3]
```

### Fixed-size arrays

A fixed-size array type includes its length:

```galfus
[int32; 3]
```

### Runtime-sized arrays

A runtime-sized array type has its length known at runtime:

```galfus
[int32]
```

### Element compatibility

All array literal elements must be compatible with the array element type.

If no expected type is available:

- integer arrays default to `[int32]`;
- float arrays default to `[float32]`;
- string literals default to `[uint8]`.

### Indexing

Indexing accesses an array position:

```galfus
values[0]
```

### Negative indexing

Negative indices count from the end:

```galfus
values[-1] // last element
values[-2] // second-to-last element
```

### Out-of-bounds indexing

If an index is outside the array bounds, the result is `null`.

Therefore, indexing may produce a nullable type:

```galfus
var value = values[10] // T | null
```

### Array spread

Array spread inserts array elements into another array literal:

```galfus
var a = [1, 2]
var b = [0, ...a, 3]
```

### Raw byte arrays and text

`[uint8]` may contain arbitrary bytes.

A string literal is simply a convenient way to create a `[uint8]` value from valid UTF-8 source text.

Rich text behavior belongs to ordinary modules or user-defined structs, not to the core semantic model.

---

## 9. Tuples

Tuples are positional data forms.

A normal tuple has at least two elements:

```galfus
var point = (10.0, 20.0)
```

The type is:

```galfus
(float32, float32)
```

A grouped expression or grouped type with one item is not a tuple:

```galfus
(value)
(int32)
```

---

## 10. Structs

### Struct semantics

A struct is a nominal aggregate with named fields:

```galfus
struct User {
  id: int64,
  name: [uint8],
}
```

### Required fields

Fields without defaults must be provided during construction.

### Field defaults

Fields with defaults may be omitted:

```galfus
struct User {
  name: [uint8],
  age: int32 = 0,
}

var user = new(User) {
  name: "Ana",
}
```

### Const fields

A `const` field cannot be reassigned after initialization:

```galfus
struct User {
  const id: int64,
  name: [uint8],
}
```

### Struct literal compatibility

A struct literal must be compatible with the expected struct type.

```galfus
var user = new(User) {
  id: 1,
  name: "Ana",
}
```

### Inferred struct literals

An inferred struct literal depends on an expected type:

```galfus
new {
  id: 1,
  name: "Ana",
}
```

Without a sufficient expected type, this is a semantic error.

### Struct expansion

Struct expansion in a declaration copies fields structurally:

```galfus
struct Person {
  ...User,
  age: int32,
}
```

It does not create class inheritance.

### Struct literal spread

Struct literal spread copies the visible field surface from an existing value into a new literal:

```galfus
var user2 = new(User) {
  ...user,
  name: "Bia",
}
```

### Shallow copy

A shallow copy copies the surface of the value and shares its contents with the original value.

### Deep copy

Deep copy is explicit. The language does not implicitly deep-copy aggregate content.

---

## 11. Enums

### Enum semantics

An enum is a nominal type with internal discriminants:

```galfus
enum Direction {
  Up,
  Down,
}
```

### Symbol preservation

An enum value preserves the enum symbol through the enum type:

```galfus
var v: Direction = Direction::Up
```

The value is semantically a `Direction`, not a raw `int32`.

### Enum casts

An explicit cast converts an enum value to its base discriminant type:

```galfus
var raw = <int32> Direction::Up
```

### Base type

The default enum base type is `int32`.

An explicit base type may be provided:

```galfus
enum<uint8> Small {
  A,
  B,
}
```

---

## 12. Choices

### Choice semantics

A choice is a nominal tagged union:

```galfus
choice Result<V, E> {
  Ok(V),
  Err(E),
}
```

### Choice constructors

A choice variant is constructed with its variant path:

```galfus
var result = Result::Ok(10)
```

### Payload tuple semantics

A choice payload is conceptually a tuple.

It may contain:

- no value;
- one value;
- multiple values.

```galfus
choice Asset {
  None,
  Texture([uint8]),
  Error([uint8], int32),
}
```

A single payload item is allowed because the payload is a choice variant payload, not a normal tuple expression.

### Decorators in payloads

Decorators inside a choice payload decorate the conceptual payload tuple field or position.

```galfus
choice Token {
  Text(@utf8 [uint8]),
}
```

### Choice matching

`match` may deconstruct choice payloads:

```galfus
match result {
  Result::Ok(value) => value,
  Result::Err(error) => 0,
}
```

### Exhaustiveness

Choice exhaustiveness is validated by the semantic checker.

A `match` over a choice should cover all variants or provide a wildcard/default branch.

---

## 13. Type aliases

### Alias semantics

A type alias creates a named symbol for another type:

```galfus
type UserId = int64
```

The alias symbol is preserved semantically.

### Alias assignability

Type alias assignability is transparent.

A `UserId` may be assigned where its underlying type is accepted, and the underlying type may be assigned where `UserId` is expected when no stronger nominal wrapper is used.

The alias name is still preserved for diagnostics, `.gfm`, IDE information, and semantic display.

---

## 14. Casts and conversions

### Explicit casts

Explicit casts use angle syntax:

```galfus
var value = <int8> other
```

### Numeric casts

Numeric casts are total at runtime.

They do not represent checked conversion helpers.

### Enum casts

Enum casts convert enum values to the enum base discriminant type.

### Checked conversions

Checked conversion helpers are not core semantics.

They may exist in modules.

---

## 15. Operators

### Boolean operators

`&&` and `||` use short-circuit evaluation.

```galfus
if ready && enabled {
  run()
}
```

### Null fallback

`a ?? b` returns `a` when `a` is not `null`; otherwise it returns `b`.

```galfus
var value = maybeValue ?? fallback
```

### Fallback assignment

`a ??= b` assigns `b` to `a` only when `a` is `null`.

```galfus
cache ??= createCache()
```

### Assignment operators

Compound assignment operators lower to an operation plus assignment:

```galfus
x += 1
x *= 2
```

### No operator overloading

Operator overloading does not exist.

### No string concatenation operator

The `+` operator does not concatenate string literals or byte arrays.

---

## 16. Functions

### Normal functions

A normal function creates a stack frame when called.

```galfus
fn sum(a: int32, b: int32): int32 {
  return a + b
}
```

### Function return type

A function with return type `null` may use `return` without a value:

```galfus
fn log(message: [uint8]): null {
  return
}
```

A function without an explicit return type also has return type `null`.

### Return validation

A `return value` must be compatible with the function return type.

A bare `return` is valid only when the function return type is `null`.

---

## 17. Stamped functions

A stamped function is declared with `fn(stamp)`:

```galfus
fn(stamp) max(a: int32, b: int32): int32 {
  if a > b {
    return a
  }

  return b
}
```

A stamped function is expanded or lowered at the callsite and does not create an additional stack frame.

Stamped functions are intended for small compile-time/lowering-time call expansion.

Restrictions:

- no direct or indirect recursion through other stamped functions;
- the body must be lowerable inline;
- the body must not require a dynamic call frame;
- any additional restriction may be enforced by the semantic checker to preserve predictable lowering.

---

## 18. Default and rest parameters

### Default parameters

Default parameters may appear in any position.

```galfus
fn call(a: int32 = 1, b: int32 = 2, c: int32 = 3, d: int32 = 4): null {
  return
}
```

A call may omit arguments in the middle by using empty argument slots:

```galfus
call(1,,,2)
```

This means:

- first argument provided as `1`;
- second argument uses its default;
- third argument uses its default;
- fourth argument provided as `2`.

### Rest parameters

A rest parameter receives a runtime-sized sequence:

```galfus
fn summarize(...values: [int32]): int32 {
  return 0
}
```

Call arguments do not support spread syntax. A rest parameter receives each
provided positional argument as one element.

---

## 19. Arrow functions and closures

Arrow functions create functions or closures depending on capture behavior.

```galfus
var double = (value: int32): int32 => value * 2
```

A block arrow may contain statements:

```galfus
var double = (value: int32): int32 => {
  return value * 2
}
```

Captured values participate in the ownership model. Captures may create lifetime anchors.

---

## 20. Anchor functions

Anchor functions exist only for structs.

```galfus
struct User {
  name: [uint8],
}

fn User::rename(self: User, name: [uint8]): User {
  self.name = name
  return self
}
```

An anchor call is a convenient function call form:

```galfus
var renamed = user::rename("Ana")
```

It behaves like a normal function call where the target is passed as the first logical argument.

There is no implicit write-back:

```galfus
user = user::rename("Ana")
```

The assignment is explicit.

Stamped anchor functions are allowed for structs when their bodies satisfy stamped function restrictions:

```galfus
fn(stamp) Vec2::lengthSq(self: Vec2): float32 {
  return self.x * self.x + self.y * self.y
}
```

---

## 21. Generics and constraints

### Generic semantics

Generics are resolved and instantiated by the checker/lowering pipeline.

```galfus
fn identity<T: int>(value: T): T {
  return value
}
```

Function generic parameters require explicit bounds. A bound may be a primitive
type family, a concrete primitive, an array type, a union, or a named
constraint:

```galfus
constraint Stringable {
  fn toString(): [uint8]
}

fn stringify(value: int | uint | float | bool | null | [uint8] | Stringable): [uint8] {
  return instanceof value {
    [uint8] text => text,
    Stringable item => item::stringify(),
    _ => "<value>",
  }
}

fn parse<T: int | uint | float | bool | null | [uint8]>(text: [uint8]): T {
  return typeof T {
    [uint8] => text,
    null => null,
    _ => <T>0,
  }
}
```

There is no implicit `Any` universe. `struct`, `enum`, `tuple`, and `array` are
not builtin generic constraints. Struct behavior is represented by named
constraints, while arrays are represented by direct array types.

### Constraint semantics

Constraints define requirements that a type must satisfy.

```galfus
constraint Comparable<T> {
  fn compare(self: T, other: T): int32
}
```

Constraint functions are anchored behavior requirements. A struct satisfies them
with anchored functions such as `fn User::toString()`, and a constrained value
invokes them through anchor access, for example `value::toString()`.

### `satisfies`

`satisfies` declares that a struct conforms to a constraint:

```galfus
struct User satisfies Identifiable {
  id: int64,
}
```

The semantic checker validates the conformance.

---

## 22. Decorators

Decorators attach metadata or trigger semantic transformation according to their target.

```galfus
@log
fn save(): null {
  return
}
```

Decorators are valid only on syntax-supported targets.

Decorator order is from the closest decorator to the farthest decorator from the associated value.

Example:

```galfus
@outer
@inner
fn save(): null {
  return
}
```

Semantic order:

```text
inner -> outer
```

---

## 23. Destructuring

Destructuring creates bindings from aggregate values.

### Struct destructuring

```galfus
var { id, name } = user
```

### Struct alias destructuring

```galfus
var { name: userName } = user
```

### Nested destructuring

```galfus
var { address: { city } } = user
```

### Tuple destructuring

```galfus
var (x, y) = point
```

### Array destructuring

```galfus
var [a, b, c] = values
```

### Array rest destructuring

```galfus
var [first, ...rest] = values
```

The rest binding receives the remaining values as an array.

---

## 24. Ranges and iteration

Ranges are sequence forms. They do not materialize as arrays automatically.

```galfus
1..9
1::4
1::4%3
```

A range can be consumed by `for in` through iterator/iterable constraints.

Array materialization, if desired, must be explicit through module behavior.

---

## 25. Control flow

### `if` / `else`

An `if` condition must be `bool`.

```galfus
if ready {
  run()
} else {
  stop()
}
```

### Loops

Galfus supports:

```galfus
for value in values {
  log(value)
}

while ready {
  run()
}

loop {
  break
}
```

### `break` and `continue`

`break` exits the nearest loop.

`continue` advances the nearest loop.

### `for in`

`for in` accepts values that satisfy iterator/iterable constraints.

Arrays and ranges participate through the same iterator/iterable model.

---

## 26. `match`

`match` is an expression and always has a result.

```galfus
var text = match value {
  0 => "zero",
  1 => "one",
  _ => "many",
}
```

`match` may also be used in statement position. In statement position, its result is discarded.

```galfus
match event {
  Event::Start => start(),
  Event::Stop => stop(),
  _ => null,
}
```

If no branch matches and a default branch exists, the default branch is used.

If no branch matches and no default branch exists, the result is `null`.

### Choice matching

`match` may deconstruct choice payloads:

```galfus
match result {
  Result::Ok(value) => value,
  Result::Err(error) => 0,
}
```

### Comparable matching

`match` may use a `Comparable<T>` constraint to compare patterns against a value.

This allows module-defined comparable values to participate in matching.

For example, a regexp module can define regexp values that compare against UTF-8 byte arrays:

```galfus
match text {
  regexp::pattern("^[a-z]+$") => "word",
  _ => "other",
}
```

The regexp syntax itself is not a core literal; it is provided by a module.

---

## 27. `instanceof`

`instanceof` is an expression and returns a value.

```galfus
var result = instanceof value {
  [uint8] name => name,
  int32 count => "number",
  null => "missing",
}
```

`instanceof` may also be used in statement position. In statement position, its result is discarded.

```galfus
instanceof value {
  [uint8] name => log(name),
  null => log("missing"),
}
```

`instanceof` narrows union values inside each branch.

If no branch matches and a default branch exists, the default branch is used.

If no branch matches and no default branch exists, the result is `null`.

---

## 28. `typeof`

`typeof` is an expression that dispatches over a specified type, not over a
value. Its subject is a type expression:

```galfus
var result = typeof T {
  int => parseInt(text),
  uint => parseUint(text),
  [uint8] => text,
  User => userText,
}
```

When the subject is a generic parameter, each arm pattern must be compatible
with that parameter's bound. Inside a matching arm, the generic parameter is
narrowed to the arm type, so values typed as `T` are checked as that concrete
branch type.

`match` dispatches on value patterns, `instanceof` dispatches on the runtime
type of a value, and `typeof` dispatches on a type known to the checker/lowering
pipeline.

---

## 29. Ownership model

Galfus uses an ownership model based on anchors, edges, and weak observers.

### Anchors

An anchor preserves the lifetime of a reachable graph.

Anchors may originate from:

- module state;
- block-local bindings;
- closures;
- temporaries;
- host/runtime roots.

### Edges

Edges are normal tracked data references inside composed values.

Edges connect values into an ownership graph. They may form cycles.

### Weak observers

Weak observers do not preserve lifetime.

A weak observer may produce `null` if the observed value is no longer alive.

### Lifetime rule

A value lives while it is reachable from at least one anchor through edges.

Weak references do not participate in preservation.

### Ownership validation

The semantic checker validates ownership rules before `.gfb` generation.

It prepares compact ownership metadata for lowering.

---

## 30. Weak fields

A weak field does not preserve the lifetime of the referenced value:

```galfus
struct Node {
  value: int32,
  weak parent: Node | null,
}
```

A weak field may observe `null` when the target is no longer alive.

Because weak observers can decay to `null`, a weak field type must be nullable:

```galfus
struct Node {
  weak parent: Node | null,
}
```

The frontend records weak field metadata for later ownership validation and lowering.

---

## 31. Module initialization and cycles

Top-level module initialization occurs according to the resolved module graph.

Import cycles are handled through graph analysis, such as strongly connected components.

Cycles are valid only when exported surfaces and initialization order can be resolved safely.

Invalid initialization cycles are semantic errors.

---

## 32. Runtime panic semantics

Runtime failures produce panic.

Examples include:

- sandbox memory limit exceeded;
- stack limit exceeded;
- invalid external adapter response;
- corrupted `.gfb`;
- integrity failure;
- unreachable bytecode state.

Out-of-bounds indexing is not a panic because indexing outside bounds returns `null`.

Numeric casts are not panic-producing checked conversions; they are total runtime casts.

---

## 33. Data forms and behavior

The core data forms do not provide built-in behavior.

This applies to:

- tuples;
- arrays;
- choices;
- enums;
- string literals as `[uint8]`.

There are no implicit methods on these forms.

Behavior such as collection helpers, rich text operations, regexp matching, formatting, sorting, filtering, parsing, and reflection belongs to modules.

---

## 34. Lowering and artifact metadata

The semantic layer produces compact lowering decisions for `.gfb` generation.

The `.gfb` contains only the minimum required for execution and integrity.

Module export/import surfaces are frontend validation artifacts. They may inform
lowering, but workspace graph state and source-level frontend data are not part
of the release `.gfb`.

The `.gfm` contains data for:

- debugging;
- source reconstruction;
- IDE path support;
- autocomplete;
- symbol display;
- source-level names and mapping.

Alias symbols, enum symbols, source paths, debug names, and rich source reconstruction data belong in `.gfm`, not in the release execution surface of `.gfb` unless required for execution or integrity.

---

## 35. Semantic exclusions

The semantic model does not include:

- core `String` object semantics;
- regex literals;
- operator overloading;
- implicit methods on arrays, tuples, choices, enums, or string literals;
- mandatory rich text behavior;
- mandatory collection helpers;
- mandatory reflection;
- unchecked arbitrary runtime module loading;
- source reconstruction from `.gfb` alone.
