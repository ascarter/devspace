# dws workspace

This workspace was created by dws (Developer Workspace).

## Structure

```
$XDG_CONFIG_HOME/dws/            # dws workspace root (reserved)
  profile/                       # Your profile repository (version controlled)
    config/                      # XDG config files → symlinked to $XDG_CONFIG_HOME
      zsh/
        .zshrc
      bash/
        .bashrc
      fish/
        config.fish
      nvim/
        init.lua
    manifests/
      tools.toml                 # Base tool definitions (all machines)
      tools-macos.toml           # Platform overrides (optional, per OS)
      tools-<hostname>.toml      # Host overrides (optional, per machine)
    README.md
```

## Publishing to GitHub

To share this workspace across machines:

```bash
cd $XDG_CONFIG_HOME/dws/profile
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

1. Add your dotfiles to `profile/config/` (e.g., `profile/config/zsh/.zshrc`)
2. Edit manifests under `profile/manifests/` to include your preferred tools
3. Commit and push changes
4. Run `dws sync` on other machines to pull updates

## Writing Manifests

Each entry in `manifests/*.toml` represents a tool. Use these fields:

| Field | Required | Description |
| ----- | -------- | ----------- |
| `installer` | ✅ | Backend to use: `ubi`, `curl`, `dmg`, or `flatpak`. |
| `project` | optional | GitHub `owner/repo` used by installers like `ubi`. |
| `version` | optional | Pin to a specific release. Omit for latest. |
| `url` | optional | Direct download URL for scripts or disk images. |
| `shell` | optional | Shell interpreter to run installer scripts (e.g. `sh`). |
| `bin` | optional | Array of executables to link into `~/.local/state/dws/bin`. |
| `symlinks` | optional | Extra files to link using `source:target` pairs. |
| `app` | optional | `.app` bundle name for macOS DMG installs. |
| `team_id` | optional | Apple Developer team ID for signed macOS apps. |
| `self_update` | optional | Set to `true` if the tool updates itself and should be skipped by `dws update`. |

Manifest precedence: `tools.toml` (base) → `tools-<platform>.toml` (e.g. `tools-macos.toml`) → `tools-<hostname>.toml`. Higher-precedence files only need to override the fields that change; leaving a key out inherits the lower layer. If your hostname can’t be sanitized, name the host file `tools-local.toml` and it will be picked up automatically.

## Shell Integration

The `dws init` command sets up shell integration automatically:

- **zsh**: Adds `eval "$(dws env --shell zsh)"` to `~/.zshenv`
- **bash**: Adds `eval "$(dws env --shell bash)"` to `~/.bashrc`
- **fish**: Adds `dws env --shell fish | source` to `~/.config/fish/config.fish`

You can run `dws init --shell <shell>` multiple times to set up different shells.
