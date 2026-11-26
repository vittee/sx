Output screens-tmp.gif

Set Shell "bash"
Set FontSize 15
Set Width 1200
Set Height 760
Set Padding 14
Set Theme "Builtin Tango Dark"
Set TypingSpeed 55ms
Set WindowBar Colorful

# Connect once without the TUI, so the host list has a last used time.
Hide
Type `export PS1='$ '`
Enter
Type "export LANG=C.UTF-8"
Enter
Type "sx init >/dev/null"
Enter
Type "sx web1"
Enter
Sleep 4s
Type "exit"
Enter
Sleep 2s
# An environment to show on the Hosts and Envs pages and in the prompt.
Type `printf '[envs.prod]\nlabel = "PROD"\ncolor = "red"\nfeatures = ["badge", "title", "confirm", "guard"]\nguard = ["rm -r*", "shutdown*"]\n' >> ~/.config/sx/config.toml`
Enter
Type "clear"
Enter
Type "sx"
Enter
Sleep 2.5s
# Tag the selected host (the one used last) with the only environment.
Ctrl+T
Sleep 500ms
Show

Sleep 500ms
Screenshot screens/hosts.png
Sleep 500ms

# The connection options form, with the jump host picker open.
Ctrl+O
Sleep 500ms
Down 3
Ctrl+K
Sleep 500ms
Screenshot screens/options.png
Sleep 500ms
Escape
Escape
Sleep 500ms

Tab
Sleep 1s
Screenshot screens/snippets.png
Sleep 500ms

Tab
Sleep 1s
# Tick a few themes for sx-theme to offer on the server, then go back to the profile's own.
Right
Down
Space
Down 3
Space
Down 5
Space
Up 9
Sleep 500ms
Screenshot screens/profiles.png
Sleep 500ms

Tab
Sleep 1s
Screenshot screens/envs.png
Sleep 500ms

# Leave the TUI, then connect to show the themed prompt.
Escape
Sleep 1s
Type "clear"
Enter
Type "sx"
Enter
Sleep 2.5s
Type "web1"
Sleep 1s
Enter
# prod asks first.
Sleep 1s
Type "y"
Sleep 3s
Type "sx-theme"
Enter
Sleep 3s
Screenshot screens/sx-theme.png
Sleep 500ms
