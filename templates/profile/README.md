# dws workspace

This workspace was created by dws (Developer Workspace).

## Structure

```
$XDG_CONFIG_HOME/dws/            # Your dotfiles repo (version controlled)
  config/                        # XDG config files → symlinked to $XDG_CONFIG_HOME
    zsh/
      .zshrc
    bash/
      .bashrc
    fish/
      config.fish
    nvim/
      init.lua
  manifests/
    cli.toml                     # Cross-platform CLI tools
    macos.toml                   # macOS-specific applications
  README.md
```

## Publishing to GitHub

To share this workspace across machines:

```bash
cd $XDG_CONFIG_HOME/dev
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

1. Add your dotfiles to `config/` (e.g., `config/zsh/.zshrc`, `config/nvim/init.lua`)
2. Edit manifests to include your preferred tools
3. Commit and push changes
4. Run `dws sync` on other machines to pull updates

## Shell Integration

The `dws init` command sets up shell integration automatically:

- **zsh**: Adds `eval "$(dws env --shell zsh)"` to `~/.zshenv`
- **bash**: Adds `eval "$(dws env --shell bash)"` to `~/.bashrc`
- **fish**: Adds `dws env --shell fish | source` to `~/.config/fish/config.fish`

You can run `dws init --shell <shell>` multiple times to set up different shells.
