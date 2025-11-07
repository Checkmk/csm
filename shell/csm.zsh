export _CSM_SHELL=zsh

csm() {
    local CSM_BIN="$(whence -p csm)"
    if [[ "$1" == "env" && ( "$2" == "activate"  || "$2" == "deactivate" ) ]]; then
        eval "$("$CSM_BIN" "$@")"
    else
        "$CSM_BIN" "$@"
    fi
}
