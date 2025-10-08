# Copilot CLI Guide

GitHub Copilot CLI users should rely on [`AGENTS.md`](AGENTS.md) for end-to-end contributor guidance. It references `README.md`, `SETUP.md`, and `ASSISTANT.md` for background; consult those as needed.

Recommended command flow:
1. `cat AGENTS.md` — confirm coding standards and required commands.
2. `cargo fmt` and `cargo clippy --all-targets --all-features` — ensure patches meet lint rules.
3. `cargo test` (plus `-- --include-ignored` when working in isolated XDG dirs) — verify behavior.
4. Open follow-up docs like `docs/architecture.md` only when a change alters core design.

Keep Copilot suggestions grounded in the documented architecture. If guidance diverges across files, update `AGENTS.md` first so every assistant stays synchronized.
