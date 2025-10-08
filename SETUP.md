# Setup Instructions

These steps bring a fresh clone of `dws` into a working state. Run them from the repository root (`/Users/acarter/Developer/dws` on this machine).

## Quick Start

1. **Check the toolchain**
   ```bash
   rustup show active-toolchain
   cargo --version
   ```
   Ensure Rust ≥ 1.70 is installed (use `rustup update` if needed).

2. **Build the project**
   ```bash
   cargo build
   ```
   This compiles the workspace and fetches dependencies.

3. **Inspect CLI wiring**
   ```bash
   cargo run -- --help
   ```
   Confirm the command list matches the README.

4. **Run tests**
   ```bash
   cargo test
   ```
   Add `-- --include-ignored` when you are operating inside throwaway XDG directories (see `AGENTS.md`).

5. **Review core docs**
   - `README.md` — product overview
   - `AGENTS.md` — contributor guidelines (canonical for AI agents)
   - `ASSISTANT.md` — deep context and roadmap
   - `docs/architecture.md` — technical design details

6. **Update documentation if anything drifts**
   Keep `AGENTS.md` and `ASSISTANT.md` consistent whenever you introduce new workflows or commands.

## File Overview

- `src/main.rs` — binary entry point
- `src/lib.rs` — public API surface
- `src/cli.rs` — Clap-based CLI definitions
- `src/commands/` — subcommand implementations
- `src/config.rs`, `src/environment.rs`, `src/workspace.rs`, `src/lockfile.rs` — domain types
- `tests/cli_test.rs` — integration-style CLI checks (currently mostly ignored)
- `templates/` — embedded profile scaffolding

## Working Conventions

- Follow `AGENTS.md` for formatting, linting, and testing cadence.
- Document major design decisions in `ASSISTANT.md` or `docs/architecture.md`.
- Use temporary directories with explicit `XDG_CONFIG_HOME`/`XDG_STATE_HOME` when running experiments that could touch real dotfiles.
- Coordinate with maintainers before making destructive or backward-incompatible changes.

Ready to build! 🦀
