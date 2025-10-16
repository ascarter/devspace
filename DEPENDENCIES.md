# Dependency Rationale and Audit Plan

This document explains why each direct (and key dev) dependency is included, the risk profile (maintenance / security / bus factor), and the structured audit & upgrade policy for the `dws` project.

Last reviewed: 2025-10-16  
Related commits:
- 26a371b: Replace `atty` with `is-terminal`
- 28fc610: Refresh `Cargo.lock` after removal of `atty`

---

## 1. Philosophy

`dws` aims for:
1. Minimal but pragmatic dependencies.
2. Predictable, deterministic builds (locked versions, vendored crypto where necessary).
3. Clear upgrade cadence (quarterly full audit; ad‑hoc for CVEs).
4. Documentation of potential future migrations to avoid surprise refactors.

Categories:
- Core CLI & UX
- Configuration & Data
- Networking / Remote
- System / Env Discovery
- Installer Backends
- Async Runtime
- Observability
- Utilities
- Development / Testing

---

## 2. Direct Runtime Dependencies

### clap (CLI parsing)
- Version: 4.x
- Rationale: Mature, feature-rich parsing (derive macros, env integration).
- Risk: Medium (fast development pace; watch MSRV).
- Alternative considered: `argp` (less feature-rich), `pico-args` (minimal).
- Action: Keep; re-check breaking changes on major upgrades.

### serde / toml
- Rationale: Standard for structured config + lockfile serialization.
- Risk: Low; serde stable, toml crate actively maintained.
- Future: If performance-critical, evaluate `toml_edit` for in-place updates.

### anyhow / thiserror
- Rationale: Ergonomic error handling (context + custom error enums).
- Risk: Low.
- Future: Stable; no action unless new error taxonomy needed.

### directories
- Rationale: Resolve XDG and platform-specific directories.
- Risk: Medium (slower release cadence).
- Future: Consider `directories-next` or explicit XDG resolution via `home` + manual fallbacks. Track issue for churn.

### tracing / tracing-subscriber
- Rationale: Structured logging with `EnvFilter`.
- Risk: Low.
- Future: Add JSON layer or hierarchical spans if needed for performance profiling.

### chrono
- Rationale: Timestamp parsing (RFC3339) for lockfile metadata.
- Risk: Medium (some legacy path/timezone concerns).
- Future: Evaluate migration to `time` crate for smaller surface & modern API. Keep until a migration reduces code complexity.

### git2 (vendored-openssl)
- Rationale: Interact with template/profile repos (commit validation, potential future sync operations).
- Risk: Medium (libgit2/OpenSSL CVEs).
- Mitigation: Vendored OpenSSL for deterministic builds; run `cargo audit` quarterly; monitor libgit2 releases.
- Future: Consider high-level wrapper or limited shelling out to `git` for simpler operations if dependency risk grows.

### whoami
- Rationale: Derive user identity for attribution in commits or metadata.
- Risk: Low.
- Future: Could inline minimal platform-specific logic; not a priority.

### tokio
- Rationale: Async foundation for future concurrent operations (network installs, status tasks).
- Risk: Low (widely adopted).
- Future: Potentially introduce a cooperative cancellation abstraction for installer tasks.

### ubi
- Rationale: GitHub release binary installer (downloading + unpack).
- Risk: Medium (smaller project, bus factor).
- Future: Abstract behind an `Installer` trait; allow mocking & fallback to custom minimal release fetch logic. Evaluate performance and reliability quarterly.

### walkdir
- Rationale: Recursive filesystem traversal for tool / config operations.
- Risk: Low.
- Future: Adequate; no change.

### reqwest (blocking + rustls-tls + json)
- Rationale: HTTP client for remote operations (future: release metadata, bootstrap scripts).
- Risk: Medium (feature breadth).
- Mitigation: Feature subset chosen; consider splitting blocking vs async usage explicitly later.
- Future: If footprint matters, evaluate `ureq` for simple blocking ops.

### url
- Rationale: Safe URL manipulation (query building for release endpoints).
- Risk: Low.

### anstyle
- Rationale: Terminal styling independent of deprecated crates.
- Risk: Low.
- Future: Consolidate color handling logic if `clap`'s built-in color styles become sufficient.

### is-terminal
- Rationale: Modern, maintained TTY detection replacing unmaintained `atty`.
- Risk: Low.
- Migration Note: Replaced `atty` due to maintenance status & advisory (Commits 26a371b, 28fc610).

---

## 3. Dev Dependencies

### assert_cmd / predicates
- Rationale: CLI integration testing (assert exit codes, output).
- Risk: Low.
- Future: Keep; possibly add snapshot strategy for stable multiline output validations.

### serial_test
- Rationale: Serialize tests that mutate global state or share temp workspace directories.
- Risk: Low.
- Future: Replace with internal test harness that scopes isolated workspaces per test if contention drops.

### tempfile
- Rationale: Safe ephemeral directories/files for tests.
- Risk: Low.

### rstest
- Rationale: Parameterized tests for repeated configuration patterns.
- Risk: Low.
- Future: Consider native table-driven tests to reduce macro reliance if compile times increase.

---

## 4. Replacement History

| Date (UTC) | Change | Reason | Commits |
|------------|--------|--------|---------|
| 2025-10-16 | `atty` → `is-terminal` | `atty` unmaintained; modern API & advisory resolution | 26a371b, 28fc610 |

---

## 5. Audit & Maintenance Policy

### Cadence
- Quarterly full dependency audit (target months: Jan, Apr, Jul, Oct).
- Immediate action for high/critical CVEs (within 72 hours).
- Minor & patch upgrades batched unless blocking a feature.

### Tools / Checks (executed locally and in CI)
1. `cargo verify` (meta script: fmt, clippy, test, audit).
2. `cargo audit` (security advisories).
3. `cargo outdated --root-deps-only` (version drift).
4. `cargo deny check` (licenses, yanked, unmaintained, duplicates).

### Acceptance Criteria
- No high/critical advisories unresolved.
- No yanked crates.
- Unmaintained crates flagged and either replaced or documented with a migration plan.
- License set restricted to: MIT, Apache-2.0, BSD-3-Clause (add others explicitly if required).

### Escalation
- If a dependency cannot be upgraded due to API breakage:
  - Open a tracking issue with: blocker description, upstream issue link, proposed migration path.
  - Revisit monthly until resolved.

---

## 6. Future Considerations / Migration Candidates

| Dependency    | Candidate Replacement | Value | Notes |
|---------------|-----------------------|-------|-------|
| chrono        | `time` crate          | Smaller API surface, modern formatting | Ensure RFC3339 parsing equivalence + local TZ formatting behavior parity. |
| directories   | `directories-next` / manual XDG resolution | Maintenance continuity | Prototype manual resolution to measure complexity. |
| ubi           | Internal installer abstraction | Control + testability | Start trait design; fallback pure Rust GitHub fetch + decompress. |
| reqwest (blocking) | Async-only + small blocking wrapper | Lean runtime | Depends on concurrency needs; reassess after installer abstraction. |
| whoami        | Inline platform logic | Reduce deps | Only if dependency churn grows. |

---

## 7. Operational Playbook

### Quarterly Audit Steps
1. Pull main & update toolchain.
2. Run:
   - `cargo fmt --check`
   - `cargo clippy --all-targets --all-features`
   - `cargo test --all`
   - `cargo audit`
   - `cargo deny check`
   - `cargo outdated --root-deps-only`
3. Record results in `AUDIT_LOG.md` (to be created on first audit).
4. Apply non-breaking minor/patch upgrades; open PR labeled `maintenance`.
5. For major version candidates, open issue summarizing API delta.

### Responding to Advisory
- Confirm transitive vs direct source (`cargo tree -d`).
- If transitive and fix requires root dependency bump, test locally with override patch.
- If no immediate patch, add temporary `[patch]` section or document risk in issue.

---

## 8. Guidelines for Adding New Dependencies

Must satisfy ALL:
1. Clear functional gap (cannot be trivially implemented under ~100 LOC).
2. Active maintenance (recent release within 12 months).
3. Compatible license.
4. Verified minimal feature set enabled (disable default features when practical).
5. Benchmarked or reasoned performance impact if in hot path.

PR must include:
- Justification in description.
- Update to this file (section 2 or 3).
- If network or filesystem heavy: add tests + mocks.

---

## 9. Monitoring & Automation

Planned CI workflow additions:
- Matrix: stable + minimal MSRV (document MSRV once frozen).
- Cache: `~/.cargo/registry` + `target/`.
- Separate jobs: Build, Lint (clippy), Security (audit/deny), Drift (outdated), Tests.
- Failure in any security job blocks merge; outdated is informational unless critical patch available.

---

## 10. Appendix: Exportable Scripts (Planned)

Local helper scripts (to be added in `scripts/`):
- `audit.sh` → installs `cargo-audit` & runs audit.
- `outdated.sh` → installs `cargo-outdated` & prints root dependency drift.
- `deny.sh` → installs `cargo-deny` & performs policy checks.

Rationale: Keep CI logic consistent with developer local environment.

---

## 11. Open Actions

Tracked in `TODO.md`:
- Create CI workflow for dependency health.
- Add `deny.toml` configuration.
- Provide color control flags.
- Abstract installer logic.

---

## 12. Verification Snapshot (Post atty Removal)

Checks performed (2025-10-16):
- Grep for `atty` → none in source & lockfile.
- `cargo check` succeeded.
- Lockfile refreshed; no residual transitive atty edges.

---

## 13. License Compatibility Set

Allowed (current policy):
- MIT
- Apache-2.0
- BSD-3-Clause
- (Add MPL-2.0 if necessary; requires explicit approval)
Denied by default:
- GPL family (unless dual-licensed fallback present)
- AGPL
- Proprietary / custom unclear terms

`cargo deny` will enforce once config added.

---

## 14. Change Management

Any edits to this file must:
1. Update Last reviewed date.
2. Summarize changes in a commit message starting with `docs(deps):`.
3. Cross-link related issues/PRs.

---

End of document.