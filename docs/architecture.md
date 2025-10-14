# Developer Workspace (dws) Design v3 (2025-10-07)

## Architecture

### Directory Structure

```
~/.config/dws/                # dws workspace root (reserved for tooling)
  config.toml                 # Workspace configuration (active profile + overrides)
  profiles/                   # User profile repositories (git)
    <profile>/
      config/                 # XDG config files → symlinked to ~/.config
      dws.toml                # Profile-local tool definitions
      README.md

~/.local/state/dws/           # XDG_STATE_HOME (local execution state)
  dws.lock                    # Lockfile tracking installed state
  bin/                        # Tool symlinks → cache
  share/
    man/
    zsh/site-functions/

~/.cache/dws/                 # XDG_CACHE_HOME (downloaded binaries)
  tools/
    <tool>/<version>/         # Actual binaries (can be cleared/rebuilt)
```

### Key Design Decisions

1. **XDG-only approach**: dws is purpose-built for XDG Base Directory layout
   - Config files symlink to `$XDG_CONFIG_HOME` (default: `~/.config`)
   - No support for dotfiles in home directory root
   - Structure mirrors XDG: `dws/profiles/<profile>/config/zsh/.zshrc` → `~/.config/zsh/.zshrc`
2. **Profile model**: user content lives under `~/.config/dws/profiles/<profile>`
   - Profiles are version-controlled by the user; the workspace root holds metadata only
   - `config.toml` records the active profile so `dws use <profile>` can switch safely
   - Different environments = different machines/containers (multi-profile ready)
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
6. **Version pinning**: Tool entries in `dws.toml` (and workspace overrides) can pin versions, and `update` respects those pins.
7. **Tool override precedence**: Profile `dws.toml` files define the base set; the workspace-level `$XDG_CONFIG_HOME/dws/config.toml` can add or replace entire entries that match the current platform/host filters.
8. **Wrapper scripts only when needed**: For tools requiring LD_LIBRARY_PATH, etc.
9. **Profile management commands**: `dws clone`, `dws use`, and `dws profiles` manage the lifecycle of profiles under `profiles/`.

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
dws profiles                     # List profiles (active profile marked)
dws use <profile>                # Switch to another profile

# Profile management
dws clone <repo> [--profile name] # Clone additional profile into profiles/

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

# ~/.local/state/dws/dws.lock (example excerpt)

[[config_symlinks]]
source = "/Users/user/.config/dws/profiles/default/config/zsh/.zshrc"
target = "/Users/user/.config/zsh/.zshrc"

[[config_symlinks]]
source = "/Users/user/.config/dws/profiles/default/config/nvim/init.lua"
target = "/Users/user/.config/nvim/init.lua"

[[tool_symlinks]]
name = "rg"
version = "14.0.0"
source = "/Users/user/.cache/dws/tools/ripgrep/14.0.0/rg"
target = "/Users/user/.local/state/dws/bin/rg"

[[tool_symlinks]]
name = "fd"
version = "9.0.0"
source = "/Users/user/.cache/dws/tools/fd/9.0.0/fd"
target = "/Users/user/.local/state/dws/bin/fd"
```

**Purpose:**
- Tracks exactly what symlinks are installed for this workspace
- Enables reliable cleanup and reinstall
- Provides audit trail for `dws status` and `dws cleanup`
- Detects drift (symlinks changed/removed outside dws)
- Generated/updated on `dws init`, `dws sync`, `dws update`

## `dws.toml` Format

```toml
# ~/.config/dws/profiles/<profile>/dws.toml

[tools.ripgrep]
installer = "ubi"
project = "BurntSushi/ripgrep"
platform = ["macos", "linux"]

[tools.uv]
installer = "curl"
url = "https://astral.sh/uv/install.sh"
shell = "sh"
self_update = true

# Workspace overrides live in $XDG_CONFIG_HOME/dws/config.toml and
# replace entire tool entries when names match.
#
# [tools.ripgrep]
# installer = "ubi"
# project = "BurntSushi/ripgrep"
# version = "latest"
```

### Field Reference

- `installer` *(required)* — Backend identifier (`ubi`, `curl`, `dmg`, `flatpak`).
- `project` — GitHub `owner/repo` used by release-based installers.
- `version` — Explicit version pin; omit for backend default/latest.
- `url` — Direct download endpoint for script or disk image installers.
- `shell` — Interpreter for installer scripts (e.g. `sh`, `bash`).
- `bin` — Executables to link into the workspace `bin/` directory (defaults to the release binary name for `ubi` when omitted).
- `symlinks` — Extra files to link, using `source:target` syntax.
- `app` — macOS `.app` bundle name extracted from a DMG.
- `team_id` — Apple Developer team identifier for notarization verification.
- `self_update` — Set to `true` for tools that maintain themselves; they will be skipped by `dws update`.
- `platform` — Optional array of platform tags. Tags use `std::env::consts::OS` values (`macos`, `linux`) plus distro-specific slugs like `linux-ubuntu` and `linux-debian` derived from `/etc/os-release`. Arch sub-tags (e.g. `macos-aarch64`) are also available. Windows is not supported; use WSL when necessary.
- `hosts` — Optional array of sanitized hostnames. Values are compared case-insensitively after converting non-alphanumeric characters to `-`.

### Precedence & Layering

1. **Profile `dws.toml`** — committed alongside the profile repository; establishes the default tool set for that profile.
2. **Workspace `config.toml`** — stored at `$XDG_CONFIG_HOME/dws/config.toml`; may add new tools or replace entire tool entries from the active profile. Workspace entries only apply when their platform/host filters match the current machine.

Because overrides replace entire entries, workspace files must restate every field they care about. This keeps layering predictable and avoids partially merged tool definitions.

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
- ✅ Manifest parsing (typed merges across precedence)
- ⏳ Cleanup command (remove unused cache, orphaned symlinks)
- ⏳ Status command (read lockfile, show installed state)
