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
installer = "ubi"
project = "BurntSushi/ripgrep"
platform = ["macos", "linux"]

[tools.uv]
installer = "curl"
url = "https://astral.sh/uv/install.sh"
shell = "sh"
self_update = true

# Optional workspace overrides live in $XDG_CONFIG_HOME/dws/config.toml
# and replace entire tool entries when names match.
```

### Tool Entry Reference

- `installer` *(required)* — Backend to use (`ubi`, `curl`, `dmg`, `flatpak`).
- `project` — GitHub `owner/repo` for release-based installers.
- `version` — Fixed release version; omit to use the backend default.
- `url` — Direct download endpoint for scripts or disk images.
- `shell` — Interpreter used to run installer scripts (e.g. `sh`, `bash`).
- `bin` — Array of executables to link into `~/.local/state/dws/bin` (defaults to installer-provided binary name when omitted for `ubi`).
- `symlinks` — Additional files to link, using `source:target` syntax.
- `app` — macOS `.app` bundle name extracted from a DMG.
- `team_id` — Apple Developer team identifier used for notarization checks.
- `self_update` — Set to `true` if the tool updates itself and should be skipped by `dws update`.
- `platform` — Optional list of platform filters. Values match Rust's `std::env::consts::OS` (`macos`, `linux`) plus distro slugs such as `linux-ubuntu` or `linux-arch` inferred from `/etc/os-release`. Windows is unsupported (use WSL instead).
- `hosts` — Optional list of sanitized hostnames (lowercase, non-alphanumeric converted to `-`).

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

**Run**:
```bash
cargo run -- --help
```

See [AGENTS.md](AGENTS.md) for contributor guidelines and agent workflow notes.

## Architecture

See [docs/architecture.md](docs/architecture.md) for the current implementation design and technical decisions.

## License

MIT - see [LICENSE](LICENSE)
