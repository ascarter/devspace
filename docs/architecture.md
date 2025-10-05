# Developer Workspace (devws) Design v2 (2025-10-02)

## Architecture

### Directory Structure

```
~/.config/devws/
  config.toml                    # { active_profile = "default" }
  profiles/
    default/
      config/                    # Dotfiles → symlinked to ~
      manifests/
        cli.toml                 # Cross-platform tools
        macos.toml               # macOS-specific
      README.md

~/.local/state/devws/         # XDG_STATE_HOME
  environments/
    default/
      bin/                       # Symlinks → cache (or wrappers if needed)
      share/
        man/
        zsh/site-functions/

~/.cache/devws/               # XDG_CACHE_HOME
  apps/
    <tool>/<version>/            # Actual binaries (shared across profiles)
```

### Key Design Decisions

1. **Cache-based storage**: Tools downloaded once to `~/.cache`, symlinked per-profile
2. **No version duplication**: Environments symlink to specific versions in cache
3. **Atomic profile switching**: Remove old symlinks, create new ones atomically
4. **Version pinning**: Manifests can pin versions, `update` respects pins
5. **Wrapper scripts only when needed**: For tools requiring LD_LIBRARY_PATH, etc.

## CLI Commands

```bash
# Bootstrap
devws init [shell] [url|user/repo] [--name <profile>]
devws clone <url|user/repo> [--name <profile>]

# Profile management
devws use <profile>         # Switch profile (requires exec $SHELL)
devws list                  # List profiles

# Daily operations
devws sync                  # Pull changes, install new, respect pins
devws update [tool]         # Update tools (respect pins, show newer)
devws status                # Show profile, tools, updates

# Maintenance
devws doctor                # Health check + repair

# Self-management
devws self                  # Show version, disk usage, profile count
devws self update           # Update devws binary
devws self uninstall        # Remove everything (with confirmation)

# Environment (called by shell)
devws env [profile]         # Output env setup for shell init
```

## Shell Integration

```bash
# Added by `devws init zsh` to ~/.zshenv
eval "$(devws env)"
```

`devws env` reads `~/.config/devws/config.toml` (or `$DEVWS_PROFILE`) and outputs:
```bash
export PATH="$HOME/.local/state/devws/environments/default/bin:$PATH"
export MANPATH="$HOME/.local/state/devws/environments/default/share/man:$MANPATH"
fpath=($HOME/.local/state/devws/environments/default/share/zsh/site-functions $fpath)
```

## Manifest Format

```toml
# ~/.config/devws/profiles/default/manifests/cli.toml

[ripgrep]
installer = "ubi"
project = "BurntSushi/ripgrep"
version = "14.0.0"              # Pin version (optional)
bin = ["rg"]
symlinks = [
  "doc/rg.1:${STATE_DIR}/share/man/man1/rg.1",
  "complete/_rg:${STATE_DIR}/share/zsh/site-functions/_rg"
]

[rustup]
installer = "curl"
url = "https://sh.rustup.rs"
shell = "sh"
self_update = true              # Has built-in update mechanism
```

## Key Workflows

### Fresh machine setup
```bash
curl -fsSL https://devws.dev/install.sh | sh
devws init zsh ascarter/dotfiles
exec $SHELL
```

### Switch profiles
```bash
devws clone ascarter/work --name work
devws use work
exec $SHELL
```

### Daily sync
```bash
devws sync      # Pull profile updates, install new tools
devws update    # Check for tool updates, respect pins
```

## Implementation Status

- ✅ CLI structure defined
- ✅ Command handlers scaffolded
- ✅ XDG helpers implemented
- ⏳ Profile management (TODO)
- ⏳ Tool installation backends (TODO)
- ⏳ Symlink management (TODO)
