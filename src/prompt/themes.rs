//! The built-in prompt themes.

/// Built-in prompt themes: (name, snippet text).
///
/// The first seven are one line. The rest are two lines: identity and path (plus git) on the first,
/// input alone on the second, so a long server name never eats typing space. They make the host
/// stand out (badge or color) so a remote shell is not mistaken for a local one, and show `✗N`
/// after a failed command. Several are adapted from one-line oh-my-zsh themes.
///
/// Some are adapted from oh-my-posh themes (MIT, see the README for the notice). The ports keep the
/// colors and the order of the blocks, drop the segments sx does not need, and put the user, host,
/// path and branch on the first line. Their branch is a block that is not drawn outside a repository.
/// Many need a Nerd Font or truecolor, which the Profiles preview says.
pub const THEMES: &[(&str, &str)] = &[
    ("minimal", "export PS1='\\u@\\h:\\w\\$ '\n"),
    (
        "green",
        "export PS1='\\[\\e[1;32m\\]\\u@\\h\\[\\e[0m\\]:\\[\\e[1;34m\\]\\w\\[\\e[0m\\] \\$ '\n",
    ),
    (
        "two-line",
        "export PS1='\\[\\e[1;36m\\]┌─[\\u@\\h]─[\\w]\\n└─\\[\\e[1;35m\\]\\$\\[\\e[0m\\] '\n",
    ),
    (
        "git-branch",
        concat!(
            r#"export PS1='\[\e[32m\]\u@\h\[\e[0m\]:\[\e[34m\]\w\[\e[33m\]"#,
            git_inline!(),
            r#"\[\e[0m\] \$ '
"#
        ),
    ),
    (
        "red-root-style",
        "export PS1='\\[\\e[1;31m\\][\\u@\\h]\\[\\e[0m\\] \\w \\[\\e[1;31m\\]\\$\\[\\e[0m\\] '\n",
    ),
    (
        "arrow",
        "export PS1='\\[\\e[32m\\]\\u@\\h\\[\\e[0m\\] \\[\\e[36m\\]\\w\\[\\e[0m\\] \\[\\e[35m\\]❯ \\$\\[\\e[0m\\] '\n",
    ),
    (
        "timestamp",
        "export PS1='\\[\\e[90m\\][\\t]\\[\\e[0m\\] \\[\\e[33m\\]\\u@\\h\\[\\e[0m\\]:\\w \\$ '\n",
    ),
    // Inspired by oh-my-zsh `bira`: user@host as a green badge.
    (
        "bira",
        concat!(
            r#"export PS1='\[\e[36m\]╭─\[\e[0m\]\[\e[1;37;42m\] \u@\h \[\e[0m\] \[\e[1;34m\]\w\[\e[0m\]\[\e[33m\]"#,
            git_inline!(),
            r#"\[\e[0m\]\[\e[31m\]"#,
            rc_inline!(),
            r#"\[\e[0m\]\n\[\e[36m\]╰─\[\e[1m\]\$\[\e[0m\] '
"#
        ),
    ),
    // Inspired by oh-my-zsh `gnzh`: the host in bold red, which gnzh uses over SSH.
    (
        "gnzh",
        concat!(
            r#"export PS1='\[\e[36m\]╭─\[\e[0;32m\]\u\[\e[36m\]@\[\e[1;31m\]\h\[\e[0m\] \[\e[1;34m\]\w\[\e[0m\]\[\e[33m\]"#,
            git_inline!(),
            r#"\[\e[0m\]\[\e[31m\]"#,
            rc_inline!(),
            r#"\[\e[0m\]\n\[\e[36m\]╰─➤ \$\[\e[0m\] '
"#
        ),
    ),
    // Inspired by oh-my-zsh `fino`: reads as a sentence, "you at server in ~/projects on (main)".
    (
        "fino",
        concat!(
            r#"export PS1='\[\e[38;5;40m\]╭─\u \[\e[38;5;245m\]at \[\e[1;38;5;33m\]\h \[\e[0;38;5;245m\]in \[\e[1;38;5;226m\]\w\[\e[0;35m\]"#,
            git_inline!(),
            r#"\[\e[31m\]"#,
            rc_inline!(),
            r#"\[\e[0m\]\n\[\e[38;5;40m\]╰─○ \$\[\e[0m\] '
"#
        ),
    ),
    // Inspired by oh-my-zsh `ys`: "# user @ host in path [time]".
    (
        "ys",
        concat!(
            r#"export PS1='\[\e[1;34m\]#\[\e[0m\] \[\e[36m\]\u \[\e[0m\]@ \[\e[1;32m\]\h \[\e[0m\]in \[\e[1;33m\]\w\[\e[0;33m\]"#,
            git_inline!(),
            r#"\[\e[0m\] [\t]\[\e[31m\]"#,
            rc_inline!(),
            r#"\[\e[0m\]\n\[\e[1;31m\]\$\[\e[0m\] '
"#
        ),
    ),
    // Inspired by oh-my-zsh `jonathan`: box drawing, host in green parentheses.
    (
        "box",
        concat!(
            r#"export PS1='\[\e[36m\]┌─\[\e[90m\](\[\e[36m\]\u\[\e[90m\]@\[\e[1;32m\]\h\[\e[0;90m\])─(\[\e[32m\]\w\[\e[90m\])\[\e[33m\]"#,
            git_inline!(),
            r#"\[\e[31m\]"#,
            rc_inline!(),
            r#"\[\e[0m\]\n\[\e[36m\]└─\[\e[34m\]─(\[\e[33m\]\t\[\e[34m\])─\[\e[36m\]▶ \$\[\e[0m\] '
"#
        ),
    ),
    // Loudest option: a white-on-red "⇄ host" badge that cannot be mistaken for a local shell.
    (
        "remote",
        concat!(
            r#"export PS1='\[\e[1;97;41m\] ⇄ \h \[\e[0m\] \[\e[36m\]\u\[\e[0m\]:\[\e[1;34m\]\w\[\e[0;33m\]"#,
            git_inline!(),
            r#"\[\e[31m\]"#,
            rc_inline!(),
            r#"\[\e[0m\]\n\[\e[1m\]\$\[\e[0m\] '
"#
        ),
    ),
    // Adapted from the one-line oh-my-zsh `robbyrussell`: `➜ dir git:(branch)`.
    // The host becomes a blue badge, the arrow moves to its own line, and the full path is shown.
    (
        "robbyrussell",
        concat!(
            r#"export PS1='\[\e[1;97;44m\] \u@\h \[\e[0m\] \[\e[1;36m\]\w\[\e[1;34m\]"#,
            git_inline!(),
            r#"\[\e[31m\]"#,
            rc_inline!(),
            r#"\[\e[0m\]\n\[\e[1;32m\]➜ \$\[\e[0m\] '
"#
        ),
    ),
    // Adapted from `pygmalion`: orange user, host in yellow, then path and branch.
    (
        "pygmalion",
        concat!(
            r#"export PS1='\[\e[38;5;208m\]\u\[\e[0m\]@\[\e[1;33m\]\h \[\e[38;5;111m\]\w\[\e[38;5;245m\]"#,
            git_inline!(),
            r#"\[\e[31m\]"#,
            rc_inline!(),
            r#"\[\e[0m\]\n\[\e[1;31m\]» \$\[\e[0m\] '
"#
        ),
    ),
    // Adapted from `candy`: time first, host in bold blue.
    (
        "candy",
        concat!(
            r#"export PS1='\[\e[1;32m\]\t\[\e[0m\] \[\e[1;34m\]\u@\h\[\e[0m\] \[\e[1;36m\]\w\[\e[0;33m\]"#,
            git_inline!(),
            r#"\[\e[31m\]"#,
            rc_inline!(),
            r#"\[\e[0m\]\n\[\e[1;33m\]❯ \$\[\e[0m\] '
"#
        ),
    ),
    // Adapted from `af-magic`: a dim rule on the left, host in gold.
    (
        "af-magic",
        concat!(
            r#"export PS1='\[\e[38;5;237m\]───\[\e[0m\] \[\e[38;5;32m\]\u\[\e[38;5;237m\]@\[\e[1;38;5;178m\]\h \[\e[0;38;5;41m\]\w\[\e[38;5;245m\]"#,
            git_inline!(),
            r#"\[\e[31m\]"#,
            rc_inline!(),
            r#"\[\e[0m\]\n\[\e[38;5;237m\]\$\[\e[0m\] '
"#
        ),
    ),
    // Adapted from `lambda`: bold magenta host, λ on the input line.
    (
        "lambda",
        concat!(
            r#"export PS1='\[\e[1;35m\]\u@\h\[\e[0m\] \[\e[36m\]\w\[\e[33m\]"#,
            git_inline!(),
            r#"\[\e[31m\]"#,
            rc_inline!(),
            r#"\[\e[0m\]\n\[\e[1;32m\]λ \$\[\e[0m\] '
"#
        ),
    ),
    // Adapted from `cloud`: a cloud, then the host in a bright cyan badge with black text (256-color
    // codes, because bold black is drawn dark gray by many terminals).
    (
        "cloud",
        concat!(
            r#"export PS1='\[\e[1;36m\]☁ \[\e[0m\]\[\e[1;38;5;16;48;5;45m\] \u@\h \[\e[0m\] \[\e[1;34m\]\w\[\e[0;33m\]"#,
            git_inline!(),
            r#"\[\e[31m\]"#,
            rc_inline!(),
            r#"\[\e[0m\]\n\[\e[1;36m\]❯ \$\[\e[0m\] '
"#
        ),
    ),
    // Adapted from the oh-my-posh theme `paradox`.
    (
        "paradox",
        concat!(
            r##"export PS1='\[\e[0;38;2;16;14;35;48;2;255;255;255m\] \u@\h \[\e[0;38;2;255;255;255;48;2;145;221;255m\]\[\e[0;38;2;16;14;35;48;2;145;221;255m\] \w $(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf "\[\e[0;38;2;145;221;255;48;2;149;255;164m\]\[\e[0;38;2;25;53;73;48;2;149;255;164m\]  %s \[\e[0;38;2;149;255;164m\]" "$b";else printf "\[\e[0;38;2;145;221;255m\]";fi;exit $s)\[\e[0m\]\[\e[31m\]"##,
            rc_inline!(),
            r##"\[\e[0m\]\n\[\e[0;38;2;0;122;204m\]❯ \[\e[0;38;2;0;122;204m\]\$\[\e[0m\] '
"##
        ),
    ),
    // Adapted from the oh-my-posh theme `markbull`.
    (
        "markbull",
        concat!(
            r##"export PS1='\[\e[0;38;2;255;206;123m\]┌ \[\e[0;38;2;29;22;38;48;2;255;206;123m\]     \u@\h \[\e[0;38;2;255;206;123;48;2;198;120;221m\]\[\e[0;38;2;255;254;249;48;2;198;120;221m\] \w $(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf "\[\e[0;38;2;198;120;221;48;2;255;255;251m\]\[\e[0;38;2;16;14;35;48;2;255;255;251m\]  %s \[\e[0;38;2;255;255;251m\]" "$b";else printf "\[\e[0;38;2;198;120;221m\]";fi;exit $s)\[\e[0m\]\[\e[31m\]"##,
            rc_inline!(),
            r##"\[\e[0m\]\n\[\e[0;38;2;255;206;123m\]└\[\e[0;38;2;255;255;255m\] \$\[\e[0m\] '
"##
        ),
    ),
    // Adapted from the oh-my-posh theme `aliens`.
    (
        "aliens",
        concat!(
            r##"export PS1='\[\e[0;38;2;97;175;239m\]\[\e[0;38;2;255;255;255;48;2;97;175;239m\] \u@\h \[\e[0;38;2;97;175;239;48;2;198;120;221m\]\[\e[0;38;2;255;255;255;48;2;198;120;221m\] \w $(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf "\[\e[0;38;2;198;120;221;48;2;149;255;164m\]\[\e[0;38;2;25;53;73;48;2;149;255;164m\]  %s \[\e[0;38;2;149;255;164m\]" "$b";else printf "\[\e[0;38;2;198;120;221m\]";fi;exit $s)\[\e[0m\]\[\e[31m\]"##,
            rc_inline!(),
            r##"\[\e[0m\]\n\$\[\e[0m\] '
"##
        ),
    ),
    // Adapted from the oh-my-posh theme `iterm2`.
    (
        "iterm2",
        concat!(
            r##"export PS1='\[\e[0;38;2;210;255;94;48;2;0;53;67m\]\u@\h \[\e[0;38;2;0;53;67;48;2;0;135;216m\]\[\e[0;38;2;0;53;68;48;2;0;135;216m\]  \w $(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf "\[\e[0;38;2;0;135;216;48;2;210;255;94m\]\[\e[0;38;2;25;53;73;48;2;210;255;94m\]  %s \[\e[0;38;2;210;255;94m\]" "$b";else printf "\[\e[0;38;2;0;135;216m\]";fi;exit $s)\[\e[0m\]\[\e[31m\]"##,
            rc_inline!(),
            r##"\[\e[0m\]\n\[\e[0;38;2;241;24;76m\]❯ \[\e[0;38;2;241;24;76m\]\$\[\e[0m\] '
"##
        ),
    ),
    // Adapted from the oh-my-posh theme `mojada`.
    (
        "mojada",
        concat!(
            r##"export PS1='\[\e[0;38;2;0;119;194;48;2;251;251;251m\]\u\[\e[0;38;2;0;0;0;48;2;251;251;251m\]@\[\e[0;38;2;0;119;194;48;2;251;251;251m\]\[\e[0;38;2;224;108;117;48;2;251;251;251m\]\h\[\e[0;38;2;0;119;194;48;2;251;251;251m\] \[\e[0;38;2;251;251;251;48;2;0;119;194m\]\[\e[0;38;2;255;255;255;48;2;0;119;194m\]  \w $(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf "\[\e[0;38;2;0;119;194;48;2;255;251;56m\]\[\e[0;38;2;25;53;73;48;2;255;251;56m\]  %s \[\e[0;38;2;255;251;56m\]" "$b";else printf "\[\e[0;38;2;0;119;194m\]";fi;exit $s)\[\e[0m\]\[\e[31m\]"##,
            rc_inline!(),
            r##"\[\e[0m\]\n\[\e[0;38;2;255;213;79m\]\$\[\e[0m\] '
"##
        ),
    ),
    // Adapted from the oh-my-posh theme `cinnamon`.
    (
        "cinnamon",
        concat!(
            r##"export PS1='\[\e[0;38;2;7;88;92m\]\[\e[0;38;2;255;255;255;48;2;7;88;92m\] \u@\h \[\e[0;38;2;255;255;255;48;2;62;144;34m\]  \w $(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf "\[\e[0;38;2;255;255;255;48;2;222;7;111m\] %s " "$b";else printf "";fi;exit $s)\[\e[0m\]\[\e[31m\]"##,
            rc_inline!(),
            r##"\[\e[0m\]\n\$\[\e[0m\] '
"##
        ),
    ),
    // Adapted from the oh-my-posh theme `larserikfinholt`.
    (
        "larserikfinholt",
        concat!(
            r##"export PS1='\[\e[0;38;2;195;134;241m\]\[\e[0;38;2;255;255;255;48;2;195;134;241m\] \u@\h \[\e[0;38;2;195;134;241;48;2;255;71;156m\]\[\e[0;38;2;255;255;255;48;2;255;71;156m\]   \w $(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf "\[\e[0;38;2;255;71;156;48;2;255;251;56m\]\[\e[0;38;2;25;53;73;48;2;255;251;56m\]  %s \[\e[0;38;2;255;251;56m\]" "$b";else printf "\[\e[0;38;2;255;71;156m\]";fi;exit $s)\[\e[0m\]\[\e[31m\]"##,
            rc_inline!(),
            r##"\[\e[0m\]\n\$\[\e[0m\] '
"##
        ),
    ),
    // Adapted from the oh-my-posh theme `M365Princess`.
    (
        "m365princess",
        concat!(
            r##"export PS1='\[\e[0;38;2;154;52;142m\]\[\e[0;38;2;255;255;255;48;2;154;52;142m\]\u@\h \[\e[0;38;2;255;255;255;48;2;218;98;125m\] \w $(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf "\[\e[0;38;2;218;98;125;48;2;252;161;125m\]\[\e[0;38;2;255;255;255;48;2;252;161;125m\] ➜ (%s) \[\e[0;38;2;252;161;125;48;2;51;101;138m\]" "$b";else printf "\[\e[0;38;2;218;98;125;48;2;51;101;138m\]";fi;exit $s)\[\e[0;38;2;255;255;255;48;2;51;101;138m\] ♥ \A \[\e[0;38;2;51;101;138m\]\[\e[0m\]\[\e[31m\]"##,
            rc_inline!(),
            r##"\[\e[0m\]\n\$\[\e[0m\] '
"##
        ),
    ),
    // Adapted from the oh-my-posh theme `remk`.
    (
        "remk",
        concat!(
            r##"export PS1='\[\e[0;95m\]\[\e[0;38;2;255;255;255;105m\]\u@\h \[\e[0;38;2;63;63;63;103m\] \w $(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf "\[\e[0;38;2;255;255;255;106m\] git(%s) " "$b";else printf "";fi;exit $s)\[\e[0;38;2;255;255;255;102m\] \A \[\e[0;92m\]\[\e[0m\]\[\e[31m\]"##,
            rc_inline!(),
            r##"\[\e[0m\]\n\$\[\e[0m\] '
"##
        ),
    ),
    // Adapted from the oh-my-posh theme `dracula`.
    (
        "dracula",
        concat!(
            r##"export PS1='\[\e[0;38;2;98;114;164m\]\[\e[0;38;2;248;248;242;48;2;98;114;164m\]\u@\h \[\e[0;38;2;248;248;242;48;2;189;147;249m\] \w $(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf "\[\e[0;38;2;189;147;249;48;2;255;184;108m\]\[\e[0;38;2;248;248;242;48;2;255;184;108m\]  (%s) \[\e[0;38;2;255;184;108;48;2;255;121;198m\]" "$b";else printf "\[\e[0;38;2;189;147;249;48;2;255;121;198m\]";fi;exit $s)\[\e[0;38;2;248;248;242;48;2;255;121;198m\] ♥ \A \[\e[0;38;2;255;121;198m\]\[\e[0m\]\[\e[31m\]"##,
            rc_inline!(),
            r##"\[\e[0m\]\n\$\[\e[0m\] '
"##
        ),
    ),
    // Adapted from the oh-my-posh theme `marcduiker`.
    (
        "marcduiker",
        concat!(
            r##"export PS1='\[\e[0;38;2;254;174;52m\]\u@\h \[\e[0;38;2;254;174;52m\]\[\e[0;38;2;38;43;68;48;2;254;174;52m\]  \w $(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf "\[\e[0;38;2;254;174;52;48;2;254;231;97m\]\[\e[0;38;2;38;43;68;48;2;254;231;97m\]  %s \[\e[0;38;2;254;231;97m\]" "$b";else printf "\[\e[0;38;2;254;174;52m\]";fi;exit $s)\[\e[0m\]\[\e[31m\]"##,
            rc_inline!(),
            r##"\[\e[0m\]\n\$\[\e[0m\] '
"##
        ),
    ),
    // Adapted from the oh-my-posh theme `slim`.
    (
        "slim",
        concat!(
            r##"export PS1='\[\e[0;38;2;250;250;250;48;2;47;47;47m\] \[\e[0;38;2;46;239;191;48;2;47;47;47m\]\u@\h\[\e[0;38;2;250;250;250;48;2;47;47;47m\]\[\e[0;38;2;122;122;122;48;2;47;47;47m\] \[\e[0;38;2;250;250;250;48;2;47;47;47m\]\[\e[0;38;2;250;250;250;48;2;47;47;47m\]\[\e[0;38;2;242;242;0;48;2;47;47;47m\]  \[\e[0;38;2;250;250;250;48;2;47;47;47m\]\w $(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf "\[\e[0;38;2;255;235;59;48;2;47;47;47m\]\[\e[0;38;2;122;122;122;48;2;47;47;47m\] \[\e[0;38;2;255;235;59;48;2;47;47;47m\] %s " "$b";else printf "";fi;exit $s)\[\e[0m\]\[\e[31m\]"##,
            rc_inline!(),
            r##"\[\e[0m\]\n\[\e[0;38;2;122;122;122m\]❯ \[\e[0;38;2;122;122;122m\]\$\[\e[0m\] '
"##
        ),
    ),
    // Adapted from the oh-my-posh theme `half-life`.
    (
        "half-life",
        concat!(
            r##"export PS1='\[\e[0;38;2;126;70;182m\]\u@\h \[\e[0;38;2;255;255;255m\]in \[\e[0;38;2;135;255;0m\]\w $(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf "\[\e[0;38;2;95;215;255m\]\[\e[0;38;2;255;255;255m\]on\[\e[0;38;2;95;215;255m\] %s " "$b";else printf "";fi;exit $s)\[\e[0m\]\[\e[31m\]"##,
            rc_inline!(),
            r##"\[\e[0m\]\n\[\e[0;38;2;215;95;0m\]λ \[\e[0;38;2;215;95;0m\]\$\[\e[0m\] '
"##
        ),
    ),
    // Adapted from the oh-my-posh theme `spaceship`.
    (
        "spaceship",
        concat!(
            r##"export PS1='\[\e[0;93m\]\u@\h \[\e[0;36m\]\[\e[0;38;2;255;255;255m\]in\[\e[0;36m\] \w $(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf "\[\e[0;38;2;255;148;223m\]\[\e[0;38;2;255;255;255m\]on\[\e[0;38;2;255;148;223m\]  \[\e[0;38;2;255;148;223m\]\[\e[0;1;38;2;255;148;223m\] \[\e[0;38;2;255;148;223m\]\[\e[0;38;2;255;148;223m\]%s " "$b";else printf "";fi;exit $s)\[\e[0m\]\[\e[31m\]"##,
            rc_inline!(),
            r##"\[\e[0m\]\n\[\e[0;92m\]❯ \[\e[0;92m\]\$\[\e[0m\] '
"##
        ),
    ),
    // Adapted from the oh-my-posh theme `material`.
    (
        "material",
        concat!(
            r##"export PS1='\[\e[0;38;2;86;182;194m\]\u@\h \[\e[0;38;2;255;255;255m\]\[\e[0;38;2;197;145;232m\]❯\[\e[0;38;2;255;255;255m\]\[\e[0;38;2;105;255;148m\]❯\[\e[0;38;2;255;255;255m\]\[\e[0;38;2;86;182;194m\] \w $(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf "\[\e[0;38;2;208;102;111m\]\[\e[0;38;2;95;170;232m\]git:(\[\e[0;38;2;208;102;111m\]%s\[\e[0;38;2;95;170;232m\])\[\e[0;38;2;208;102;111m\]" "$b";else printf "";fi;exit $s)\[\e[0;38;2;102;246;143m\] \A \[\e[0m\]\[\e[31m\]"##,
            rc_inline!(),
            r##"\[\e[0m\]\n\$\[\e[0m\] '
"##
        ),
    ),
    // Adapted from the oh-my-posh theme `kali`.
    (
        "kali",
        concat!(
            r##"export PS1='\[\e[0;94m\]\[\e[0;32m\]┌──(\[\e[0;94m\]\u㉿\h\[\e[0;32m\])\[\e[0;94m\]\[\e[0m\]\[\e[0;32m\]-[\[\e[0m\]\w\[\e[0;32m\]]\[\e[0m\]$(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf "\[\e[0m\]\[\e[0;32m\]-[\[\e[0m\] %s\[\e[0;32m\]]\[\e[0m\]" "$b";else printf "";fi;exit $s)\[\e[0m\]\[\e[31m\]"##,
            rc_inline!(),
            r##"\[\e[0m\]\n\[\e[0;94m\]\[\e[0;32m\]└─\[\e[0;94m\]\$\[\e[0m\] '
"##
        ),
    ),
    // Adapted from the oh-my-posh theme `mt`.
    (
        "mt",
        concat!(
            r##"export PS1='\[\e[0;38;2;180;0;158m\]\[\e[0;38;2;255;255;255;48;2;180;0;158m\]\u@\h \[\e[0;38;2;0;0;0;48;2;255;255;0m\] \w $(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf "\[\e[0;38;2;255;255;0;48;2;78;68;255m\]\[\e[0;38;2;255;255;255;48;2;78;68;255m\] ➜ (%s) \[\e[0;38;2;78;68;255;48;2;22;198;12m\]" "$b";else printf "\[\e[0;38;2;255;255;0;48;2;22;198;12m\]";fi;exit $s)\[\e[0;38;2;255;255;255;48;2;22;198;12m\] ♥ \A \[\e[0;38;2;22;198;12m\]\[\e[0m\]\[\e[31m\]"##,
            rc_inline!(),
            r##"\[\e[0m\]\n\$\[\e[0m\] '
"##
        ),
    ),
];
