Previous: [Bindings, Literals and Primitive Types](./03-bindings-literals-and-primitive-types.md) | Index: [Galfus Core System](./00-index.md) | Next: [Generics](./05-generics.md)

---

# 4. Type System, Inference and Propagation

This document defines Galfus type categories, inference, expected type propagation, unions, aliases, casts, and assignability.

## 4.1 Type Categories

Core type categories:

```txt
primitive scalar
null
named type
path type
runtime-sized array
tuple
function
union
type alias
generic type
constraint facade type
```

There is no core `any`, `unknown`, `void`, `String`, `str`, or `char`.

## 4.2 Named and Path Types

Named types resolve through local declarations, imports, and module surfaces.

```galfus
User
math::Vector2
```

Type names are case-sensitive.

## 4.3 Arrays

Array type:

```galfus
[int32]
[User]
```

String literals are `[uint8]`.

## 4.4 Tuples

Tuple types are positional.

```galfus
(int32, int32)
([uint8], int32)
```

A normal tuple has at least two elements.

`(int32)` is a grouped type, not a tuple.

## 4.5 Function Types

Function types use `fn(...): Return`.

```galfus
fn(int32, int32): int32
fn([uint8]): null
```

Function types are invariant by default.

## 4.6 Unions and Nullable Types

A union uses `|`.

```galfus
int32 | int64
User | null
```

Nullable type is written as `T | null`.

There is no `T?` syntax.

Union normalization MUST flatten nested unions and remove duplicates.

The compiler SHOULD preserve aliases for diagnostics where useful.

## 4.7 Type Aliases

Type aliases are transparent for assignability but preserve their symbol for diagnostics and tooling.

```galfus
type UserId = int64

var id: UserId = 10
```

## 4.8 Type Inference

A binding with an initializer can infer its type.

```galfus
var count = 10
var name = "Ana"
```

A plain `null` initializer needs expected type.

Invalid:

```galfus
var value = null
```

Valid:

```galfus
var value: User | null = null
```

## 4.9 Expected Type Propagation

Expected types come from:

```txt
binding annotations
function return type
assignment target
function parameter type
array context
tuple context
match result context
instanceof result context
explicit cast
```

Example:

```galfus
var value: int64 = 10
```

The literal `10` is typed as `int64`.

## 4.10 Bottom-Up Local Typing

Expressions are typed from their local structure first.

```galfus
var small: int16 = 2
var result = 10 + small
```

The literal `10` may refine to `int16` if it fits, and the result may be `int16`.

An outer expected type may then widen the final expression.

## 4.11 Numeric Compatibility

Numeric compatibility rules:

```txt
same numeric type -> valid
literal fitting expected type -> valid
safe widening -> valid
narrowing non-literal -> explicit cast required
integer-to-float non-literal -> explicit cast unless defined otherwise
float-to-integer non-literal -> explicit cast
```

Invalid without explicit cast:

```galfus
var wide: int64 = 1000
var small: int8 = wide
```

Valid with explicit cast if allowed:

```galfus
var small = <int8> wide
```

## 4.12 Explicit Casts

Explicit cast syntax:

```galfus
<int8> value
<bool> count
```

Casts request conversion to a target type.

Checked conversion helpers may be provided by modules, but they are not the core cast syntax.

## 4.13 Boolean Casts

Boolean contexts conceptually apply `<bool>`.

```galfus
if count {
  run()
}
```

Conceptual:

```galfus
if <bool> count {
  run()
}
```

Recommended core behavior:

```txt
null -> false
false -> false
true -> true
0 -> false
nonzero -> true
```

Aggregates SHOULD NOT cast to bool implicitly.

## 4.14 Assignability

Assignability may accept:

```txt
exact type match
union containment
alias-compatible type
literal refinement
safe numeric widening
contextual cast
constraint facade target
```

Invalid assignments should report expected type, actual type, span, reason, and possible explicit cast.

## 4.15 `instanceof`, `typeof`, and Type Sets

`instanceof` is the value narrowing expression.

It can narrow only through statically known possible type sets.

For generics, this means `instanceof` is valid only when the generic bound defines a known possible runtime set.

```galfus
fn render<T: int32 | [uint8] | bool>(value: T): [uint8] {
  return instanceof value {
    int32 number => "number",
    [uint8] text => text,
    _ => "other",
  }
}
```

`typeof` dispatches over a type expression instead of a value expression.

```galfus
fn parse<T: int32 | [uint8] | bool>(text: [uint8]): T | null {
  return typeof T {
    int32 => parseInt32(text),
    [uint8] => text,
    bool => parseBool(text),
  }
}
```

`typeof` arms are type arms. A bare identifier arm names a type; it does not create a binding.

When the input type has a bounded known type set, `typeof` must cover every possible member or provide a final `_` wildcard arm.

When the input type is an unconstrained generic, `typeof` cannot prove a closed set and must provide a final `_` wildcard arm.

Inside each non-wildcard arm, the checker specializes the input generic to the matched arm type for expected type propagation.

## 4.16 Contract

The checker MUST:

- Reject unknown types.
- Reject implicit `any` or `unknown` behavior.
- Normalize unions deterministically.
- Preserve aliases for diagnostics/tooling where useful.
- Use expected type propagation for literals and expression contexts.
- Reject impossible narrowing arms.
- Require explicit casts for narrowing conversions.
- Reject implicit aggregate-to-bool unless explicitly defined later.

---

Previous: [Bindings, Literals and Primitive Types](./03-bindings-literals-and-primitive-types.md) | Index: [Galfus Core System](./00-index.md) | Next: [Generics](./05-generics.md)
