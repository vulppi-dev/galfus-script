# Galfus semantic rules

## Authority and phase order

Implemented behavior flows through lexing, parsing, name/module resolution, type binding/inference, semantic and ownership validation, MIR construction, bytecode lowering, graph assembly, and VM execution. The frontend tests and the relevant Rust implementation decide behavior when prose differs.

## Typing

Galfus is statically typed. Expected types propagate from annotations, assignments, parameters, returns, arrays, patterns, and casts. Numeric literals default to `i32` and floats to `f32` unless context refines them. Safe widening may work; narrowing non-literal values needs an explicit cast. Normalize unions deterministically. Aliases are assignable transparently but should remain useful in diagnostics.

Generics are static and invariant. Infer them from arguments/expected result or provide every argument explicitly. Do not introduce dynamic `any`/`unknown`. `instanceof` on a generic needs a statically known runtime type set; `typeof` needs exhaustive arms for a closed bound or final `_` otherwise.

## Ownership and mutation

Scalars copy by value. Structs, arrays, tuples, payload choices, byte arrays, facades, and resources share ownership graphs on assignment and parameter passing. `copy` duplicates owning topology explicitly; it does not promote weak references and is rejected for fieldless/otherwise non-copyable values.

Bindings create anchors; fields/elements/payloads create owning edges; `weak` fields require nullable types and become null when their target is released. Release is deterministic at safe points such as reassignment, block end, and function return. A `const` binding does not make a mutable reachable graph immutable.

Reads beyond an array bound produce `null`; writes beyond a bound are runtime errors. Negative indexes count from the end. Array `.length` is its built-in property.

## Patterns, constraints, and lowering

`match` matches values and data shape; `instanceof` narrows values by type; `typeof` dispatches on a type. Pattern bindings share the matched graph and are const. Non-wildcard unreachable arms are warnings; a wildcard before the final arm is an error.

Constraints specify fields/functions, not operator overloads. Facade values only expose their declared constraint surface. Compiler-known constraint concepts still need imports when referenced by source.

Lowering must not add hidden dynamic types, imports, copies, return inference, or operator behavior. Bytecode is an in-memory `BytecodeGraph`; runtime state is per module and initializes dependencies before the entry module.
