# sx

Use your own shell prompt and aliases on any server.

`sx` is a wrapper around `ssh`. Use it like `ssh`: `sx user@host`.
The remote bash then starts with your prompt and aliases.

Nothing is installed or written on the server, and nothing is left behind when the session ends.

Details are under [How it works](#how-it-works).

## What you get:

- Your prompt and aliases on every server, from small snippet files you keep locally.
- Preset prompt themes. You can edit them or write your own.
- Profiles that group snippets and a prompt, assigned per host.
- The same arguments as `ssh`.
- Your `vim`, `tmux` and readline (`inputrc`) settings on the server as well.

Quick start:

```sh
cargo build --release   # binary: target/release/sx
sx init                 # create sample snippets and the default profile
sx user@host            # log in with your rc
```

Requires bash on the server, see [Limits](#limits).

## Usage

```sh
sx user@host            # same arguments as ssh, with your rc injected
sx -p 2222 -i key host
sx --dry-run host       # print the ssh command, do not run it
sx init                 # create sample snippets and the default profile
```

## Configurations

`sx` keeps its files in `$SX_HOME`, or `~/.config/sx` when that is not set:

- `snippets/<name>.sh` holds the rc pieces.
  A host gets its profile's snippets plus its own extras, joined in name order,
  so names like `10-prompt` and `20-aliases` control the order.
  A snippet in both places is sent once.
  `sx init` creates `20-aliases`, which the default profile sends, and the vim presets below,
  which stay off until a profile lists them.
- `config.toml` holds the profiles (snippets and prompt theme), the default profile,
  and each host's profile and extra snippets.

The prompt theme goes after the snippets, so choosing one overrides any `PS1` a snippet sets.
A fresh setup starts with the `fino` theme.

### Snippet kinds

The other kinds are wrapped in shell that hands the text to the program without writing a file on
the server:

| Name | Effect on the server |
|---|---|
| `x.sh` (or just `x`) | sourced as shell |
| `x.vim` | `vim` and `nvim` run with `-u` on this config, through process substitution |
| `x.tmux` | `tmux` runs with `-f` on this config, all enabled `.tmux` snippets joined like vim |
| `x.inputrc` | loaded once with `bind -f` |
| `x.prompt` | not sent as a snippet: it is a prompt theme that a profile can name, see below |

### Vim presets

All the enabled `.vim` snippets are joined, in name order, into one config.
A later name wins when two set the same option, so the presets work as layers.
`sx init` creates these, all off.

| Snippet | What it sets |
|---|---|
| `vim-basic` | syntax, filetype indent, line numbers, ruler, incremental and highlighted search |
| `vim-recommended` | everything in `vim-basic`, plus smart-case search, scroll margin, whitespace markers, split directions, `j`/`k` on screen lines, Enter clears the highlight |
| `vim-indent-2`, `vim-indent-4` | spaces for indenting |
| `vim-indent-tabs` | real tabs, 8 wide |
| `vim-mouse` | mouse and system clipboard |
| `vim-keys` | space as leader, `jk` for Esc, `Ctrl-h/j/k/l` between windows, a few leader mappings |
| `vim-dark` | dark background and the `desert` color scheme |
| `vim-no-traces` | no swap, backup, undo or viminfo files on the server |

Turn on `vim-recommended` or `vim-basic` (not both), then add the layers you want.

### Prompt themes

A `.prompt` file contains only the `PS1` string, for example `\[\e[32m\]\u@\h\[\e[0m\]:\w\$ `.<br>
`sx` puts it in quotes as `export PS1='...'`.

Eleven built-in themes are based on [oh-my-zsh](https://ohmyz.sh/) themes:<br>
`bira`, `gnzh`, `fino`, `ys`, `box`, `robbyrussell`, `pygmalion`, `candy`, `af-magic`, `lambda` and `cloud`.

They are rewritten as bash prompts and are not exact copies.<br>
Each one makes the host stand out, so a remote shell does not look like a local one,<br>
and adds the git branch and `✗N` after a failed command.<br>
Every built-in theme ends in `$`, or `#` for root.

A `.prompt` file with a `PS1=` line is used as bash, unchanged, so any other lines in it run.<br>
Put anything else you want on the server in a normal `.sh` snippet.

## How it works

`sx` runs `ssh` with two extra options:

- `-o RequestTTY=force`
- `-o RemoteCommand=...` carries your rc, gzipped and base64-encoded, inside the command.<br>
  The command decodes it and starts `bash --rcfile <(...)`.<br>
  The server's own `~/.bashrc` is sourced first, then yours.

While you are logged in, other users on the server can see the command in the process list,
so do not put secrets in snippets.

## Limitations

- The server must run bash.
- If bash, `base64 -d` or `gunzip` is missing on the server,
  you get the plain login shell and a one-line `sx:` message that gives the reason.
- Payloads over about 24 KB (encoded) trigger a warning.
- Only bash is supported. zsh and fish rc files are not.

## Build

```sh
cargo build --release   # binary: target/release/sx
cargo test
```
