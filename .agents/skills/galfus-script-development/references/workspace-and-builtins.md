# Workspace, builtins, and verification

## Workspace

`galfus.toml` declares `[module]` with `name`, `target` (`app` or `lib`), and usually `entry = "src/main.gfs"`. `[run]` may select the exported entry name and arguments. A workspace host loads config and source, then calls `check`, `compile`, and `run`; do not bypass this facade for normal host integration.

The workspace keeps source, semantic, and bytecode graph snapshots. It loads only reachable source modules from configured roots and resolves builtins on demand. Failed checks/compiles leave the prior graph snapshot intact.

```toml
[module]
name = "example"
target = "app"
entry = "src/main.gfs"

[run]
entry = "main"
args = []
```

## Current builtin modules

Read the `.gfs` source before assuming a signature.

| Import | Source | Main surface |
| --- | --- | --- |
| `std/io` | `rich_builtins/io.gfs` | `read`, `print`, `println` |
| `std/constraints` | `constraints.gfs` | `Iterator`, `Iterable`, `Comparable` |
| `std/iterable` | `iterable.gfs` | iterable helpers |
| `std/thread` | `thread.gfs` | virtual thread APIs |
| `text` | `text.gfs` | byte-string helpers |
| `format` | `format.gfs` | parsing/formatting |
| `format/ansi` | `format/ansi.gfs` | ANSI styling |

The host provides native capability implementations. A program remains compilable without a provider; it fails only if it executes the native builtin requiring that provider. Do not expose or call `__builtin_*` names from user code.

## Commands

From the repository root:

```bash
cargo run -- check <workspace-directory>
cargo run -- run <workspace-directory>
cargo test -p galfus-frontend
cargo test -p galfus-workspace
cargo test --workspace
cargo fmt --check
```

Choose the narrowest command that covers a change. For parser/type semantics, add or update only the relevant frontend test if behavior intentionally changes. For Rust changes, preserve the repository's source-order and import rules in `AGENTS.md`.
