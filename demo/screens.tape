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
Type "clear"
Enter
Type "sx"
Enter
Sleep 2.5s
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
Screenshot screens/profiles.png
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
Sleep 3s
Type "sx-theme"
Enter
Sleep 3s
Screenshot screens/sx-theme.png
Sleep 500ms
