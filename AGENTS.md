## Project Configuration

- **Languages**: Rust (Core), TypeScript / Bun (CI/CD and Automation Scripts)

# Instructions for Agents

## 1) Priority and Scope

- Follow these instructions in this order:
  1. Correctness and safety.
  2. User intent and task completion.
  3. Clarity and concision.
  4. Token efficiency.

## 2) Communication Rules

- Keep responses short and concise whenever possible.
- Use the user's language for conversation.
- Write all code, comments, commit messages, and technical documentation in English.
- Do not blindly approve proposals. Evaluate them logically, explain trade-offs, and suggest a better alternative when needed.

## 3) Pre-Implementation Self-Assessment

- Before generating or modifying any code, the agent must ask and answer these four questions:
  1. Is this code necessary?
  2. Does a function/utility already exist in the codebase that does this?
  3. Does a standard library or a well-maintained published library solve this problem?
  4. Are there any dependencies that need to be installed first?
- Ensure the final code generated is minimal, highly efficient, and focused solely on completing the task.

## 4) Planning Rules

- Always create a strategic plan before implementation.
- Plans must be brief and actionable (steps, risks, validation).
- If the user asks only for analysis, do not modify files or generate code.

## 5) Implementation Rules

- Prefer minimal, high-impact changes over broad refactors.
- Reuse existing utilities and patterns before adding new abstractions.
- Remove unused variables and unused functions introduced or discovered during the task.
- Target source file size around 300 lines; avoid exceeding 600 lines when splitting is practical (does not apply to `.md` documentation files).
- If files are changed externally while you work, treat those changes as intentional and do not revert them unless explicitly requested.
- Detect repetitive work and propose or create scripts to automate it.
- Files on .tmp/ are temporary and used for planning, prototyping, and testing. They can be modified or deleted without warning. These files are not version controlled and should not be used for permanent code or documentation.

## 6) Validation and Uncertainty Rules

- When uncertain about framework or API usage, consult official documentation before implementing.
- Before concluding, validate changes using the following commands:
  - **Rust**:
    - Build/Check: `cargo check`
    - Formatter: `cargo fmt --check`
    - Linter: `cargo clippy --workspace --all-targets`
    - Tests: `cargo test --workspace` (Note: running `cargo test` alone only runs CLI tests; always use `--workspace` to run all project tests)
  - **TypeScript/Bun**:
    - Execution/Validation: `bun run <script_path>`
- Surface blockers, assumptions, and risks early.

## 7) Project Rules

- Variables that hold ownership and are no longer used afterward must always receive the `_` prefix (e.g., `let _guard = lock.write();`).
- If variables are unused and do not hold ownership/side-effects, they must be removed.
- Unused functions must also be removed.
- Internal Rust properties use `snake_case`.
