# devspace profile: {PROFILE_NAME}

This profile was created by devspace.

## Structure

- `config/` - Dotfiles that will be symlinked to your home directory
- `manifests/` - Tool definitions
  - `cli.toml` - Cross-platform CLI tools
  - `macos.toml` - macOS-specific applications

## Publishing to GitHub

To share this profile across machines:

```bash
cd {PROFILE_PATH}
git init
git add .
git commit -m "Initial devspace profile"
gh repo create {PROFILE_NAME} --public --source=. --push
```

Then on another machine:

```bash
devspace clone yourusername/{PROFILE_NAME}
```

## Customizing

1. Add your dotfiles to `config/`
2. Edit manifests to include your preferred tools
3. Commit and push changes
4. Run `devspace sync` on other machines to pull updates
