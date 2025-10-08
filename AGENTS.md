# Repository Guidelines

## Project Structure & Module Organization
The crate entry point lives in `src/main.rs`, which hands off to the public API in `src/lib.rs`. CLI definitions are concentrated in `src/cli.rs`, while individual subcommands stay under `src/commands/` (each command receives its own module such as `sync.rs` or `env.rs`). Cross-cutting domain logic is grouped into dedicated modules (`src/config.rs`, `src/environment.rs`, `src/workspace.rs`, `src/lockfile.rs`). Integration tests reside in `tests/cli_test.rs`, and any future fixtures should accompany them in `tests/`. Documentation belongs in `docs/` (see `docs/architecture.md`), and reusable starter assets are under `templates/`. Maintain project metadata in `Cargo.toml` and keep user-facing notes in `README.md` and `ASSISTANT.md`.

## Build, Test, and Development Commands
- `cargo fmt` — Format the codebase before committing.
- `cargo clippy --all-targets --all-features` — Run the linter and address warnings or add targeted `allow` attributes.
- `cargo build` — Compile the project for quick regression checks.
- `cargo test` — Execute the full test suite (unit + current integration tests).
- `cargo test -- --include-ignored` — Opt-in to the ignored CLI integration tests when working in a temp workspace with `XDG_CONFIG_HOME`/`XDG_STATE_HOME` set.
- `cargo run -- --help` — Verify CLI wiring and command documentation.

## Coding Style & Naming Conventions
Follow standard Rust 2021 idioms: four-space indentation, `snake_case` for modules/functions, `CamelCase` for types, and `SCREAMING_SNAKE_CASE` for constants. Keep modules cohesive by colocating tests with their implementation when practical. Prefer expressive error types over `anyhow::Error`. Run `cargo fmt` and `cargo clippy` locally before pushing, and capture invariants with doc comments where APIs are public.

## Testing Guidelines
Unit tests should live alongside their modules using Rust’s `#[cfg(test)]` blocks. Integration tests belong in `tests/` and currently rely on `assert_cmd`, `serial_test`, and `tempfile` to isolate CLI behavior. Ignored tests demonstrate the expected use of `XDG_CONFIG_HOME` and `XDG_STATE_HOME`; un-ignore them only when the commands are safe for local state. Add regression tests for every bug fix, and ensure new commands expose at least one CLI-level assertion of their help text or success path. Aim to leave no ignored tests behind when shipping features tied to their scenarios.

## Commit & Pull Request Guidelines
Commits follow the existing history: short imperative subjects in sentence case (e.g., “Add manifest parser”). Group related changes and avoid mixed concerns. Each pull request should describe the behavior change, reference any relevant issues, and note how it was validated (commands run, tests added). Include screenshots or terminal excerpts for user-visible output when helpful, and double-check that documentation (`README.md`, `docs/`, `ASSISTANT.md`) stays in sync. Request review once CI is green and lint/test commands have succeeded locally.

## Agent Workflow Notes
Before implementing features, skim `ASSISTANT.md` and `docs/architecture.md` to confirm design intent. When manipulating the developer environment, prefer temporary directories and explicitly set `XDG_*` paths to avoid mutating the host config. Record notable decisions in documentation files so future agents have traceable context.
