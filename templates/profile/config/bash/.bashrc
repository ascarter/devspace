# dws bash configuration
# This is a minimal starter configuration - customize to your needs

# History configuration
HISTFILE="$HOME/.local/state/bash/history"
HISTSIZE=10000
HISTFILESIZE=20000
HISTCONTROL=ignoredups:ignorespace
shopt -s histappend

# Create history directory if it doesn't exist
mkdir -p "$(dirname "$HISTFILE")"

# Check window size after each command
shopt -s checkwinsize

# Enable color support
if [ -x /usr/bin/dircolors ]; then
    test -r ~/.dircolors && eval "$(dircolors -b ~/.dircolors)" || eval "$(dircolors -b)"
fi

# Basic prompt (customize as desired)
PS1='\[\033[01;34m\]\w\[\033[00m\]\$ '

# Aliases
alias ls='ls --color=auto'
alias ll='ls -lh'
alias la='ls -lAh'
alias ..='cd ..'
alias ...='cd ../..'
alias grep='grep --color=auto'

# Add your custom configuration below
