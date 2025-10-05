# devws

> Lightweight, portable development environment bootstrapper

**devws (Developer Workspace)** manages your dotfiles and development tools through declarative manifests. Bootstrap new machines, sync configurations, and maintain your dev environment with a single portable binary.

## Status

🚧 **Early Development** - CLI structure complete, implementation in progress.

## What is devws?

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
curl -fsSL https://devws.dev/install.sh | sh
```

**Bootstrap new machine**:
```bash
# Clone your profile and setup shell integration
devws init zsh username/dotfiles

# Reload shell
exec $SHELL

# Done! Your environment is ready
```

**Or start from scratch**:
```bash
# Create template profile and setup shell
devws init zsh --name myconfig

# Edit your profile
cd ~/.config/devws/profiles/myconfig

# Publish to GitHub
gh repo create myconfig --public --source=. --push
```

## Daily Usage

```bash
# Pull latest profile changes and install new tools
devws sync

# Check for tool updates (respects version pins)
devws update

# Show current status
devws status

# Check environment health
devws doctor
```

## Profile Management

```bash
# Clone a new profile
devws clone username/work-dotfiles --name work

# Switch profiles
devws use work
exec $SHELL

# List all profiles
devws list
```

## Profile Structure

```
~/.config/devws/profiles/default/
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

1. **Shell integration**: `devws init` adds one line to `.zshenv`:
   ```bash
   eval "$(devws env)"
   ```

2. **Environment setup**: `devws env` outputs:
   ```bash
   export PATH="$HOME/.local/state/devws/environments/default/bin:$PATH"
   export MANPATH="$HOME/.local/state/devws/environments/default/share/man:$MANPATH"
   fpath=($HOME/.local/state/devws/environments/default/share/zsh/site-functions $fpath)
   ```

3. **Tool installation**: Tools cached in `~/.cache/devws/`, symlinked per-profile

4. **Profile switching**: Atomically updates symlinks and config

## Self-Management

```bash
# Show version, disk usage, profile count
devws self

# Update devws itself
devws self update

# Remove everything (with confirmation)
devws self uninstall
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

See [ASSISTANT.md](ASSISTANT.md) for AI assistant context and development guidelines.

See [docs/architecture.md](docs/architecture.md) for the current implementation design and technical decisions.

## License

MIT - see [LICENSE](LICENSE)
