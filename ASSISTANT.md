# AI Assistant Context for devspace

This document provides context for AI coding assistants (like Claude Code) working on the devspace project.

## Project Overview

**devspace** is a personal development environment manager that combines dotfile management with tool installation through declarative manifests. It's implemented in Rust as a single binary that can be deployed anywhere.

**Repository**: https://github.com/ascarter/devspace
**License**: MIT
**Language**: Rust (2021 edition)

## Project Vision

### The devspace Concept

A **devspace** is a "profile" of dotfiles + development tools that can be deployed in different environments:

- **Single binary deployment**: Download and run, no dependencies
- **Profile-based**: Different configs for work/personal/projects
- **Platform-aware**: macOS, Linux, BSD (POSIX-focused)
- **Works everywhere**: Host systems, toolbox, devcontainer, codespaces
- **User-maintained**: Profile stored in user's GitHub repository
- **Declarative**: Everything defined in TOML manifests

### Key Goals

1. **Simplicity**: One binary, minimal configuration
2. **Portability**: Works on any POSIX system
3. **Reproducibility**: Same manifest → same environment
4. **Flexibility**: Multiple profiles for different contexts
5. **Speed**: Rust performance for fast operations
6. **Hackability**: Clear code, comprehensive tests

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

### Module Structure

```
devspace/
├── src/
│   ├── main.rs              # Entry point
│   ├── cli.rs               # Clap CLI definitions
│   ├── lib.rs               # Public library interface
│   ├── commands/            # Command implementations
│   │   ├── mod.rs
│   │   ├── init.rs
│   │   ├── config.rs
│   │   ├── app.rs
│   │   ├── profile.rs
│   │   └── doctor.rs
│   ├── config/              # Config & manifest types
│   │   ├── mod.rs
│   │   ├── manifest.rs      # App manifest types
│   │   ├── profile.rs       # Profile configuration
│   │   └── parser.rs        # TOML parsing
│   ├── backends/            # Installer backends
│   │   ├── mod.rs
│   │   ├── ubi.rs           # UBI backend (use as library)
│   │   ├── dmg.rs           # DMG installer (macOS)
│   │   ├── flatpak.rs       # Flatpak (Linux)
│   │   └── curl.rs          # Curl-based installers
│   ├── platform/            # Platform detection
│   │   ├── mod.rs
│   │   └── detect.rs
│   ├── symlinks/            # Symlink management
│   │   ├── mod.rs
│   │   └── manager.rs
│   └── util/                # Utilities
│       ├── mod.rs
│       ├── xdg.rs           # XDG directory helpers
│       └── git.rs           # Git operations
├── tests/
│   └── integration/         # Integration tests
└── examples/                # Usage examples
```

### Core Types (Planned)

```rust
// Manifest types
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

### Phase 1: Foundation (Current)
- [x] GitHub repository created
- [x] Rust project initialized
- [x] CLI scaffolding with clap
- [ ] Basic command structure (no-ops)
- [ ] Verify `--help` output
- [ ] CI/CD setup (GitHub Actions)

### Phase 2: Core Infrastructure
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
- [ ] Cross-compilation (macOS, Linux x86_64/ARM64)
- [ ] GitHub Actions for releases
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
2. Check NOTES.md for current status and decisions
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
