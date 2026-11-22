//! Prompt themes and a small PS1 renderer used for the live preview.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// The git branch and the exit status as code inside `PS1`, with no function to define. A prompt that
/// calls a function breaks in a shell that has the prompt but not the function (`sudo -i`, `su`,
/// `tmux`), because functions are not inherited and `PS1` is. Both substitutions end with `exit`, so
/// `$?` keeps the status of the command you ran for the one after them.
macro_rules! git_inline {
    () => {
        r#"$(s=$?;b=$(git branch --show-current 2>/dev/null)&&[ -n "$b" ]&&printf " (%s)" "$b";exit $s)"#
    };
}

macro_rules! rc_inline {
    () => {
        r#"$(c=$?;[ $c -ne 0 ]&&printf " ✗%s" $c;exit $c)"#
    };
}

const GIT: &str = git_inline!();
const RC: &str = rc_inline!();

/// Built-in prompt themes: (name, snippet text).
///
/// The first seven are one line. The rest are two lines: identity and path (plus git) on the first,
/// input alone on the second, so a long server name never eats typing space. They make the host
/// stand out (badge or color) so a remote shell is not mistaken for a local one, and show `✗N`
/// after a failed command. Several are adapted from one-line oh-my-zsh themes.
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
        "export PS1='\\[\\e[1;31m\\][\\h]\\[\\e[0m\\] \\w \\[\\e[1;31m\\]\\$\\[\\e[0m\\] '\n",
    ),
    (
        "arrow",
        "export PS1='\\[\\e[32m\\]\\h\\[\\e[0m\\] \\[\\e[36m\\]\\w\\[\\e[0m\\] \\[\\e[35m\\]❯ \\$\\[\\e[0m\\] '\n",
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
            r#"export PS1='\[\e[1;97;44m\] \h \[\e[0m\] \[\e[1;36m\]\w\[\e[1;34m\]"#,
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
            r#"export PS1='\[\e[1;35m\]\h\[\e[0m\] \[\e[36m\]\w\[\e[33m\]"#,
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
            r#"export PS1='\[\e[1;36m\]☁ \[\e[0m\]\[\e[1;38;5;16;48;5;45m\] \h \[\e[0m\] \[\e[1;34m\]\w\[\e[0;33m\]"#,
            git_inline!(),
            r#"\[\e[31m\]"#,
            rc_inline!(),
            r#"\[\e[0m\]\n\[\e[1;36m\]❯ \$\[\e[0m\] '
"#
        ),
    ),
];

/// Theme given to a fresh default profile.
pub const DEFAULT_THEME: &str = "fino";

/// Snippet text for a built-in theme.
pub fn theme(name: &str) -> Option<&'static str> {
    THEMES.iter().find(|(n, _)| *n == name).map(|(_, text)| *text)
}

/// Finds the value of the first `PS1=` assignment in a snippet (single or double quoted, or bare).
pub fn extract_ps1(text: &str) -> Option<String> {
    for line in text.lines() {
        let Some(pos) = line.find("PS1=") else { continue };
        let value = line[pos + 4..].trim();
        let mut chars = value.chars();
        return match chars.next() {
            Some(q @ ('\'' | '"')) => {
                let rest = &value[1..];
                Some(rest.rfind(q).map_or(rest, |end| &rest[..end]).to_string())
            }
            Some(_) => Some(value.split_whitespace().next().unwrap_or("").to_string()),
            None => None,
        };
    }
    None
}

/// True when a `.prompt` file is bash (it has a `PS1=` line) and not just the PS1 string.
/// Such a file is used as is, which keeps files written for earlier versions working.
pub fn is_script(text: &str) -> bool {
    text.lines().any(|l| l.contains("PS1="))
}

/// The PS1 string of a `.prompt` file: the value of its `PS1=` line when it is bash, else the
/// whole text without the trailing newline. `None` when there is nothing to show.
pub fn file_ps1(text: &str) -> Option<String> {
    if is_script(text) {
        return extract_ps1(text);
    }
    let ps1 = text.trim_end_matches(['\n', '\r']);
    (!ps1.trim().is_empty()).then(|| ps1.to_string())
}

/// Bash that sets the prompt from a `.prompt` file. A file that already is bash is returned as it
/// is. A plain PS1 string is single-quoted into `export PS1='...'`, and `$(__sx_git)` and `$(__sx_rc)`
/// in it become the inline code for the git branch and the exit status.
pub fn script_for(text: &str) -> String {
    if is_script(text) {
        return text.to_string();
    }
    let Some(ps1) = file_ps1(text) else { return String::new() };
    let ps1 = ps1.replace("$(__sx_git)", GIT).replace("$(__sx_rc)", RC);
    format!("export PS1='{}'\n", ps1.replace('\'', r"'\''"))
}

fn sgr_color(code: u32) -> Option<Color> {
    Some(match code {
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::Gray,
        _ => return None,
    })
}

fn bright_color(code: u32) -> Option<Color> {
    Some(match code {
        0 => Color::DarkGray,
        1 => Color::LightRed,
        2 => Color::LightGreen,
        3 => Color::LightYellow,
        4 => Color::LightBlue,
        5 => Color::LightMagenta,
        6 => Color::LightCyan,
        7 => Color::White,
        _ => return None,
    })
}

fn apply_sgr(style: Style, params: &str) -> Style {
    let codes: Vec<u32> = params.split(';').map(|p| p.parse().unwrap_or(0)).collect();
    let mut style = style;
    let mut i = 0;
    while i < codes.len() {
        let c = codes[i];
        style = match c {
            0 => Style::default(),
            1 => style.add_modifier(Modifier::BOLD),
            4 => style.add_modifier(Modifier::UNDERLINED),
            30..=37 => style.fg(sgr_color(c - 30).unwrap_or(Color::Reset)),
            39 => style.fg(Color::Reset),
            40..=47 => style.bg(sgr_color(c - 40).unwrap_or(Color::Reset)),
            49 => style.bg(Color::Reset),
            90..=97 => style.fg(bright_color(c - 90).unwrap_or(Color::Reset)),
            38 | 48 if codes.get(i + 1) == Some(&5) => {
                let color = Color::Indexed(codes.get(i + 2).copied().unwrap_or(7) as u8);
                i += 2;
                if c == 38 { style.fg(color) } else { style.bg(color) }
            }
            _ => style,
        };
        i += 1;
    }
    style
}

/// Renders a bash PS1 string as styled lines using made-up values: `you@server`, the directory
/// `~/projects/sx`, the branch `main`, and a last command that failed with status 1.
pub fn render_ps1(ps1: &str) -> Vec<Line<'static>> {
    // They print the branch and, after a failed command, " ✗N".
    let ps1 = ps1.replace(GIT, " (main)").replace(RC, " ✗1").replace("$(__sx_git)", " (main)").replace("$(__sx_rc)", " ✗1");
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut buf = String::new();
    let mut style = Style::default();

    let flush = |buf: &mut String, spans: &mut Vec<Span<'static>>, style: Style| {
        if !buf.is_empty() {
            spans.push(Span::styled(std::mem::take(buf), style));
        }
    };

    let chars: Vec<char> = ps1.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '\\' {
            buf.push(chars[i]);
            i += 1;
            continue;
        }
        i += 1;
        let Some(&c) = chars.get(i) else { break };
        i += 1;
        match c {
            'u' => buf.push_str("you"),
            'h' | 'H' => buf.push_str("server"),
            'w' => buf.push_str("~/projects/sx"),
            'W' => buf.push_str("sx"),
            '$' => buf.push('$'),
            't' => buf.push_str("12:00:00"),
            'A' => buf.push_str("12:00"),
            '\\' => buf.push('\\'),
            'n' => {
                flush(&mut buf, &mut spans, style);
                lines.push(Line::from(std::mem::take(&mut spans)));
            }
            'e' | '0' => {
                // `\e[...m` or `\033[...m`
                if c == '0' {
                    while chars.get(i) == Some(&'3') {
                        i += 1;
                    }
                }
                if chars.get(i) == Some(&'[')
                    && let Some(end) = chars[i..].iter().position(|&ch| ch == 'm')
                {
                    let params: String = chars[i + 1..i + end].iter().collect();
                    flush(&mut buf, &mut spans, style);
                    style = apply_sgr(style, &params);
                    i += end + 1;
                }
            }
            _ => {} // \[ \] and unsupported escapes render nothing
        }
    }
    flush(&mut buf, &mut spans, style);
    lines.push(Line::from(spans));
    lines
}

#[cfg(test)]
mod tests;
