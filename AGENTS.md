# Repository Guidelines

## Use This First
This file is the canonical quickstart for coding assistants (Claude, Codex, GitHub Copilot CLI). Read it together with `README.md` for product context and `ASSISTANT.md` for deep background. When in doubt, prefer updating this document so every agent receives consistent guidance.

## Project Structure & Module Organization
The binary entry point is `src/main.rs`, which calls into the public API exposed by `src/lib.rs`. CLI argument parsing lives in `src/cli.rs`; each subcommand has a focused module inside `src/commands/`. Core domain types are defined in `src/config.rs`, `src/environment.rs`, `src/workspace.rs`, `src/manifest.rs`, and `src/lockfile.rs`. Integration tests live in `tests/cli_test.rs` (extend with additional files under `tests/`). Long-form documentation belongs in `docs/`, while starter profile assets reside in `templates/`. Keep user-facing guidance in `README.md`, `SETUP.md`, and this file. Manifests follow config-style precedence: base (`tools.toml`) → platform (e.g. `tools-macos.toml`) → host-specific overrides (`tools-<hostname>.toml`).

## Build, Test, and Development Commands
- `cargo fmt` — Run before commits to enforce formatting.
- `cargo clippy --all-targets --all-features` — Lint the project; fix or explicitly `allow` warnings.
- `cargo build` — Compile for a quick regression check.
- `cargo test` — Execute unit and integration tests.
- `cargo test -- --include-ignored` — Run the CLI tests that require isolated XDG directories.
- `cargo run -- --help` — Verify CLI wiring and user-facing help text.

## Coding Style & Naming Conventions
Use Rust 2021 idioms: four-space indentation, `snake_case` for items, `CamelCase` for types, and `SCREAMING_SNAKE_CASE` for constants. Keep modules cohesive; colocate unit tests with implementations (`#[cfg(test)]` blocks). Prefer descriptive error types over generic `anyhow::Error`. Avoid `unsafe` unless approved. Document public APIs with rustdoc comments that explain invariants or side effects.

## Testing Guidelines
Unit tests should isolate logic with temporary directories (`tempfile::TempDir`) and avoid the real filesystem. Integration tests under `tests/` use `assert_cmd`, `serial_test`, and explicit `XDG_CONFIG_HOME` / `XDG_STATE_HOME` overrides. Keep ignored tests as living examples; only un-ignore them once the underlying behavior is stable and safe for local runs. Add a regression test whenever you fix a bug or add functionality that manipulates the workspace filesystem.

## Commit & Pull Request Guidelines
Use imperative, sentence-case subjects (e.g., “Add manifest parser”). Group related changes and keep commits reviewable. PR descriptions should explain the scenario, list key changes, and note validation steps (`cargo test`, manual commands). Link issues or design docs when relevant, and capture user-visible updates in `README.md` or `docs/architecture.md` as part of the same change.

## Reference Map
- `README.md` — Product overview and basic usage.
- `SETUP.md` — Local bootstrap checklist.
- `ASSISTANT.md` — Deep context, roadmap, and architectural intent.
- `docs/architecture.md` — Technical design details for the workspace.
- `CLAUDE.md`, `CODEX.md`, `COPILOT.md` — Agent-specific entry points that point back here.
