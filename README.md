# sx

Use your own shell prompt and aliases on any server.

`sx` is a wrapper around `ssh`. Use it like `ssh`: `sx user@host`.
The remote bash then starts with your prompt and aliases.

Nothing is installed or written on the server, and nothing is left behind when the session ends.

Details are under [How it works](#how-it-works).

## What you get

- Your prompt and aliases on every server, from small snippet files you keep locally.
- Profiles that choose which snippets are sent, with a profile for each host or none at all.
- The same arguments as `ssh`.

## Quick start

```sh
cargo build --release   # binary: target/release/sx
sx init                 # create a sample snippet in ~/.config/sx/snippets
sx user@host            # log in with your snippets
```

Put your prompt and aliases in `~/.config/sx/snippets/*.sh`, for example `10-prompt.sh`.

Requires bash on the server, see [Limits](#limits).

## Usage

```sh
sx user@host            # same arguments as ssh, with your snippets injected
sx -p 2222 -i key host
sx --dry-run host       # print the ssh command, do not run it
sx init                 # create the sample snippet
sx --help
```

`sx` runs plain `ssh` when you pass a remote command (`sx host ls`),
any of `-N -T -W -O -Q -V -G -s -f -n`, or `-o RemoteCommand`, and when the host's profile is off.
It also runs plain `ssh`, without a message, when there are no snippets or they are all empty.

## Config

`sx` keeps its files in `$SX_HOME`, or `~/.config/sx` when that is not set.

- `snippets/<name>.sh` is a piece of plain bash:
  your prompt, aliases and anything else you want in the server's shell.
  A profile's snippets are joined in name order on every login,
  so names like `10-prompt` and `20-aliases` control the order.
- `config.toml` has the profiles and each host's profile.

The first run creates a sample `20-aliases.sh` and a `default` profile that sends it.
`sx init` does the same on request.
A snippet is sent only when a profile lists it, so add new snippets to a profile.

```toml
default_profile = "default"

[profiles.default]
snippets = ["10-prompt", "20-aliases"]

[profiles.work]
snippets = ["20-aliases"]

[hosts.box]
profile = "work"

[hosts.legacy]
profile = "-"       # plain ssh for this host
```

A host uses the profile named under `[hosts.<name>]`, else `default_profile`.
The name is the host as you type it, without a `user@` prefix.
The profile `-` turns `sx` off for that host.
A host that names a profile that does not exist uses `default_profile`.

## How it works

`sx` runs `ssh` with two extra options:

- `-o RequestTTY=force`
- `-o RemoteCommand=...` carries your rc, gzipped and base64-encoded, inside the command.<br>
  The command decodes it and starts `bash --rcfile <(...)`.<br>
  The server's own `~/.bashrc` is sourced first, then yours.

While you are logged in, other users on the server can see the command in the process list,
so do not put secrets in snippets.

## Limits

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
