#!/bin/bash
# Shell integration setup for winutils

setup_bash_completion() {
    local completion_dir="$HOME/.local/share/bash-completion/completions"
    mkdir -p "$completion_dir"

    cat > "$completion_dir/winutils" << 'COMP'
_winutils_completion() {
    local cur prev opts
    COMPREPLY=()
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"

    opts="--help --version"
    COMPREPLY=( $(compgen -W "${opts}" -- ${cur}) )
    return 0
}

complete -F _winutils_completion wu-ls wu-cat wu-echo wu-wc wu-sort
COMP

    echo "Bash completion installed to $completion_dir/winutils"
}

setup_zsh_completion() {
    local completion_dir="$HOME/.local/share/zsh/site-functions"
    mkdir -p "$completion_dir"

    cat > "$completion_dir/_winutils" << 'COMP'
#compdef wu-ls wu-cat wu-echo wu-wc wu-sort

_winutils() {
    _arguments \
        '--help[Show help message]' \
        '--version[Show version information]'
}

_winutils "$@"
COMP

    echo "Zsh completion installed to $completion_dir/_winutils"
}

main() {
    echo "Setting up shell integration for winutils..."

    if [[ -n "$BASH_VERSION" ]]; then
        setup_bash_completion
    fi

    if [[ -n "$ZSH_VERSION" ]]; then
        setup_zsh_completion
    fi

    echo "Shell integration setup completed!"
    echo "Restart your shell or source your shell configuration to enable completions."
}

main "$@"
