# TODO

- [x] Phase 0: Remove `ubi`; scaffold installer core layout; bump lockfile schema to v2 (tool_receipts)
- [ ] Phase 1: New manifest parser (tables for bin/extras, asset_filter regex list, checksum) + `dws check` structural validation
- [ ] Phase 2: GitHub backend (release metadata fetch, asset selection scoring, download, extract, receipt write)
- [ ] Phase 3: Checksum discovery & verification (asset + script) + receipt status updates
- [ ] Phase 4: Script installer backend (download, checksum verify, execute, explicit binaries)
- [ ] Phase 5: Interactive `dws add` (regex refinement loop, binary/extras detection, immediate install, anchored regex for pinned)
- [ ] Phase 6: GitLab backend integration
- [ ] Phase 7: Update command (pinned/latest/self-update/script logic, mandatory post-update `dws check`)
- [ ] Phase 8: Cleanup enhancements (auto repair/remove broken symlinks, prune inactive versions, stale downloads, keep-previous flag)
- [ ] Phase 9: Extended validation (strict deterministic asset matching, platform filters)
- [ ] Phase 10: Concurrency (parallel installs & metadata fetch)
- [ ] Phase 11: Policy flags (optional ignore checksum, require global checksum)
- [ ] Phase 12: Signature verification groundwork (GPG/minisign)
- [ ] Phase 13: Raw URL archive backend (if needed)
- [ ] Phase 14: Semantic version range support

- [ ] Container-based sandbox workflow
  - Build Linux target binary (`cargo build --target x86_64-unknown-linux-gnu` etc.)
  - Launch default image (e.g. `quay.io/fedora/fedora-toolbox:latest`) via podman/docker
  - Bind mount repo + compiled `dws` binary into container
  - Start interactive shell with isolated XDG directories; document usage so it can replace the host sandbox

- [ ] Container-based sandbox workflow
  - Build Linux target binary (`cargo build --target x86_64-unknown-linux-gnu` etc.)
  - Launch default image (e.g. `quay.io/fedora/fedora-toolbox:latest`) via podman/docker
  - Bind mount repo + compiled `dws` binary into container
  - Start interactive shell with isolated XDG directories; document usage so it can replace the host sandbox

- [ ] Create `DEPENDENCIES.md`
  - Document rationale for each direct dependency
  - Note replacement of `atty` with `is-terminal` (commits 26a371b, 28fc610 for lockfile refresh)
  - Define quarterly audit cadence (cargo audit/outdated/deny)
  - Highlight potential future migrations (e.g. chrono -> time, directories -> alternative)

- [ ] Add GitHub Actions workflow for dependency health
  - Jobs: cargo fmt + clippy, cargo audit, cargo deny, cargo outdated (root dependencies), build & tests
  - Cache Cargo registry & target to speed builds
  - Use matrix for stable + minimal supported Rust version

- [ ] Introduce `deny.toml` (cargo-deny configuration)
  - Enforce allowed licenses (MIT/Apache-2.0/BSD-3-Clause)
  - Deny unmaintained/yanked crates; warn about stale versions
  - Exclude vendored openssl warnings from git2 as needed

- [ ] Add scripts `audit.sh`, `outdated.sh`, `deny.sh` (optional if workflow suffices)
  - Provide local developer convenience wrappers

- [ ] Implement CLI color control flags
  - `--color=auto|always|never` overriding NO_COLOR env
  - Respect `NO_COLOR` if `--color` not explicitly set

- [ ] Document Dependabot alert handling
  - Steps to dismiss resolved alerts with explanatory comment
  - Encourage re-run after lockfile changes

- [ ] Evaluate abstraction layer over installer backends
  - Trait for installers enabling mock & offline tests
  - Prepare for potential replacement / augmentation of `ubi`

