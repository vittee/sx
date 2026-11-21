# Changelog

## v0.3.0 - 2026-11-21

### Features

- Show the version with --version and in the TUI title
- Pick the identity file from the keys in ~/.ssh with Ctrl-K
- Use saved options from the command line, with --options <name>
- Size the host, target and profile columns to the terminal
- Add shell completion with sx completions
- Color the help text
- Show the config directory in the help text

### Fixes

- Keep the cursor on the host after a connection re-sorts the list
- Export the prompt helper functions so tmux panes find them
- Update last used for connections made from the command line

## v0.2.0 - 2026-11-21

- Per-host connection options in the TUI.
  `Ctrl-O` opens a form for the user, port, identity file, jump host and extra ssh arguments.
  `Enter` connects with them once, `Ctrl-S` saves them in `config.toml`.
  `~/.ssh/config` is never changed.
- A host can keep several named options, such as `root` or `deploy via bastion`.
  Add, rename, delete and switch them in the form, and pick one with `Ctrl-L` when connecting.
  New named options that you connect with straight away are saved.
- Typing `root@web` in the search box connects to the matching host as `root`, for that connection only.
- The options form and the picker take the mouse and can be dragged by their title bar.
- A new `invert_scroll` setting reverses the mouse wheel, for trackpads that scroll the wrong way.

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
