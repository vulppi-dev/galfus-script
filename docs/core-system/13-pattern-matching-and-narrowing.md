Previous: [Iteration and Ranges](./12-iteration-and-ranges.md) | Index: [Galfus Core System](./00-index.md) | Next: [Decorators and Keyword Metadata](./14-decorators-and-keyword-metadata.md)

---

# 13. Pattern Matching and Narrowing

This document defines `match`, `instanceof`, patterns, wildcard rules, exhaustiveness, narrowing, pattern bindings, and unreachable pattern diagnostics.

## 13.1 Core Principle

Galfus has one narrowing expression:

```txt
instanceof
```

`match` handles value/data-pattern matching.

`instanceof` handles type, null, union, generic known-set, nominal, and facade narrowing.

## 13.2 Match

```galfus
var label = match value {
  0 => "zero",
  1 => "one",
  _ => "many",
}
```

`match` is value-producing and must be exhaustive.

All arms must produce compatible result types.

## 13.3 Wildcard Arm Position

If `_` appears in an arm list, it must be the final arm.

Valid:

```galfus
match value {
  0 => "zero",
  _ => "other",
}
```

Invalid:

```galfus
match value {
  _ => "other",
  0 => "zero",
}
```

Wildcard before the final arm is an error, not a warning.

The same rule applies to `instanceof`.

## 13.4 Pattern Bindings Are Const

Pattern bindings inside `match` and `instanceof` are constant by default.

```galfus
match result {
  Result::Ok(value) => {
    value = 10 // invalid
    value
  },
  Result::Err(error) => 0,
}
```

Complex pattern bindings share the matched graph. They are not deep-copied.

## 13.5 Literal Patterns

```galfus
match count {
  0 => "zero",
  1 => "one",
  _ => "many",
}
```

Supported literal patterns:

```txt
integer
float
bool
null
string as [uint8]
```

## 13.6 Enum Patterns

```galfus
var label = match direction {
  Direction::North => "north",
  Direction::East => "east",
  Direction::South => "south",
  Direction::West => "west",
}
```

All variants must be handled unless `_` is present.

## 13.7 Choice Patterns

```galfus
var value = match result {
  Result::Ok(value) => value,
  Result::Err(error) => 0,
}
```

No-payload variants are matched without parentheses.

```galfus
Maybe::None => 0
```

Payload arity must match the variant declaration.

## 13.8 Tuple and Array Patterns

Tuple patterns:

```galfus
match point {
  (0, 0) => "origin",
  (x, y) => "point",
}
```

Array patterns are initially recommended for fixed-size arrays.

```galfus
match pair {
  [first, second] => first + second,
}
```

Runtime-sized array destructuring can be added later if needed.

## 13.9 Struct Patterns

```galfus
match user {
  User { id, name } => name,
}
```

Omitted fields are ignored.

```galfus
match user {
  User { name } => name,
}
```

Struct destructuring does not use `_` for omitted fields.

Field rename:

```galfus
match user {
  User { name: displayName } => displayName,
}
```

## 13.10 `instanceof`

`instanceof` is the single narrowing expression.

It handles:

```txt
primitive narrowing
null narrowing
union narrowing
generic narrowing with known type set
nominal struct narrowing
facade/concrete narrowing
```

Example:

```galfus
var size = instanceof value {
  int32 number => number,
  [uint8] text => text.length,
  null => 0,
}
```

Nullable narrowing:

```galfus
var name = instanceof user {
  User value => value.name,
  null => "missing",
}
```

Facade narrowing:

```galfus
var label = instanceof item {
  Button button => button.label,
  Icon icon => icon.name,
  _ => "unknown",
}
```

## 13.11 `instanceof` and Generics

Valid only when possible runtime type set is statically known.

```galfus
fn render<T: int32 | [uint8] | bool>(value: T): [uint8] {
  return instanceof value {
    int32 number => "number",
    [uint8] text => text,
    _ => "other",
  }
}
```

Invalid over unconstrained `T`.

```galfus
fn render<T>(value: T): [uint8] {
  return instanceof value {
    int32 number => "number",
    _ => "other",
  }
}
```

## 13.12 Destructuring Bindings

Patterns may be used in destructuring bindings.

```galfus
var (x, y) = point
var [first, second] = pair
var User { name } = user
```

Outer binding mutability controls created bindings.

```galfus
var (x, y) = point
const (a, b) = point
```

This differs from match/instanceof bindings, which are const by default.

## 13.13 No Pattern Guards

Pattern guards are not part of Galfus.

Invalid:

```galfus
match value {
  x if x > 10 => "large",
  _ => "small",
}
```

Custom conditional matching should be expressed through explicit comparable behavior or normal code inside an arm.

## 13.14 Comparable Matching

For custom value comparison patterns, the type must provide deterministic comparable behavior.

If source references `Comparable`, it must import it.

```galfus
import { Comparable } from "std/constraints"
```

Operators are still not overloaded.

## 13.15 Unreachable Patterns

Unreachable non-wildcard patterns are warnings.

```galfus
match value {
  0 => "zero",
  0 => "again",
  _ => "other",
}
```

The second `0` is unreachable.

Wildcard before the final arm is an error.

## 13.16 Contract

The checker MUST:

- Use `instanceof` for all narrowing.
- Require `match` and `instanceof` expressions to be exhaustive.
- Require `_` to be the final arm when present.
- Treat wildcard-before-final as error.
- Treat unreachable non-wildcard patterns as warnings.
- Treat pattern bindings in arms as const.
- Reject pattern guards.
- Reject impossible type/narrowing arms.
- Avoid implicit deep copy during pattern binding.

---

Previous: [Iteration and Ranges](./12-iteration-and-ranges.md) | Index: [Galfus Core System](./00-index.md) | Next: [Decorators and Keyword Metadata](./14-decorators-and-keyword-metadata.md)
