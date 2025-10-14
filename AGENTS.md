# Developer Workspace Agent Handbook

This document is the single source of truth for AI and human collaborators who are acting as “agents” while working on **dws**. Pair it with `README.md` for user-facing instructions and `docs/architecture.md` for deep technical design.

## Project Overview

- **Mission**: ship a lightweight, portable bootstrapper that turns a fresh POSIX machine into a ready-to-code workspace using declarative `dws.toml` manifests (with workspace overrides in `config.toml`).
- **License**: MIT &nbsp;|&nbsp; **Primary language**: Rust 2021
- **Repository**: `https://github.com/ascarter/dws`
- Everything lives inside the XDG directory hierarchy so uninstalling the tool simply removes `$XDG_CONFIG_HOME/dws`, `$XDG_STATE_HOME/dws`, and `$XDG_CACHE_HOME/dws`.

### Core Goals
1. Bootstrap laptops quickly with a single binary.
2. Remain XDG compliant and self-contained.
3. Favor native tooling (rustup, uv, fnm, etc.) over shims.
4. Track dotfiles and profile `dws.toml` tool definitions in version control.
5. Support multiple “profiles” so users can switch between contexts (personal, client, project).

### Non-goals
- Runtime version switching (not a direnv/mise replacement).
- CI/production orchestration.
- Wrapper shells or shim-based PATH hacks.

## Fast Start Checklist

1. **Read the task** and skim the relevant sections of this handbook.
2. **Inspect code**: stick to the files mentioned in the instructions; review `docs/architecture.md` if you’re touching foundational behavior.
3. **Implement** changes with Rust 2021 idioms and minimal surprises.
4. **Validate** with `cargo fmt`, `cargo clippy --all-targets --all-features`, and the appropriate `cargo test` invocation (`-- --include-ignored` when working with isolated XDG dirs).
5. **Document updates** in this file or `README.md` whenever processes change.
6. **Summarize** work clearly before handing the task back.

## Repository Map

- `src/main.rs` – Binary entry point.
- `src/lib.rs` – Public API surface exposing the core types.
- `src/cli.rs` – Clap configuration for every `dws` subcommand.
- `src/commands/` – One module per subcommand (`init`, `clone`, `use`, `profiles`, etc.).
- `src/config.rs` – Workspace-level settings persisted to `config.toml` (active profile + overrides).
- `src/dotfiles.rs` – Discovers, installs, and removes profile-managed symlinks.
- `src/environment.rs` – Emits shell environment exports.
- `src/workspace.rs` – High-level orchestration: directory layout, profile management, installers.
- `src/toolset.rs`, `src/lockfile.rs`, `src/installers/` – Tool resolution and installer backends.
- `templates/` – Embedded starter profile (copied during `dws init`).
- `tests/` – Integration tests (top-level harness in `tests/cli_test.rs`).
- `docs/architecture.md` – Long-form technical design.

## Build, Test, and Development Commands

- `cargo fmt` – Required before every commit.
- `cargo clippy --all-targets --all-features` – Keep lint output clean; add targeted `allow` only with justification.
- `cargo build` – Quick sanity compile.
- `cargo test` – Run unit + integration suites.
- `cargo test -- --include-ignored` – Executes CLI tests that assume isolated XDG directories.
- `cargo run -- --help` – Confirm CLI wiring and help text.

## Coding Style & Naming Conventions

- Uphold Rust 2021 idioms: four-space indentation, `snake_case` for items, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- Keep modules cohesive and colocate unit tests with implementations (`#[cfg(test)]`).
- Prefer expressive error types over `anyhow::Error` when feasible.
- Avoid `unsafe` unless explicitly approved.
- Document public APIs with rustdoc comments that cover invariants and side effects.
- Keep `src/commands/*` modules wafer-thin: parse CLI flags and call into `Workspace`/library code. Business logic lives in the library just like the Go `cmd` pattern.

## Testing Guidelines

- Unit tests should isolate logic using `tempfile::TempDir`; never mutate a contributor’s real filesystem.
- Integration tests under `tests/` rely on `assert_cmd`, `serial_test`, and explicit `XDG_*` overrides.
- Ignored tests are living documentation—only unignore when behavior is stable and safe for local runs.
- Add regression tests whenever you fix a bug or introduce new behavior around workspace filesystem management.

## Workflow & Collaboration Norms

- Commit subjects use imperative, sentence-case phrasing (e.g., “Add tool resolver”).
- Keep commits reviewable; group related changes logically.
- PR descriptions should explain the scenario, list key changes, and record validation (commands run, manual steps taken).
- Surface user-visible updates by adjusting `README.md` or `docs/architecture.md` as part of the same change.
- When documentation conflicts arise, resolve them here first so every agent sees the same guidance.

## Architecture In Brief

```
Workspace                    // Root context (~/.config/dws)
  └─ Profile                 // Named context stored under profiles/<name>
       ├─ Dotfiles           // Symlink installation into XDG config dirs
       ├─ Environment        // Shell-specific exports
       └─ ToolSet            // Tool definitions with profile/workspace overrides
```

- The active profile is persisted in `$XDG_CONFIG_HOME/dws/config.toml`.
- Profile repositories live under `$XDG_CONFIG_HOME/dws/profiles/<profile>`.
- Tool precedence: profile `dws.toml` defines the baseline, and `$XDG_CONFIG_HOME/dws/config.toml` can add or replace entire tool entries that match the current platform/host filters.
- Template scaffolding seeds a `default` profile during `dws init`.
- `Dotfiles` installs symlinks from `<profile>/config/**` into the target XDG directory, respecting `.dwsignore`.
- `Lockfile` captures installed symlinks and tools to support idempotent reinstall/update flows.

## Profiles & Commands

- `dws init [repo] [--profile name]` – Create or update the active profile; clones the repo into `profiles/<slug>` when URL provided.
- `dws clone <repo>` – Clone an additional profile without activating it.
- `dws use <profile>` – Switch the active profile, reinstalling symlinks based on the new configuration.
- `dws profiles` – List all profiles, marking the active one.
- `dws self uninstall` – Removes installations but intentionally leaves `profiles/` intact so the user can keep their repositories.

## Prior Art & Inspiration

- Original shell prototype (private repo) demonstrated manifest-driven installs, platform separation, and XDG compliance. Rust implementation borrows the data model but improves type safety, performance, and parallelism opportunity.
- Key learnings carried over: declarative configuration files, dedicated installers per backend, strong defaults, and explicit health checks.
- Improvements targeted in Rust: richer error handling, better progress output, parallel installs, and first-class profile switching.

## Reference Map

- `README.md` – Product overview, CLI usage, and developer setup checklist.
- `docs/architecture.md` – Detailed design and system diagrams.
- `CLAUDE.md` – Lightweight pointer for Claude agents (kept for compatibility); redirects here.
- Issue tracker / discussions – Use repository issues to capture open questions or future work.

Keep this handbook up to date whenever workflows, commands, or architecture expectations change. If information from other docs goes stale, migrate the authoritative version here first and then adjust the supporting references.
