---
name: galfus-script-development
description: Implement, review, debug, or explain Galfus Script (`.gfs`) source and its language-facing compiler behavior. Use when a task involves Galfus syntax, types, modules, builtins, ownership, patterns, workspace configuration, or the Rust frontend/compiler/runtime that implements those semantics.
---

# Galfus Script development

Treat the repository as the authority for the currently implemented language. Use `docs/core-system/` as the normative design reference, then confirm the relevant rule in the parser, type validator, or tests before changing behavior. Documentation and implementation can be at different stages.

## Start

1. Identify whether the task changes `.gfs` user code, compiler behavior, a builtin, workspace behavior, or VM behavior.
2. Read the matching reference below before proposing syntax or semantics.
3. Inspect the nearest parser/type-validation test for implemented behavior. Do not implement a documented future capability merely because it appears in a design document.
4. Make the smallest change consistent with existing diagnostics and lowering.
5. Validate with the narrowest relevant command, then run `cargo fmt --check`.

## Reference routing

- Read [language.md](references/language.md) for `.gfs` syntax, declarations, types, data, calls, operators, flow, patterns, and ranges.
- Read [semantics.md](references/semantics.md) for ownership, static typing, generics, constraints, lowering, and runtime guarantees.
- Read [workspace-and-builtins.md](references/workspace-and-builtins.md) for imports, supplied modules, `galfus.toml`, the execution pipeline, and validation commands.

## Implementation map

- Lexer/tokens: `crates/galfus-frontend/src/lexer/` and `tokens/kind.rs`.
- Grammar: `crates/galfus-frontend/src/parser/`.
- Names/modules: `crates/galfus-frontend/src/resolver/` and `modules/`.
- Types and semantics: `crates/galfus-frontend/src/type_validation/`.
- Semantic tests: `crates/galfus-frontend/src/{parser,type_validation,resolver}/tests/`.
- Lowering: `crates/galfus-ir/src/{builder,lower}/`.
- Runtime behavior: `crates/galfus-vm/src/runtime/` and its tests.
- Workspace facade/config: `crates/galfus-workspace/src/`.
- Actual builtin API: `crates/galfus-builtins/rich_builtins/` and `crates/galfus-builtins/src/lib.rs`.

## Guardrails

- Preserve explicit imports, exports, and function return annotations.
- Never assume `String`, `any`, `unknown`, `void`, implicit deep copies, or operator overloading exist; strings are `[u8]`.
- Treat complex assignment as graph sharing; use `copy` only for an explicit deep copy.
- Keep `match`, `instanceof`, and `typeof` exhaustive as required by the actual validator. Keep `_` final in an arm list.
- Prefer existing rich builtins over inventing host behavior. `__builtin_*` identifiers are compiler-trusted internals, not user-facing APIs.
- Resolve a mismatch by priority: parser/type validator/tests, then active builtin source, then `docs/core-system/`, then other documentation.

## Validation

For a workspace project, use `cargo run -- check <workspace>`. For an example or runnable app, use `cargo run -- run <workspace>`. For Rust changes, run the smallest relevant test target; use `cargo test --workspace` only when the scope warrants it. Always run `cargo fmt --check` after Rust edits.
