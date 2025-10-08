# CLAUDE.md

This project uses standard Rust tooling and documentation.

## Primary References
- [`README.md`](README.md): overview, goals, and build instructions  
- [`CONTRIBUTING.md`](CONTRIBUTING.md): contribution workflow and coding standards  
- [`ARCHITECTURE.md`](ARCHITECTURE.md): module and crate structure  
- [`STYLEGUIDE.md`](STYLEGUIDE.md): formatting, naming, and safety conventions  
- [`AGENTS.md`](AGENTS.md): AI Agent instructions

## AI Agent Context
- Language: Rust (edition 2024)
- Tooling: Cargo, rust-analyzer
- Preferred completion focus:
  - Show working, idiomatic code
  - Use `cargo fmt` / `clippy` compliance
  - Avoid unsafe unless explicitly allowed
- Key context files: `Cargo.toml`, `src/main.rs`, `src/lib.rs`

---

_This file is intended for coding assistants (Claude, Copilot, Codex) to locate relevant project documentation._
- Always run clippy/format before you commit