# Developer Workspace (devws) Design v3 (2025-10-07)

## Architecture

### Directory Structure

```
~/.config/devws/              # Your dotfiles repo (version controlled)
  config/                     # XDG config files → symlinked to ~/.config
    zsh/
      .zshrc
    nvim/
      init.lua
  manifests/
    cli.toml                  # Cross-platform tools
    macos.toml                # macOS-specific
  README.md

~/.local/state/devws/         # XDG_STATE_HOME (local execution state)
  devws.lock                  # Lockfile tracking installed state
  bin/                        # Tool symlinks → cache
  share/
    man/
    zsh/site-functions/

~/.cache/devws/               # XDG_CACHE_HOME (downloaded binaries)
  apps/
    <tool>/<version>/         # Actual binaries (can be cleared/rebuilt)
```

### Key Design Decisions

1. **XDG-only approach**: devws is purpose-built for XDG Base Directory layout
   - Config files symlink to `$XDG_CONFIG_HOME` (default: `~/.config`)
   - No support for dotfiles in home directory root
   - Structure mirrors XDG: `devws/config/zsh/.zshrc` → `~/.config/zsh/.zshrc`
2. **Single workspace model**: `~/.config/devws` IS your dotfiles repo
   - No "profiles" - one workspace per machine/container
   - Version controlled (git repo)
   - Different environments = different machines/containers
3. **Separation of concerns**:
   - `~/.config/devws`: Source of truth (version controlled)
   - `~/.local/state/devws`: Execution state (local, not in git)
   - `~/.cache/devws`: Downloaded binaries (can be cleared)
4. **Lockfile-based state tracking**: Similar to Cargo.lock
   - `devws.lock` in state directory tracks installed symlinks
   - Records exact paths, versions, and timestamps
   - Enables reliable cleanup and drift detection
   - Not checked into git (machine-specific, lives in XDG_STATE_HOME)
5. **Cache-based storage**: Tools downloaded once to `~/.cache`, symlinked to state
6. **Version pinning**: Manifests can pin versions, `update` respects pins
7. **Wrapper scripts only when needed**: For tools requiring LD_LIBRARY_PATH, etc.

## CLI Commands

```bash
# Bootstrap (one-time setup)
devws init [repository]          # Initialize workspace (auto-detects shell from $SHELL)
                                 # --shell <shell>: Override shell detection
                                 # --force: Overwrite existing workspace

# Daily operations
devws sync                       # Pull changes, reinstall configs/tools
devws update [tool]              # Update tools (respect pins, show newer)
devws status                     # Show workspace status

# Maintenance
devws reset                      # Clean git state + reinstall everything
                                 # --force: Skip confirmation
devws cleanup                    # Remove unused cache, orphaned symlinks

# Self-management
devws self info                  # Show version, disk usage
devws self update                # Update devws binary
devws self uninstall             # Remove everything (with confirmation)

# Environment (called by shell)
devws env --shell <shell>        # Output env setup for shell init
```

## Shell Integration

```bash
# Added by `devws init zsh` to ~/.zshenv
eval "$(devws env)"
```

`devws env` outputs environment setup for the shell:
```bash
export PATH="$HOME/.local/state/devws/bin:$PATH"
export MANPATH="$HOME/.local/state/devws/share/man:$MANPATH"
fpath=($HOME/.local/state/devws/share/zsh/site-functions $fpath)
```

## Lockfile Format

```toml
# ~/.local/state/devws/devws.lock
# Machine-generated - tracks resolved state of installed workspace

version = 1

[metadata]
installed_at = "2025-10-07T12:34:56.789Z"

[[config_symlinks]]
source = "/Users/user/.config/devws/config/zsh/.zshrc"
target = "/Users/user/.config/zsh/.zshrc"

[[config_symlinks]]
source = "/Users/user/.config/devws/config/nvim/init.lua"
target = "/Users/user/.config/nvim/init.lua"

[[tool_symlinks]]
name = "rg"
version = "14.0.0"
source = "/Users/user/.cache/devws/apps/ripgrep/14.0.0/rg"
target = "/Users/user/.local/state/devws/bin/rg"

[[tool_symlinks]]
name = "fd"
version = "9.0.0"
source = "/Users/user/.cache/devws/apps/fd/9.0.0/fd"
target = "/Users/user/.local/state/devws/bin/fd"
```

**Purpose:**
- Tracks exactly what symlinks are installed for this workspace
- Enables reliable cleanup and reinstall
- Provides audit trail for `devws status` and `devws cleanup`
- Detects drift (symlinks changed/removed outside devws)
- Generated/updated on `devws init`, `devws sync`, `devws update`

## Manifest Format

```toml
# ~/.config/devws/manifests/cli.toml

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
devws init ascarter/dotfiles  # Auto-detects shell from $SHELL
exec $SHELL
```

### Manually cloned workspace
```bash
git clone git@github.com:ascarter/dotfiles.git ~/.config/devws
devws init  # Auto-detects shell
exec $SHELL
```

### Different workspace (different machine/container)
```bash
# On work machine/container
devws init ascarter/work-dotfiles  # Auto-detects shell
exec $SHELL
```

### Multiple shells on same machine
```bash
# Setup for zsh
devws init --shell zsh

# Also setup for bash (updates bash integration only)
devws init --shell bash

# Also setup for fish (updates fish integration only)
devws init --shell fish
```

### Daily sync
```bash
devws sync      # Pull workspace updates, reinstall
devws update    # Check for tool updates, respect pins
```

### Clean reinstall
```bash
devws reset     # Clean git state + reinstall everything
```

### Maintenance
```bash
devws cleanup   # Remove unused cache and orphaned symlinks
devws status    # Show what's installed
```

## Implementation Status

- ✅ CLI structure defined
- ✅ Command handlers scaffolded
- ✅ XDG helpers implemented
- ✅ Config symlink management
- ✅ Lockfile-based state tracking
- ✅ Single workspace model (no profiles)
- ✅ Shell auto-detection from $SHELL
- ✅ Template structure with default shell configs
- ⏳ Tool installation backends (TODO)
- ⏳ Manifest parsing (TODO)
- ⏳ Cleanup command (remove unused cache, orphaned symlinks)
- ⏳ Status command (read lockfile, show installed state)
