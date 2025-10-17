# dws workspace

This workspace was created by dws (Developer Workspace).

## Structure

```
$XDG_CONFIG_HOME/dws/             # dws workspace root (reserved)
  config.toml                     # Workspace configuration (active profile + overrides)
  profiles/                       # Your profile repositories (version controlled)
    <profile>/                    # e.g., default, personal, work
      config/                     # XDG config files → symlinked to $XDG_CONFIG_HOME
        zsh/
          .zshrc
        bash/
          .bashrc
        fish/
          config.fish
        nvim/
          init.lua
      dws.toml                    # Profile-level tool definitions
      README.md
```

## Publishing to GitHub

To share this workspace across machines:

```bash
cd $XDG_CONFIG_HOME/dws/profiles/<profile>
git init
git add .
git commit -m "Initial dws workspace"
gh repo create dotfiles --public --source=. --push
```

Then on another machine:

```bash
dws init yourusername/dotfiles
```

## Customizing

1. Add your dotfiles to `profiles/<profile>/config/` (e.g., `profiles/default/config/zsh/.zshrc`)
2. Edit `profiles/<profile>/dws.toml` to include your preferred tools
3. Commit and push changes
4. Run `dws sync` on other machines to pull updates

### Managing profiles

- `dws profiles` lists all available profiles and marks the active one.
- `dws clone <repo> [--profile name]` clones another profile into `profiles/<name>` without activating it.
- `dws use <profile>` switches the active profile (updates symlinks and `config.toml`).

## Editing `dws.toml`

Each entry under the `[tools.*]` table describes a single tool. Supported fields:

| Field | Required | Description |
| ----- | -------- | ----------- |
| `installer` | ✅ | Backend to use: `curl`, `dmg`, or `flatpak`. (UBI removed; future internal GitHub release backend may be added) |
| `project` | optional | GitHub `owner/repo` (reserved for future GitHub release backend; ignored by current `curl`/`dmg`/`flatpak`). |
| `version` | optional | Pin to a specific release. Omit for latest. |
| `url` | optional | Direct download URL for scripts or disk images. |
| `shell` | optional | Shell interpreter to run installer scripts (e.g. `sh`). |
| `bin` | optional | Array of executables to link into `~/.local/state/dws/bin`. |
| `symlinks` | optional | Extra files to link using `source:target` pairs. |
| `asset_filters` | optional | List of regex patterns (OR semantics) matched against release asset filenames to select an install candidate. |
| `checksum` | optional | SHA256 (hex) expected for the selected asset; required when `asset_filters` is non-empty for integrity verification. |
| `app` | optional | `.app` bundle name for macOS DMG installs. |
| `team_id` | optional | Apple Developer team ID for signed macOS apps. |
| `self_update` | optional | Set to `true` if the tool updates itself and should be skipped by `dws update`. |
| `platform` | optional | List of platforms this tool applies to (`macos`, `linux`, `linux-ubuntu`, etc.). |
| `hosts` | optional | List of sanitized hostnames for machine-specific entries. |

Profile `dws.toml` files form the base. You can add workspace-specific overrides by editing `$XDG_CONFIG_HOME/dws/config.toml`; when a tool name appears in both places, the workspace entry replaces the profile entry entirely. Filters that do not match the current platform or hostname fall back to the profile definition.

## Shell Integration

The `dws init` command sets up shell integration automatically:

- **zsh**: Adds `eval "$(dws env --shell zsh)"` to `~/.zshenv`
- **bash**: Adds `eval "$(dws env --shell bash)"` to `~/.bashrc`
- **fish**: Adds `dws env --shell fish | source` to `~/.config/fish/config.fish`

You can run `dws init --shell <shell>` multiple times to set up different shells.
