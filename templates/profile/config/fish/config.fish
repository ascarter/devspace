# dws fish configuration
# This is a minimal starter configuration - customize to your needs

# History configuration
set -g fish_history_dir $HOME/.local/state/fish
mkdir -p $fish_history_dir

# Don't show welcome message
set -g fish_greeting

# Basic prompt (customize as desired)
# The default fish prompt is already pretty good, but you can override it:
# function fish_prompt
#     set_color blue
#     echo -n (prompt_pwd)
#     set_color normal
#     echo -n ' > '
# end

# Aliases
alias ls='ls --color=auto'
alias ll='ls -lh'
alias la='ls -lAh'
alias ..='cd ..'
alias ...='cd ../..'

# Add your custom configuration below
