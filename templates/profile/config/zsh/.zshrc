# dws zsh configuration
# This is a minimal starter configuration - customize to your needs

# History configuration
HISTFILE=$HOME/.local/state/zsh/history
HISTSIZE=10000
SAVEHIST=10000
setopt SHARE_HISTORY
setopt HIST_IGNORE_DUPS
setopt HIST_IGNORE_SPACE

# Create history directory if it doesn't exist
mkdir -p "$(dirname "$HISTFILE")"

# Basic prompt (customize as desired)
PS1='%F{blue}%~%f %# '

# Enable completion system
autoload -Uz compinit
compinit -d $HOME/.cache/zsh/zcompdump

# Aliases
alias ls='ls --color=auto'
alias ll='ls -lh'
alias la='ls -lAh'
alias ..='cd ..'
alias ...='cd ../..'

# Add your custom configuration below
