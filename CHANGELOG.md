# Changelog

## v0.1.0 - 2026-11-21

First release.

- `sx [ssh args] host` runs `ssh` so the remote bash starts with your own prompt and aliases.
  The rc travels inside `RemoteCommand`, so the server needs no `AcceptEnv` setting.
  Nothing is installed or written on the server.
- Snippets are files in `~/.config/sx/snippets`.
  The file name sets the kind: shell, `.vim`, `.tmux`, `.inputrc` or `.prompt`.
- Nine vim presets (`sx init --presets` adds the ones that are missing, switched off).
- Eleven prompt themes based on oh-my-zsh themes, with `fino` as the default.
- Profiles group snippets and a prompt. Each host gets a profile, extra snippets, or none.
- A TUI with Hosts, Snippets and Profiles pages: host list from `~/.ssh/config` and
  `known_hosts` with a source column, sorting, favorites, a per-host session history,
  a snippet editor with highlighting for bash, vim and PS1, and a live prompt preview.
- Ctrl-C while connecting aborts cleanly and removes the "connecting" line.
- Prebuilt binaries and install scripts for Windows, macOS and Linux.
