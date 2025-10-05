# AI Assistant Context for Developer Workspace (devws)

This document provides context for AI coding assistants (like Claude Code) working on the devws project.

## Project Overview

**devws** is a lightweight, portable development environment bootstrapper. It manages dotfiles and development tools through declarative manifests, optimized for interactive development on laptops and workstations.

**Repository**: https://github.com/ascarter/devws
**License**: MIT
**Language**: Rust (2021 edition)

## Project Vision

### Core Goals

1. **Bootstrap new machines quickly**: Single binary → full dev environment
2. **Lightweight & portable**: No heavy package managers, works on any POSIX system
3. **XDG compliant**: Self-contained in standard directories, easy to remove
4. **Native tool experience**: Use rustup/uv/fnm directly, no runtime overhead
5. **Version controlled**: Dotfiles + manifests in GitHub
6. **Zero impact**: Symlinks in standard locations, no PATH hacks

### What devws Does

- **Manages dotfiles**: Symlinks zsh/git/editor configs from versioned profile
- **Installs dev tools**: CLI tools, language toolchains (rustup/uv/fnm), native apps
- **Profile switching**: Multiple environments (personal/work/project-specific)
- **Keeps tools updated**: Respects version pins, shows available updates
- **Self-contained**: All state in XDG dirs, single command to uninstall

### What devws Does NOT Do

- ❌ Runtime environment switching (not direnv/mise)
- ❌ Version management (use rustup/uv/fnm for that)
- ❌ Shims/wrappers (native binaries, direct PATH)
- ❌ CI/production builds (for interactive dev only)

## Prior Art & Reference Implementation

### Shell-based Prototype

**Location**: `/Users/acarter/.local/share/dev` (on development machine)
**Repository**: Private implementation (may be made public)

The shell-based prototype demonstrates all core functionality:
- Manifest-based app management (TOML)
- Multiple installer backends (UBI, DMG, Flatpak, Curl)
- Platform detection (macos.toml vs linux.toml)
- Symlink management (dotfiles)
- XDG directory compliance
- Health checks (dev doctor)

**Key files to reference**:
- `lib/app.sh` - App management architecture
- `lib/app/ubi.sh` - UBI backend implementation
- `lib/app/dmg.sh` - DMG backend (macOS)
- `lib/app/flatpak.sh` - Flatpak backend (Linux)
- `lib/app/curl.sh` - Curl-based installers
- `hosts/cli.toml` - Cross-platform tool manifest example
- `hosts/macos.toml` - macOS-specific manifest
- `hosts/linux.toml` - Linux-specific manifest
- `docs/app-management.md` - Complete documentation

### Key Learnings from Shell Implementation

**What works well**:
1. **Manifest-based approach**: Declarative TOML manifests are intuitive
2. **Platform separation**: Different manifests for different OSes
3. **bin vs symlinks**: Separate arrays for executables vs supplementary files
4. **Smart defaults**: Most apps don't need much configuration
5. **XDG compliance**: Standard directory locations
6. **Health checks**: `dev doctor` finds and fixes issues
7. **Self-update flag**: For apps with built-in update mechanisms

**What to improve in Rust**:
1. **Performance**: Native TOML parsing (no yq subprocess)
2. **Parallelism**: Install multiple apps simultaneously
3. **Error handling**: Better error messages with context
4. **Version checking**: Without spawning processes
5. **Type safety**: Manifest validation at parse time
6. **Progress output**: Structured, beautiful progress bars
7. **Profiles**: Easy switching between contexts

## Architecture

### Design Principles

**Rust Data-Oriented Design**:
- Use proper Rust types to represent domain concepts
- Implement behavior through `impl` blocks on types
- Clear ownership and composition relationships
- NOT procedural/shell-script style with loose functions
- Fully unit testable at every level
- Type-safe relationships between entities

**Type Hierarchy**:
```
Workspace                    // Root context (~/.config/devws/)
  └─ Profile                 // Active or named profile
       ├─ Config             // Config file management (symlinked dotfiles)
       ├─ Environment        // Shell environment for this profile
       └─ Manifest (future)  // Tool installation manifests
```

**Usage Pattern**:
```rust
// CLI creates workspace, gets profile, accesses typed components
let workspace = Workspace::new()?;
let profile = workspace.get_profile("default")?;
let env = profile.environment(Shell::Zsh)?;
println!("{}", env.format());
```

**Key Rules**:
1. Types own their data and behavior
2. Composition over loose coupling
3. Methods return typed results, not raw primitives
4. Each module has comprehensive unit tests
5. Public API surfaces types, not implementation details

### Module Structure

```
devws/
├── src/
│   ├── main.rs              # Entry point
│   ├── cli.rs               # Clap CLI definitions
│   ├── lib.rs               # Public library interface
│   ├── commands/            # Command implementations (thin layer)
│   │   └── mod.rs           # Dispatch to workspace/profile methods
│   │
│   ├── config/              # Core domain types
│   │   ├── mod.rs           # Re-exports public types
│   │   ├── workspace.rs     # Workspace type (root context)
│   │   ├── profile.rs       # Profile type and management
│   │   ├── config.rs        # Config type (dotfiles/symlinks)
│   │   └── environment.rs   # Environment type (shell env generation)
│   │
│   ├── manifest/            # Tool manifest types (future)
│   │   ├── mod.rs
│   │   └── parser.rs        # TOML manifest parsing
│   │
│   ├── backends/            # Tool installer backends (future)
│   │   ├── mod.rs
│   │   ├── ubi.rs           # UBI backend
│   │   ├── dmg.rs           # DMG installer (macOS)
│   │   ├── flatpak.rs       # Flatpak (Linux)
│   │   └── curl.rs          # Curl-based installers
│   │
│   └── platform/            # Platform detection (future)
│       └── mod.rs
│
├── tests/
│   └── integration/         # Integration tests
└── templates/               # Embedded profile templates
```

### Core Types

```rust
// Workspace - root entry point
pub struct Workspace {
    config_dir: PathBuf,  // ~/.config/devws
    state_dir: PathBuf,   // ~/.local/state/devws
}

impl Workspace {
    pub fn new() -> Result<Self>;
    pub fn get_profile(&self, name: &str) -> Result<Profile>;
    pub fn active_profile(&self) -> Result<Profile>;
    pub fn list_profiles(&self) -> Result<Vec<Profile>>;
    pub fn create_profile(&self, name: &str) -> Result<Profile>;
}

// Profile - represents a dev environment profile
pub struct Profile {
    name: String,
    path: PathBuf,
    workspace: Workspace,  // Reference to parent
}

impl Profile {
    pub fn config(&self) -> Result<Config>;
    pub fn environment(&self, shell: Shell) -> Result<Environment>;
    pub fn activate(&self) -> Result<()>;
    // Future: pub fn manifest(&self) -> Result<Manifest>;
}

// Config - manages dotfiles/config file symlinking
pub struct Config {
    config_dir: PathBuf,
    entries: Vec<ConfigEntry>,
    ignore_patterns: Vec<String>,
}

impl Config {
    pub fn new(profile_path: &Path) -> Result<Self>;
    pub fn discover_entries(&self, target_dir: &Path) -> Result<Vec<ConfigEntry>>;
    pub fn install(&self, target_dir: &Path) -> Result<()>;
    pub fn uninstall(&self, target_dir: &Path) -> Result<()>;
}

// ConfigEntry - a single config file to symlink
pub struct ConfigEntry {
    pub source: PathBuf,
    pub target: PathBuf,
}

// Environment - shell environment for a profile
pub struct Environment {
    bin_path: PathBuf,
    man_path: PathBuf,
    completions_path: PathBuf,
}

impl Environment {
    pub fn new(profile: &Profile) -> Result<Self>;
    pub fn format(&self, shell: Shell) -> String;
}

// Manifest types (future)
struct AppManifest {
    apps: HashMap<String, AppConfig>,
}

struct AppConfig {
    installer: InstallerType,
    project: Option<String>,      // For UBI
    url: Option<String>,           // For DMG/Curl
    bin: Vec<String>,              // Binaries to symlink
    symlinks: Vec<String>,         // Supplementary files
    // ... platform-specific fields
}

enum InstallerType {
    Ubi,
    Dmg,
    Flatpak,
    Curl,
}

// Profile types
struct Profile {
    name: String,
    path: PathBuf,
    config: ProfileConfig,
}

struct ProfileConfig {
    shell: Option<String>,
    manifests: Vec<PathBuf>,
    // ... other settings
}
```

### Installer Backend Trait

```rust
#[async_trait]
trait Backend {
    async fn install(&self, app: &AppConfig) -> Result<()>;
    async fn uninstall(&self, app: &AppConfig) -> Result<()>;
    async fn status(&self, app: &AppConfig) -> Result<InstallStatus>;
    async fn update(&self, app: &AppConfig) -> Result<()>;
}
```

## Development Guidelines

### Code Style

- **Rust 2021 edition**: Use modern Rust idioms
- **Error handling**: Use `anyhow::Result` for applications, `thiserror` for libraries
- **Async**: Use `tokio` for async operations
- **Logging**: Use `tracing` crate with structured logging
- **CLI**: Use `clap` v4 with derive macros
- **Testing**: Comprehensive unit and integration tests
- **Documentation**: rustdoc comments for all public APIs

### Testing Strategy

1. **Unit tests**: Test individual functions and modules
2. **Integration tests**: Test command execution end-to-end
3. **Example tests**: Examples that also serve as documentation
4. **Platform tests**: Conditional compilation for platform-specific code

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_manifest() {
        // ...
    }

    #[tokio::test]
    async fn test_install_app() {
        // ...
    }
}

#[cfg(target_os = "macos")]
mod macos_tests {
    // macOS-specific tests
}
```

### Dependencies

**Core**:
- `clap` - CLI parsing
- `serde` + `toml` - Config parsing
- `anyhow` + `thiserror` - Error handling
- `tokio` - Async runtime
- `tracing` - Structured logging

**Utilities**:
- `directories` - XDG directories
- `git2` - Git operations (for profile cloning)

**Future** (when implementing backends):
- `ubi` - As a library dependency
- Platform-specific crates as needed

## Implementation Roadmap

### Phase 1: Foundation ✅
- [x] GitHub repository created
- [x] Rust project initialized
- [x] CLI scaffolding with clap
- [x] Basic command structure (no-ops)
- [x] Verify `--help` output
- [x] CI/CD setup (GitHub Actions)

### Phase 2: Core Infrastructure (Next)
- [ ] TOML manifest parsing
- [ ] Manifest type definitions
- [ ] Platform detection (macOS, Linux, BSD)
- [ ] Error types
- [ ] XDG directory helpers
- [ ] Logging setup
- [ ] Tests for core functionality

### Phase 3: Feature Implementation
Priority order:
1. Profile loading (local directory)
2. Symlink management
3. UBI backend integration
4. App status/list commands
5. Config management
6. DMG backend (macOS)
7. Flatpak backend (Linux)
8. Curl backend
9. Profile cloning (GitHub)
10. Advanced features (parallel installs, doctor, version checking)

### Phase 4: Release
- [x] Cross-compilation (macOS, Linux x86_64/ARM64)
- [x] GitHub Actions for releases (dormant until tagged)
- [ ] macOS code signing (requires Apple Developer account)
- [ ] Installation script
- [ ] Profile repository template
- [ ] User documentation
- [ ] Migration guide from shell version

## Manifest Format Reference

See shell implementation for working examples. Key concepts:

### App Manifest Structure

```toml
# Cross-platform tools (cli.toml)
[ripgrep]
installer = "ubi"
project = "BurntSushi/ripgrep"
bin = ["rg"]
symlinks = [
  "doc/rg.1:${XDG_DATA_HOME}/man/man1/rg.1",
  "complete/_rg:${XDG_DATA_HOME}/zsh/completions/_rg"
]

# macOS-specific (macos.toml)
[ghostty]
installer = "dmg"
url = "https://ghostty.org/download"
app = "Ghostty.app"
team_id = "24VZTF6M5V"
self_update = true

# Linux-specific (linux.toml)
[vivaldi]
installer = "flatpak"
app_id = "com.vivaldi.Vivaldi"
remote = "flathub"
```

### Important Patterns

1. **bin vs symlinks**:
   - `bin`: Executables → `$XDG_BIN_HOME`
   - `symlinks`: Supplementary files (man pages, completions)

2. **Platform-specific manifests**:
   - `cli.toml`: Cross-platform tools
   - `macos.toml`: macOS-only apps
   - `linux.toml`: Linux-only apps

3. **Smart defaults**:
   - `bin` defaults to `["<app_name>"]`
   - `check_cmd` tries `--version` then `version`
   - `shell` defaults to `"sh"` for curl installers

4. **Environment variable expansion**:
   - `${XDG_DATA_HOME}`, `${XDG_BIN_HOME}`, etc.

## CLI Commands (v2 Design)

**See `docs/architecture.md` for complete architecture**

### Bootstrap
```bash
devws init [shell] [url|user/repo] [--name <profile>]
devws clone <url|user/repo> [--name <profile>]
```

### Profile Management
```bash
devws use <profile>         # Switch profile
devws list                  # List profiles
```

### Daily Operations
```bash
devws sync                  # Pull + install + respect pins
devws update [tool]         # Update tools (respect pins)
devws status                # Show status
```

### Maintenance
```bash
devws doctor                # Health check + repair
devws self                  # Show info
devws self update           # Update devws
devws self uninstall        # Remove all
```

### Environment (Shell Integration)
```bash
devws env [profile]         # Output env setup
```

## Common Tasks

### Adding a New Command

1. Add variant to `Commands` enum in `src/cli.rs`
2. Implement handler in `src/commands/mod.rs` or dedicated module
3. Add integration test in `tests/integration/`
4. Update README with usage example

### Adding a New Backend

1. Create `src/backends/<backend>.rs`
2. Implement `Backend` trait
3. Add to backend factory/dispatcher
4. Add unit tests
5. Add integration test with example manifest
6. Document in README and rustdoc

### Running Tests

```bash
# All tests
cargo test

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture

# Integration tests only
cargo test --test integration
```

## Environment Variables

- `RUST_LOG`: Control tracing output (e.g., `RUST_LOG=debug`)
- `XDG_CONFIG_HOME`: Config directory (default: `~/.config`)
- `XDG_DATA_HOME`: Data directory (default: `~/.local/share`)
- `XDG_BIN_HOME`: Binary directory (default: `~/.local/bin`)

## Platform-Specific Notes

### macOS
- DMG mounting requires `hdiutil` (built-in)
- Code signature verification uses `codesign` (built-in)
- App bundles go to `/Applications` or `~/Applications`

### Linux
- Flatpak requires `flatpak` command
- Desktop files go to `~/.local/share/applications`
- D-Bus used for Flatpak communication

### BSD
- Should work like Linux for most things
- May need platform-specific handling for package management

## Resources

- **Shell implementation**: `/Users/acarter/.local/share/dev`
- **UBI**: https://github.com/houseabsolute/ubi
- **Clap**: https://docs.rs/clap/
- **Tokio**: https://tokio.rs/
- **Tracing**: https://docs.rs/tracing/
- **XDG Base Directory**: https://specifications.freedesktop.org/basedir-spec/

## Getting Help

When working on this project:
1. Reference the shell implementation for behavior
2. Check `.claude/session-notes.md` for recent session details (git-ignored)
3. Look at existing tests for patterns
4. Run `cargo doc --open` for API docs
5. Check GitHub issues for known problems

## Notes for AI Assistants

- **Context preservation**: This project ports a working shell implementation to Rust
- **Reference implementation**: Always check `/Users/acarter/.local/share/dev` for behavior
- **Test-driven**: Write tests before/during implementation
- **Incremental**: One feature at a time, fully tested and documented
- **User-focused**: The end user is a developer managing their own environment
- **Performance matters**: But correctness and clarity come first
