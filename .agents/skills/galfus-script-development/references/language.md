# Galfus syntax reference

Use this as a compact authoring guide. Confirm edge cases in `docs/core-system/` and frontend tests.

## Modules and declarations

Each `.gfs` source is a private module by default. Export declarations explicitly. Use `import name from "path"` for a module or `import { Name, function } from "path"` for named bindings. Paths use `::`; fields use `.` and null-safe fields use `?.`.

```galfus
import { println } from "std/io"
import text from "text"

export fn main(args: [[u8]]): i32 {
  println(text::trim(args[0]))
  return 0
}
```

Supported top-level forms are `import`, `export`, `var`, `const`, `type`, `struct`, `enum`, `choice`, `constraint`, and `fn`. Top-level control flow is invalid. Identifiers are case-sensitive; use PascalCase for type-like names and camelCase for values. `_` is a wildcard, never a normal readable binding.

## Types and bindings

Primitive types are `bool`, `null`, signed/unsigned `i8..i128` and `u8..u128`, and `f32`/`f64`. Strings are UTF-8 byte arrays: `[u8]`. Other core shapes are arrays (`[T]`), tuples (`(A, B)`), function types (`fn(A): B`), unions (`A | B`), aliases, named types, generic instances, and constraints.

`var` is mutable; `const`, parameters, iteration bindings, and pattern-arm bindings are not rebindable. All functions declare `: ReturnType`. Use `T | null` for nullable values, not `T?`. Empty arrays require contextual type.

```galfus
type UserId = i64
var ids: [UserId] = []
const name: [u8] = "Ana"
var current: User | null = null
```

## Data and functions

```galfus
struct User {
  const id: i64,
  name: [u8],
  weak parent: User | null,
}

choice Result<T, E> { Ok(T), Err(E) }
enum(i16) Direction { North, South }

fn User::rename(self, name: [u8]): User {
  self.name = name
  return self
}
```

Construct structs with `new(Type) { field: value }`, arrays with `new([T], length)`, and expected-type struct literals with `new { ... }`. Generic call arguments are `call<T>(value)` and must be complete when present. Anchor calls are `user::rename("Bia")`; they do not write back automatically. Arrow functions use `(x: i32): i32 => x * 2`.

## Expressions and control flow

Operators are fixed; no overloading and `+` is not text concatenation. Use `<T> value` for casts, `copy value` for explicit deep copy, and `??` / `??=` for null fallback. Assignment is statement-only.

`if` is statement-only. General loops use `loop`, `loop condition`, or `loop(after) condition`; use `for item, index in iterable`. Ranges are literal forms: `start..end`, `start::count`, and `start::count%step`.

```galfus
var total = 0
for value in 1..4 {
  total += value
}
```

Use value patterns with `match`, narrowing with `instanceof value`, and type dispatch with `typeof T`. Arms use `=>`, are comma-separated, produce a common type, and require exhaustiveness. A wildcard `_` must be last.

```galfus
return instanceof value {
  i32 number => number,
  [u8] text => text.length,
  null => 0,
}
```

## Declarative features

`struct T satisfies Constraint` declares a required surface. Constraints can also bound a generic (`T: Drawable`) or form a facade. Import builtin constraint names before naming them. Decorators (`@trace`) are typed transforms; keyword metadata is syntax-specific, such as `fn(stamp)`, `loop(after)`, `loop(name: label)`, `for(name: label)`, `new(T, shared)`, and `enum(i8)`.

For the full contract, read `docs/core-system/01-...` through `15-...` in the target repository.
