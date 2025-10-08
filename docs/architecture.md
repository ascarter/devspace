# Developer Workspace (dws) Design v3 (2025-10-07)

## Architecture

### Directory Structure

```
~/.config/dws/                # Your dotfiles repo (version controlled)
  config/                     # XDG config files → symlinked to ~/.config
    zsh/
      .zshrc
    nvim/
      init.lua
  manifests/
    cli.toml                  # Cross-platform tools
    macos.toml                # macOS-specific
  README.md

~/.local/state/dws/           # XDG_STATE_HOME (local execution state)
  dws.lock                    # Lockfile tracking installed state
  bin/                        # Tool symlinks → cache
  share/
    man/
    zsh/site-functions/

~/.cache/dws/                 # XDG_CACHE_HOME (downloaded binaries)
  apps/
    <tool>/<version>/         # Actual binaries (can be cleared/rebuilt)
```

### Key Design Decisions

1. **XDG-only approach**: dws is purpose-built for XDG Base Directory layout
   - Config files symlink to `$XDG_CONFIG_HOME` (default: `~/.config`)
   - No support for dotfiles in home directory root
   - Structure mirrors XDG: `dws/config/zsh/.zshrc` → `~/.config/zsh/.zshrc`
2. **Single workspace model**: `~/.config/dws` IS your dotfiles repo
   - No "profiles" - one workspace per machine/container
   - Version controlled (git repo)
   - Different environments = different machines/containers
3. **Separation of concerns**:
   - `~/.config/dws`: Source of truth (version controlled)
   - `~/.local/state/dws`: Execution state (local, not in git)
   - `~/.cache/dws`: Downloaded binaries (can be cleared)
4. **Lockfile-based state tracking**: Similar to Cargo.lock
   - `dws.lock` in state directory tracks installed symlinks
   - Records exact paths, versions, and timestamps
   - Enables reliable cleanup and drift detection
   - Not checked into git (machine-specific, lives in XDG_STATE_HOME)
5. **Cache-based storage**: Tools downloaded once to `~/.cache`, symlinked to state
6. **Version pinning**: Manifests can pin versions, `update` respects pins
7. **Manifest override precedence**: `cli.toml` defines global defaults → platform manifests (e.g. `macos.toml`) override globals → host-specific files (any other name) override platform; conflicting definitions at the same scope are rejected.
8. **Wrapper scripts only when needed**: For tools requiring LD_LIBRARY_PATH, etc.

## CLI Commands

```bash
# Bootstrap (one-time setup)
dws init [repository]            # Initialize workspace (auto-detects shell from $SHELL)
                                 # --shell <shell>: Override shell detection
                                 # --force: Overwrite existing workspace

# Daily operations
dws sync                         # Pull changes, reinstall configs/tools
dws update [tool]                # Update tools (respect pins, show newer)
dws status                       # Show workspace status

# Maintenance
dws reset                        # Clean git state + reinstall everything
                                 # --force: Skip confirmation
dws cleanup                      # Remove unused cache, orphaned symlinks

# Self-management
dws self info                    # Show version, disk usage
dws self update                  # Update dws binary
dws self uninstall               # Remove everything (with confirmation)

# Environment (called by shell)
dws env --shell <shell>          # Output env setup for shell init
```

## Shell Integration

```bash
# Added by `dws init` to ~/.zshenv
eval "$(dws env)"
```

`dws env` outputs environment setup for the shell:
```bash
export PATH="$HOME/.local/state/dws/bin:$PATH"
export MANPATH="$HOME/.local/state/dws/share/man:$MANPATH"
fpath=($HOME/.local/state/dws/share/zsh/site-functions $fpath)
```

## Lockfile Format

```toml
# ~/.local/state/dws/dws.lock
# Machine-generated - tracks resolved state of installed workspace

version = 1

[metadata]
installed_at = "2025-10-07T12:34:56.789Z"

[[config_symlinks]]
source = "/Users/user/.config/dws/config/zsh/.zshrc"
target = "/Users/user/.config/zsh/.zshrc"

[[config_symlinks]]
source = "/Users/user/.config/dws/config/nvim/init.lua"
target = "/Users/user/.config/nvim/init.lua"

[[tool_symlinks]]
name = "rg"
version = "14.0.0"
source = "/Users/user/.cache/dws/apps/ripgrep/14.0.0/rg"
target = "/Users/user/.local/state/dws/bin/rg"

[[tool_symlinks]]
name = "fd"
version = "9.0.0"
source = "/Users/user/.cache/dws/apps/fd/9.0.0/fd"
target = "/Users/user/.local/state/dws/bin/fd"
```

**Purpose:**
- Tracks exactly what symlinks are installed for this workspace
- Enables reliable cleanup and reinstall
- Provides audit trail for `dws status` and `dws cleanup`
- Detects drift (symlinks changed/removed outside dws)
- Generated/updated on `dws init`, `dws sync`, `dws update`

## Manifest Format

```toml
# ~/.config/dws/manifests/cli.toml

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
curl -fsSL https://dws.ascarter.dev/install.sh | sh
dws init ascarter/dotfiles  # Auto-detects shell from $SHELL
exec $SHELL
```

### Manually cloned workspace
```bash
git clone git@github.com:ascarter/dotfiles.git ~/.config/dws
dws init  # Auto-detects shell
exec $SHELL
```

### Different workspace (different machine/container)
```bash
# On work machine/container
dws init ascarter/work-dotfiles  # Auto-detects shell
exec $SHELL
```

### Multiple shells on same machine
```bash
# Setup for zsh
dws init --shell zsh

# Also setup for bash (updates bash integration only)
dws init --shell bash

# Also setup for fish (updates fish integration only)
dws init --shell fish
```

### Daily sync
```bash
dws sync      # Pull workspace updates, reinstall
dws update    # Check for tool updates, respect pins
```

### Clean reinstall
```bash
dws reset     # Clean git state + reinstall everything
```

### Maintenance
```bash
dws cleanup   # Remove unused cache and orphaned symlinks
dws status    # Show what's installed
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
