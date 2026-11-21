//! Colors for `sx --help`, in the style of `cargo --help`: green headings, cyan commands and flags.
//! The help text stays plain in `main.rs`. This only adds escape codes when `color` is on, so
//! removing them gives back the text exactly.

const HEADING: &str = "\x1b[1;32m";
const LITERAL: &str = "\x1b[1;36m";
const PLACEHOLDER: &str = "\x1b[36m";
const RESET: &str = "\x1b[0m";

/// The help with color codes when `color` is true, else the text as it is.
pub fn paint(text: &str, color: bool) -> String {
    if !color {
        return text.to_string();
    }
    text.lines().map(paint_line).collect::<Vec<_>>().join("\n")
}

fn paint_line(line: &str) -> String {
    if let Some((head, rest)) = ["Usage:", "Config:"].iter().find_map(|h| line.strip_prefix(h).map(|r| (*h, r))) {
        return format!("{HEADING}{head}{RESET}{rest}");
    }
    if !line.starts_with("  sx") {
        return line.to_string();
    }
    // A usage line is the command, then its description after a gap of two spaces or more.
    let gap = line[4..].find("  ").map(|i| i + 4);
    let (command, description) = line.split_at(gap.unwrap_or(line.len()));
    format!("{}{description}", paint_command(command))
}

/// `sx completions --install [shell]`: words are bold cyan, `<x>` and `[x ...]` are cyan.
fn paint_command(command: &str) -> String {
    let mut open = 0i32; // how many `[` are still open, since `[ssh args]` has a space in it
    command
        .split(' ')
        .map(|word| {
            let placeholder = open > 0 || word.starts_with('<') || word.starts_with('[');
            open += word.matches('[').count() as i32 - word.matches(']').count() as i32;
            match word {
                "" => String::new(),
                w if placeholder => format!("{PLACEHOLDER}{w}{RESET}"),
                w => format!("{LITERAL}{w}{RESET}"),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "sx: run ssh.\n\nUsage:  ([] is optional)\n  sx [ssh args] <host>        like ssh\n  sx init [--presets]         create the files,\n                              --presets adds more\n  sx --help\n\nConfig: $SX_HOME\nplain text, -N and -T";

    fn strip(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                for e in chars.by_ref() {
                    if e == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn without_color_the_text_is_unchanged() {
        assert_eq!(paint(SAMPLE, false), SAMPLE);
    }

    #[test]
    fn color_only_adds_codes() {
        assert_eq!(strip(&paint(SAMPLE, true)), SAMPLE);
    }

    #[test]
    fn headings_are_green_and_commands_cyan() {
        let p = paint(SAMPLE, true);
        assert!(p.contains(&format!("{HEADING}Usage:{RESET}")), "{p}");
        assert!(p.contains(&format!("{HEADING}Config:{RESET}")));
        assert!(p.contains(&format!("{LITERAL}sx{RESET} {PLACEHOLDER}[ssh{RESET} {PLACEHOLDER}args]{RESET} {PLACEHOLDER}<host>{RESET}")), "{p}");
        assert!(p.contains(&format!("{LITERAL}init{RESET} {PLACEHOLDER}[--presets]{RESET}")));
        assert!(p.contains(&format!("{PLACEHOLDER}<host>{RESET}")));
        assert!(p.contains(&format!("{LITERAL}--help{RESET}")));
    }

    #[test]
    fn descriptions_and_prose_stay_plain() {
        let p = paint(SAMPLE, true);
        assert!(p.contains("        like ssh\n"), "{p}");
        assert!(p.contains("--presets adds more"), "the wrapped description is not colored");
        assert!(p.contains("plain text, -N and -T"));
    }
}
