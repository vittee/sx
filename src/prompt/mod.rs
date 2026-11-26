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

pub const GIT: &str = git_inline!();
pub const RC: &str = rc_inline!();

mod themes;
pub use themes::THEMES;

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
            100..=107 => style.bg(bright_color(c - 100).unwrap_or(Color::Reset)),
            38 | 48 if codes.get(i + 1) == Some(&5) => {
                let color = Color::Indexed(codes.get(i + 2).copied().unwrap_or(7) as u8);
                i += 2;
                if c == 38 { style.fg(color) } else { style.bg(color) }
            }
            38 | 48 if codes.get(i + 1) == Some(&2) => {
                let channel = |at: usize| codes.get(at).copied().unwrap_or(0).min(255) as u8;
                let color = Color::Rgb(channel(i + 2), channel(i + 3), channel(i + 4));
                i += 4;
                if c == 38 { style.fg(color) } else { style.bg(color) }
            }
            _ => style,
        };
        i += 1;
    }
    style
}

/// A git block is a `$(...)` that prints a whole colored piece of the prompt, with its own
/// separators, when the directory is in a repository, and another piece when it is not. It is
/// `GIT_BLOCK_START`, the `printf` format for a repository (`%s` is the branch), `GIT_BLOCK_MID`,
/// the format for anywhere else and `GIT_BLOCK_END`. The colors inside are written as
/// `\[\e[...m\]`, which bash turns into the real escape codes before it runs the command. Like the
/// branch part of [`GIT`], it ends with the status it was given, so `$?` stays the same.
pub const GIT_BLOCK_START: &str = r#"$(s=$?;b=$(git branch --show-current 2>/dev/null);if [ -n "$b" ];then printf ""#;
pub const GIT_BLOCK_MID: &str = r#"" "$b";else printf ""#;
pub const GIT_BLOCK_END: &str = r#"";fi;exit $s)"#;

/// Replaces every git block with the text it prints in a repository, the branch being `main`.
fn show_git_blocks(ps1: &str) -> String {
    let mut out = String::new();
    let mut rest = ps1;
    while let Some(start) = rest.find(GIT_BLOCK_START) {
        let after = &rest[start + GIT_BLOCK_START.len()..];
        let Some(mid) = after.find(GIT_BLOCK_MID) else { break };
        let elsewhere = &after[mid + GIT_BLOCK_MID.len()..];
        let Some(end) = elsewhere.find(GIT_BLOCK_END) else { break };
        out.push_str(&rest[..start]);
        out.push_str(&after[..mid].replace("%s", "main"));
        rest = &elsewhere[end + GIT_BLOCK_END.len()..];
    }
    out.push_str(rest);
    out
}

/// Whether the prompt draws symbols from the Private Use Area (powerline arrows, icons), which
/// only a Nerd Font or a powerline font has.
pub fn needs_nerd_font(ps1: &str) -> bool {
    ps1.chars().any(|c| matches!(c, '\u{E000}'..='\u{F8FF}' | '\u{F0000}'..='\u{FFFFD}'))
}

/// Whether the prompt has `#rrggbb` colors, which a terminal without truecolor shows wrong.
pub fn needs_truecolor(ps1: &str) -> bool {
    ps1.contains("38;2;") || ps1.contains("48;2;")
}

/// Renders a bash PS1 string as styled lines using made-up values: `you@server`, the directory
/// `~/app`, the branch `main`, and a last command that failed with status 1.
pub fn render_ps1(ps1: &str) -> Vec<Line<'static>> {
    // They print the branch and, after a failed command, " ✗N".
    let ps1 = show_git_blocks(ps1);
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
            'w' => buf.push_str("~/app"),
            'W' => buf.push_str("app"),
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
