# sx

Use your own shell prompt and aliases on any server.

`sx` is a wrapper around `ssh`. Use it like `ssh`: `sx user@host`.
The remote bash then starts with your prompt and aliases.

Nothing is installed or written on the server, and nothing is left behind when the session ends.

Details are under [How it works](#how-it-works).

![sx in action: connect from the TUI, switch theme, then plain ssh next to sx](demo/sx.gif)

## What you get:

- Your prompt and aliases on every server, from small snippet files you keep locally.
- A terminal UI to browse hosts, edit snippets and see recent sessions for each host.
- Preset prompt themes. You can edit them or write your own.
- Profiles that group snippets and a prompt, assigned per host.
- The same arguments as `ssh`.
- Your `vim`, `tmux` and readline (`inputrc`) settings on the server as well.

Quick start:

```sh
sx init                 # create sample snippets and the default profile
sx user@host            # log in with your rc
sx                      # or open the TUI
```

Needs `ssh` on your `PATH` and bash on the server, see [Install](#install) and [Limits](#limits).

## Install

Linux and macOS:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/vittee/sx/releases/latest/download/sx-ssh-installer.sh | sh
```

Windows (PowerShell):

```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/vittee/sx/releases/latest/download/sx-ssh-installer.ps1 | iex"
```

Or download an archive from the [releases page](https://github.com/vittee/sx/releases),
or build it yourself, see [Build](#build).

`sx` runs the `ssh` it finds on your `PATH`.
Linux and macOS ship it.
On Windows, turn on the optional feature "OpenSSH Client" (Settings, Apps, Optional features).

## Usage

```sh
sx                      # open the TUI
sx user@host            # same arguments as ssh, with your rc injected
sx -p 2222 -i key host
sx --dry-run host       # print the ssh command, do not run it
sx init                 # create sample snippets and the default profile
sx init --presets       # add vim presets that are missing, switched off
```

## TUI

Keyboard and mouse both work (click, double-click, wheel).<br>
While the mouse is captured, the terminal cannot select text.<br>
`F2` hands the mouse back to the terminal and takes it over again on the next press.<br>
Most terminals also let you select while holding `Shift`.

### Hosts page

The list comes from `~/.ssh/config` (with `Include`) and `~/.ssh/known_hosts`.<br>
The `source` column shows which one a host came from: `config` or `known_hosts`.<br>
Or simply connect to the typed text in the search box.

#### Keys
- `Left`/`Right`, or a click on the profile column,
  changes the host's profile (`-` means off, plain ssh).
- `Ctrl-F`, or a click on the star, marks a favorite.
- `Ctrl-E` picks extra snippets for the selected host on top of its profile's.
  The profile column then reads `default+2`.

To sort, click a column header (click again to reverse),
or press `Ctrl-S` to cycle the column and `Ctrl-R` to reverse.

When the terminal is tall enough,
a history pane under the list shows the last five sessions of the selected host:
when, how long, how it ended.

### Snippets page

Edit your rc pieces in place.

#### Keys
- `Esc` autosaves
- `Ctrl-S` saves.
- `n` creates a snippet
- `c` copy
- `d` delete

A snippet that sets `PS1`, and every `.prompt` snippet, gets a prompt preview underneath.<br>
The file name decides what a snippet is (see [Snippet kinds](#snippet-kinds)).

### Profiles page

Each profile has its own prompt and snippets.
For the prompt, pick a built-in theme, one of your own, or "(server's own)"
to keep the server's prompt.

#### Keys
- `n` creates a profile
- `c` copy
- `d` delete
- `*` makes it the default

## Configurations

`sx` keeps its files in `$SX_HOME`, or `~/.config/sx` when that is not set:

- `snippets/<name>.sh` holds the rc pieces.
  A host gets its profile's snippets plus its own extras, joined in name order,
  so names like `10-prompt` and `20-aliases` control the order.
  A snippet in both places is sent once.
  `sx init` creates `20-aliases`, which the default profile sends, and the vim presets below,
  which stay off until you turn them on in Profiles.
- `config.toml` holds the profiles (snippets and prompt theme), the default profile,
  each host's profile and extra snippets, favorites and last-used times.
- `history.tsv` has one line per session (time, host, seconds, outcome), trimmed to the newest 1000.

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
| `x.prompt` | not sent as a snippet: it is a prompt theme you can pick in Profiles, see below |

### Vim presets

All the enabled `.vim` snippets are joined, in name order, into one config.
A later name wins when two set the same option, so the presets work as layers.
`sx init` creates these, all off.
For a setup made before they existed, `sx init --presets` adds the ones that are missing:
it keeps your files, turns nothing on, and re-creates any preset you deleted.

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
`sx` puts it in quotes as `export PS1='...'`, and the preview draws it unchanged.


To change a built-in theme, press `e` on it in Profiles.<br>
You get the plain string to edit.

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
- Hosts that appear only as hashed entries in `~/.ssh/known_hosts` are not listed.<br>
  When `HashKnownHosts` is on (the default on some systems), ssh stores a host as `|1|salt|hash`
  instead of its name, and a hash cannot be turned back into a name.<br>
  Hosts from `~/.ssh/config` are always listed.<br>
  To reach a hashed host, type its name in the search box and press Enter,
  or add a `Host` entry for it in `~/.ssh/config`.

## Build

```sh
cargo build --release   # binary: target/release/sx
cargo test
```

## Releasing

The changelog is built from the conventional commit messages with [git-cliff](https://git-cliff.org),
and the release workflow uses the matching section as the GitHub release notes.

```sh
# set the new version in Cargo.toml, then
cargo check                                               # updates Cargo.lock
git cliff --unreleased --tag v0.2.0 --prepend CHANGELOG.md
git commit -am "chore: release v0.2.0"
git tag v0.2.0
git push origin main
git push origin v0.2.0                                    # push the tag on its own
```

## License

[MIT](LICENSE)
