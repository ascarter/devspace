# devws workspace

This workspace was created by devws (Developer Workspace).

## Structure

```
$XDG_CONFIG_HOME/devws/          # Your dotfiles repo (version controlled)
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
cd $XDG_CONFIG_HOME/devws
git init
git add .
git commit -m "Initial devws workspace"
gh repo create dotfiles --public --source=. --push
```

Then on another machine:

```bash
devws init yourusername/dotfiles
```

## Customizing

1. Add your dotfiles to `config/` (e.g., `config/zsh/.zshrc`, `config/nvim/init.lua`)
2. Edit manifests to include your preferred tools
3. Commit and push changes
4. Run `devws sync` on other machines to pull updates

## Shell Integration

The `devws init` command sets up shell integration automatically:

- **zsh**: Adds `eval "$(devws env --shell zsh)"` to `~/.zshenv`
- **bash**: Adds `eval "$(devws env --shell bash)"` to `~/.bashrc`
- **fish**: Adds `devws env --shell fish | source` to `~/.config/fish/config.fish`

You can run `devws init --shell <shell>` multiple times to set up different shells.
