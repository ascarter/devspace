# devspace

> Personal development environment manager - declarative dotfiles + tools as code

**devspace** manages your development environment through declarative manifests. It combines dotfile management with tool installation in a single, portable binary.

## Status

🚧 **Early Development** - This project is in active development. The shell-based prototype can be found at [ascarter/dev](https://github.com/ascarter/dev).

## Concept

A **devspace** is a profile containing:
- Shell configuration (zsh, bash, etc.)
- Development tools and their configurations
- Application manifests (what to install)
- Environment-specific settings

Key features:
- Single Rust binary - just download and run
- Profile-based - maintain different configs for work/personal/projects
- Platform-aware - macOS, Linux, BSD support
- XDG-compliant - respects standard directories
- Works everywhere - host, toolbox, devcontainer, codespaces

## Quick Start

**Installation** (coming soon):
```bash
curl -fsSL https://devspace.dev/install.sh | sh
```

**Initialize a profile**:
```bash
# Clone an existing profile from GitHub
devspace profile clone username/dotfiles

# Or create a new one
devspace profile create my-profile
```

**Install tools**:
```bash
# Install all apps from manifest
devspace app install

# Install specific app
devspace app install ripgrep
```

**Manage configs**:
```bash
# Link configuration files
devspace config link

# Show status
devspace status
```

## Profile Repository

Your profile repository should contain:
```
my-profile/
├── config/           # Dotfiles to symlink
│   ├── zsh/
│   ├── git/
│   └── ...
├── manifests/        # Application manifests
│   ├── cli.toml
│   ├── macos.toml
│   └── linux.toml
└── devspace.toml     # Profile configuration
```

See [profile-template](https://github.com/ascarter/devspace-profile) for an example.

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

See [ASSISTANT.md](ASSISTANT.md) for detailed architecture and design decisions.

## License

MIT - see [LICENSE](LICENSE)
