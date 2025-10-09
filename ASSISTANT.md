# AI Assistant Context for Developer Workspace (dws)

This document provides context for AI coding assistants (like Claude Code) working on the dws project.

## Project Overview

**dws** is a lightweight, portable development environment bootstrapper. It manages dotfiles and development tools through declarative manifests, optimized for interactive development on laptops and workstations.

**Repository**: https://github.com/ascarter/dws
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

### What dws Does

- **Manages dotfiles**: Symlinks zsh/git/editor configs from versioned profile
- **Installs dev tools**: CLI tools, language toolchains (rustup/uv/fnm), native apps
- **Profile switching**: Multiple environments (personal/work/project-specific)
- **Keeps tools updated**: Respects version pins, shows available updates
- **Self-contained**: All state in XDG dirs, single command to uninstall

### What dws Does NOT Do

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
- Platform detection (`tools-macos.toml` vs `tools-linux.toml`)
- Symlink management (dotfiles)
- XDG directory compliance
- Health checks (dev doctor)

**Key files to reference**:
- `lib/app.sh` - App management architecture
- `lib/app/ubi.sh` - UBI backend implementation
- `lib/app/dmg.sh` - DMG backend (macOS)
- `lib/app/flatpak.sh` - Flatpak backend (Linux)
- `lib/app/curl.sh` - Curl-based installers
- `hosts/tools.toml` - Base tool manifest example
- `hosts/tools-macos.toml` - macOS-specific manifest
- `hosts/tools-linux.toml` - Linux-specific manifest
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
Workspace                    // Root context (~/.config/dws/)
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

### Module Structure (Current)

```
dws/
├── src/
│   ├── main.rs          # Binary entry point
│   ├── lib.rs           # Public API exports
│   ├── cli.rs           # Clap command definitions
│   ├── commands/        # Subcommand implementations (init, sync, etc.)
│   ├── config.rs        # Dotfile discovery and symlinking
│   ├── environment.rs   # Shell environment emission
│   ├── manifest.rs      # Manifest parsing and tool definitions
│   ├── lockfile.rs      # Lockfile serialization (TBD wiring)
│   └── workspace.rs     # Workspace orchestration and templates
├── templates/           # Embedded workspace starter files
├── tests/cli_test.rs    # CLI integration harness (ignored pending isolation)
└── docs/architecture.md # Narrative design reference
```

### Core Types (Rust)

```rust
pub struct Workspace {
    workspace_dir: PathBuf, // ~/.config/dws
    state_dir: PathBuf,     // ~/.local/state/dws
}

pub struct Config {
    config_dir: PathBuf,
    target_dir: PathBuf,
    ignore_patterns: Vec<String>,
}

#[derive(Clone)]
pub struct ConfigEntry {
    pub source: PathBuf,
    pub target: PathBuf,
}

pub struct Environment {
    pub bin_path: PathBuf,
    pub man_path: PathBuf,
    pub completions_path: PathBuf,
}

pub struct Lockfile {
    version: u32,
    pub metadata: Metadata,
    pub config_symlinks: Vec<SymlinkEntry>,
    pub tool_symlinks: Vec<ToolEntry>,
}

pub struct ManifestSet {
    entries: Vec<ManifestEntry>,
}

pub struct ManifestEntry {
    pub name: String,
    pub source: PathBuf,
    pub definition: ToolDefinition,
}

pub struct ToolDefinition {
    pub installer: InstallerKind,
    pub project: Option<String>,
    pub version: Option<String>,
    pub url: Option<String>,
    pub shell: Option<String>,
    pub bin: Vec<String>,
    pub symlinks: Vec<String>,
    pub app: Option<String>,
    pub team_id: Option<String>,
    pub self_update: bool,
}

pub enum InstallerKind {
    Ubi,
    Dmg,
    Flatpak,
    Curl,
}
```

`Workspace` is the façade used by CLI commands. It exposes helpers for initialization, template installation, and shell integration. `Config` discovers and installs symlinks from `workspace/config/` into `$XDG_CONFIG_HOME`. `Environment` renders `dws env` output so shells can source a deterministic PATH. `ManifestSet` loads typed tool definitions from `workspace/manifests/`, applying overrides in config order (base `tools.toml` → platform manifest such as `tools-macos.toml` → host-specific files like `tools-<hostname>.toml`). `Lockfile` currently serializes state but still needs to be integrated with install/update flows.

### Planned Extensions

The roadmap introduces richer layering while keeping the current modules intact:

- **Manifest parsing (`Manifest` module)**: strongly typed TOML manifests for tool installation.
- **Installer backends**: async traits for UBI, DMG, Flatpak, and Curl implementations.
- **Profiles / workspaces per context**: potential evolution if multi-profile support is reintroduced.
- **Platform abstraction**: detect OS/architecture to select manifests and installers.

When implementing these pieces, update `docs/architecture.md` with the final shape and extend `AGENTS.md` so all assistants share the same workflow guidance.

## Development Guidelines

### Pre-Commit Rules

**IMPORTANT**: Before every commit, you MUST:

1. **Review all documentation** - Check README.md, ASSISTANT.md, docs/architecture.md
   - Ensure they accurately reflect the current code state
   - Update any outdated references, examples, or architecture descriptions
   - Verify command examples match current CLI

2. **Review all code comments** - Check inline comments, doc comments, TODOs
   - Update comments that reference old code patterns or names
   - Ensure TODOs are still relevant and accurate
   - Verify doc comments match actual function signatures

3. **Search for stale references** - Use grep/search to find:
   - Old project names or terminology
   - Outdated path references
   - Removed features still mentioned in comments
   - Changed command names or flags

4. **Run all tests** - Ensure nothing breaks
   - Unit tests
   - Integration tests
   - Doc tests

**Rationale**: Documentation and comments rot quickly. Regular, systematic review prevents the codebase from becoming confusing or misleading. This is especially critical after refactoring, renaming, or architectural changes.

### Code Style

- **Rust 2021 edition**: Use modern Rust idioms
- **Error handling**: Use `anyhow::Result` for applications, `thiserror` for libraries
- **Async**: Keep workflows synchronous for now; add an async runtime (Tokio) when installer backends need it
- **Logging**: Use `tracing` crate with structured logging
- **CLI**: Use `clap` v4 with derive macros
- **Testing**: Comprehensive unit and integration tests
- **Documentation**: rustdoc comments for all public APIs

**Comments:**
- **Only write valuable comments** - Comments should explain WHY, not WHAT
- **Never restate what the code obviously does** - Bad: `// Create directory`, Good: `// Ensure parent exists to avoid ENOENT`
- **Explain non-obvious decisions** - Why you chose an approach, edge cases handled, assumptions made
- **No comments is better than pointless comments** - If the code is clear, don't add commentary
- Examples:
  ```rust
  // BAD - restates the obvious
  // Auto-detect shell if not provided
  let shell = match shell {
      Some(s) => s,
      None => detect_shell()?,
  };

  // GOOD - explains why
  // Normalize URLs for comparison (handle .git suffix differences)
  let expected_normalized = expected_url.trim_end_matches(".git");

  // BEST - code is self-documenting, no comment needed
  let shell = match shell {
      Some(s) => s,
      None => detect_shell()?,
  };
  ```

### Testing Strategy

**Two types of tests in Rust:**

1. **Unit tests** (`#[cfg(test)] mod tests` in source files)
   - Test individual functions and methods
   - Have access to private functions
   - Live in the same file as the code they test
   - Example: `src/commands/init.rs` tests shell detection logic

2. **Integration tests** (`tests/` directory)
   - Test the public API as a black box
   - Each file is compiled as a separate test binary
   - Test real command execution (e.g., using `assert_cmd`)
   - Example: `tests/cli_test.rs` tests the actual CLI binary end-to-end

**Test Naming Conventions:**

Follow Rust idioms - describe the scenario being tested, not the expected outcome:

```rust
// Good - describes scenario
#[test]
fn test_detect_shell_when_unset() { ... }

#[test]
fn test_config_with_multiple_entries() { ... }

// Less idiomatic - describes outcome
#[test]
fn test_detect_shell_fails() { ... }

#[test]
fn test_config_returns_three() { ... }
```

**Parameterized Tests with rstest:**

Use `rstest` for testing multiple cases (more idiomatic than loops):

```rust
use rstest::rstest;

#[rstest]
#[case("/bin/zsh", "zsh")]
#[case("/usr/bin/bash", "bash")]
#[case("/bin/fish", "fish")]
fn test_detect_shell(#[case] shell_path: &str, #[case] expected: &str) {
    env::set_var("SHELL", shell_path);
    assert_eq!(detect_shell().unwrap(), expected);
}
```

Benefits:
- Each case runs as a separate test (shows up individually in output)
- If one case fails, others still run
- Clear test output showing which specific case failed

**Fixtures with rstest:**

```rust
#[fixture]
fn temp_workspace() -> TempDir {
    TempDir::new().unwrap()
}

#[rstest]
fn test_with_fixture(temp_workspace: TempDir) {
    // Use fixture
}
```

**Platform-Specific Tests:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_platform() {
        // Runs on all platforms
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_only() {
        // macOS-specific test
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_linux_only() {
        // Linux-specific test
    }
}
```

**Serial Tests (when tests modify shared state):**

```rust
use serial_test::serial;

#[test]
#[serial]
fn test_modifies_env() {
    env::set_var("FOO", "bar");
    // ...
    env::remove_var("FOO");
}
```

### Dependencies

**Core**:
- `clap` - CLI parsing
- `serde` + `toml` - Config parsing
- `anyhow` + `thiserror` - Error handling
- `tracing` - Structured logging

**Utilities**:
- `directories` - XDG directories
- `git2` - Git operations (for profile cloning)

**Future** (when implementing backends):
- Async runtime (Tokio) once installers need concurrent downloads
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

### Phase 2: Core Infrastructure ✅
- [x] XDG directory helpers (Workspace type)
- [x] Config management (symlink discovery, installation)
- [x] Lockfile tracking (state management)
- [x] Environment generation (shell integration)
- [x] Init command (template creation, git clone, shell setup)
- [x] Tests for core functionality (27 passing unit tests)

### Phase 3: Feature Implementation (Next)
Priority order:
1. **TOML manifest parsing** - Parse tool manifests from workspace
2. **Platform detection** - Detect macOS, Linux, BSD for conditional manifests
3. **UBI backend integration** - Install tools from GitHub releases
4. **Status command** - Show installed tools and versions
5. **Sync command** - Pull workspace, install/update tools
6. **Update command** - Update tools (respect pins)
7. **DMG backend** (macOS) - Install macOS apps
8. **Flatpak backend** (Linux) - Install Linux apps
9. **Curl backend** - Custom install scripts
10. **Advanced features** - Parallel installs, doctor, version checking

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
# Base tools (tools.toml)
[ripgrep]
installer = "ubi"
project = "BurntSushi/ripgrep"
bin = ["rg"]
symlinks = [
  "doc/rg.1:${XDG_DATA_HOME}/man/man1/rg.1",
  "complete/_rg:${XDG_DATA_HOME}/zsh/completions/_rg"
]

# macOS-specific (tools-macos.toml)
[ghostty]
installer = "dmg"
url = "https://ghostty.org/download"
app = "Ghostty.app"
team_id = "24VZTF6M5V"
self_update = true

# Linux-specific (tools-linux.toml)
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
   - `tools.toml`: Base tools shared across platforms
   - `tools-macos.toml`: macOS-only apps
 - `tools-linux.toml`: Linux-only apps
  - `tools-local.toml`: Fallback used when hostname can't be sanitized

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
dws init [shell] [url|user/repo] [--name <profile>]
dws clone <url|user/repo> [--name <profile>]
```

### Profile Management
```bash
dws use <profile>         # Switch profile
dws list                  # List profiles
```

### Daily Operations
```bash
dws sync                  # Pull + install + respect pins
dws update [tool]         # Update tools (respect pins)
dws status                # Show status
```

### Maintenance
```bash
dws doctor                # Health check + repair
dws self                  # Show info
dws self update           # Update dws
dws self uninstall        # Remove all
```

### Environment (Shell Integration)
```bash
dws env [profile]         # Output env setup
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
