# Development Notes

## Current Status (2025-10-02)

**Phase**: Foundation - Initial scaffolding complete

The devspace project is in early development. We're porting a working shell-based implementation to Rust for better performance, type safety, and advanced features like profiles.

### Completed
- ✅ GitHub repository created
- ✅ Rust project initialized with Cargo.toml
- ✅ CLI structure defined with clap
- ✅ Command scaffolding (no-op implementations)
- ✅ README.md with project overview
- ✅ ASSISTANT.md with full context
- ✅ NOTES.md (this file)

### Next Steps
1. Verify `devspace --help` output and command structure
2. Set up GitHub Actions for CI/CD
3. Add basic integration tests
4. Begin Phase 2: Core infrastructure

## Project Context

### Origin Story

This project evolved from a shell-based development environment manager (`dev`). The shell version proved the concept works:
- Manifest-based app management
- Platform-aware (macOS/Linux)
- XDG-compliant
- Multiple installer backends (UBI, DMG, Flatpak, Curl)

**Reference implementation**: `/Users/acarter/.local/share/dev`

### Why Rust?

The shell version works but has limitations:
- Performance: Subprocess overhead for every operation
- Type safety: No compile-time validation
- Error handling: Shell error handling is primitive
- Parallelism: Hard to do safely in shell
- Advanced features: Profiles system difficult in shell

Rust provides:
- 10-50x faster manifest parsing (native TOML)
- 2-5x faster operations (no process spawns)
- Type-safe manifests (catch errors early)
- Built-in UBI library (no subprocess)
- Easy parallelism (tokio)
- Better error messages with context

## Implementation Roadmap

### Phase 1: Foundation ✅
- [x] GitHub repository
- [x] Rust project structure
- [x] CLI scaffolding with clap
- [ ] Verify ergonomics
- [ ] GitHub Actions (CI)
- [ ] Basic integration tests

### Phase 2: Core Infrastructure (Next)
- [ ] TOML manifest parsing with serde
- [ ] Manifest type definitions
- [ ] Platform detection (macOS, Linux, BSD)
- [ ] Error types (thiserror)
- [ ] XDG directory helpers
- [ ] Logging setup (tracing)
- [ ] Unit tests for all core modules

### Phase 3: Feature Implementation
Priority order (one at a time, with tests + docs):

1. **Profile loading** (local directory)
   - Load profile from filesystem
   - Parse devspace.toml
   - Load manifest files
   - Tests + documentation

2. **Symlink management**
   - Create/remove symlinks
   - Handle ${XDG_*} variable expansion
   - Detect broken symlinks
   - Tests for edge cases

3. **UBI backend**
   - Integrate ubi as library
   - Install/uninstall/status
   - Parallel downloads
   - Platform-specific binary detection

4. **Status/list commands**
   - List apps from manifests
   - Show installation status
   - Pretty output formatting

5. **Config management**
   - Link dotfiles
   - Track symlink state
   - Unlink cleanly

6. **Additional backends**
   - DMG (macOS) - mount, verify, install
   - Flatpak (Linux) - D-Bus communication
   - Curl - pipe to shell with safety checks

7. **Profile management**
   - Clone from GitHub
   - Switch between profiles
   - Create new profiles

8. **Advanced features**
   - Parallel installs
   - Version checking
   - Doctor command (health checks)
   - Self-update

### Phase 4: Release
- [ ] Cross-compilation (multiple platforms/architectures)
- [ ] GitHub Actions for releases
- [ ] Installation script (curl | sh)
- [ ] Profile repository template
- [ ] User guide
- [ ] Migration guide from shell version

## Design Decisions

### CLI Design

**Follows shell version but improves**:
```bash
# Core commands
devspace init [shell]           # Shell integration
devspace env                    # Export environment
devspace status                 # Overall status

# Config management
devspace config status          # Show symlink status
devspace config link            # Link configs
devspace config unlink          # Unlink configs

# App management
devspace app list               # List all apps
devspace app status [name]      # Status (all or specific)
devspace app install [name]     # Install (all or specific)
devspace app update [name]      # Update (all or specific)
devspace app uninstall <name>   # Uninstall specific

# Profile management (NEW)
devspace profile list           # List profiles
devspace profile current        # Show current
devspace profile clone <repo>   # Clone from GitHub
devspace profile activate <name> # Switch profile
devspace profile create <name>  # Create new

# Health checks
devspace doctor                 # Check & repair
```

### Profile Structure

```
~/.config/devspace/
├── profiles/
│   ├── personal/
│   │   ├── devspace.toml       # Profile config
│   │   ├── config/             # Dotfiles
│   │   │   ├── zsh/
│   │   │   ├── git/
│   │   │   └── ...
│   │   └── manifests/          # App manifests
│   │       ├── cli.toml
│   │       ├── macos.toml
│   │       └── linux.toml
│   └── work/
│       └── ...
└── active -> profiles/personal  # Symlink to active profile
```

### Manifest Format

**Follows shell implementation exactly** (proven to work):

```toml
# UBI backend (GitHub releases)
[ripgrep]
installer = "ubi"
project = "BurntSushi/ripgrep"
bin = ["rg"]                    # Binaries → $XDG_BIN_HOME
symlinks = [                    # Supplementary files
  "doc/rg.1:${XDG_DATA_HOME}/man/man1/rg.1",
  "complete/_rg:${XDG_DATA_HOME}/zsh/completions/_rg"
]

# DMG backend (macOS)
[ghostty]
installer = "dmg"
url = "https://ghostty.org/download"
app = "Ghostty.app"
team_id = "24VZTF6M5V"
self_update = true

# Flatpak backend (Linux)
[vivaldi]
installer = "flatpak"
app_id = "com.vivaldi.Vivaldi"
remote = "flathub"

# Curl backend (install scripts)
[claude]
installer = "curl"
url = "https://claude.ai/install.sh"
shell = "bash"
update_cmd = "claude update"
```

### Key Patterns

1. **bin vs symlinks separation**
   - `bin`: Executables only → `$XDG_BIN_HOME`
   - `symlinks`: Everything else (man pages, completions, etc.)

2. **Platform-specific manifests**
   - `cli.toml`: Cross-platform tools
   - `macos.toml`: macOS-only apps
   - `linux.toml`: Linux-only apps
   - Load all that apply to current platform

3. **Smart defaults**
   - `bin` defaults to `["<app_name>"]`
   - Platform-specific binary detection (e.g., `yq_darwin_arm64`)
   - Environment variable expansion in paths

4. **XDG compliance**
   - Config: `~/.config/devspace`
   - Data: `~/.local/share/devspace`
   - Binaries: `~/.local/bin`
   - Follow XDG spec strictly

## Shell Implementation Learnings

### What Works Well

✅ **Manifest-based approach**: Declarative TOML is intuitive and version-controllable

✅ **Platform separation**: Different manifests for different OSes avoids conditionals

✅ **bin vs symlinks**: Clear separation of concerns

✅ **Smart defaults**: 90% of apps need minimal config

✅ **XDG directories**: Standard locations, no surprises

✅ **Health checks**: `dev doctor` finds and fixes common issues

✅ **Self-update flag**: Respects apps with built-in update mechanisms

### What to Improve

🔧 **Performance**: Native TOML parsing instead of yq subprocess

🔧 **Parallelism**: Install multiple apps simultaneously

🔧 **Error messages**: Rich context and suggestions

🔧 **Type safety**: Catch manifest errors at parse time

🔧 **Progress output**: Beautiful progress bars, not just logs

🔧 **Version checking**: Built-in, not spawning processes

🔧 **Profiles**: First-class feature, easy switching

## Testing Strategy

### Unit Tests
Test individual functions and modules in isolation:
```rust
#[test]
fn test_parse_manifest() {
    let toml = r#"
        [ripgrep]
        installer = "ubi"
        project = "BurntSushi/ripgrep"
    "#;
    let manifest = parse_manifest(toml).unwrap();
    assert_eq!(manifest.apps.len(), 1);
}
```

### Integration Tests
Test actual command execution:
```rust
#[test]
fn test_app_list_command() {
    let output = Command::cargo_bin("devspace")
        .unwrap()
        .arg("app")
        .arg("list")
        .output()
        .unwrap();
    assert!(output.status.success());
}
```

### Example Tests
Examples that also serve as documentation:
```rust
// examples/install_app.rs
fn main() {
    // This example shows how to install an app
    // Also runs as an integration test
}
```

### Platform Tests
Conditional compilation for platform-specific code:
```rust
#[cfg(target_os = "macos")]
mod macos_tests {
    #[test]
    fn test_dmg_mount() {
        // ...
    }
}
```

## Development Workflow

### Building
```bash
cargo build              # Debug build
cargo build --release    # Optimized build
```

### Testing
```bash
cargo test                      # All tests
cargo test test_name            # Specific test
cargo test -- --nocapture       # Show output
cargo test --test integration   # Integration tests only
```

### Running
```bash
cargo run -- --help                     # Show help
cargo run -- app list                   # Run command
RUST_LOG=debug cargo run -- app status  # With logging
```

### Documentation
```bash
cargo doc --open         # Generate and open docs
```

## Known Issues

None yet - project just started!

## Future Enhancements

**Profile Features**:
- Profile templates
- Profile inheritance (extend base profile)
- Profile-specific environment variables
- Profile activation hooks

**Advanced App Management**:
- Version pinning
- Dependency resolution (if app A needs app B)
- Manifest validation (catch errors before install)
- Rollback on failed install

**Performance**:
- Parallel downloads and installs
- Incremental updates (only changed apps)
- Caching (downloaded archives, manifests)

**User Experience**:
- Interactive mode for first-time setup
- Fuzzy search for apps
- Suggestions for common tools
- Shell completions (zsh, bash, fish)

## Resources

### Reference Implementation
- **Location**: `/Users/acarter/.local/share/dev`
- **Key files**: `lib/app.sh`, `lib/app/*.sh`, `hosts/*.toml`
- **Documentation**: `docs/app-management.md`

### Rust Resources
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Async Book](https://rust-lang.github.io/async-book/)

### Dependencies
- [clap](https://docs.rs/clap/) - CLI parsing
- [serde](https://serde.rs/) - Serialization
- [toml](https://docs.rs/toml/) - TOML parsing
- [tokio](https://tokio.rs/) - Async runtime
- [anyhow](https://docs.rs/anyhow/) - Error handling
- [tracing](https://docs.rs/tracing/) - Logging

### Standards
- [XDG Base Directory](https://specifications.freedesktop.org/basedir-spec/)
- [TOML Spec](https://toml.io/)

## Notes

### 2025-10-02: Project Initialization

Created devspace repository and initial Rust scaffolding. Comprehensive context transferred from shell implementation via ASSISTANT.md. Ready to begin development.

**Key decisions**:
- Use clap for CLI (derive macros)
- Use tokio for async (enables parallelism)
- Use serde + toml for config
- Follow shell implementation's manifest format exactly
- Test-driven development (write tests as we go)

**Next session**: Verify CLI structure, add GitHub Actions, begin core infrastructure.
