# fish completion for sx. Load it with:  sx completions fish | source
function __sx_complete
    set -l words (commandline -opc)
    set -e words[1]
    set -l cur (commandline -ct)
    if test -z "$cur"
        command sx __complete --new $words
    else
        command sx __complete $words $cur
    end
end
complete -c sx -f -a '(__sx_complete)'
