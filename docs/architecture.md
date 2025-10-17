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
# Machine-generated - tracks resolved tool installation state
# Schema version 2 introduces tool_receipts (replaces tool_symlinks)

version = 2

[metadata]
generated_at = "2025-10-07T12:34:56.789Z"
dws_version = "0.1.0"

# Example excerpt

[[config_symlinks]]
source = "/Users/user/.config/dws/profiles/default/config/zsh/.zshrc"
target = "/Users/user/.config/zsh/.zshrc"

[[tool_receipts]]
name = "ripgrep"
installer_kind = "github"
manifest_version = "v14.0.0"
resolved_version = "v14.0.0"
asset = "/Users/user/.cache/dws/downloads/ripgrep-v14.0.0-x86_64-unknown-linux-musl.tar.gz"
checksum = "sha256:deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
asset_regex = "^ripgrep-14\\.0\\.0-x86_64-unknown-linux-musl\\.tar\\.gz$"
asset_size = 9211234
pinned = true
self_update = false
installed_at = "2025-10-07T12:34:56.900Z"
status = "ok"

  [[tool_receipts.binaries]]
  link = "rg"
  source = "/Users/user/.cache/dws/tools/ripgrep/v14.0.0/rg"

  [[tool_receipts.extras]]
  kind = "man"
  source = "/Users/user/.cache/dws/tools/ripgrep/v14.0.0/doc/rg.1"
  target = "/Users/user/.local/state/dws/share/man/man1/rg.1"

[[tool_receipts]]
name = "uv"
installer_kind = "script"
manifest_version = "latest"
resolved_version = "0.1.20"
asset = "/Users/user/.cache/dws/downloads/uv-latest-script"
checksum = "sha256:cafebabe0123456789abcdefcafebabe0123456789abcdefcafebabe01234567"
asset_regex = ""
pinned = false
self_update = true
installed_at = "2025-10-07T12:35:10.000Z"
status = "ok"

  [[tool_receipts.binaries]]
  link = "uv"
  source = "/Users/user/.cache/dws/tools/uv/0.1.20/uv"
```

**Purpose:**
- Captures authoritative receipt per installed tool/version (binaries & extras)
- Enables reliable cleanup, update decisions, and integrity verification
- Provides audit trail (resolved vs manifest version, checksum, asset path)
- Detects drift (missing sources / checksum mismatch)
- Updated atomically after successful add/install/update operations

## `dws.toml` Format

```toml
# ~/.config/dws/profiles/<profile>/dws.toml
# New schema: structured tables for binaries & extras, regex-based asset selection

[tools.ripgrep]
installer = "github"                 # github | gitlab | script
project = "BurntSushi/ripgrep"
version = "v14.0.0"                  # or "latest" (unpinned)
asset_filter = ["^ripgrep-14\\.0\\.0-x86_64-unknown-linux-musl\\.tar\\.gz$"]
checksum = "sha256:deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
self_update = false
platform = ["linux"]

[[tools.ripgrep.bin]]
source = "rg"                        # binary inside archive

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

# Workspace overrides in $XDG_CONFIG_HOME/dws/config.toml replace whole tool entries.
```

### Field Reference

- `installer` *(required)* — Backend identifier (`github`, `gitlab`, `script`).
- `project` — Forge `owner/repo` (GitHub/GitLab) for release installers.
- `version` — Explicit tag (pinned) or `"latest"` (unpinned; still deterministic asset selection).
- `url` — Script download URL (only for `installer = "script"`).
- `shell` — Interpreter for script installers (e.g. `sh`, `bash`).
- `[[tools.<name>.bin]]` — Structured binary entries (`source`, optional `link`).
- `[[tools.<name>.extras]]` — Additional linkables (`source`, `kind` = man|completion|other, optional `shell`, optional explicit `target`).
- `asset_filter` — Ordered list of regex patterns; first that yields a single asset (after scoring/refinement) is used.
- `checksum` — Mandatory `sha256:<hex>` for asset or script content (integrity & reproducibility).
- `self_update` — Tool manages its own updates; `dws update` verifies presence & checksum but does not reinstall.
- `platform` — Optional array of platform tags (e.g. `linux`, `macos`, `linux-ubuntu`). Non-matching entries are treated as errors during validation.
- `hosts` — Optional hostname filters (sanitized). Entry ignored if host does not match.

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
- ✅ Lockfile-based state tracking (v2 receipts planned)
- ✅ Single workspace model (multi-profile support via workspaces directory)
- ✅ Shell auto-detection from $SHELL
- ✅ Template structure with default shell configs
- ✅ Base CI workflow (fmt, clippy, build, test on Ubuntu + macOS)
- 🚧 Refactor: internal forge/script installer backends (github/gitlab/script) replacing `ubi`
- 🚧 New manifest parser (structured bin/extras, asset_filter regex list, mandatory checksum)
- 🚧 Cleanup enhancements (auto repair/remove broken symlinks, prune inactive versions)
- 🚧 Update: status to surface receipt integrity (checksum_mismatch, missing_source) post-refactor
