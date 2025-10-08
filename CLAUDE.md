# Claude Agent Guide

Welcome! Use [`AGENTS.md`](AGENTS.md) as the authoritative checklist for working on this repository. It links to `README.md`, `SETUP.md`, `ASSISTANT.md`, and `docs/architecture.md` for deeper context.

Minimum workflow for Claude Code:
1. Skim `AGENTS.md` → follow structure, commands, and style rules.
2. Read the task-specific files plus any references mentioned in `ASSISTANT.md`.
3. Run `cargo fmt` and `cargo clippy --all-targets --all-features` before proposing changes.
4. Add or update tests as directed in `AGENTS.md`.

If something in the docs appears stale, update `AGENTS.md` first so every agent stays in sync. Ping a human for conflicting requirements or destructive actions. Good luck!
