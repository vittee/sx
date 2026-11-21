# nushell completion for sx. Save it and load it from your config.nu:
#   sx completions nushell | save -f ($nu.default-config-dir | path join sx-completions.nu)
#   use sx-completions.nu *
def "nu-complete sx" [context: string] {
    let words = ($context | split row ' ' | where {|w| $w != '' } | skip 1)
    if ($context | str ends-with ' ') {
        ^sx __complete --new ...$words | lines
    } else {
        ^sx __complete ...$words | lines
    }
}

export extern sx [
    ...args: string@"nu-complete sx"
    --options: string@"nu-complete sx"
    --dry-run
    --version
    --help
]
