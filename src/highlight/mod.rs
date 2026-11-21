//! Small bash syntax highlighter for the snippet editor.
//!
//! It classifies every character of every line. It is not a parser: it knows comments, quotes (also
//! across lines), `$` expansions, keywords, builtins, function names, assignments and operators,
//! which is enough to make a shell snippet readable. Heredocs are not tracked.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Plain,
    Comment,
    Str,
    Var,
    Keyword,
    Builtin,
    Func,
    Op,
    Number,
}

/// Classifies a PS1 string, line by line. Prompt escapes (`\u`, `\h`, `\w`, `\$`) are keywords,
/// the `\[` `\]` markers around invisible text are comments, color sequences (`\e[32m`) are
/// numbers, and `$(...)` and `$NAME` are variables.
pub fn highlight_ps1(lines: &[String]) -> Vec<Vec<Kind>> {
    lines.iter().map(|l| ps1_line(&l.chars().collect::<Vec<_>>())).collect()
}

fn ps1_line(c: &[char]) -> Vec<Kind> {
    let mut out = vec![Kind::Plain; c.len()];
    let mut i = 0;
    while i < c.len() {
        let (len, kind) = match c[i] {
            '\\' if matches!(c.get(i + 1), Some('[' | ']')) => (2, Kind::Comment),
            '\\' => match color_escape_len(&c[i..]) {
                Some(n) => (n, Kind::Number),
                None => (if i + 1 < c.len() { 2 } else { 1 }, Kind::Keyword),
            },
            '$' if c.get(i + 1) == Some(&'(') => {
                let end = c[i..].iter().position(|&x| x == ')').map_or(c.len() - i, |p| p + 1);
                (end, Kind::Var)
            }
            '$' => {
                let n = c[i + 1..].iter().take_while(|x| x.is_alphanumeric() || **x == '_').count();
                (if n == 0 { 1 } else { n + 1 }, if n == 0 { Kind::Plain } else { Kind::Var })
            }
            _ => (1, Kind::Plain),
        };
        out[i..i + len].fill(kind);
        i += len;
    }
    out
}

/// Length of a color sequence such as `\e[1;32m`, `\033[0m` or `\x1b[0m` at the start of `c`.
fn color_escape_len(c: &[char]) -> Option<usize> {
    let intro = [&['\\', 'e', '['][..], &['\\', '0', '3', '3', '['], &['\\', 'x', '1', 'b', '[']]
        .into_iter()
        .find(|p| c.starts_with(p))?
        .len();
    let m = c[intro..].iter().position(|&x| x == 'm')?;
    c[intro..intro + m].iter().all(|x| x.is_ascii_digit() || *x == ';').then_some(intro + m + 1)
}

const VIM_COMMANDS: &[&str] = &[
    "set", "setlocal", "setglobal", "let", "unlet", "syntax", "syn", "filetype", "colorscheme", "highlight", "hi",
    "autocmd", "au", "augroup", "command", "call", "echo", "echom", "execute", "source", "runtime", "if", "elseif",
    "else", "endif", "for", "endfor", "while", "endwhile", "function", "endfunction", "return", "try", "catch",
    "finally", "endtry", "silent", "map", "noremap", "nmap", "nnoremap", "imap", "inoremap", "vmap", "vnoremap",
    "xmap", "xnoremap", "cmap", "cnoremap", "omap", "onoremap", "tmap", "tnoremap", "unmap", "nunmap", "iunmap",
    "abbreviate", "iabbrev", "cabbrev", "packadd", "scriptencoding", "language",
];

/// Classifies vim script, line by line. A line that starts with `"` is a comment, the command
/// word (`set`, `nnoremap`, `let`) is a keyword, option and variable names are variables, `<CR>`
/// style key names are operators, and quoted text and numbers have their own kinds. Not a parser.
pub fn highlight_vim(lines: &[String]) -> Vec<Vec<Kind>> {
    lines.iter().map(|l| vim_line(&l.chars().collect::<Vec<_>>())).collect()
}

fn vim_line(c: &[char]) -> Vec<Kind> {
    let mut out = vec![Kind::Plain; c.len()];
    let mut i = c.iter().position(|x| !x.is_whitespace()).unwrap_or(c.len());
    if c.get(i) == Some(&'"') {
        out[i..].fill(Kind::Comment);
        return out;
    }
    // command word(s): `silent! nnoremap`, `set`, `let`
    let mut cmd = String::new();
    let mut first = true;
    loop {
        let start = i;
        while i < c.len() && (c[i].is_ascii_alphabetic() || (c[i] == '!' && i > start)) {
            i += 1;
        }
        let word: String = c[start..i].iter().collect();
        let name = word.trim_end_matches('!');
        if name.is_empty() || !VIM_COMMANDS.contains(&name) {
            i = start;
            break;
        }
        out[start..i].fill(Kind::Keyword);
        if first {
            cmd = name.to_string();
            first = false;
        }
        while i < c.len() && c[i].is_whitespace() {
            i += 1;
        }
        if name != "silent" {
            break;
        }
    }
    let is_set = matches!(cmd.as_str(), "set" | "setlocal" | "setglobal");
    let is_let = matches!(cmd.as_str(), "let" | "unlet");
    let mut word_index = 0;
    let mut in_value = false; // after `=` in a `set` word, up to the next space
    while i < c.len() {
        let ch = c[i];
        if ch == '"' {
            // a closing quote makes a string, otherwise the rest of the line is a comment
            match c[i + 1..].iter().position(|&x| x == '"') {
                Some(p) => {
                    out[i..i + p + 2].fill(Kind::Str);
                    i += p + 2;
                }
                None => {
                    out[i..].fill(Kind::Comment);
                    return out;
                }
            }
        } else if ch == '\'' {
            let end = c[i + 1..].iter().position(|&x| x == '\'').map_or(c.len(), |p| i + p + 2);
            out[i..end].fill(Kind::Str);
            i = end;
        } else if ch == '<' {
            match c[i..].iter().position(|&x| x == '>') {
                Some(p) if p > 1 && c[i + 1..i + p].iter().all(|x| x.is_ascii_alphanumeric() || matches!(x, '-' | '_')) => {
                    out[i..=i + p].fill(Kind::Op);
                    i += p + 1;
                }
                _ => i += 1,
            }
        } else if ch.is_ascii_digit() {
            let n = c[i..].iter().take_while(|x| x.is_ascii_digit()).count();
            out[i..i + n].fill(Kind::Number);
            i += n;
        } else if ch.is_alphabetic() || ch == '&' || ch == '_' {
            let n = c[i..].iter().take_while(|x| x.is_alphanumeric() || matches!(x, '_' | ':' | '&')).count();
            let scoped = !in_value && c[i..i + n].contains(&':'); // g:name, s:name
            let kind = if (is_set && !in_value) || (is_let && word_index == 0) || ch == '&' || scoped {
                Kind::Var
            } else {
                Kind::Plain
            };
            out[i..i + n].fill(kind);
            word_index += 1;
            i += n;
        } else if matches!(ch, '=' | '+' | '-') && is_set && c.get(i + 1) == Some(&'=') {
            out[i..i + 2].fill(Kind::Op);
            in_value = true;
            i += 2;
        } else if matches!(ch, '=' | '!' | '|') {
            out[i] = Kind::Op;
            in_value = ch == '=' && is_set;
            i += 1;
        } else {
            if ch.is_whitespace() {
                in_value = false;
            }
            i += 1;
        }
    }
    out
}

const KEYWORDS: &[&str] = &[
    "if", "then", "else", "elif", "fi", "for", "while", "until", "do", "done", "case", "esac", "in", "function",
    "select", "time", "!", "{", "}", "[[", "]]",
];

/// Keywords after which the next word is again at the start of a command.
const OPENERS: &[&str] = &["if", "then", "else", "elif", "while", "until", "do", "time", "!", "{"];

const BUILTINS: &[&str] = &[
    "alias", "bg", "bind", "break", "builtin", "cd", "command", "continue", "declare", "echo", "eval", "exec",
    "exit", "export", "false", "fg", "getopts", "hash", "history", "jobs", "kill", "let", "local", "logout",
    "popd", "printf", "pushd", "pwd", "read", "readonly", "return", "set", "shift", "shopt", "source", "test",
    "trap", "true", "type", "typeset", "ulimit", "umask", "unalias", "unset", "wait", ".", ":", "[",
];

fn is_word_char(c: char) -> bool {
    !(c.is_whitespace() || "|&;<>()\"'$\\`=".contains(c))
}

/// Highlight every line. The result has one `Kind` per `char` of each line.
pub fn highlight(lines: &[String]) -> Vec<Vec<Kind>> {
    let mut quote: Option<char> = None; // a quote left open at the end of the previous line
    lines.iter().map(|l| line(l, &mut quote)).collect()
}

fn line(text: &str, quote: &mut Option<char>) -> Vec<Kind> {
    let ch: Vec<char> = text.chars().collect();
    let mut out = vec![Kind::Plain; ch.len()];
    let mut i = 0;
    let mut cmd_pos = quote.is_none();
    let mut after_function = false;

    while i < ch.len() {
        let c = ch[i];

        if let Some(q) = *quote {
            if c == q {
                out[i] = Kind::Str;
                *quote = None;
                i += 1;
            } else if q == '"' && c == '\\' {
                out[i] = Kind::Str;
                if i + 1 < ch.len() {
                    out[i + 1] = Kind::Str;
                }
                i += 2;
            } else if q == '"' && c == '$' {
                i = expansion(&ch, i, &mut out);
            } else {
                out[i] = Kind::Str;
                i += 1;
            }
            continue;
        }

        match c {
            c if c.is_whitespace() => i += 1,
            '#' => {
                out[i..].fill(Kind::Comment);
                break;
            }
            '\'' | '"' => {
                out[i] = Kind::Str;
                *quote = Some(c);
                cmd_pos = false;
                i += 1;
            }
            '\\' => {
                i += 2;
                cmd_pos = false;
            }
            '$' => {
                i = expansion(&ch, i, &mut out);
                cmd_pos = false;
            }
            '`' => {
                out[i] = Kind::Var;
                i += 1;
            }
            '|' | '&' | ';' | '(' => {
                out[i] = Kind::Op;
                cmd_pos = true;
                i += 1;
            }
            ')' | '<' | '>' | '=' => {
                out[i] = Kind::Op;
                i += 1;
            }
            _ => {
                let start = i;
                while i < ch.len() && is_word_char(ch[i]) {
                    i += 1;
                }
                if i == start {
                    i += 1; // a lone character that is no word char and no case above
                    continue;
                }
                let word: String = ch[start..i].iter().collect();
                let mut j = i;
                while j < ch.len() && ch[j] == ' ' {
                    j += 1;
                }
                let kind = if after_function || (cmd_pos && ch.get(j) == Some(&'(') && ch.get(j + 1) == Some(&')')) {
                    Kind::Func
                } else if ch.get(i) == Some(&'=') && !word.starts_with('-') && !word.contains('/') {
                    Kind::Var // assignment: NAME=value
                } else if KEYWORDS.contains(&word.as_str()) && (cmd_pos || matches!(word.as_str(), "in" | "then" | "do" | "fi" | "done" | "esac" | "else" | "elif" | "}" | "]]")) {
                    Kind::Keyword
                } else if cmd_pos && BUILTINS.contains(&word.as_str()) {
                    Kind::Builtin
                } else if word.chars().all(|c| c.is_ascii_digit()) {
                    Kind::Number
                } else {
                    Kind::Plain
                };
                out[start..i].fill(kind);
                after_function = kind == Kind::Keyword && word == "function";
                // A function name or keyword like `then` leaves us at a command start.
                cmd_pos = (kind == Kind::Keyword && OPENERS.contains(&word.as_str())) || (cmd_pos && kind == Kind::Var);
                if kind == Kind::Func {
                    cmd_pos = false;
                }
            }
        }
    }
    out
}

/// Mark a `$` expansion starting at `i`, return the index after it.
fn expansion(ch: &[char], i: usize, out: &mut [Kind]) -> usize {
    let mut j = i + 1;
    match ch.get(j) {
        Some('{') => {
            while j < ch.len() && ch[j] != '}' {
                j += 1;
            }
            j = (j + 1).min(ch.len());
        }
        Some('(') => j += 1, // the command inside is highlighted by the caller's loop
        Some(c) if c.is_alphabetic() || *c == '_' => {
            while j < ch.len() && (ch[j].is_alphanumeric() || ch[j] == '_') {
                j += 1;
            }
        }
        Some(c) if c.is_ascii_digit() || "?#@*!$-".contains(*c) => j += 1,
        _ => {}
    }
    out[i..j].fill(Kind::Var);
    j
}

#[cfg(test)]
mod tests;
