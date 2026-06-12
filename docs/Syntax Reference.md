# Galfus Script — Syntax Reference

Updated after the SyntaxGraph/parser MVP work.

This document describes the current MVP syntax surface for Galfus Script. It focuses on parsed syntax, not full semantic validation. Some rules are intentionally left to later compiler phases, especially resolver, type checker, constant evaluation, ownership validation, and decorator semantics.

## Table of Contents

1. [Source files](#1-source-files)
2. [Comments](#2-comments)
3. [Names and paths](#3-names-and-paths)
4. [Imports](#4-imports)
5. [Exports](#5-exports)
6. [Variables and constants](#6-variables-and-constants)
7. [Primitive types](#7-primitive-types)
8. [Literals](#8-literals)
9. [`null`](#9-null)
10. [Types](#10-types)
11. [Casts](#11-casts)
12. [Operators](#12-operators)
13. [Structs](#13-structs)
14. [Struct expansion and spread](#14-struct-expansion-and-spread)
15. [Enums](#15-enums)
16. [Choices](#16-choices)
17. [Functions](#17-functions)
18. [Parameters, defaults, rest, and argument spread](#18-parameters-defaults-rest-and-argument-spread)
19. [Arrow functions](#19-arrow-functions)
20. [Anchor functions](#20-anchor-functions)
21. [Generics](#21-generics)
22. [Constraints](#22-constraints)
23. [`satisfies`](#23-satisfies)
24. [Decorators](#24-decorators)
25. [Arrays](#25-arrays)
26. [Tuples](#26-tuples)
27. [Destructuring](#27-destructuring)
28. [Ranges](#28-ranges)
29. [Control flow](#29-control-flow)
30. [`match`](#30-match)
31. [`instanceof`](#31-instanceof)
32. [Collections and modules](#32-collections-and-modules)
33. [Weak references](#33-weak-references)
34. [Integrated example](#34-integrated-example)
35. [Current MVP exclusions](#35-current-mvp-exclusions)

---

## 1. Source files

A source file is a sequence of top-level items.

Top-level item forms currently include:

```galfus
import string from "string"

export const version = 1

var localCount = 0
const maxCount = 100

type UserId = int64

struct User {
  id: UserId,
  name: String,
}

enum Status {
  Off,
  On,
}

choice Result<V, F> {
  Ok(V),
  Err(F),
}

constraint Stringable<T> {
  fn toString(self: T): String
}

fn main(): null {
  return
}
```

`var` and `const` are valid both at the top level and inside blocks. At the top level they are items. Inside blocks they are statements.

The parser keeps validation intentionally light. For example, some syntactically accepted forms may later be rejected by semantic analysis.

---

## 2. Comments

Line comments:

```galfus
// comment
```

Block comments:

```galfus
/* comment */
```

---

## 3. Names and paths

A simple name is an identifier:

```galfus
user
User
String
```

A path uses `::`:

```galfus
string::trim
collections::list::push
User::rename
```

The syntax layer does not decide whether a path is a module path, type path, namespace function, or anchor function. It parses the shape. The resolver decides meaning later.

Examples:

```galfus
math::random()
user::rename("Ana")
String::from(value)
```

All of these are syntactically path expressions. Their meaning depends on name resolution and type information.

---

## 4. Imports

Whole module import:

```galfus
import string from "string"
import math from "math"
import collections from "collections"
```

Named import:

```galfus
import { trim, startsWith } from "string"
import { List, Map, Set } from "collections"
```

Local import:

```galfus
import user from "./user"
import { User } from "./user"
```

Native, WASM, or host module import:

```galfus
import physics from "./libphysics"
import wasm_math from "./math.wasm"
import engine from "engine"
```

Search file forms are expected to include:

```text
*.gfs                  Galfus source
*.gfb                  Galfus binary/module artifact
[lib]*.{dylib,so,dll}  dynamic library
*.wasm                 WebAssembly module or component
```

---

## 5. Exports

Supported exported items:

```galfus
export var counter = 0
export const version = 1

export type UserId = int64

export struct User {
  id: UserId,
  name: String,
}

export enum Status {
  Off,
  On,
}

export choice Result<V, F> {
  Ok(V),
  Err(F),
}

export constraint Stringable<T> {
  fn toString(self: T): String
}

export fn sum(a: int32, b: int32): int32 {
  return a + b
}
```

Everything exported is public to importing modules. Everything not exported is private to the module.

Decorator/export interaction is not part of the MVP. Decorators currently apply directly to supported non-export item forms.

---

## 6. Variables and constants

Top-level:

```galfus
var counter = 0
const version = 1
```

Inside blocks:

```galfus
fn main(): null {
  var count = 10
  const limit = 100
  return
}
```

With explicit type:

```galfus
var count: int32 = 10
const limit: int64 = 100
```

`var` creates a mutable binding according to later assignment and ownership rules.

`const` creates an immutable binding.

Bindings may use destructuring patterns:

```galfus
var { id, name } = user
var (x, y) = point
var [first, ...rest] = values
```

---

## 7. Primitive types

Integer types:

```galfus
int8
int16
int32
int64

uint8
uint16
uint32
uint64
```

Float types:

```galfus
float16
float32
float64
```

Boolean and null:

```galfus
bool
null
```

Primitive type names intentionally not included in the MVP:

```galfus
any
unknown
void
char
str
byte
short
long
double
```

Use `uint8` for raw bytes.

Use `uint16` for manual UTF-16 representation when needed.

---

## 8. Literals

Integer literals:

```galfus
0
10
123
```

Binary, octal, and hexadecimal numeric forms may be supported by the lexer/parser depending on current implementation:

```galfus
0b1010
0o755
0xff
```

Float literals:

```galfus
0.0
10.5
6.24
```

Boolean literals:

```galfus
true
false
```

String literals:

```galfus
"hello"
"Renato"
```

Regex literals:

```galfus
/^[a-z]+$/
```

Array literals:

```galfus
[1, 2, 3]
["a", "b", "c"]
```

Struct literals:

```galfus
User {
  id: 1,
  name: "Renato",
}
```

Struct literal shorthand:

```galfus
var id = 1
var name = "Renato"

var user = User {
  id,
  name,
}
```

Inferred struct literals are parser-supported when the target type is known later:

```galfus
struct {
  id: 1,
  name: "Renato",
}
```

---

## 9. `null`

`null` represents absence.

```galfus
var name: String | null = null
```

A function with no useful result returns `null`:

```galfus
fn log(message: String): null {
  return
}
```

Equivalent:

```galfus
fn log(message: String): null {
  return null
}
```

Invalid semantically:

```galfus
fn sum(): int32 {
  return
}
```

The parser accepts `return`; the type checker rejects it when the function result type is not `null`.

---

## 10. Types

Named type:

```galfus
String
User
int32
```

Path type:

```galfus
collections::List
module::User
```

Generic type:

```galfus
List<User>
Map<String, User>
Result<Texture, LoadError>
```

Union type:

```galfus
String | null
int32 | int64
User | LoadError | null
```

Array type:

```galfus
[int32]
[String]
[User]
```

Fixed-size array type:

```galfus
[int32; 3]
[float32; 16]
```

Function type:

```galfus
fn(int32, int32): int32
fn(String): null
```

Grouped type:

```galfus
(int32)
```

Tuple type:

```galfus
(int32, String)
(float32, float32, float32)
```

A trailing comma is syntactically accepted:

```galfus
(int32, String,)
```

A single-element tuple shape like `(int32,)` is syntax-valid. Whether it is semantically useful is left to later phases.

---

## 11. Casts

Explicit cast:

```galfus
var a = <int8> 6.24
var b = <bool> value
var c = <collections::Id> value
```

Assignment cast:

```galfus
var a: int8 = 6.24
var b: bool = 10
```

Conceptually equivalent to:

```galfus
var a: int8 = <int8> 6.24
var b: bool = <bool> 10
```

Numeric casts are total at runtime, unless a checked conversion helper is explicitly used.

Examples:

```galfus
var a = <int8> 6.24
var b = <int8> -6.24
var c = <uint8> -1
var d = <int8> 128
var e = <uint8> 257
```

Checked/validated conversion should be expressed through library functions:

```galfus
integer::checked<int8>(value)
integer::fits<int8>(value)
integer::clamp<int8>(value)
```

---

## 12. Operators

### Arithmetic

```galfus
+
-
*
/
%
**
```

Examples:

```galfus
var a = 10 + 2
var b = 10 - 2
var c = 10 * 2
var d = 10 / 2
var e = 10 % 3
var f = 2 ** 8
```

String concatenation should use module functions or declared anchors:

```galfus
string::concat("hello", "world")
text::concat(other)
```

### No increment/decrement operators

`++` and `--` are intentionally excluded.

Use explicit assignment instead:

```galfus
count += 1
count -= 1
```

This avoids prefix/postfix return-value expectations and keeps mutation explicit.

### Comparison

```galfus
==
!=
<
<=
>
>=
```

Equality semantics are type-specific and handled by later semantic/runtime rules.

### Boolean

```galfus
!
&&
||
```

`&&` and `||` short-circuit.

### Null fallback

```galfus
??
```

Checks only for `null`, not truthiness:

```galfus
var name: String | null = getName()
var safeName = name ?? "Anonymous"
```

There is no `??=`.

### Bitwise

```galfus
&
|
^
~
<<
>>
```

Examples:

```galfus
var a = 0b1010 & 0b1100
var b = 0b1010 | 0b1100
var c = 0b1010 ^ 0b0101
var d = ~0b1010
var e = 1 << 3
var f = 8 >> 1
```

### Assignment

```galfus
=
+=
-=
*=
/=
%=
**=
&=
|=
^=
<<=
>>=
```

Examples:

```galfus
count += 1
flags |= 0b0010
mask &= 0b1111
```

### Access

```galfus
.    field/member access
?.   null-safe member access
[]   indexing
::   path, module function, namespace function, or anchor call
```

Examples:

```galfus
user.name
user.parent?.name
nums[0]
string::trim(text)
text::trim()
```

`?.` is parsed as a `NullSafeMemberExpression`.

`::` is parsed as a path expression unless it is recognized as a numeric quantity range literal, such as `1::4`.

---

## 13. Structs

Declaration:

```galfus
struct User {
  const id: int64,
  name: String,
  age: int32 = 0,
}
```

Instantiation:

```galfus
var user = User {
  id: 1,
  name: "Renato",
}
```

With all fields:

```galfus
var user = User {
  id: 1,
  name: "Renato",
  age: 30,
}
```

A field without default is required.

A field with default may be omitted.

A `const` field cannot be reassigned after initialization.

```galfus
user.id = 2 // semantically invalid when id is const
```

Invalid static-style access:

```galfus
User.name
User.getName()
```

Valid value access:

```galfus
user.name
```

---

## 14. Struct expansion and spread

Struct expansion copies field declarations from another struct blueprint.

```galfus
struct User {
  name: String,
}

struct Person {
  ...User,
  age: int32,
}
```

This is not inheritance. It is field expansion at the structural level.

Struct literal spread copies fields from an existing value:

```galfus
var user2 = User {
  ...user,
}
```

Spread with override:

```galfus
var user2 = User {
  ...user,
  name: "Ana",
}
```

The parser accepts spread shape. Semantic analysis decides whether the spread source is compatible with the target struct.

Deep copy remains explicit:

```galfus
var user2 = copy user
```

By default, complex value assignment may share the same owner depending on ownership semantics:

```galfus
var a = User { name: "Ana" }
var b = a
```

---

## 15. Enums

`enum` is discriminated and has no payload.

```galfus
enum Direction {
  North,
  East,
  South,
  West,
}
```

Explicit discriminants:

```galfus
enum State {
  Off(1),
  On(2),
}
```

Discriminants may be constant expressions syntactically:

```galfus
enum<int64> TextureType {
  Float32(1 << 32),
  Float64,
}
```

The parser accepts the expression. Later constant evaluation validates that it is a valid enum discriminant.

An enum may specify a numeric base type:

```galfus
enum<int64> BigKind {
  A(1),
  B(2),
}
```

Without explicit base type, the default is intended to be `int32`.

Usage:

```galfus
var direction = Direction.North
```

---

## 16. Choices

`choice` represents alternatives with optional payload.

```galfus
choice Asset {
  None,
  Texture(String),
  Mesh(int64),
  Image(String, int32, int32),
  Error(String),
}
```

Usage:

```galfus
var asset = Asset.Texture("grass.png")
```

Generic choice:

```galfus
choice Result<V, F> {
  Ok(V),
  Err(F),
}
```

Choice payloads are parsed as types.

---

## 17. Functions

Function declaration:

```galfus
fn sum(a: int32, b: int32): int32 {
  return a + b
}
```

No useful return value:

```galfus
fn log(message: String): null {
  return
}
```

Function calls:

```galfus
sum(1, 2)
log("hello")
```

Generic function:

```galfus
fn identity<T>(value: T): T {
  return value
}
```

Function declarations may be decorated in the MVP:

```galfus
@log
fn save(user: User): bool {
  return true
}
```

Decorated exported functions are not part of the MVP syntax surface unless explicitly modeled later.

---

## 18. Parameters, defaults, rest, and argument spread

Parameters:

```galfus
fn rename(user: User, name: String): User {
  user.name = name
  return user
}
```

Default parameters are allowed only at the end of the parameter list:

```galfus
fn connect(host: String, port: int32 = 80): bool {
  return true
}
```

The parser captures defaults; the rule that only trailing parameters may have defaults belongs to semantic validation.

Parameter decorators are allowed:

```galfus
fn createUser(
  @string::trim name: String,
  @min(0) age: int32,
): User {
  return User { name, age }
}
```

Rest/variadic parameter:

```galfus
fn summarize(...values: [int32]): int32 {
  var total = 0

  for value in values {
    total += value
  }

  return total
}
```

A rest parameter represents a runtime-sized list/array-like sequence supplied by arguments. Its syntax uses `...` before the parameter name.

Function argument spread:

```galfus
var values = [1, 2, 3]
var total = summarize(...values)
```

Trailing arguments are allowed:

```galfus
summarize(1, 2, 3,)
```

Argument spread works for normal functions and arrow functions:

```galfus
var add = (...values: [int32]): int32 => {
  var total = 0

  for value in values {
    total += value
  }

  return total
}

var total = add(...values)
```

---

## 19. Arrow functions

Expression-style arrow:

```galfus
var double = (value: int32): int32 => value * 2
```

Block-style arrow:

```galfus
var double = (value: int32): int32 => {
  return value * 2
}
```

With rest parameter:

```galfus
var sum = (...values: [int32]): int32 => {
  var total = 0

  for value in values {
    total += value
  }

  return total
}
```

---

## 20. Anchor functions

Declaration:

```galfus
struct User {
  name: String,
}

fn User::rename(self: User, name: String): User {
  self.name = name
  return self
}
```

Call:

```galfus
var user = User {
  name: "Renato",
}

var renamed = user::rename("Ana")
```

The anchor call passes the target as the first logical argument. There is no automatic write-back.

This:

```galfus
user::rename("Ana")
```

is not equivalent to:

```galfus
user = User::rename(user, "Ana")
```

To replace the value:

```galfus
user = user::rename("Ana")
```

`math::random()` and `user::rename("Ana")` are syntactically the same kind of path/call shape. The resolver decides whether it is a module function, namespace function, static-like function, or anchor call.

---

## 21. Generics

Generic struct:

```galfus
struct Box<T> {
  value: T,
}
```

Generic function:

```galfus
fn identity<T>(value: T): T {
  return value
}
```

Generic choice:

```galfus
choice Result<V, F> {
  Ok(V),
  Err(F),
}
```

Generic constraints:

```galfus
fn add<T: int>(a: T, b: T): T {
  return a + b
}
```

Basic constraint names:

```galfus
T: struct
T: enum
T: int
T: float
T: fn
```

Direct type constraint:

```galfus
T: int32
T: float32
T: User
T: Result<Texture, LoadError>
```

---

## 22. Constraints

Constraint declaration syntax:

```galfus
constraint Identifiable<T> {
  id: int64,
}
```

Usage:

```galfus
fn getId<T: Identifiable>(value: T): int64 {
  return value.id
}
```

Function requirement:

```galfus
constraint Stringable<T> {
  fn toString(self: T): String
}
```

Usage:

```galfus
fn stringify<T: Stringable>(value: T): String {
  return value::toString()
}
```

Iterator-style constraint:

```galfus
constraint Iterator<T, Item> {
  fn next(self: T): Item | null
}
```

Iterable-style constraint:

```galfus
constraint Iterable<T, Item, Iter> {
  fn iter(self: T): Iter
}
```

The older `constraint Name::T` form is not the current syntax.

---

## 23. `satisfies`

Structs can declare that they satisfy constraints:

```galfus
constraint Identifiable<T> {
  id: int64,
}

constraint Stringable<T> {
  fn toString(self: T): String
}

struct User satisfies Identifiable, Stringable {
  id: int64,
  name: String,
}

fn User::toString(user: User): String {
  return user.name
}
```

Generic satisfies:

```galfus
constraint Iterable<T, Item, Iter> {
  fn iter(self: T): Iter
}

struct Range satisfies Iterable<Range, int32, RangeIterator> {
  start: int32,
  end: int32,
}
```

---

## 24. Decorators

Decorator target forms:

```galfus
@name
@module::name
@name(args)
@module::name(args)
```

Decorators are syntax-level annotations. Their meaning is resolved later.

Allowed MVP targets:

```text
function declarations
function parameters, including rest parameters
struct declarations
struct fields, including weak fields
```

Function decorator:

```galfus
@log
fn saveUser(user: User): bool {
  return true
}
```

Struct decorator:

```galfus
@frozen
struct User {
  name: String,
}
```

Struct field decorator:

```galfus
struct User {
  @min(0)
  age: int32,
}
```

Parameter decorator:

```galfus
fn createUser(
  @string::trim name: String,
  @min(0) age: int32,
): User {
  return User { name, age }
}
```

Not part of the MVP:

```galfus
@memo var value = 1
@flags enum State { Off, On }
@tracked choice Result<V, F> { Ok(V), Err(F) }
```

Decorator/export interaction is intentionally deferred.

---

## 25. Arrays

Runtime-sized array-like type:

```galfus
[int32]
[String]
```

Fixed-size array:

```galfus
[int32; 3]
[float32; 16]
```

Array literal:

```galfus
var nums = [1, 2, 3]
```

Array spread element:

```galfus
var a = [1, 2]
var b = [0, ...a, 3]
```

Indexing:

```galfus
var first = nums[0]
```

String indexing is semantically distinct from array indexing and should be defined by string/runtime rules.

---

## 26. Tuples

Tuple type:

```galfus
(float32, float32)
(float32, float32, float32)
```

Tuple expression:

```galfus
var point = (10.0, 20.0)
```

Grouped expression remains distinct:

```galfus
var value = (1 + 2)
```

Grouped type remains distinct:

```galfus
var value: (int32) = 10
```

Tuple destructuring:

```galfus
var point = (10.0, 20.0)
var (x, y) = point
```

Function return:

```galfus
fn getPosition(): (float32, float32) {
  return (10.0, 20.0)
}

var (x, y) = getPosition()
```

Trailing comma is accepted:

```galfus
var point = (10.0, 20.0,)
```

---

## 27. Destructuring

Destructuring is valid in `var` and `const` bindings.

Struct destructuring:

```galfus
struct User {
  id: int64,
  name: String,
  age: int32,
}

var user = User {
  id: 1,
  name: "Renato",
  age: 30,
}

var { id, name } = user
```

Alias:

```galfus
var { name: userName, age: userAge } = user
```

Nested pattern:

```galfus
var { address: { city } } = user
```

Tuple destructuring:

```galfus
var point = (10.0, 20.0)
var (x, y) = point
```

Array destructuring:

```galfus
var nums: [int32; 3] = [10, 20, 30]
var [a, b, c] = nums
```

Array rest binding:

```galfus
var [first, ...rest] = nums
```

The parser does not enforce rest-position validation. The semantic checker should reject invalid patterns such as rest not being the last element if that rule is desired.

Not intended for semantic destructuring in MVP:

```text
List
Map
Set
Buffer
String
```

---

## 28. Ranges

Exclusive numeric range literal:

```galfus
1..9
```

Produces conceptually:

```text
1, 2, 3, 4, 5, 6, 7, 8
```

Quantity range literal:

```galfus
1::4
```

Produces conceptually:

```text
1, 2, 3, 4
```

Quantity range with step:

```galfus
1::4%3
```

Produces conceptually:

```text
1, 4, 7, 10
```

The short range literal syntax is intended for numeric literal-style ranges. Dynamic ranges should use library functions:

```galfus
math::range(start, end)
math::qRange(start, count, step)
```

This keeps `::` unambiguous with path and anchor syntax:

```galfus
math::random()
user::rename("Ana")
```

---

## 29. Control flow

### `if`

```galfus
if value > 10 {
  return true
} else {
  return false
}
```

Else-if:

```galfus
if value < 0 {
  return -1
} else if value > 0 {
  return 1
} else {
  return 0
}
```

### `for`

```galfus
for value in values {
  log(value)
}
```

The iterable semantics are defined by later resolver/type-checker rules.

### `while`

```galfus
while count < 10 {
  count += 1
}
```

### `loop`, `break`, and `continue`

```galfus
loop {
  if done {
    break
  }

  continue
}
```

`break` and `continue` are statements.

---

## 30. `match`

Basic match:

```galfus
match value {
  0 => "zero",
  1 => "one",
  _ => "many",
}
```

Choice-style match:

```galfus
match result {
  Result.Ok(value) => value,
  Result.Err(error) => 0,
}
```

Pattern syntax is parsed; exhaustiveness and type compatibility are semantic checks.

---

## 31. `instanceof`

`instanceof` handles type refinement over union-like values.

```galfus
instanceof value {
  String name => log(name),
  int32 count => log(count),
  null => log("missing"),
}
```

The exact narrowing rules belong to semantic analysis.

---

## 32. Collections and modules

Import collections:

```galfus
import collections from "collections"
```

Common generic collection types:

```galfus
List<T>
Map<K, V>
Set<T>

WeakVec<T>
WeakMap<K, V>
WeakSet<T>
```

Instantiation syntax may be provided by collection constructors or literals depending on runtime/library design:

```galfus
var users = List<User> {}
var ids = Set<int64> {}
var table = Map<String, User> {}
```

Submodule functions:

```galfus
collections::list::push(users, user)
collections::set::add(ids, 10)
collections::map::set(table, "id", user)
```

Declared anchors can make usage shorter:

```galfus
users::push(user)
ids::add(10)
table::set("id", user)
```

Equality helpers:

```galfus
collections::equals(a, b)
collections::list::equals(a, b)
collections::set::equals(a, b)
collections::map::equals(a, b)
```

---

## 33. Weak references

Weak fields are syntax-supported when using the chosen weak-field marker from the implementation.

Example shape:

```galfus
struct Node {
  value: int32,
  weak parent: Node | null,
}
```

Weak collections may be represented through library types:

```galfus
WeakVec<Node>
WeakMap<String, Node>
WeakSet<Node>
```

Weak semantics are runtime/type-checker concerns.

---

## 34. Integrated example

```galfus
import string from "string"
import math from "math"

@frozen
export struct User {
  const id: int64,

  @string::trim
  name: String,

  @min(0)
  age: int32 = 0,
}

constraint Stringable<T> {
  fn toString(self: T): String
}

fn User::toString(self: User): String {
  return self.name
}

choice Result<V, F> {
  Ok(V),
  Err(F),
}

enum<int64> TextureType {
  Float32(1 << 32),
  Float64,
}

fn createUser(
  id: int64,
  @string::trim name: String,
  @min(0) age: int32 = 0,
): User {
  return User {
    id,
    name,
    age,
  }
}

fn getPosition(): (float32, float32) {
  return (10.0, 20.0)
}

fn main(): null {
  var user = createUser(1, " Renato ", 30)
  var label = user::toString()

  var user2 = User {
    ...user,
    name: "Ana",
  }

  var { id, name } = user2
  var (x, y) = getPosition()

  for value in 1::4%3 {
    log(value)
  }

  var parentName = user2.parent?.name ?? "none"

  return
}
```

Note: this integrated example shows intended surface syntax. If a specific line depends on semantic/runtime features not implemented yet, the parser may still accept the syntax while later phases decide validity.

---

## 35. Current MVP exclusions

The following are intentionally not part of the current syntax MVP:

```text
++ and -- operators
operator overloading
??= assignment
any / unknown / void primitive types
decorators on var, const, enum, choice, constraint, type alias, statements, or expressions
implicit write-back for anchor calls
semantic validation inside the syntax parser beyond basic structure
full parser recovery strategy
```

Recommended explicit forms:

```galfus
count += 1
count -= 1
user = user::rename("Ana")
math::range(start, end)
math::qRange(start, count, step)
```
