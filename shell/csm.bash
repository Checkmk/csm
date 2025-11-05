# We use "bash" to mean POSIX-compatible
export _CSM_SHELL=bash

csm() {
    CSM_BIN="$(command -v csm)"
    if [[ "$1" == "env" && ( "$2" == "activate"  || "$2" == "deactivate" ) ]]; then
        eval "$("$CSM_BIN" "$@")"
    else
        "$CSM_BIN" "$@"
    fi
}
