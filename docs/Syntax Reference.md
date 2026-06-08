# Galfus Script — Syntax Reference

## Table of Contents

1. [Comments](#1-comments)
2. [Imports](#2-imports)
3. [Exports](#3-exports)
4. [Variables](#4-variables)
5. [Primitive types](#5-primitive-types)
6. [Numeric literals](#6-numeric-literals)
7. [Strings](#7-strings)
8. [`null`](#8-null)
9. [Cast](#9-cast)
10. [Bool cast and truthiness](#10-bool-cast-and-truthiness)
11. [Operators](#11-operators)
12. [Type alias](#12-type-alias)
13. [Union types](#13-union-types)
14. [Struct](#14-struct)
15. [Struct expansion](#15-struct-expansion)
16. [Shallow copy and deep copy](#16-shallow-copy-and-deep-copy)
17. [Enum](#17-enum)
18. [Choice](#18-choice)
19. [Match](#19-match)
20. [Instanceof](#20-instanceof)
21. [Functions](#21-functions)
22. [Default parameters](#22-default-parameters)
23. [Function argument spread](#23-function-argument-spread)
24. [Variadic parameters](#24-variadic-parameters)
25. [Arrow functions](#25-arrow-functions)
26. [Anchor functions](#26-anchor-functions)
27. [Modules and declared anchors](#27-modules-and-declared-anchors)
28. [Generics](#28-generics)
29. [Constraints](#29-constraints)
30. [`satisfies`](#30-satisfies)
31. [Decorators](#31-decorators)
32. [Arrays](#32-arrays)
33. [Runtime-sized arrays](#33-runtime-sized-arrays)
34. [String indexing](#34-string-indexing)
35. [Tuples](#35-tuples)
36. [Destructuring](#36-destructuring)
37. [Collections](#37-collections)
38. [Ranges](#38-ranges)
39. [`for`](#39-for)
40. [`loop`, `break`, and `continue`](#40-loop-break-and-continue)
41. [`if`](#41-if)
42. [Weak](#42-weak)
43. [`sizeof`](#43-sizeof)
44. [Integer parts](#44-integer-parts)
45. [Integrated example](#45-integrated-example)

---

## 1. Comments

```galfus
// line comment
/* block comment */
```

---

## 2. Imports

Whole module import:

```galfus
import string from "string"
import math from "math"
import collections from "collections"
```

Named import:

```galfus
import { startsWith, trim } from "string"
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
import image from "./image_component.wasm"
import engine from "engine"
```

Search file name for import:

- `*.gfs` - script
- `*.gfb` - binary
- `[lib]*.{dylib,so,dll}` - dynamic lib
- `*.wasm` - Wasm module and component

Submodules:

```galfus
collections::list::push(users, user)
collections::set::add(ids, 10)
collections::map::set(table, "id", user)
```

Submodule reexport:

```galfus
export list from "list"
export map from "map"
export set from "set"
export buffer from "buffer"
```

---

## 3. Exports

```galfus
export const version = 1

export struct User {
  name: String,
}

export fn sum(a: int32, b: int32): int32 {
  return a + b
}
```

Everything exported is public.

Everything not exported is private to the module.

---

## 4. Variables

```galfus
var a = 10
const b = 20
```

With explicit type:

```galfus
var a: int32 = 10
const b: int64 = 20
```

`var` allows reassignment according to value rules.

`const` fixes the binding.

---

## 5. Primitive types

```galfus
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

bool
null
```

Primitive types that do not exist:

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

For raw bytes:

```galfus
uint8
```

For manual UTF-16 representation:

```galfus
uint16
```

---

## 6. Numeric literals

Decimal:

```galfus
0
10
123
999
```

Hexadecimal:

```galfus
0x0
0xff
0xFF
0x10
```

Octal:

```galfus
0o0
0o755
```

Binary:

```galfus
0b0
0b1010
0b1111_0000
```

Digit separator `_`:

```galfus
1_000
1_000_000
0xff_ff
0b1010_0101
0_000_0_0
```

Valid:

```galfus
1_000
0b1010_0101
0xFF_EC_DE_5E
```

Invalid:

```galfus
_1000
1000_
1__000
0x_FF
```

---

## 7. Strings

Double quotes and single quotes are valid:

```galfus
var a = "hello"
var b = 'hello'
var c = 'a'
```

`'a'` is `String`, not `char`.

An empty string exists:

```galfus
var text = ""
```

Multiline string:

```galfus
var text = `
line 1
line 2
line 3
`
```

`String` is not primitive. It is an internally optimized `struct`.

String operations come from the `string` module:

```galfus
import string from "string"

var ok = string::startsWith(text, "h")
```

Declared anchors can also exist on `String`:

```galfus
var ok = text::startsWith("h")
```

---

## 8. `null`

`null` represents absence.

```galfus
var name: String | null = null
```

A function with no useful return value returns `null`:

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

Invalid:

```galfus
fn sum(): int32 {
  return
}
```

---

## 9. Cast

Explicit cast:

```galfus
var a = <int8> 6.24
var b = <bool> value
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

Numeric casts are total.

Examples:

```galfus
var a = <int8> 6.24     // 6
var b = <int8> -6.24    // -6
var c = <uint8> -1      // 255
var d = <int8> 128      // -128
var e = <uint8> 257     // 1
```

Explicit validation through modules:

```galfus
integer::checked<int8>(value)
integer::fits<int8>(value)
integer::clamp<int8>(value)
```

---

## 10. Bool cast and truthiness

Every value can be cast to `bool`.

Numbers:

```galfus
0      // false
!= 0   // true
NaN    // true
+Inf   // true
-Inf   // true
```

Complex values:

```galfus
null    // false
exists  // true
```

Examples:

```galfus
var a: bool = 0       // false
var b: bool = 1       // true
var c: bool = -10     // true
var d: bool = 0.0     // false
var e: bool = 0.01    // true
var f: bool = ""      // true
var g: bool = null    // false
```

Union uses the active type’s rule:

```galfus
var value: int32 | String | null = getValue()

var ok: bool = value
```

---

## 11. Operators

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

No operator overloading in the MVP.

Invalid:

```galfus
"hello" + "world"
```

Concatenation:

```galfus
string::concat("hello", "world")
```

Or through a declared anchor:

```galfus
text::concat(other)
```

---

### Increment and decrement

```galfus
++
--
```

They are statement-only. They do not return a value.

Allowed:

```galfus
i++
i--
++i
--i
```

Invalid:

```galfus
var x = i++
var y = ++i
foo(i++)
```

Equivalences:

```galfus
i++  // i += 1
i--  // i -= 1
++i  // i += 1
--i  // i -= 1
```

---

### Comparison

```galfus
==
!=
<
<=
>
>=
```

Equality rule:

```galfus
primitives  -> compare by value
enum        -> compare by discriminant
null        -> compare absence
owner       -> compare owner identity
union       -> use the active type’s rule
```

Owner example:

```galfus
var a = User { name: "Ana" }
var b = a

a == b // true
```

```galfus
var a = User { name: "Ana" }
var b = User { name: "Ana" }

a == b // false
```

Structural/deep equality:

```galfus
reflect::equals(a, b)
string::equals(a, b)
collections::equals(a, b)
collections::list::equals(a, b)
collections::set::equals(a, b)
collections::map::equals(a, b)
```

---

### Boolean

```galfus
!
&&
||
```

Operands are cast to `bool`.

The result is always `bool`.

`&&` and `||` short-circuit.

Examples:

```galfus
!0          // true
!1          // false
!""         // false
!null       // true

"hello" && 10   // true
null && 10      // false
0 || "hello"    // true
```

---

### Null fallback

```galfus
??
```

Checks only `null`, not truthiness.

```galfus
var name: String | null = getName()
var safeName = name ?? "Anonymous"
```

Example:

```galfus
var value: int32 | null = 0
var result = value ?? 10 // 0
```

Does not exist:

```galfus
??=
```

---

### Bitwise

```galfus
&   bitwise and
|   bitwise or
^   bitwise xor
~   bitwise not
<<  shift left
>>  shift right
```

Examples:

```galfus
var a = 0b1010 & 0b1100
var b = 0b1010 | 0b1100
var c = 0b1010 ^ 0b1100
var d = ~0b1010

var e = 1 << 3
var f = 8 >> 1
```

Bitwise assignments:

```galfus
flags |= 0b0010
flags &= 0b1111
flags ^= 0b0101
flags <<= 1
flags >>= 1
```

---

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

Example:

```galfus
count += 1
flags |= 0b0010
mask &= 0b1111
```

---

### Access

```galfus
.   // real field access
?.  // null safety field access
[]  // indexing
::  // module, namespace, or anchor function
```

Examples:

```galfus
user.name
text.length
user.parent?.name
nums[0]

string::startsWith(text, "h")
text::startsWith("h")
```

---

## 12. Type alias

Type aliases are only aliases.

```galfus
type SmallInt = int32 | int8
type MaybeString = String | null
type UserId = int64
```

Usage:

```galfus
var id: UserId = 10
var value: SmallInt = 5
var name: MaybeString = null
```

---

## 13. Union types

```galfus
var value: int32 | String | null = null
```

With alias:

```galfus
type Value = int32 | String | null

var value: Value = null
```

Union does not create `any`.

The set of possible types is always known.

---

## 14. Struct

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

A field with default can be omitted.

Invalid:

```galfus
User.name
User.getName()
```

Valid:

```galfus
user.name
```

A `const` field cannot be changed:

```galfus
user.id = 2 // invalid
```

---

## 15. Struct expansion

```galfus
struct User {
  name: String,
}

struct Person {
  ...User,
  age: int32,
}
```

This copies fields from the blueprint.

It does not create inheritance.

---

## 16. Shallow copy and deep copy

Shallow copy:

```galfus
var user2 = User {
  ...user,
}
```

Override:

```galfus
var user2 = User {
  ...user,
  name: "Ana",
}
```

Deep copy:

```galfus
var user2 = copy user
```

By default, complex values share the owner:

```galfus
var a = User { name: "Ana" }
var b = a
```

---

## 17. Enum

`enum` is discriminated and has no payload.

```galfus
enum Direction {
  North,
  East,
  South,
  West,
}
```

Explicit discriminant:

```galfus
enum E {
  Initial,      // 0
  Changed(10), // 10
  Another,     // 11
}
```

`enum` is by default int32, but can another number.

```galfus
enum<int64> TextureType {
  Float32(1<<32),
  Float64,
  // ...
}
```

Usage:

```galfus
var direction = Direction.North
```

---

## 18. Choice

`choice` is for alternatives with optional payload.

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

```galfus
choice Option<T> {
  Some(T),
  None,
}
```

---

## 19. Match

```galfus
match asset {
  Texture(path) => {
    return loadTexture(path)
  }

  Image(path, width, height) => {
    return loadImage(path, width, height)
  }

  None => {
    return fallback()
  }

  _ => {
    return fallback()
  }
}
```

With enum:

```galfus
match direction {
  North => {
    return
  }

  South => {
    return
  }

  _ => {
    return
  }
}
```

---

## 20. Instanceof

```galfus
instanceof value {
  int32(v) => {
    return v ** 2
  }

  String(text) => {
    return text.length
  }

  _ => {
    return 0
  }
}
```

---

## 21. Functions

```galfus
fn square(v: int32): int32 {
  return v ** 2
}
```

```galfus
fn createUser(name: String, age: int32): User {
  return User {
    id: 1,
    name,
    age,
  }
}
```

Function returning `null`:

```galfus
fn print(message: String): null {
  return null
}
// or
fn print(message: String): null {
  return
}
// or
fn print(message: String): null {}
```

---

## 22. Default parameters

Only trailing parameters can have defaults.

```galfus
fn createUser(name: String, age: int32 = 0): User {
  return User {
    id: 1,
    name,
    age,
  }
}
```

Multiple trailing defaults:

```galfus
fn connect(
  host: String,
  port: int32 = 8080,
  secure: bool = false,
): Connection {
  return Connection {
    host,
    port,
    secure,
  }
}
```

Invalid:

```galfus
fn createUser(name: String = "Anonymous", age: int32): User {
  return User {
    id: 1,
    name,
    age,
  }
}
```

---

## 23. Function argument spread

Spread expands an array or tuple into positional arguments.

```galfus
fn sum(a: int32, b: int32, c: int32): int32 {
  return a + b + c
}

var args: [int32; 3] = [1, 2, 3]

var result = sum(...args)
```

Tuple spread:

```galfus
var point = (10.0, 20.0)

fn move(x: float32, y: float32): null {
  return
}

move(...point)
```

Mixed arguments:

```galfus
fn createUser(name: String, age: int32, active: bool): User {
  return User {
    id: 1,
    name,
    age,
  }
}

var rest = (30, true)

var user = createUser("Renato", ...rest)
```

Spread must match the expected parameter count and types. If the array is more large then arguments length, the rest is discarded.

For non-variadic functions, the arity must be exact.

```galfus
sum(...args) // valid if args has 3 or greater int32 items
```

---

## 24. Variadic parameters

Functions can receive multiple trailing arguments with a spread parameter.

```galfus
fn summarize(...values: [int32]): int32 {
  var total: int32 = 0

  for value in values {
    total += value
  }

  return total
}
```

Call:

```galfus
var result = summarize(1, 2, 3, 4)
```

Inside the function:

```galfus
values: [int32] // conceptually [int32; n]
```

The parameter:

```galfus
...values: [int32]
```

means:

```galfus
collect all trailing arguments into an internal array
```

---

### Variadic with header

```galfus
fn log(prefix: String, ...messages: [String]): null {
  for message in messages {
    print(string::concat(prefix, message))
  }

  return
}
```

Call:

```galfus
log("[info] ", "started", "running", "done")
```

The variadic parameter must be the last parameter.

Invalid:

```galfus
fn invalid(...values: [int32], end: int32): int32 {
  return end
}
```

---

### Spread into variadic function

```galfus
var values: [int32; 3] = [1, 2, 3]

var result = summarize(...values)
```

Tuple spread:

```galfus
var values = (1, 2, 3)

var result = summarize(...values)
```

Mixed call:

```galfus
var more: [int32; 2] = [3, 4]

var result = summarize(1, 2, ...more)
```

---

### Internal function array

A variadic function receives trailing arguments through an internal array.

```galfus
fn summarize(...values: [int32]): int32 {
  return values.length
}
```

Conceptually:

```galfus
values: [int32]
```

The runtime stores:

```galfus
element type: int32
length: n
data: contiguous elements
```

This array is fixed after construction.

---

## 25. Arrow functions

```galfus
var sum = (a: int32, b: int32): int32 => {
  return a + b
}
```

Expression body:

```galfus
var double = (v: int32): int32 => v * 2
```

No parameter:

```galfus
var getValue = (): int32 => 10
```

Variadic:

```galfus
var summarize = (...values: [int32]): int32 => {
  var total: int32 = 0

  for value in values {
    total += value
  }

  return total
}
```

Variadic expression body:

```galfus
var count = (...values: [int32]): int32 => values.length
```

Mixed:

```galfus
var log = (prefix: String, ...messages: [String]): null => {
  for message in messages {
    print(string::concat(prefix, message))
  }

  return
}
```

Parentheses are always required.

---

## 26. Anchor functions

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

The anchor call passes the target as the first argument:

```galfus
var renamed = User::rename(user, "Ana")
```

There is no automatic write-back.

This:

```galfus
user::rename("Ana")
```

does not mean:

```galfus
user = User::rename(user, "Ana")
```

To replace the value:

```galfus
user = user::rename("Ana")
```

---

## 27. Modules and declared anchors

Module function:

```galfus
string::startsWith(text, "h")
```

Anchor declared on the type:

```galfus
fn String::startsWith(self: String, prefix: String): bool {
  return string::startsWith(self, prefix)
}
```

Usage:

```galfus
var text = "hello"

var a = string::startsWith(text, "h")
var b = text::startsWith("h")
```

Anchors are not magical module resolution.

The anchor must exist on the type.

Invalid on direct literals:

```galfus
"hello"::startsWith("h")
```

Use a variable/binding:

```galfus
var text = "hello"
var ok = text::startsWith("h")
```

---

## 28. Generics

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

Generic constraints:

```galfus
fn add<T: int>(a: T, b: T): T {
  return a + b
}
```

Basic constraints:

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

## 29. Constraints

Field constraint:

```galfus
constraint Identifiable::T {
  id: int64,
}
```

Usage:

```galfus
fn getId<T: Identifiable>(value: T): int64 {
  return value.id
}
```

Function anchor constraint:

```galfus
constraint Stringable::T {
  fn T::toString(self: T): String
}
```

Usage:

```galfus
fn stringify<T: Stringable>(value: T): String {
  return value::toString()
}
```

Iterator:

```galfus
constraint Iterator::T<Item> {
  fn T::next(self: T): Item | null
}
```

Iterable:

```galfus
constraint Iterable::T<Item, Iter> {
  fn T::iter(self: T): Iter
}
```

---

## 30. `satisfies`

```galfus
constraint Identifiable::T {
  id: int64,
}

constraint Stringable::T {
  fn T::toString(self: T): String
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
constraint Iterable::T<Item, Iter> {
  fn T::iter(self: T): Iter
}

struct Range satisfies Iterable<int32, RangeIterator> {
  start: int32,
  end: int32,
}
```

---

## 31. Decorators

Decorator function:

```galfus
fn frozen<T: struct>(target: T): T {
  return target
}
```

Struct decorator:

```galfus
@frozen
struct User {
  name: String,
}
```

Field decorator:

```galfus
fn min<T: int>(value: T, limit: T): T {
  if value < limit {
    return limit
  }

  return value
}

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
  return User {
    id: 1,
    name,
    age,
  }
}
```

Function decorator:

```galfus
fn log<T: fn>(target: T): T {
  return target
}

@log
fn saveUser(user: User): bool {
  return true
}
```

Decorator order:

```galfus
@a
@b
fn run(): null {
  return
}
```

Equivalent to:

```galfus
run = a(b(run))
```

---

## 32. Arrays

Compile-time-sized fixed array:

```galfus
var nums: [int32; 3] = [10, 20, 30]
```

Indexing:

```galfus
var a = nums[0]
var b = nums[-1]
```

Dynamic index:

```galfus
var i: int32 = getIndex()
var value = nums[i] // int32 | null
```

Invalid:

```galfus
nums[3]
nums[-4]
```

Array destructuring:

```galfus
var [a, b, c] = nums
```

Partial:

```galfus
var [first, second] = nums

var [first, ...rest] = nums
```

Empty arrays do not exist.

---

## 33. Runtime-sized arrays

Arrays are fixed after creation, but their size does not always need to be known at compile time. The minimal length is 1.

Compile-time-sized array:

```galfus
var a: [int32; 3] = [1, 2, 3]
```

Runtime-sized array:

```galfus
var size = 2

var values = buffer::array<int32>(size)
// type: [int32]
```

`[int32]` means:

```galfus
array of int32 with fixed size known at runtime
```

`[int32; 3]` means:

```galfus
array of int32 with fixed size known at compile time
```

Indexing into a runtime-sized array:

```galfus
var size = 2
var values = buffer::array<int32>(size)

values[0] = 10
values[1] = 20
```

Because the compiler does not know `n`, even literal indexing returns nullable:

```galfus
var dynamic: [int32] = buffer::array<int32>(size)

var a = dynamic[0] // int32 | null
```

---

## 34. String indexing

```galfus
var text = "abc"

var a = text[0]
var b = text[1]
var c = text[-1]
var d = text[99]
```

Return type:

```galfus
String | null
```

Indexing is by code point.

For bytes:

```galfus
var bytes = string::bytes(text)
```

or through a declared anchor:

```galfus
var bytes = text::bytes()
```

---

## 35. Tuples

```galfus
var pos: (float32, float32, float32) = (10.0, 2.0, 5.0)
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

---

## 36. Destructuring

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

Partial:

```galfus
var { name } = user
```

Invalid:

```galfus
var { email } = user
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

Not allowed for:

```galfus
List
Map
Set
Buffer
String
```

---

## 37. Collections

```galfus
import collections from "collections"
```

Types:

```galfus
List<T>
Map<K, V>
Set<T>

WeakVec<T>
WeakMap<K, V>
WeakSet<T>
```

Instantiation:

```galfus
var users = List<User> {}
var ids = Set<int64> {}
var table = Map<String, User> {}
```

Submodules:

```galfus
collections::list::push(users, user)
collections::set::add(ids, 10)
collections::map::set(table, "id", user)
```

Declared anchors:

```galfus
users::push(user)
ids::add(10)
table::set("id", user)
```

Equality:

```galfus
collections::equals(a, b)
collections::list::equals(a, b)
collections::set::equals(a, b)
collections::map::equals(a, b)
```

---

## 38. Ranges

Exclusive range:

```galfus
1..9
```

Produces:

```galfus
1, 2, 3, 4, 5, 6, 7, 8
```

Quantity range:

```galfus
1::4
```

Produces:

```galfus
1, 2, 3, 4
```

Quantity range with step:

```galfus
1::4%3
```

Produces:

```galfus
1, 4, 7, 10
```

Dynamic range:

```galfus
math::range(start, end)
math::qRange(start, count, step)
```

Usage:

```galfus
for i in 1..9 {
  print(i)
}

for i in 1::4%3 {
  print(i)
}

for i in math::range(start, end) {
  print(i)
}
```

---

## 39. `for`

```galfus
for item in source {
  ...
}
```

With index:

```galfus
for item, index in source {
  ...
}
```

Map iteration:

```galfus
for entry, index in table {
  var key = entry[0]
  var value = entry[1]
}
```

Tuple destructuring:

```galfus
for (key, value), index in table {
  ...
}
```

---

## 40. `loop`, `break`, and `continue`

```galfus
loop {
  ...
}
```

Break:

```galfus
loop {
  if done {
    break
  }
}
```

Continue:

```galfus
loop {
  if skip {
    continue
  }
}
```

---

## 41. `if`

```galfus
if value {
  return true
}
```

```galfus
if value {
  return true
} else {
  return false
}
```

Conditions accept any value castable to `bool`.

---

## 42. Weak

```galfus
struct CacheEntry {
  weak resource: Resource | null = null,
}
```

Invalid:

```galfus
struct CacheEntry {
  weak resource: Resource,
}
```

Weak collections:

```galfus
var nodes = WeakVec<Node> {}
var cache = WeakMap<String, Node> {}
var selected = WeakSet<Node> {}
```

---

## 43. `sizeof`

Native `sizeof` is shallow.

```galfus
sizeof(int32)
sizeof(float16)
sizeof([int32; 4])
sizeof(User)
sizeof(user)
sizeof((int32, float32))
```

Internal/dynamic memory:

```galfus
string::sizeof(text)
buffer::sizeof(bytes)
collections::list::sizeof(items)
collections::map::sizeof(table)
collections::set::sizeof(values)
```

Declared anchors may exist:

```galfus
text::sizeof()
items::sizeof()
bytes::sizeof()
```

---

## 44. Integer parts

```galfus
enum Endian {
  Little,
  Big,
  Native,
}
```

```galfus
var value: int64 = 9223372036854775807

var a = integer::toParts<int64, int32>(value, Endian.Little)
var b = integer::toParts<int64, int16>(value, Endian.Little)
var c = integer::toParts<int32, uint8>(value, Endian.Big)
```

```galfus
var value = integer::fromParts<int64, int32>(parts, Endian.Little)
```

---

## 45. Integrated example

```galfus
import string from "string"
import collections from "collections"

type UserName = String
type MaybeUser = User | null

constraint Stringable::T {
  fn T::toString(self: T): String
}

choice Result<V, F> {
  Ok(V),
  Err(F),
}

enum Role {
  Admin,
  Member,
  Guest,
}

struct User satisfies Stringable {
  const id: int64,
  name: UserName,
  role: Role = Role.Member,
  tags: Set<String> = Set<String> {},
}

fn User::toString(user: User): String {
  return user.name
}

fn User::rename(user: User, name: String): User {
  user.name = name
  return user
}

fn createUser(
  @string::trim name: String,
  role: Role = Role.Member,
): User {
  return User {
    id: 1,
    name,
    role,
  }
}

fn summarize(...values: [int32]): int32 {
  var total: int32 = 0

  for value in values {
    total += value
  }

  return total
}

var user = createUser(' Renato ')

user = user::rename("Ana")

user.tags::add("active")

var first = user.name[0]
var safeFirst = first ?? ""

if user.name::startsWith("A") {
  print(user.name)
}

var total = summarize(1, 2, 3, 4)

for i in 1::4%3 {
  print(i)
}

match user.role {
  Admin => {
    print("admin")
  }

  Member => {
    print("member")
  }

  _ => {
    print("guest")
  }
}
```
