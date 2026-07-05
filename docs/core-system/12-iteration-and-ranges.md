Previous: [Control Flow](./11-control-flow.md) | Index: [Galfus Core System](./00-index.md) | Next: [Pattern Matching and Narrowing](./13-pattern-matching-and-narrowing.md)

---

# 12. Iteration and Ranges

This document defines `for`, named `for`, item/index bindings, compiler-known iteration concepts, arrays in iteration, and range literals.

## 12.1 Basic `for`

```galfus
for value in values {
  log(value)
}
```

With index:

```galfus
for value, index in values {
  log(index)
  log(value)
}
```

The first binding receives the item.

The second receives the zero-based index.

Item and index bindings are constant bindings.

```galfus
for value, index in values {
  value = 10 // invalid
  index = 1  // invalid
}
```

## 12.2 Wildcard in `for`

Ignore item:

```galfus
for _, index in values {
  log(index)
}
```

Ignore index:

```galfus
for value, _ in values {
  log(value)
}
```

## 12.3 Named `for`

```galfus
for(name: users) user in users {
  loop {
    break users
  }
}
```

`break users` exits the named `for`.

`continue users` advances it to the next iteration.

## 12.4 Source Evaluation

The source expression is evaluated once.

```galfus
for value in createRange() {
  log(value)
}
```

Conceptual behavior:

```txt
source = createRange()
iterator = source::iter()
repeat next()
```

## 12.5 Compiler-Known Iteration Concepts

`Range`, `Iterable`, and `Iterator` are known by the compiler for validation and lowering.

They are not globally imported.

If source code references them directly, it must import them.

```galfus
import { Range, Iterable, Iterator } from "std/constraints"
```

Compiler-known does not mean globally available.

## 12.6 Iterator and Iterable

Conceptual model:

```galfus
constraint Iterator<Item> {
  fn next(self): Item | null
}

constraint Iterable<Item, Iter> {
  fn iter(self): Iter
}
```

Iteration completes when `next()` returns `null`.

If nullable items are required, a future iterator shape may use a choice such as `Next<T>`.

## 12.7 Arrays in `for`

Arrays are iterable in index order.

```galfus
var values = [10, 20, 30]

for value, index in values {
  log(index)
  log(value)
}
```

Array iteration order:

```txt
0
1
2
...
length - 1
```

Mutating the iterated array structure during iteration SHOULD be rejected initially.

Mutating fields inside complex elements may be allowed if normal mutation rules allow it.

## 12.8 Range Literals

Supported literal range forms:

```txt
start..end
start::count
start::count%step
```

Examples:

```galfus
1..9
1::4
1::4%3
```

Range operands must be integer literals.

Invalid:

```galfus
a..b
start::count
1.5..10.5
```

Dynamic ranges should be built through explicit functions/modules if needed later.

## 12.9 Exclusive Range `start..end`

`start..end` is exclusive.

```galfus
1..4
```

Produces:

```txt
1
2
3
```

The end value is not included.

The range must produce at least one value.

Invalid:

```galfus
1..1
```

Descending ranges are valid if non-empty.

```galfus
4..1
```

Produces:

```txt
4
3
2
```

## 12.10 Quantity Range `start::count`

`start::count` produces `count` values.

```galfus
1::4
```

Produces:

```txt
1
2
3
4
```

`count` must be positive.

Invalid:

```galfus
1::0
1::-1
```

## 12.11 Quantity Range with Step

```galfus
1::4%3
```

Produces:

```txt
1
4
7
10
```

Step must not be zero.

Invalid:

```galfus
1::2%0
```

Negative step is valid.

```galfus
4::3%-1
```

Produces:

```txt
4
3
2
```

## 12.12 Range Allocation

Ranges do not materialize arrays.

They lower to lightweight iterable values or direct iteration state.

Two loops over the same range are independent.

## 12.13 Iterating Complex Values

Iteration does not deep-copy complex items.

```galfus
for user in users {
  user.name = "Updated"
}
```

The item binding is const, but the reachable graph may still be mutable if normal rules allow it.

Use `copy` for independent values.

## 12.14 Contract

The checker/lowering MUST:

- Evaluate `for` source once.
- Treat item and index bindings as const.
- Support named `for` targets.
- Keep `Range`, `Iterable`, and `Iterator` compiler-known but not globally imported.
- Require explicit imports when those names are used in source.
- Restrict literal ranges to integer literals.
- Reject `1..1`, `1::0`, `1::-1`, and `1::2%0`.
- Avoid implicit array materialization for ranges.
- Avoid implicit deep copy of iteration items.

---

Previous: [Control Flow](./11-control-flow.md) | Index: [Galfus Core System](./00-index.md) | Next: [Pattern Matching and Narrowing](./13-pattern-matching-and-narrowing.md)
