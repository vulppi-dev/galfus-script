Previous: [Constraints as Traits](./06-constraints-as-traits.md) | Index: [Galfus Core System](./00-index.md) | Next: [Expressions and Operators](./08-expressions-and-operators.md)

---

# 7. Data Forms

This document defines arrays, byte strings, tuples, structs, enums, choices, weak fields, shallow copy, and explicit deep copy.

## 7.1 Core Principle

Data forms are data shapes, not implicit behavior containers.

Core data forms:

```txt
array
fixed-size array
byte string as [uint8]
tuple
struct
enum
choice
```

Rich behavior belongs to explicit functions, constraints, or modules.

## 7.2 Arrays

Runtime-sized array type:

```galfus
[int32]
[User]
```

Fixed-size array type:

```galfus
[int32; 10]
[User | null; 4]
```

Array literal:

```galfus
var values = [1, 2, 3]
```

All elements must be compatible with the element type.

## 7.3 Fixed-Size Arrays

A fixed-size array includes length in the type.

```galfus
var values: [int32; 3] = [1, 2, 3]
```

Invalid length mismatch:

```galfus
var values: [int32; 3] = [1, 2]
```

## 7.4 Array Construction with `new`

Array construction uses a fully defined fixed-size array type and no body.

```galfus
var values = new([int32; 10])
```

Default element initialization:

```txt
nullable type -> null
bool -> false
integer -> 0
float -> 0.0
```

Invalid when no default exists:

```galfus
var users = new([User; 4])
```

## 7.5 Array Indexing and Length

Indexing uses square brackets.

```galfus
var first = values[0]
var last = values[-1]
```

Negative indexes count from the end.

Out-of-bounds read returns `null`.

```galfus
var value = values[999]
```

Arrays expose one built-in property:

```galfus
values.length
```

`.length` is the only built-in array property.

Invalid core assumptions:

```galfus
values.push(10)
values.pop()
values.map(callback)
```

## 7.6 Byte Strings

A string literal is a UTF-8 byte array.

```galfus
var name: [uint8] = "Ana"
```

`[uint8]` may also contain arbitrary bytes.

```galfus
var bytes: [uint8] = [65, 66, 67]
```

There is no core `String` object.

## 7.7 Tuples

A tuple is positional and has at least two elements.

```galfus
var point = (10.0, 20.0)
```

A single parenthesized expression is not a tuple.

```galfus
var value = (10)
```

This is just `10`.

Choice payloads may contain one value, but that is choice payload syntax, not normal tuple syntax.

## 7.8 Structs

A struct is nominal and field-based.

```galfus
struct User {
  id: int64,
  name: [uint8],
}
```

Construction:

```galfus
var user = new(User) {
  id: 1,
  name: "Ana",
}
```

Structs are nominal. Two structs with the same fields are still distinct types.

## 7.9 Field Defaults

```galfus
struct User {
  id: int64,
  name: [uint8],
  age: int32 = 0,
}
```

Fields with defaults may be omitted.

Fields without defaults must be provided.

## 7.10 Const Fields

```galfus
struct User {
  const id: int64,
  name: [uint8],
}
```

`const` fields must be initialized during construction or have a valid default.

After construction, they cannot be reassigned.

## 7.11 Struct Construction Metadata

```galfus
var user = new(User, shared) {
  id: 1,
  name: "Ana",
}
```

`shared` is keyword metadata in construction position. It is not globally reserved.

## 7.12 Inferred Struct Construction

```galfus
var user: User = new {
  id: 1,
  name: "Ana",
}
```

This requires an expected struct type.

Invalid without expected type:

```galfus
var user = new {
  id: 1,
  name: "Ana",
}
```

## 7.13 Struct Shorthand and Spread

Shorthand:

```galfus
var id = 1
var name = "Ana"

var user = new(User) {
  id,
  name,
}
```

Struct literal spread is shallow-copy-like.

```galfus
var user2 = new(User) {
  ...user,
  name: "Bia",
}
```

Nested values are not deep-copied.

## 7.14 Struct Expansion

Struct expansion copies field declarations into a new struct declaration.

```galfus
struct Employee {
  ...User,
  role: [uint8],
}
```

This is not inheritance.

Conflicting expanded fields are semantic errors.

## 7.15 Enums

Enums are nominal discriminant types.

```galfus
enum Direction {
  North,
  East,
  South,
  West,
}
```

Explicit base type uses keyword metadata:

```galfus
enum(uint8) SmallKind {
  A(1),
  B(2),
}
```

Default base type is `int32`.

`enum<int64>` is not used.

Enum-to-integer conversion requires explicit cast.

## 7.16 Choices

A choice is a nominal tagged union.

```galfus
choice Result<V, E> {
  Ok(V),
  Err(E),
}
```

Choice payloads may have:

```txt
no value
one value
multiple values
```

```galfus
choice Asset {
  None,
  Texture([uint8]),
  Image([uint8], int32, int32),
}
```

## 7.17 Weak Fields

A field may be weak.

```galfus
struct Node {
  value: int32,
  weak parent: Node | null,
}
```

Weak fields must be nullable.

Invalid:

```galfus
struct Node {
  weak parent: Node,
}
```

## 7.18 Complex Assignment

Primitive scalar values are copied by value.

Complex values are shared by assignment.

```galfus
var user = new(User) {
  name: "Ana",
}

var other = user
```

`user` and `other` reference the same value graph.

## 7.19 Copy

`copy` performs explicit deep copy.

```galfus
var cloned = copy user
```

`copy` preserves topology for owning edges, validates weak fields, and respects resource policies.

Fieldless structs cannot be copied with `copy`.

```galfus
struct RuntimeToken {}

var token2 = copy token // invalid
```

A fieldless struct behaves like an opaque runtime identity or unique handle.

## 7.20 Contract

The checker MUST:

- Treat arrays as single-element-type sequences.
- Expose only `.length` as built-in array property.
- Type strings as `[uint8]`.
- Keep structs nominal.
- Reject invalid missing struct fields.
- Reject reassignment to `const` fields.
- Use `enum(T)` for enum base type metadata.
- Require weak fields to be nullable.
- Avoid implicit deep copy.
- Reject `copy` on fieldless structs.

---

Previous: [Constraints as Traits](./06-constraints-as-traits.md) | Index: [Galfus Core System](./00-index.md) | Next: [Expressions and Operators](./08-expressions-and-operators.md)
