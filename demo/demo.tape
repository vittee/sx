Output sx.gif

Set Shell "bash"
Set FontSize 15
Set Width 1200
Set Height 760
Set Padding 14
Set Theme "Builtin Tango Dark"
Set TypingSpeed 55ms
Set WindowBar Colorful

Hide
Type `export PS1='$ '`
Enter
Type "sx init >/dev/null"
Enter
Type "clear"
Enter
Show

Set TypingSpeed 35ms
Type "# 1. sx opens a TUI: pick a host and press Enter"
Sleep 600ms
Enter
Set TypingSpeed 55ms
Sleep 1s
Type "sx"
Sleep 900ms
Enter
Sleep 2.5s
Type "web1"
Sleep 1.5s
Enter
Sleep 3s
Set TypingSpeed 35ms
Type "# This is your fino prompt. ll is an alias from your sx snippet, the server never had it"
Sleep 600ms
Enter
Set TypingSpeed 55ms
Sleep 1s
Type "ll"
Sleep 900ms
Enter
Sleep 2.5s
Type "exit"
Sleep 900ms
Enter
Sleep 2s
Escape
Sleep 500ms
Escape
Sleep 1s
Type "clear"
Enter
Sleep 700ms
Set TypingSpeed 35ms
Type "# 2. Switch the prompt theme on the Profiles page"
Sleep 600ms
Enter
Set TypingSpeed 55ms
Sleep 1s
Type "sx"
Sleep 900ms
Enter
Sleep 2s
Tab
Sleep 1s
Tab
Sleep 1.5s
Right
Sleep 1s
Down 2
Sleep 1.5s
Down 2
Sleep 1.5s
Down 2
Sleep 2s
Left
Sleep 500ms
Type "q"
Sleep 1s
Type "clear"
Enter
Sleep 700ms
Set TypingSpeed 35ms
Type "# 3. Connect again: same host, new theme"
Sleep 600ms
Enter
Set TypingSpeed 55ms
Sleep 1s
Type "sx"
Sleep 900ms
Enter
Sleep 2.5s
Type "web1"
Sleep 1.5s
Enter
Sleep 3s
Set TypingSpeed 35ms
Type "# Same host, new theme. The alias is still there"
Sleep 600ms
Enter
Set TypingSpeed 55ms
Sleep 1s
Type "ll"
Sleep 900ms
Enter
Sleep 2.5s
Type "exit"
Sleep 900ms
Enter
Sleep 2s
Escape
Sleep 500ms
Escape
Sleep 1s
Type "clear"
Enter
Sleep 700ms
Set TypingSpeed 35ms
Type "# 4. Plain ssh: the server's own prompt, no aliases"
Sleep 600ms
Enter
Set TypingSpeed 55ms
Sleep 1s
Type "ssh web1"
Sleep 900ms
Enter
Sleep 2s
Set TypingSpeed 35ms
Type "# Plain ssh has no ll: the server knows nothing about your aliases"
Sleep 600ms
Enter
Set TypingSpeed 55ms
Sleep 1s
Type "ll"
Sleep 900ms
Enter
Sleep 2.5s
Type "exit"
Sleep 900ms
Enter
Sleep 1s
Type "clear"
Enter
Sleep 700ms
Set TypingSpeed 35ms
Type "# 5. Same host through sx: your prompt and aliases"
Sleep 600ms
Enter
Set TypingSpeed 55ms
Sleep 1s
Type "sx web1"
Sleep 900ms
Enter
Sleep 3s
Set TypingSpeed 35ms
Type "# With sx, ll works again. Nothing was installed on the server"
Sleep 600ms
Enter
Set TypingSpeed 55ms
Sleep 1s
Type "ll"
Sleep 900ms
Enter
Sleep 3s
Type "exit"
Sleep 900ms
Enter
Sleep 1s
