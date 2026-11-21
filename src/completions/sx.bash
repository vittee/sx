# bash completion for sx. Load it with:  eval "$(sx completions bash)"
_sx() {
    local line=${COMP_LINE:0:COMP_POINT} words
    read -ra words <<< "$line"
    if [[ $line == *[[:space:]] ]]; then
        set -- --new "${words[@]:1}"
    else
        set -- "${words[@]:1}"
    fi
    local IFS=$'\n'
    COMPREPLY=($(command sx __complete "$@"))
}
complete -F _sx sx
