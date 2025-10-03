# devspace

> Lightweight, portable development environment bootstrapper

**devspace** manages your dotfiles and development tools through declarative manifests. Bootstrap new machines, sync configurations, and maintain your dev environment with a single portable binary.

## Status

🚧 **Early Development** - CLI structure complete, implementation in progress.

## What is devspace?

A personal dev environment bootstrapper optimized for interactive development:

- **Quick bootstrap**: Single binary → full dev environment
- **Version controlled**: Dotfiles + tool manifests in GitHub
- **XDG compliant**: Self-contained, easy to remove
- **Native tools**: Use rustup/uv/fnm directly, no shims
- **Profile-based**: Switch between work/personal/project configs
- **Lightweight**: No heavy package managers or runtime overhead

## Quick Start

**Installation** (coming soon):
```bash
curl -fsSL https://devspace.dev/install.sh | sh
```

**Bootstrap new machine**:
```bash
# Clone your profile and setup shell integration
devspace init zsh username/dotfiles

# Reload shell
exec $SHELL

# Done! Your environment is ready
```

**Or start from scratch**:
```bash
# Create template profile and setup shell
devspace init zsh --name myconfig

# Edit your profile
cd ~/.config/devspace/profiles/myconfig

# Publish to GitHub
gh repo create myconfig --public --source=. --push
```

## Daily Usage

```bash
# Pull latest profile changes and install new tools
devspace sync

# Check for tool updates (respects version pins)
devspace update

# Show current status
devspace status

# Check environment health
devspace doctor
```

## Profile Management

```bash
# Clone a new profile
devspace clone username/work-dotfiles --name work

# Switch profiles
devspace use work
exec $SHELL

# List all profiles
devspace list
```

## Profile Structure

```
~/.config/devspace/profiles/default/
├── config/                    # Dotfiles (symlinked to ~)
│   ├── zsh/
│   │   └── .zshrc
│   ├── git/
│   │   └── .gitconfig
│   └── ...
├── manifests/                 # Tool definitions
│   ├── cli.toml              # Cross-platform tools
│   └── macos.toml            # macOS-specific
└── README.md                  # Auto-generated
```

### Example Manifest

```toml
# manifests/cli.toml
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

## How It Works

1. **Shell integration**: `devspace init` adds one line to `.zshenv`:
   ```bash
   eval "$(devspace env)"
   ```

2. **Environment setup**: `devspace env` outputs:
   ```bash
   export PATH="$HOME/.local/state/devspace/environments/default/bin:$PATH"
   export MANPATH="$HOME/.local/state/devspace/environments/default/share/man:$MANPATH"
   fpath=($HOME/.local/state/devspace/environments/default/share/zsh/site-functions $fpath)
   ```

3. **Tool installation**: Tools cached in `~/.cache/devspace/`, symlinked per-profile

4. **Profile switching**: Atomically updates symlinks and config

## Self-Management

```bash
# Show version, disk usage, profile count
devspace self

# Update devspace itself
devspace self update

# Remove everything (with confirmation)
devspace self uninstall
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

## Architecture

See [ASSISTANT.md](ASSISTANT.md) for detailed architecture and design decisions.

See [.claude/design-v2.md](.claude/design-v2.md) for current implementation design.

## License

MIT - see [LICENSE](LICENSE)
