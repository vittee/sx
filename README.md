# sx

Use your own shell prompt and aliases on any server.

`sx` is a wrapper around `ssh`. Use it like `ssh`: `sx user@host`.
The remote bash then starts with your prompt and aliases.

Nothing is installed or written on the server, and nothing is left behind when the session ends.

Details are under [How it works](#how-it-works).

## What you get

- Your prompt and aliases on every server, from an rc file you keep locally.
- The same arguments as `ssh`.

## Quick start

```sh
cargo build --release   # binary: target/release/sx
mkdir -p ~/.config/sx
cat > ~/.config/sx/rc <<'RC'
export PS1='\u@\h:\w\$ '
alias ll='ls -l'
RC
sx user@host            # log in with your rc
```

Requires bash on the server, see [Limits](#limits).

## Usage

```sh
sx user@host            # same arguments as ssh, with your rc injected
sx -p 2222 -i key host
sx --dry-run host       # print the ssh command, do not run it
sx --help
```

`sx` runs plain `ssh` when you pass a remote command (`sx host ls`),
any of `-N -T -W -O -Q -V -G -s -f -n`, or `-o RemoteCommand`.
It also runs plain `ssh`, after a message on stderr, when the rc file is missing,
and without a message when it is empty.

## Config

`sx` reads one file: `$SX_HOME/rc`, or `~/.config/sx/rc` when `SX_HOME` is not set.
It is plain bash: your prompt, aliases and anything else you want in the server's shell.
`sx` reads it on every login.

## How it works

`sx host` runs `ssh` with three extra options:

- `-o SetEnv=LC_MONETARY=<base64(gzip(rc))>` carries your rc.
  sshd accepts `LC_*` by default (`AcceptEnv LANG LC_*`).
- `-o RequestTTY=force`
- `-o RemoteCommand=...` decodes the variable and starts `bash --rcfile <(...)`.
  The server's own `~/.bashrc` is sourced first, then yours.

The bootstrap unsets `LC_MONETARY` right after copying it.

## Limits

- The server must run bash and accept `LC_MONETARY`.
  Debian and Ubuntu do by default, other distros may not.
- If bash, `base64 -d` or `gunzip` is missing on the server, or it drops `LC_MONETARY`,
  you get the plain login shell and a one-line `sx:` message that gives the reason.
- Payloads over about 8 KB (encoded) trigger a warning and may be rejected.
- Only bash is supported. zsh and fish rc files are not.

## Build

```sh
cargo build --release   # binary: target/release/sx
cargo test
```
