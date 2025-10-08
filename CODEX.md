# Codex Agent Guide

Codex agents should start with [`AGENTS.md`](AGENTS.md) for project rules, commands, and style expectations. Pair it with `README.md` for product positioning and `ASSISTANT.md` for deeper architectural intent.

Workflow checklist:
1. Read the task instructions and cross-reference relevant sections in `AGENTS.md`.
2. Inspect or modify only the files mentioned; consult `docs/architecture.md` when touching core design.
3. Run `cargo fmt` and `cargo clippy --all-targets --all-features` before sharing a patch.
4. Add tests or fixtures as advised in `AGENTS.md` and existing test suites.

If documentation conflicts, update `AGENTS.md` to keep every agent aligned and surface open questions to the maintainer. Sandboxed operations must avoid mutating the real user config unless instructions explicitly allow it.
