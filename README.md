# dws

> Developer Workspace - Lightweight, portable development environment manager

**dws (Developer Workspace)** manages your dotfiles and development tools through declarative manifests: each profile ships a `dws.toml`, and the workspace can provide overrides in `config.toml`. Bootstrap new machines, sync configurations, and maintain your dev environment with a single portable binary.

## Status

🚧 **Early Development** - Core architecture complete, implementation in progress.

## What is dws?

A personal dev environment manager optimized for interactive development:

- **Quick bootstrap**: Single binary → full dev environment
- **Version controlled**: Dotfiles + profile `dws.toml` tool definitions in GitHub
- **XDG compliant**: Self-contained, easy to remove
- **Native tools**: Use rustup/uv/fnm directly, no shims
- **Single workspace**: One environment per machine/container
- **Lightweight**: No heavy package managers or runtime overhead

## Quick Start

**Installation** (coming soon):
```bash
curl -fsSL https://dws.ascarter.dev/install.sh | sh
```

**Bootstrap new machine**:
```bash
# Clone your dotfiles and setup shell integration
dws init username/dotfiles

# Reload shell
exec $SHELL

# Done! Your environment is ready
```

**Or start from scratch**:
```bash
# Create template workspace and setup shell
dws init --profile personal

# Edit your workspace
cd ~/.config/dws/profiles/personal

# Publish to GitHub
gh repo create dotfiles --public --source=. --push
```

## Daily Usage

```bash
# Pull latest changes and reinstall
dws sync

# Check for tool updates (respects version pins)
dws update

# Show current status
dws status

# Clean up unused cache and orphaned symlinks
dws cleanup

# Switch profiles
dws profiles
dws use work
```

## Workspace Structure

```
~/.config/dws/                 # dws workspace root (reserved for tooling)
├── config.toml               # Workspace configuration (active profile + overrides)
├── profiles/                 # User-managed profiles (each is a git repo)
│   ├── default/              # Default profile created by dws init
│   │   ├── config/           # XDG config files → symlinked to ~/.config
│   │   └── dws.toml          # Profile-level tool definitions
│   └── <profile>/            # Additional profiles cloned or created
└── (state/cache live under $XDG_STATE_HOME/$XDG_CACHE_HOME)
```

### Profiles

- `dws profiles` &mdash; list available profiles (the active one is marked).
- `dws clone <repo> [--profile name]` &mdash; clone another profile into `profiles/<name>` without activating it.
- `dws use <profile>` &mdash; switch to a profile (symlinks are updated and `$XDG_CONFIG_HOME/dws/config.toml` is rewritten).

### Example `dws.toml`

```toml
# profiles/<profile>/dws.toml
[tools.ripgrep]
installer = "github"
project = "BurntSushi/ripgrep"
version = "v14.0.0"
asset_filter = ["^ripgrep-14\\.0\\.0-x86_64-unknown-linux-musl\\.tar\\.gz$"]
checksum = "sha256:deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
self_update = false
platform = ["linux"]

[[tools.ripgrep.bin]]
source = "rg"

[[tools.ripgrep.extras]]
source = "doc/rg.1"
kind = "man"

[[tools.ripgrep.extras]]
source = "complete/_rg"
kind = "completion"
shell = "zsh"

[tools.uv]
installer = "script"
url = "https://astral.sh/uv/install.sh"
shell = "sh"
version = "latest"
checksum = "sha256:cafebabe0123456789abcdefcafebabe0123456789abcdefcafebabe01234567"
self_update = true

[[tools.uv.bin]]
source = "uv"

# Workspace overrides live in $XDG_CONFIG_HOME/dws/config.toml and replace entire tool entries.
```

### Tool Entry Reference

- `installer` *(required)* — Backend identifier (`github`, `gitlab`, `script`).
- `project` — Forge `owner/repo` (GitHub/GitLab) required for release installers.
- `version` — Explicit tag (pinned) or `"latest"` (unpinned but still deterministic).
- `url` — Script download URL (only for `installer = "script"`).
- `shell` — Interpreter for script installers (e.g. `sh`, `bash`).
- `[[tools.<name>.bin]]` — Structured binary entries (`source`, optional `link` alias).
- `[[tools.<name>.extras]]` — Additional linkables (`source`, `kind` = man|completion|other, optional `shell`, optional explicit `target`).
- `asset_filter` — Ordered list of regex patterns; first that yields exactly one asset (after scoring/refinement) is used.
- `checksum` — Mandatory `sha256:<hex>` for asset or script content.
- `self_update` — Tool manages its own updates; `dws update` verifies presence & checksum but does not reinstall.
- `platform` — Optional platform tags (e.g. `linux`, `macos`, distro variants). Non-matching entries are treated as errors during validation.
- `hosts` — Optional sanitized host filters; entry ignored (error surfaced) if host does not match current machine.

Tool entries are layered as follows:

1. **Profile `dws.toml`** — checked into the profile repository; forms the base definition set.
2. **Workspace `config.toml`** — optional overrides stored at `$XDG_CONFIG_HOME/dws/config.toml`. When a tool name appears in both files, the workspace entry replaces the profile entry entirely. Entries that fail platform/host filters are ignored, leaving lower-precedence definitions intact.

## How It Works

1. **Shell integration**: `dws init` adds one line to `.zshenv`:
   ```bash
   eval "$(dws env)"
   ```

2. **Environment setup**: `dws env` outputs:
   ```bash
   export PATH="$HOME/.local/state/dws/bin:$PATH"
   export MANPATH="$HOME/.local/state/dws/share/man:$MANPATH"
   fpath=($HOME/.local/state/dws/share/zsh/site-functions $fpath)
   ```

3. **Tool installation**: Tools cached in `~/.cache/dws/`, symlinked to state

4. **Lockfile tracking**: `~/.local/state/dws/dws.lock` tracks installed symlinks

## Self-Management

```bash
# Show version, disk usage
dws self info

# Update dws itself
dws self update

# Remove everything (with confirmation)
dws self uninstall
```

## Development

### Developer Setup

1. **Check the toolchain**
   ```bash
   rustup show active-toolchain
   cargo --version
   ```
   Ensure Rust 1.70 or newer is installed (`rustup update` if needed).

2. **Build the project**
   ```bash
   cargo build
   ```

3. **Inspect CLI wiring**
   ```bash
   cargo run -- --help
   ```

4. **Run tests**
   ```bash
   cargo test
   ```
   Append `-- --include-ignored` when running CLI tests that rely on isolated XDG directories.

5. **Review core docs**
   - `AGENTS.md` — canonical contributor handbook for human and AI agents
   - `docs/architecture.md` — deeper technical design context

**Prerequisites**:
- Rust 1.70+
- Git

**Build**:
```bash
cargo build
```

**Test**:
```bash
cargo test
```

### Continuous Integration

GitHub Actions runs on pushes and pull requests to `main`. The workflow enforces:

- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo build --locked --all-targets`
- `cargo test --locked`

Linux (`ubuntu-latest`) and macOS runners execute the build/test matrix so regressions surface across both platforms. Keep local runs clean before pushing.

**Run**:
```bash
cargo run -- --help
```

See [AGENTS.md](AGENTS.md) for contributor guidelines and agent workflow notes.

## Architecture

See [docs/architecture.md](docs/architecture.md) for the current implementation design and technical decisions.

## License

MIT - see [LICENSE](LICENSE)
