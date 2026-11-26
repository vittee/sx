# Changelog

## v0.5.0 - 2026-11-27

### Features

- Let a profile turn the terminal window title and prompt marks off
- Add front matter to snippets and show its description on Profiles
- Give the presets front matter with a version and replace older ones on init --presets
- Leave out snippets of the profile for one host in the Extras popup and let it be dragged
- Edit the Hosts search box with the shared editor and mark the matches in the list
- Tag hosts with an environment that shows a colored badge and title and can ask before connecting
- Ask before running guarded commands on a host of an environment
- Show the server, guarded commands and payload size in sx-info, in a new order and in color
- Add sx-zsh, which starts zsh with your prompt translated from bash
- Color the question that the guard asks
- Port the oh-my-posh themes and say in the preview when a theme needs a Nerd Font or truecolor
- Choose per profile the themes that sx-theme offers on the server
- Let a prompt snippet say in its front matter that it needs a Nerd Font
- Select text in the snippet editor, the host search and the environment rows
- Edit the connection options with the shared editor and switch options with Ctrl-Left and Ctrl-Right
- Undo and redo in every text box with Ctrl-Z and Ctrl-Y
- Ask what to do with unsaved changes when leaving the snippet editor
- Make the questions modal and blink them on a click
- Hide a status message after a few seconds

### Fixes

- Show (y/N) in the delete prompt
- Show the payload size in its own box on the Profiles page
- Ignore the wheel over an unfocused Prompt pane and open a prompt snippet on double click
- Block Ctrl-C at the unsaved-changes question

### Refactoring

- Raise the payload size warning to 28 KB

### Documentation

- Re-record the demo, add the Envs page and show the badge in the screenshots
- Describe environments in the feature list, the Envs page and the layout
- Note the server bash, the no-write rule and the demo tape in AGENT.md
- Add the licenses of the code that the themes come from
- Rewrite the introduction, install, usage and how it works
- Rewrite the TUI section
- Rewrite the configuration and snippet kinds sections
- Add the front matter section and rewrite the prompt themes and snippet presets
- Rewrite the commands on the server
- Rewrite the environments section
- Rewrite the terminal integration, limitations, build and license sections
- Describe how to write the guard rules
- Note the shared editor, the modal questions and the status timeout in AGENT.md
- Note the README style in AGENT.md
- Say that Ctrl-Left and Ctrl-Right switch the named options in the options form
- Say that Esc in the snippet editor asks about unsaved changes
- Re-record the demo and the screenshots


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
