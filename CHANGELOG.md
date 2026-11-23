# Changelog

## v0.4.0 - 2026-11-23

### Features

- Add the colorterm preset
- Add the no-history preset
- Add clip to copy to the local clipboard
- Set the terminal title on the server and restore it after
- Mark prompts for the terminal with OSC 133
- Add sx-theme to switch prompts in a session
- Add sx-sudo to keep your shell under sudo
- Add sx-tmux to give tmux windows your shell
- Add man-colors, cmd-colors and utf8 shell presets
- Add dir-keys preset to walk back and forward through directories
- Add history-search, keys-fix and sudo-esc inputrc presets
- Add completion-colors, menu-complete and no-bell inputrc presets
- Add sx-info to show what sx gave this shell
- Open a snippet in the Snippets tab when its name is double-clicked
- Pick the jump host from the other hosts with Ctrl-K
- List likely jump hosts first in the jump host picker

### Fixes

- Put the prompt's git and status code inside PS1 so new shells need no functions
- Show user@host in the built-in themes that only had the host
- Carry the sx-theme choice into the sx-sudo shell
- Use Ctrl keys on the Snippets and Profiles pages like Hosts
- Change the host profile with Ctrl-P so arrow keys cannot change it
- Preview a prompt first and apply it with Enter
- Make Esc go back one pane in Profiles
- Order the footer hints and show Tab on Hosts
- Apply inputrc snippets with bind one line at a time
- Highlight inputrc snippets and color them light blue
- Toggle a snippet in Profiles only from its checkbox
- Fill the Profiles snippet grid column by column

### Documentation

- Record the demo again and add screenshots for the README
- Restructure the README with a contents list and screenshots


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
