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

/// `sx-sudo [user]` starts a shell for `user` (root by default) with your rc, like `sudo -i` but
/// with your prompt and aliases. `sudo` cleans the environment and the new shell reads only the
/// other user's own files, so the payload is sent again. It goes through the standard input of
/// `sudo`, not its arguments, because sudo writes its arguments in the server's log. The shell
/// takes the terminal back from its standard output, not from `/dev/tty`: `tty` then names the real
/// terminal, which `tmux` needs (its server cannot open the name `/dev/tty`).
const SUDO: &str = "sx-sudo() { case ${1-} in -h|--help) printf '%s\\n' 'usage: sx-sudo [user]' 'Starts a shell for [user] (root by default) with your prompt and aliases,' 'like sudo -i. It works only in a shell that sx started.'; return;; \
-*) echo \"sx-sudo: unknown option $1, see sx-sudo --help\" >&2; return 1;; esac; local u=${1:-root}; [ -n \"${__sx_payload-}\" ] || { echo 'sx-sudo: this shell was not started by sx' >&2; return 1; }; \
printf '%s\\n%s\\n' \"$__sx_payload\" \"${__sx_theme_cur-}\" | sudo -H -u \"$u\" bash -c 'read -r P; read -r T; export SX_RC=$P SX_THEME=$T; cd ~ 2>/dev/null; \
exec bash --rcfile <(printf %s \"$P\" | base64 -d | gunzip) -i <&1'; }\n";

/// The shell that `sx-sudo` starts gets the theme you chose with `sx-theme` in `SX_THEME`, and the
/// rc applies it last, since the payload only holds the theme of the profile. Without `sx-theme`
/// there is nothing to apply, and a name the rc does not know is ignored.
const RESTORE_THEME: &str = "__sx_t0=${SX_THEME-}; unset SX_THEME\n\
[ -n \"$__sx_t0\" ] && command -v sx-theme >/dev/null && sx-theme \"$__sx_t0\" 2>/dev/null; unset __sx_t0\n";

/// Sets the window and tab title of your terminal (OSC 0) to `[sx] user@host: dir` at every prompt,
/// so it follows `cd` and a different connection. The code in `__sx_tc` prints the sequence. It is added to the
/// end of `PS1`, inside `\[ \]` so it has no width, because bash prints `PS1` after `PROMPT_COMMAND`
/// and a server's own `PS1` usually sets a title of its own, which would win from anywhere earlier.
/// The code sits inside `PS1` as a command substitution, not in a function, so it also runs in a
/// shell that did not inherit one, such as the one `sudo -i` starts.
///
/// The same hooks mark the prompt for the terminal (OSC 133, "semantic prompt"): `A` starts the
/// prompt, at the front of `PS1`, `B` ends it, after the title, `C` starts the command's output
/// (`PS0`, bash 4.4 or newer), and `D;status` ends the command, sent before the next prompt except
/// the first. A terminal that knows them can jump between prompts, select one command's output and
/// mark failed commands. `__sx_ps1` adds the marks to `PS1`, once, also after `sx-theme` replaced it.
/// `__sx_title` (in `PROMPT_COMMAND`) sends `D`, calls `__sx_ps1`, and returns the status it got so
/// `$?` is intact.
const TITLE: &str = r##"__sx_tc='d=$PWD;case $d in "$HOME"*) d="~${d#"$HOME"}";; esac;printf "\033]0;[sx] %s@%s: %s\a" "${USER:-$(id -un)}" "${HOSTNAME%%.*}" "$d"'
__sx_ps1() { case $PS1 in *'133;A'*) ;; *) PS1='\[\e]133;A\a\]'$PS1;; esac; case $PS1 in *'133;B'*) ;; *) PS1="$PS1\[\$($__sx_tc)\]\[\e]133;B\a\]";; esac; }
__sx_title() { local s=$?; [ -n "$__sx_seen" ] && printf '\033]133;D;%s\a' "$s"; __sx_seen=1; __sx_ps1; return $s; }
case ${PS0-} in *'133;C'*) ;; *) PS0=${PS0-}'\e]133;C\a';; esac
case $PROMPT_COMMAND in *__sx_title*) ;; *) PROMPT_COMMAND="__sx_title${PROMPT_COMMAND:+;$PROMPT_COMMAND}";; esac
"##;

/// Bash that defines `sx-theme`, which switches the prompt in a running session. `sx-theme` lists
/// previews them, `sx-theme -l` lists the names and `sx-theme <name>` applies one. `sxt` is an
/// alias for it. The built-in themes and your `.prompt` snippets are both in it, since the rc is sent once and the server has nothing to fetch them from later.
fn theme_switcher(dir: &Path) -> String {
    let mut names: Vec<String> = prompt::THEMES.iter().map(|(n, _)| n.to_string()).collect();
    names.extend(list_snippets(dir).into_iter().filter(|n| is_prompt_snippet(n)));
    names.retain(|n| !n.contains(['\'', ' ']));
    let mut out = String::from("__sx_theme_text() { case $1 in\n");
    let mut listed = Vec::new();
    for (i, name) in names.iter().enumerate() {
        let Some(text) = theme_text(dir, name) else { continue };
        let tag = heredoc_tag(&format!("THEME{i}"), &text);
        let nl = if text.ends_with('\n') { "" } else { "\n" };
        out.push_str(&format!("'{name}') cat <<'{tag}'\n{text}{nl}{tag}\n;;\n"));
        listed.push(name.as_str());
    }
    out.push_str("*) return 1;; esac; }\n");
    out.push_str(&SWITCHER.replace("@LIST@", &listed.join(" ")).replace("@GIT@", prompt::GIT).replace("@RC@", prompt::RC));
    out
}

/// The commands around the theme texts. `sx-theme` alone previews every theme: each one is run in a
/// subshell and its `PS1` is expanded with `${PS1@P}` (bash 4.4 or newer), so the preview shows the
/// real host, directory and git branch. Older bash gets the names only. The `\001` and `\002` that
/// `\[` and `\]` leave in the expanded text are removed, and so is the CR: an interactive bash
/// expands the `\n` of a prompt to CR LF, and a CR inside a box would send the cursor back to the
/// first column.
const SWITCHER: &str = r#"__sx_strip() { local s=$1 o=; while [[ $s == *$'\e['* ]]; do o+=${s%%$'\e['*}; s=${s#*$'\e['}; s=${s#*m}; done; REPLY=$o$s; }
__sx_theme_show() {
  local n p l r i j ci cols g d row pad nw=0 mw=0 H=1 gap=2 iw G='@GIT@' R='@RC@' W='~/app'
  local -a names=() lines=()
  local dim=$'\e[90m' bold=$'\e[1m' off=$'\e[0m'
  if ! (( BASH_VERSINFO[0] * 100 + BASH_VERSINFO[1] >= 404 )) || [[ ${LC_ALL:-${LC_CTYPE:-$LANG}} != *[Uu][Tt][Ff]*8* ]]; then
    echo '@LIST@'; return
  fi
  for n in @LIST@; do
    p=$( cd ~ 2>/dev/null; eval "$(__sx_theme_text "$n")"; PS1=${PS1//"$G"/ (main)}; PS1=${PS1//"$R"/ ✗1}; PS1=${PS1//\\u/you}; PS1=${PS1//\\[hH]/server}; PS1=${PS1//\\w/$W}; PS1=${PS1//\\W/app}; printf %s "${PS1@P}" )
    p=${p//[$'\001\002\r']/}
    names+=("$n"); ci=$(( ${#names[@]} - 1 )); (( ${#n} > nw )) && nw=${#n}
    r=0
    while IFS= read -r l; do
      if (( r < 4 )); then
        lines[ci * 4 + r]=$l; __sx_strip "$l"; (( ${#REPLY} > mw )) && mw=${#REPLY}
        (( r + 1 > H )) && H=$(( r + 1 ))
      fi
      r=$(( r + 1 ))
    done <<<"$p"
  done
  iw=$(( mw + 2 )); (( nw + 3 > iw )) && iw=$(( nw + 3 ))
  cols=${COLUMNS:-$(tput cols 2>/dev/null)}; cols=$(( (${cols:-80} + gap) / (iw + 2 + gap) ))
  (( cols < 1 )) && cols=1; (( cols > ${#names[@]} )) && cols=${#names[@]}
  printf -v g '%*s' "$gap" ''
  for (( i = 0; i < ${#names[@]}; i += cols )); do
    row=
    for (( j = i; j < i + cols && j < ${#names[@]}; j++ )); do
      n=${names[j]}; printf -v d '%*s' $(( iw - ${#n} - 3 )) ''; d=${d// /─}
      [[ -n $row ]] && row+=$g
      row+="${dim}┌─ ${off}${bold}${n}${off}${dim} ${d}┐${off}"
    done
    printf '%s\n' "$row"
    for (( r = 0; r < H; r++ )); do
      row=
      for (( j = i; j < i + cols && j < ${#names[@]}; j++ )); do
        l=${lines[j * 4 + r]}; __sx_strip "$l"; printf -v pad '%*s' $(( iw - 2 - ${#REPLY} )) ''
        [[ -n $row ]] && row+=$g
        row+="${dim}│${off} ${l}${off}${pad} ${dim}│${off}"
      done
      printf '%s\n' "$row"
    done
    row=
    for (( j = i; j < i + cols && j < ${#names[@]}; j++ )); do
      printf -v d '%*s' "$iw" ''; d=${d// /─}
      [[ -n $row ]] && row+=$g
      row+="${dim}└${d}┘${off}"
    done
    printf '%s\n' "$row"
  done
  printf '%sApply one with: sx-theme <name>%s\n' "$dim" "$off"
}
sx-theme() {
  case $1 in
    '') __sx_theme_show; return;;
    -l|--list) echo '@LIST@'; return;;
    -h|--help)
      printf '%s\n' 'usage: sx-theme [name]' '       sx-theme -l | --list' '       sx-theme -h | --help' \
        'Changes the prompt of this shell.' \
        '  (no name)    preview every theme' \
        '  <name>       use the theme <name>' \
        '  -l, --list   list the theme names' \
        'sxt is the same command.'
      return;;
    -*) echo "sx-theme: unknown option $1, see sx-theme --help" >&2; return 1;;
  esac
  local t
  t=$(__sx_theme_text "$1") || { echo "sx-theme: no theme named $1" >&2; return 1; }
  eval "$t"; __sx_ps1; __sx_theme_cur=$1
}
alias sxt=sx-theme
complete -W '@LIST@' sx-theme sxt
"#;

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
    push_block(&mut out, "sx-sudo", SUDO);
    if let Some(name) = p.prompt.as_deref()
        && let Some(text) = theme_text(dir, name)
    {
        out.push_str(&format!("# --- prompt: {name} ---\n{text}"));
        if !text.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("# --- sx-theme ---\n");
        out.push_str(&theme_switcher(dir));
    }
    push_block(&mut out, "title", TITLE);
    if out.contains("# --- sx-theme ---") {
        push_block(&mut out, "theme from sx-sudo", RESTORE_THEME);
    }
    out
}
