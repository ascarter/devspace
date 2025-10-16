# TODO

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

