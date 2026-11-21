#compdef sx
# zsh completion for sx. Load it (after compinit) with:  source <(sx completions zsh)
# or save it as `_sx` in a directory of your $fpath.
_sx() {
    local -a candidates
    candidates=(${(f)"$(command sx __complete "${(@)words[2,CURRENT]}")"})
    compadd -a candidates
}
if [[ ${funcstack[1]} == _sx ]]; then
    _sx "$@"
else
    compdef _sx sx
fi
