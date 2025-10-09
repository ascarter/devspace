# dws

> Developer Workspace - Lightweight, portable development environment manager

**dws (Developer Workspace)** manages your dotfiles and development tools through declarative manifests. Bootstrap new machines, sync configurations, and maintain your dev environment with a single portable binary.

## Status

🚧 **Early Development** - Core architecture complete, implementation in progress.

## What is dws?

A personal dev environment manager optimized for interactive development:

- **Quick bootstrap**: Single binary → full dev environment
- **Version controlled**: Dotfiles + tool manifests in GitHub
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
├── config.toml                # Workspace configuration (active profile, etc.)
├── profiles/                  # User-managed profiles (each is a git repo)
│   ├── default/               # Default profile created by dws init
│   │   ├── config/            # XDG config files → symlinked to ~/.config
│   │   └── manifests/         # Tool definitions
│   └── <profile>/             # Additional profiles cloned or created
└── (future dws config)
```

### Profiles

- `dws profiles` &mdash; list available profiles (the active one is marked).
- `dws clone <repo> [--profile name]` &mdash; clone another profile into `profiles/<name>` without activating it.
- `dws use <profile>` &mdash; switch to a profile (symlinks are updated and `config.toml` is rewritten).
### Example Manifest

```toml
# profiles/<profile>/manifests/tools.toml
[ripgrep]
installer = "ubi"
project = "BurntSushi/ripgrep"
version = "14.0.0"            # Pin version (optional)

[rustup]
installer = "curl"
url = "https://sh.rustup.rs"
self_update = true            # Has built-in updates

[uv]
installer = "curl"
url = "https://astral.sh/uv/install.sh"
```

### Manifest Reference

- `installer` *(required)* — Backend to use (`ubi`, `curl`, `dmg`, `flatpak`).
- `project` — GitHub `owner/repo` for release-based installers.
- `version` — Fixed release version; omit to use the backend default.
- `url` — Direct download endpoint for scripts or disk images.
- `shell` — Interpreter used to run installer scripts (e.g. `sh`, `bash`).
- `bin` — Array of executables to link into `~/.local/state/dws/bin`.
- `symlinks` — Additional files to link, using `source:target` syntax.
- `app` — macOS `.app` bundle name extracted from a DMG.
- `team_id` — Apple Developer team identifier used for notarization checks.
- `self_update` — Set to `true` if the tool updates itself and should be skipped by `dws update`.

Manifests merge in precedence order: base (`tools.toml`) → platform (e.g. `tools-macos.toml`) → host (`tools-<hostname>.toml`). Higher-precedence manifests only override the fields they specify; omitted keys inherit from lower layers. If the hostname resolves to something unusable, dws falls back to looking for `tools-local.toml`.

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

See [ASSISTANT.md](ASSISTANT.md) for AI assistant context and development guidelines.

See [docs/architecture.md](docs/architecture.md) for the current implementation design and technical decisions.

## License

MIT - see [LICENSE](LICENSE)
