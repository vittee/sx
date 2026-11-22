//! Snippet files (`snippets/*`), what each kind of file means, and building the rc text that is
//! sent to the server from a profile's snippets, a host's extras and the prompt theme.

use super::*;

pub(super) fn snippets_dir(dir: &Path) -> PathBuf {
    dir.join("snippets")
}

/// What a snippet file is for, decided by its name: `.vim`, `.tmux`, `.inputrc` and `.prompt` files
/// are not shell code. `sx` wraps them into shell that uses them without writing a file on the
/// server (process substitution or a here-document). Everything else is plain shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnippetKind {
    Shell,
    Vim,
    Tmux,
    Inputrc,
    Prompt,
}

const KIND_SUFFIXES: &[(&str, SnippetKind)] = &[
    (".vim", SnippetKind::Vim),
    (".tmux", SnippetKind::Tmux),
    (".inputrc", SnippetKind::Inputrc),
    (".prompt", SnippetKind::Prompt),
];

pub fn snippet_kind(name: &str) -> SnippetKind {
    KIND_SUFFIXES.iter().find(|(s, _)| name.ends_with(s)).map_or(SnippetKind::Shell, |(_, k)| *k)
}

/// File name of a snippet: shell snippets get `.sh`, the other kinds keep the name as is.
pub fn snippet_file(name: &str) -> String {
    match snippet_kind(name) {
        SnippetKind::Shell => format!("{name}.sh"),
        _ => name.to_string(),
    }
}

/// True for a user-written prompt theme (`something.prompt`).
pub fn is_prompt_snippet(name: &str) -> bool {
    snippet_kind(name) == SnippetKind::Prompt
}

/// Names a profile or host can send (everything except prompt themes).
pub fn sendable(names: &[String]) -> Vec<String> {
    names.iter().filter(|n| !is_prompt_snippet(n)).cloned().collect()
}

/// Snippet names, sorted. Shell snippets are listed by file stem (`snippets/10-x.sh` is `10-x`),
/// the other kinds by full file name (`vimrc.vim`). Prefix names with numbers to order them.
pub fn list_snippets(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(snippets_dir(dir))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            let file = path.file_name()?.to_str()?;
            match path.extension()?.to_str()? {
                "sh" => path.file_stem()?.to_str().map(String::from),
                _ if snippet_kind(file) != SnippetKind::Shell => Some(file.to_string()),
                _ => None,
            }
        })
        .collect();
    names.sort();
    names
}

pub fn read_snippet(dir: &Path, name: &str) -> io::Result<String> {
    fs::read_to_string(snippets_dir(dir).join(snippet_file(name)))
}

pub fn write_snippet(dir: &Path, name: &str, text: &str) -> io::Result<()> {
    if !is_valid_name(name) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "use letters, digits, - _ . only"));
    }
    fs::create_dir_all(snippets_dir(dir))?;
    write_atomic(&snippets_dir(dir).join(snippet_file(name)), text.as_bytes())
}

pub fn delete_snippet(dir: &Path, name: &str) -> io::Result<()> {
    fs::remove_file(snippets_dir(dir).join(snippet_file(name)))
}

/// A here-document delimiter that does not occur on a line of its own in `text`.
fn heredoc_tag(base: &str, text: &str) -> String {
    let mut tag = format!("SX_EOF_{base}");
    while text.lines().any(|l| l == tag) {
        tag.push('_');
    }
    tag
}

/// Shell code that makes the server use `text` as a config file of the given kind.
pub(super) fn wrap_snippet(kind: SnippetKind, text: &str) -> String {
    let (base, func, cmds): (&str, &str, &[&str]) = match kind {
        SnippetKind::Vim => ("VIM", "__sx_vimrc", &["vim", "nvim"]),
        SnippetKind::Tmux => ("TMUX", "__sx_tmuxconf", &["tmux"]),
        SnippetKind::Inputrc => ("INPUTRC", "", &[]),
        SnippetKind::Shell | SnippetKind::Prompt => return text.to_string(),
    };
    let tag = heredoc_tag(base, text);
    let nl = if text.ends_with('\n') { "" } else { "\n" };
    if kind == SnippetKind::Inputrc {
        // Applied once, when the shell starts.
        return format!("bind -f /dev/stdin <<'{tag}'\n{text}{nl}{tag}\n");
    }
    let flag = if kind == SnippetKind::Tmux { "-f" } else { "-u" };
    let mut out = format!("{func}() {{ cat <<'{tag}'\n{text}{nl}{tag}\n}}\n");
    for cmd in cmds {
        out.push_str(&format!("{cmd}() {{ command {cmd} {flag} <({func}) \"$@\"; }}\n"));
    }
    out
}

/// The bash of a prompt theme: a built-in one, or a `.prompt` snippet of your own.
pub fn theme_text(dir: &Path, name: &str) -> Option<String> {
    if is_prompt_snippet(name) {
        read_snippet(dir, name).ok().map(|text| prompt::script_for(&text)).filter(|t| !t.is_empty())
    } else {
        prompt::theme(name).map(String::from)
    }
}

/// `clip` copies to the clipboard of the terminal you are sitting at, with the OSC 52 escape
/// sequence: `clip text`, or `cmd | clip` when something is piped in. It writes to `/dev/tty`, so
/// it still works at the end of a pipeline. The base64 is joined into one line because GNU
/// `base64` wraps at 76 columns.
const CLIP: &str = "clip() { case ${1-} in -h|--help) printf '%s\\n' 'usage: clip [text]' 'Copies text, or what is piped in, to the clipboard of the machine you sit at.' 'Example: cat file | clip'; return;; -*) echo \"clip: unknown option $1, see clip --help\" >&2; return 1;; esac; \
local d; if [ $# -gt 0 ]; then d=$(printf %s \"$*\" | base64); else d=$(base64); fi; \
printf '\\033]52;c;%s\\a' \"$(printf %s \"$d\" | tr -d '\\n')\" >/dev/tty; }\n";

/// The PS1 string of a prompt theme, for the preview.
pub fn theme_ps1(dir: &Path, name: &str) -> Option<String> {
    if is_prompt_snippet(name) {
        read_snippet(dir, name).ok().and_then(|text| prompt::file_ps1(&text))
    } else {
        prompt::theme(name).and_then(prompt::extract_ps1)
    }
}

/// Builds the rc text sent to the server: the profile's snippets (sorted by name), then its prompt
/// theme. The theme comes last so an explicit choice wins over any `PS1` set in a snippet.
pub fn build_rc(dir: &Path, cfg: &Config, profile: &str) -> String {
    build_rc_with(dir, cfg, profile, &[])
}

/// Like [`build_rc`], plus the extra snippets set for one host.
pub fn build_rc_for_host(dir: &Path, cfg: &Config, profile: &str, host: &str) -> String {
    build_rc_with(dir, cfg, profile, cfg.host_snippets(host))
}

fn push_block(out: &mut String, title: &str, body: &str) {
    out.push_str(&format!("# --- {title} ---\n{body}"));
    if !body.ends_with('\n') {
        out.push('\n');
    }
}

fn build_rc_with(dir: &Path, cfg: &Config, profile: &str, extra: &[String]) -> String {
    let Some(p) = cfg.profiles.get(profile) else { return String::new() };
    let mut out = String::new();
    let wanted = |n: &String| p.snippets.contains(n) || extra.contains(n);
    // Every vim snippet is joined into one config, and every tmux snippet into one, because the
    // server runs the program with a single file. They go after the shell snippets.
    let mut merged: Vec<(SnippetKind, Vec<String>, String)> = Vec::new();
    for name in list_snippets(dir).into_iter().filter(|n| wanted(n) && !is_prompt_snippet(n)) {
        let Ok(text) = read_snippet(dir, &name) else { continue };
        let kind = snippet_kind(&name);
        if matches!(kind, SnippetKind::Vim | SnippetKind::Tmux) {
            match merged.iter_mut().find(|m| m.0 == kind) {
                Some(m) => {
                    m.1.push(name);
                    if !m.2.ends_with('\n') {
                        m.2.push('\n');
                    }
                    m.2.push_str(&text);
                }
                None => merged.push((kind, vec![name], text)),
            }
            continue;
        }
        push_block(&mut out, &name, &wrap_snippet(kind, &text));
    }
    for (kind, names, text) in &merged {
        push_block(&mut out, &names.join(", "), &wrap_snippet(*kind, text));
    }
    push_block(&mut out, "clip", CLIP);
    if let Some(name) = p.prompt.as_deref()
        && let Some(text) = theme_text(dir, name)
    {
        out.push_str(&format!("# --- prompt: {name} ---\n{text}"));
        if !text.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}
