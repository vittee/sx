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

For shell completion, see [Usage](#usage).

`sx` runs the `ssh` it finds on your `PATH`.
Linux and macOS ship it.
On Windows, turn on the optional feature "OpenSSH Client" (Settings, Apps, Optional features).

## Usage

```sh
sx                                                                   # open the TUI
sx [ssh args] <host>                                                 # like ssh, with your rc injected
sx <host> --options <name>                                           # use the options named <name>, saved for the host (Ctrl-O in the TUI)
sx --dry-run [ssh args] <host>                                       # print the ssh command, do not run it
sx init [--presets]                                                  # create sample snippets and the default profile, --presets adds the presets that are missing, switched off
sx completions <bash|zsh|fish|powershell|nushell>                    # print a completion script
sx completions --install [bash|zsh|fish|powershell|nushell] [--yes]  # set up shell completion (asks first)
sx --version                                                         # print the sx version (ssh's -V shows the ssh one)
```

## TUI

Keyboard and mouse both work (click, double-click, wheel).<br>
While the mouse is captured, the terminal cannot select text.<br>
`F2` hands the mouse back to the terminal and takes it over again on the next press.<br>
Most terminals also let you select while holding `Shift`.

If the wheel or trackpad scrolls the wrong way (natural scrolling on macOS, for example),

Enable `invert_scroll`

```toml
# config.toml

invert_scroll = true
```

### Hosts page

The list comes from `~/.ssh/config` (with `Include`) and `~/.ssh/known_hosts`.<br>
The `source` column shows which one a host came from: `config` or `known_hosts`.<br>
Or simply connect to the typed text in the search box.<br>
Typing `root@web` filters the list on `web` and connects to the selected host as `root`, for this
connection only.

#### Keys
- `Ctrl-P`, or a click on the profile column,
  changes the host's profile (`-` means off, plain ssh).
  The arrow keys only move in the list, so a stray key cannot change a profile.
- `Ctrl-F`, or a click on the star, marks a favorite.
- `Ctrl-E` picks extra snippets for the selected host on top of its profile's.
  The profile column then reads `default+2`.
- `Ctrl-O` edits the connection options of the selected host.
  A host can keep several named options, such as `root` or `deploy via bastion`, and the form lists its keys.
- `Ctrl-L` picks one of the host's named options and connects with them.

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
- `Ctrl-N` creates a snippet
- `Ctrl-Y` copy
- `Ctrl-D` delete

A snippet that sets `PS1`, and every `.prompt` snippet, gets a prompt preview underneath.<br>
The file name decides what a snippet is (see [Snippet kinds](#snippet-kinds)).

### Profiles page

Each profile has its own prompt and snippets.
For the prompt, pick a built-in theme, one of your own, or "(server's own)"
to keep the server's prompt.<br>
The arrow keys only preview a theme, `Enter` uses it for the profile.

#### Keys
- `Ctrl-N` creates a profile
- `Ctrl-Y` copy
- `Ctrl-D` delete
- `Ctrl-G` makes it the default
- `Ctrl-E` edits the selected prompt theme

## Configurations

`sx` keeps its files in `$SX_HOME`, or `~/.config/sx` when that is not set:

- `snippets/<name>.sh` holds the rc pieces.
  A host gets its profile's snippets plus its own extras, joined in name order,
  so names like `10-prompt` and `20-aliases` control the order.
  A snippet in both places is sent once.
  `sx init` creates `20-aliases`, which the default profile sends, and the presets below,
  which stay off until you turn them on in Profiles.
- `config.toml` holds the profiles (snippets and prompt theme), the default profile,
  each host's profile, extra snippets and connection options, favorites and last-used times.
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
| `x.inputrc` | applied once with `bind`, one line at a time, so `$if` rules work |
| `x.prompt` | not sent as a snippet: it is a prompt theme you can pick in Profiles, see below |

### Vim presets

All the enabled `.vim` snippets are joined, in name order, into one config.
A later name wins when two set the same option, so the presets work as layers.
`sx init` creates these, all off.
For a setup made before they existed, `sx init --presets` adds the presets that are missing, vim and shell:
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

### Truecolor

`ssh` does not send `COLORTERM`, so `vim` and `less` on a server fall back to 256 colors.<br>
The `colorterm` snippet sets `COLORTERM=truecolor`. It is off until you turn it on in Profiles.<br>
Turn it on only when your terminal supports 24-bit color.
Windows Terminal, iTerm2, kitty and WezTerm do.

### No history on the server

A normal login reads the server's login files, and a hardened server may keep `~/.bash_history` empty there, for example with `unset HISTFILE`.<br>
`sx` starts bash without those files, so that rule is lost and bash writes the history file again.<br>
The `no-history` snippet runs `unset HISTFILE`, so this shell writes no history file.
It is off until you turn it on in Profiles.<br>
Put it in the profile you use for those servers, so other servers keep their history.
Up-arrow history still works during the session.

### More shell presets

These small snippets are also created off, and `sx init --presets` adds any that are missing.
They change only this shell, so nothing is written on the server.
Turn on the ones you want in Profiles.

| Snippet | What it gives you |
|---|---|
| `man-colors` | colored man pages, and `less` keeps colors and ignores case in searches |
| `cmd-colors` | color for `ls`, `grep` and `diff`, only where the server supports it |
| `dir-keys` | `Alt-Up` goes to the parent directory, `Alt-Left` and `Alt-Right` go back and forward through the directories you visited. What you had typed goes to the kill ring, `Ctrl-Y` brings it back |
| `utf8` | switches to a UTF-8 locale when the shell is not in one, so box drawing and accents show |

### Prompt themes

A `.prompt` file contains only the `PS1` string, for example `\[\e[32m\]\u@\h\[\e[0m\]:\w\$ `.<br>
`sx` puts it in quotes as `export PS1='...'`, and the preview draws it unchanged.


To change a built-in theme, press `Ctrl-E` on it in Profiles.<br>
You get the plain string to edit.

Eleven built-in themes are based on [oh-my-zsh](https://ohmyz.sh/) themes:<br>
`bira`, `gnzh`, `fino`, `ys`, `box`, `robbyrussell`, `pygmalion`, `candy`, `af-magic`, `lambda` and `cloud`.

They are rewritten as bash prompts and are not exact copies.<br>
Each one makes the host stand out, so a remote shell does not look like a local one,<br>
and adds the git branch and `✗N` after a failed command.<br>
Every built-in theme ends in `$`, or `#` for root.

A `.prompt` file with a `PS1=` line is used as bash, unchanged, so any other lines in it run.<br>
Put anything else you want on the server in a normal `.sh` snippet.

To try another theme without reconnecting, run `sx-theme <name>` on the server.<br>
`sx-theme` alone previews every theme in a small box, as many per row as the terminal is wide,<br>
with a made-up user, host and directory, a git branch and a failed command, so you can compare them.<br>
The preview needs bash 4.4 or newer and a UTF-8 locale. Without them it prints the names.<br>
`sx-theme -l` lists the names, and Tab completes them.<br>
`sxt` is a short alias for it.<br>
`--help` prints the usage of `sx-theme`, `sx-sudo`, `sx-tmux` and `clip`.<br>
It lasts until you exit, and the profile's theme is used again on the next connection.

### Copy to your clipboard

`clip` on the server puts text in the clipboard of the machine you are sitting at.<br>
Use `clip some text`, or pipe into it: `cat ~/.ssh/id_ed25519.pub | clip`.

Inside tmux, add `set -g set-clipboard on` to your tmux config.

### Shell as another user

`sudo -i` starts a new shell for root that reads only root's own files, so your prompt and aliases are gone.<br>
`sx-sudo` does the same job and keeps them: `sx-sudo`, or `sx-sudo someuser`.

It sends your setup to `sudo` again, through its standard input.<br>
The shell starts in the user's home directory, as with `sudo -i`, but it is not a login shell,
so the user's `.profile` does not run.<br>

### tmux

A tmux window starts a new bash that reads only the server's own files, so it has none of your aliases or `sx-*` commands.<br>
`sx-tmux` starts tmux so that every window gets your setup, the same as the shell `sx` started.<br>
It takes the same arguments as `tmux`, for example `sx-tmux new -s work` or `sx-tmux attach`.

Your tmux config is loaded as usual, or your tmux snippet when you have one.<br>
A `default-command` of your own wins over ours.<br>
A tmux server that was already running keeps its own shells, so quit it first (`tmux kill-server`).<br>


### Window and tab title

`[sx]` in the title of your terminal window or tab tells you that `sx` is running.<br>
The title changes as you go:

| When | Title |
|---|---|
| the TUI is open | `[sx]` |
| connecting | `[sx] host` |
| on the server, before every prompt | `[sx] user@host: dir` |
| `sx` has ended | `user@host: dir` of your own machine |

The server part follows `cd`.<br>
A prompt theme replaces the server's own `PS1`, which is where many servers set the title,
so `sx` does it itself.<br>
Inside tmux, add `set -g set-titles on` to your tmux config to pass it on to your terminal.

### Prompt marks

The server also marks where each prompt, command and output starts and ends.<br>
A terminal that understands the marks can jump from one prompt to the next, select the output of one command,
and show failed commands.<br>

The output and exit status marks need bash 4.4 or newer.

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

## License

[MIT](LICENSE)
