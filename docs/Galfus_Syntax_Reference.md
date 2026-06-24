# Galfus Script — Syntax Reference

This document defines the current syntax surface of Galfus Script. It describes the source forms accepted by the language parser. Semantic validation, type resolution, ownership validation, module resolution, bundling, and runtime behavior are defined in separate references.

## Table of Contents

1. [Source files](#1-source-files)
2. [Comments](#2-comments)
3. [Names and paths](#3-names-and-paths)
4. [Imports](#4-imports)
5. [Exports](#5-exports)
6. [Variables and constants](#6-variables-and-constants)
7. [Primitive types](#7-primitive-types)
8. [Literals](#8-literals)
9. [Types](#9-types)
10. [Casts](#10-casts)
11. [Operators](#11-operators)
12. [Structs](#12-structs)
13. [Struct expansion and literal spread](#13-struct-expansion-and-literal-spread)
14. [Enums](#14-enums)
15. [Choices](#15-choices)
16. [Functions](#16-functions)
17. [Stamped functions](#17-stamped-functions)
18. [Parameters, defaults, rest, and spread](#18-parameters-defaults-rest-and-spread)
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
32. [Weak fields](#32-weak-fields)
33. [Core exclusions](#33-core-exclusions)
34. [Integrated example](#34-integrated-example)

---

## 1. Source files

A Galfus source file is a sequence of top-level items.

Top-level item forms are:

```galfus
import math from "$math"

export const version = 1

var localCount = 0
const maxCount = 100

type UserId = int64

struct User {
  id: UserId,
  name: [uint8],
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
  fn toString(self: T): [uint8]
}

fn main(): null {
  return
}

fn(stamp) max(a: int32, b: int32): int32 {
  if a > b {
    return a
  }

  return b
}
```

`var` and `const` are valid both at the top level and inside blocks. At the top level they are items. Inside blocks they are statements.

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
Result
```

A path uses `::`:

```galfus
math::random
module::User
user::rename
```

Path syntax is shared by module-style calls, namespace-like calls, type paths, and anchor function calls. The parser recognizes the shape. Later phases resolve the meaning.

Examples:

```galfus
math::random()
user::rename("Ana")
Type::from(value)
```

---

## 4. Imports

Whole module import:

```galfus
import math from "$math"
import user from "./user"
import text from "text"
import vectors from "@vulppi/math-core/vector"
```

Named import:

```galfus
import { Vec2, Vec3 } from "./vectors"
```

The syntax only defines the import statement shape. Address classification and module discovery are workspace concerns.

---

## 5. Exports

Supported exported items:

```galfus
export var counter = 0
export const version = 1

export type UserId = int64

export struct User {
  id: UserId,
  name: [uint8],
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
  fn toString(self: T): [uint8]
}

export fn sum(a: int32, b: int32): int32 {
  return a + b
}

export fn(stamp) min(a: int32, b: int32): int32 {
  if a < b {
    return a
  }

  return b
}
```

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

The following names are not core primitive types:

```text
any
unknown
void
char
str
byte
short
long
double
String
```

Use `uint8` for raw bytes. String literals are UTF-8 byte arrays and therefore have an array-based type such as `[uint8]`.

---

## 8. Literals

Integer literals:

```galfus
0
10
123
```

Binary, octal, and hexadecimal literals:

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

Null literal:

```galfus
null
```

String literals are UTF-8 byte array literals:

```galfus
"hello"
"Renato"
```

Example:

```galfus
var name: [uint8] = "Renato"
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

Inferred struct literals:

```galfus
struct {
  id: 1,
  name: "Renato",
}
```

---

## 9. Types

Named type:

```galfus
User
UserId
int32
```

Path type:

```galfus
module::User
collection::List
```

Generic type:

```galfus
List<User>
Map<[uint8], User>
Result<Texture, LoadError>
```

Union type:

```galfus
[uint8] | null
int32 | int64
User | LoadError | null
```

Array type:

```galfus
[int32]
[uint8]
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
fn([uint8]): null
```

Grouped type:

```galfus
(int32)
```

Tuple type:

```galfus
(int32, int32)
(float32, float32, float32)
```

A common tuple has at least two elements. Single payloads remain valid inside `choice` variants.

Type alias:

```galfus
type UserId = int64
type Name = [uint8]
type Number = int32 | int64
```

---

## 10. Casts

Explicit cast:

```galfus
var a = <int8> 6.24
var b = <bool> value
var c = <module::Id> value
```

Assignment cast through type annotation:

```galfus
var a: int8 = 6.24
var b: bool = 10
```

Enum cast:

```galfus
var raw = <int32> Direction::North
```

---

## 11. Operators

Arithmetic:

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

`+` is not string or array concatenation in the core language.

Increment and decrement operators do not exist:

```text
++
--
```

Comparison:

```galfus
==
!=
<
<=
>
>=
```

Boolean:

```galfus
!
&&
||
```

Null fallback:

```galfus
value ?? fallback
```

Null fallback assignment:

```galfus
value ??= fallback
```

Bitwise:

```galfus
&
|
^
~
<<
>>
```

Assignment:

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
??=
```

Access:

```galfus
user.name
user.parent?.name
nums[0]
nums[-1]
module::function()
```

---

## 12. Structs

Declaration:

```galfus
struct User {
  const id: int64,
  name: [uint8],
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

Value access:

```galfus
user.name
```

---

## 13. Struct expansion and literal spread

Struct expansion copies field declarations from another struct blueprint:

```galfus
struct User {
  name: [uint8],
}

struct Person {
  ...User,
  age: int32,
}
```

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

---

## 14. Enums

Declaration:

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

Enum with numeric base type:

```galfus
enum<int64> BigKind {
  A(1),
  B(2),
}
```

Usage:

```galfus
var direction = Direction::North
```

Casting an enum symbol to its numeric representation:

```galfus
var raw = <int32> Direction::North
```

---

## 15. Choices

A `choice` represents alternatives with optional payload.

```galfus
choice Asset {
  None,
  Texture([uint8]),
  Mesh(int64),
  Image([uint8], int32, int32),
  Error([uint8]),
}
```

Choice construction:

```galfus
var asset = Asset::Texture("grass.png")
```

Generic choice:

```galfus
choice Result<V, F> {
  Ok(V),
  Err(F),
}
```

Choice payloads are conceptually tuple-shaped. A choice payload may contain one item or multiple items.

Decorators may appear on individual choice payload items:

```galfus
choice Asset {
  Texture(@path [uint8]),
  Image(@path [uint8], @min(1) int32, @min(1) int32),
  Error(@message [uint8]),
}
```

---

## 16. Functions

Function declaration:

```galfus
fn sum(a: int32, b: int32): int32 {
  return a + b
}
```

No useful return value:

```galfus
fn log(message: [uint8]): null {
  return
}
```

Equivalent:

```galfus
fn log(message: [uint8]): null {
  return null
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

---

## 17. Stamped functions

A stamped function uses the `fn(stamp)` function metadata form:

```galfus
fn(stamp) max(a: int32, b: int32): int32 {
  if a > b {
    return a
  }

  return b
}
```

A stamped anchor function:

```galfus
fn(stamp) Vec2::lengthSq(self: Vec2): float32 {
  return self.x * self.x + self.y * self.y
}
```

The syntax marks the function item. Stack behavior, lowering behavior, and stamp-specific validation are semantic and architecture concerns.

---

## 18. Parameters, defaults, rest, and spread

Parameters:

```galfus
fn rename(user: User, name: [uint8]): User {
  user.name = name
  return user
}
```

Default parameters:

```galfus
fn connect(host: [uint8], port: int32 = 80): bool {
  return true
}
```

Parameter decorators:

```galfus
fn createUser(
  @trim name: [uint8],
  @min(0) age: int32,
): User {
  return User { name, age }
}
```

Rest parameter:

```galfus
fn summarize(...values: [int32]): int32 {
  var total = 0

  for value in values {
    total += value
  }

  return total
}
```

Function argument spread:

```galfus
var values = [1, 2, 3]
var total = summarize(...values)
```

Trailing arguments:

```galfus
summarize(1, 2, 3,)
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
  name: [uint8],
}

fn User::rename(self: User, name: [uint8]): User {
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

There is no implicit write-back:

```galfus
user = user::rename("Ana")
```

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

Inline generic constraints:

```galfus
fn add<T: int>(a: T, b: T): T {
  return a + b
}
```

---

## 22. Constraints

Constraint declaration:

```galfus
constraint Identifiable<T> {
  id: int64,
}
```

Function requirement:

```galfus
constraint Stringable<T> {
  fn toString(self: T): [uint8]
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

---

## 23. `satisfies`

Structs can declare that they satisfy constraints:

```galfus
constraint Identifiable<T> {
  id: int64,
}

struct User satisfies Identifiable {
  id: int64,
  name: [uint8],
}
```

Multiple constraints:

```galfus
struct User satisfies Identifiable, Stringable {
  id: int64,
  name: [uint8],
}
```

Generic satisfies:

```galfus
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
  name: [uint8],
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
  @trim name: [uint8],
  @min(0) age: int32,
): User {
  return User { name, age }
}
```

Rest parameter decorator:

```galfus
fn summarize(@nonempty ...values: [int32]): int32 {
  return 0
}
```

Weak field decorator:

```galfus
struct Node {
  @nullable
  weak parent: Node | null,
}
```

Choice payload item decorator:

```galfus
choice Asset {
  Texture(@path [uint8]),
  Image(@path [uint8], @min(1) int32, @min(1) int32),
}
```

Decorators only exist on the supported syntax targets above.

---

## 25. Arrays

Runtime-sized array-like type:

```galfus
[int32]
[uint8]
[User]
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

String literal as byte array:

```galfus
var bytes: [uint8] = "hello"
```

Array spread element:

```galfus
var a = [1, 2]
var b = [0, ...a, 3]
```

Indexing:

```galfus
var first = nums[0]
var last = nums[-1]
```

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

Grouped expression:

```galfus
var value = (1 + 2)
```

Grouped type:

```galfus
var value: (int32) = 10
```

Tuple destructuring:

```galfus
var point = (10.0, 20.0)
var (x, y) = point
```

A common tuple has at least two elements. A single-item payload is valid inside `choice` variants.

---

## 27. Destructuring

Struct destructuring:

```galfus
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

---

## 28. Ranges

Exclusive numeric range literal:

```galfus
1..9
```

Quantity range literal:

```galfus
1::4
```

Quantity range with step:

```galfus
1::4%3
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

### `return`

```galfus
return
return value
```

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
  Result::Ok(value) => value,
  Result::Err(error) => 0,
}
```

Wildcard pattern:

```galfus
_
```

---

## 31. `instanceof`

`instanceof` handles type refinement over union-like values:

```galfus
instanceof value {
  [uint8] name => log(name),
  int32 count => log(count),
  null => log("missing"),
}
```

---

## 32. Weak fields

Weak field shape:

```galfus
struct Node {
  value: int32,
  weak parent: Node | null,
}
```

Decorated weak field:

```galfus
struct Node {
  @nullable
  weak parent: Node | null,
}
```

---

## 33. Core exclusions

The following syntax does not exist:

```text
String as a core primitive type
regex literals
++ and -- operators
operator overloading
class syntax
methods built into data forms
decorators on var/const/enum/choice/constraint/type alias/statements/expressions
```

Tuples, choices, enums, arrays, structs, and string literals are data forms. Rich manipulation belongs to reachable modules, not to the core syntax.

---

## 34. Integrated example

```galfus
import text from "text"

@frozen
export struct User {
  const id: int64,

  @trim
  name: [uint8],

  @min(0)
  age: int32 = 0,
}

constraint Stringable<T> {
  fn toString(self: T): [uint8]
}

fn User::toString(self: User): [uint8] {
  return self.name
}

choice Result<V, F> {
  Ok(V),
  Err(@message F),
}

enum<int64> TextureType {
  Float32(1 << 32),
  Float64,
}

fn createUser(
  id: int64,
  @trim name: [uint8],
  @min(0) age: int32 = 0,
): User {
  return User {
    id,
    name,
    age,
  }
}

fn(stamp) max(a: int32, b: int32): int32 {
  if a > b {
    return a
  }

  return b
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

  var values = [10, 20, 30]
  var last = values[-1]

  var fallback: [uint8] | null = null
  fallback ??= "none"

  for value in 1::4%3 {
    log(value)
  }

  return
}
```
