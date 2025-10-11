#!/usr/bin/env bash
set -euo pipefail

# Launch a fresh sandbox shell with an isolated HOME/XDG layout. This does not
# modify the current shell; instead it execs a clean shell (zsh -f by default)
# so you can safely experiment with dws.

script_dir=$(cd -- "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)
sandbox_root=$(mktemp -d -t dws-sandbox-XXXXXX)
keep_sandbox=${DWS_SANDBOX_KEEP:-}

cleanup() {
  if [[ -z "$keep_sandbox" ]]; then
    rm -rf "$sandbox_root"
  else
    echo "Preserving sandbox at $sandbox_root" >&2
  fi
}
trap cleanup EXIT

export HOME="$sandbox_root/home"
mkdir -p "$HOME"

export XDG_CONFIG_HOME="$HOME/.config"
export XDG_STATE_HOME="$HOME/.local/state"
export XDG_CACHE_HOME="$HOME/.cache"
mkdir -p "$XDG_CONFIG_HOME" "$XDG_STATE_HOME" "$XDG_CACHE_HOME"

base_path=${DWS_SANDBOX_BASE_PATH:-/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin}
original_terminfo=${TERMINFO:-}
original_terminfo_dirs=${TERMINFO_DIRS:-}
path_parts=()
if [[ -x "$script_dir/target/debug/dws" ]]; then
  path_parts+=("$script_dir/target/debug")
fi
if [[ -x "$script_dir/target/release/dws" ]]; then
  path_parts+=("$script_dir/target/release")
fi
if (( ${#path_parts[@]} > 0 )); then
  PATH_OVERRIDE="$(IFS=:; echo "${path_parts[*]}"):$base_path"
else
  PATH_OVERRIDE="$base_path"
  cat >&2 <<'EOF_WARN'
warning: dws binary not found in target/{debug,release}. Run `cargo build`
         so the sandbox shell can invoke `dws` directly, or call `cargo run`.
EOF_WARN
fi

shell_bin=${SHELL:-/bin/sh}
case "$(basename "$shell_bin")" in
  zsh)
    sandbox_zdotdir="$HOME/.zsh"
    mkdir -p "$sandbox_zdotdir"
    cat > "$sandbox_zdotdir/.zshrc" <<'ZSHRC'
bindkey -e
bindkey '^?' backward-delete-char
bindkey '^H' backward-delete-char
bindkey '^[[3~' delete-char
bindkey -M vicmd '^?' backward-delete-char
bindkey -M vicmd '^H' backward-delete-char
bindkey -M viins '^?' backward-delete-char
bindkey -M viopp '^?' backward-delete-char
bindkey -M main '^?' backward-delete-char
bindkey -M command '^?' backward-delete-char
bindkey -M visual '^?' backward-delete-char
bindkey -M isearch '^?' backward-delete-char
PROMPT='%n@sandbox %~ %# '
ZSHRC
    export ZDOTDIR="$sandbox_zdotdir"
    shell_args=("zsh" "-i")
    ;;
  bash)
    shell_args=("bash" "--noprofile" "--norc")
    ;;
  *)
    shell_args=("$shell_bin")
    ;;
esac

cd "$HOME"

echo "Sandbox shell starting with isolated HOME." >&2
echo "  HOME -> $HOME" >&2
echo "  XDG_CONFIG_HOME -> $XDG_CONFIG_HOME" >&2
echo "Type 'exit' when you want to tear it down." >&2

if [[ -t 0 ]] && command -v stty >/dev/null 2>&1; then
  stty sane || true
  stty erase '^?' || true
fi

env_args=(
  HOME="$HOME"
  PATH="$PATH_OVERRIDE"
  TERM="${TERM:-xterm-256color}"
  LANG="${LANG:-C.UTF-8}"
  LC_ALL="${LANG:-C.UTF-8}"
  XDG_CONFIG_HOME="$XDG_CONFIG_HOME"
  XDG_STATE_HOME="$XDG_STATE_HOME"
  XDG_CACHE_HOME="$XDG_CACHE_HOME"
  DWS_SANDBOX_ROOT="$sandbox_root"
  DWS_SANDBOX_KEEP="$keep_sandbox"
  SHELL="$shell_bin"
  PWD="$HOME"
)

if [[ -n "${ZDOTDIR:-}" ]]; then
  env_args+=(ZDOTDIR="$ZDOTDIR")
fi

if [[ -n "$original_terminfo" ]]; then
  env_args+=(TERMINFO="$original_terminfo")
fi

if [[ -n "$original_terminfo_dirs" ]]; then
  env_args+=(TERMINFO_DIRS="$original_terminfo_dirs")
fi

exec env -i "${env_args[@]}" "${shell_args[@]}"
