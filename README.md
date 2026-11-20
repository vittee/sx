# sx

Run `ssh` so the remote bash uses **your** rc (prompt, aliases) instead of the server's.
No server setup. Nothing is written on the server.

Idea from [viSSHous](https://github.com/gWOLF3/viSSHous), which does the same for `~/.vimrc`.

## How it will work

`sx` will start the real `ssh` with three extra options:

- `-o SetEnv=LC_MONETARY=<payload>` carries your rc to the server.
  sshd accepts `LC_*` variables by default (`AcceptEnv LANG LC_*`).
- `-o RequestTTY=force`
- `-o RemoteCommand=<bootstrap>` starts bash on the server with the payload as its rc file.

The payload is your rc, gzipped and base64-encoded.

## Status

Only the first piece exists: the payload encoder.
`sx` reads an rc from standard input and prints the value it would send:

```sh
echo 'alias ll="ls -l"' | cargo run
```

The server's own `~/.bashrc` is sourced first, so its environment stays.
Line endings are normalised to `
`.
`sx` warns on standard error when the encoded value is over 8192 bytes, because the server may reject it.

The `ssh` wrapper itself, the bootstrap command and the configuration are not written yet.

## Development

```sh
cargo test
```
