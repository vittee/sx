use super::*;
use crate::testutil::bash_at_least;

fn text(lines: &[Line]) -> Vec<String> {
    lines.iter().map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect()).collect()
}

#[test]
fn extracts_quoted_values() {
    assert_eq!(extract_ps1("export PS1='\\u@\\h\\$ '\n").as_deref(), Some("\\u@\\h\\$ "));
    assert_eq!(extract_ps1("x=1\nPS1=\"a b\" # c\n").as_deref(), Some("a b"));
    assert_eq!(extract_ps1("alias x=y\n"), None);
}

#[test]
fn renders_text_and_newlines() {
    let lines = render_ps1("\\u@\\h:\\w\\$ ");
    assert_eq!(text(&lines), ["you@server:~/app$ "]);
    let two = render_ps1("\\u\\n> ");
    assert_eq!(text(&two), ["you", "> "]);
}

#[test]
fn applies_colors() {
    let lines = render_ps1("\\[\\e[1;32m\\]hi\\[\\e[0m\\]!");
    let spans = &lines[0].spans;
    assert_eq!(spans[0].content, "hi");
    assert_eq!(spans[0].style.fg, Some(Color::Green));
    assert!(spans[0].style.add_modifier.contains(Modifier::BOLD));
    assert_eq!(spans[1].content, "!");
    assert_eq!(spans[1].style.fg, None);
}

#[test]
fn every_theme_has_a_renderable_ps1() {
    for (name, snippet) in THEMES {
        let ps1 = extract_ps1(snippet).unwrap_or_else(|| panic!("{name}: no PS1"));
        assert!(!render_ps1(&ps1).is_empty(), "{name}");
    }
}

#[test]
fn every_theme_shows_the_server_name() {
    for (name, snippet) in THEMES {
        let ps1 = extract_ps1(snippet).unwrap_or_else(|| panic!("{name}: no PS1"));
        assert!(ps1.contains("\\h") || ps1.contains("\\H"), "{name} has no \\h: {ps1}");
        let shown: String = render_ps1(&ps1).iter().flat_map(|l| l.spans.iter()).map(|s| s.content.as_ref()).collect();
        assert!(shown.contains("server"), "{name} preview shows no server name: {shown}");
    }
}

const TWO_LINE: &[&str] = &[
    "two-line", "bira", "gnzh", "fino", "ys", "box", "remote", "robbyrussell", "pygmalion",
    "candy", "af-magic", "lambda", "cloud",
];

fn ps1_of(name: &str) -> String {
    extract_ps1(theme(name).unwrap_or_else(|| panic!("no theme {name}"))).unwrap()
}

#[test]
fn two_line_themes_have_the_server_name_on_the_first_line_only() {
    for name in TWO_LINE {
        let lines = text(&render_ps1(&ps1_of(name)));
        assert_eq!(lines.len(), 2, "{name}: {lines:?}");
        assert!(lines[0].contains("server"), "{name}: {lines:?}");
        assert!(!lines[1].contains("server") && !lines[1].contains('~'), "{name}: {lines:?}");
    }
}

#[test]
fn two_line_themes_mark_the_host_distinctly() {
    // The host must be styled (color or badge), never plain default text.
    for name in TWO_LINE {
        let lines = render_ps1(&ps1_of(name));
        let host = lines[0].spans.iter().find(|s| s.content.contains("server")).unwrap();
        assert!(
            host.style.fg.is_some() || host.style.bg.is_some(),
            "{name}: host span is unstyled: {host:?}"
        );
    }
}

#[test]
fn themes_wrap_color_codes_in_brackets_and_use_only_real_newlines_escapes() {
    for (name, snippet) in THEMES {
        let ps1 = extract_ps1(snippet).unwrap();
        assert_eq!(ps1.matches("\\[").count(), ps1.matches("\\]").count(), "{name}: unbalanced \\[ \\]");
        // every color escape must sit inside \[ \] so bash measures the prompt width right
        let mut inside = false;
        let mut chars = ps1.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('[') => inside = true,
                    Some(']') => inside = false,
                    Some('e') => assert!(inside, "{name}: \\e outside \\[ \\] in {ps1}"),
                    _ => {}
                }
            }
        }
        assert!(!ps1.contains('\n'), "{name}: literal newline in PS1");
    }
}

#[test]
fn the_inline_status_and_branch_code_keep_the_status_they_were_given() {
    // Regression: the branch part once leaked its own exit status into the status part.
    let snippet = theme("bira").unwrap();
    assert!(snippet.contains("exit $s)"), "the branch part must exit with the status it was given");
    assert!(snippet.contains("exit $c)"), "the status part must exit with the status it was given");
}

/// Dev aid: `SX_DUMP_THEMES=<dir> cargo test dump_themes -- --ignored` writes each theme as
/// `<dir>/<name>.sh`, for checking the real bash expansion (`echo "${PS1@P}"`).
#[test]
#[ignore]
fn dump_themes() {
    let dir = std::env::var("SX_DUMP_THEMES").expect("set SX_DUMP_THEMES");
    std::fs::create_dir_all(&dir).unwrap();
    for (name, text) in THEMES {
        std::fs::write(std::path::Path::new(&dir).join(format!("{name}.sh")), text).unwrap();
    }
}

#[test]
fn theme_names_are_unique() {
    let mut names: Vec<_> = THEMES.iter().map(|(n, _)| *n).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), THEMES.len());
}

#[test]
fn git_theme_shows_branch_sample() {
    let ps1 = extract_ps1(THEMES.iter().find(|t| t.0 == "git-branch").unwrap().1).unwrap();
    assert!(text(&render_ps1(&ps1))[0].contains("(main)"));
}

#[test]
fn a_prompt_file_may_be_just_the_ps1_string() {
    assert!(!is_script("\\u@\\h:\\w\\$ \n"));
    assert_eq!(file_ps1("\\u@\\h\\$ \n").as_deref(), Some("\\u@\\h\\$ "));
    assert_eq!(file_ps1("  \n"), None);
    assert_eq!(script_for("\\u@\\h\\$ \n"), "export PS1='\\u@\\h\\$ '\n");
    assert_eq!(script_for(""), "");
}

#[test]
fn a_single_quote_in_the_ps1_string_is_escaped() {
    assert_eq!(script_for("it's> "), "export PS1='it'\\''s> '\n");
}

#[test]
fn a_ps1_string_that_calls_the_old_helpers_gets_the_inline_code() {
    let s = script_for("\\w$(__sx_git)$(__sx_rc) \\$ ");
    assert!(s.contains(GIT) && s.contains(RC) && s.ends_with("\\$ '\n"), "{s}");
    assert!(!s.contains("__sx_"), "{s}");
    assert_eq!(script_for("\\w \\$ "), "export PS1='\\w \\$ '\n");
}

#[test]
fn no_theme_needs_a_function_so_a_new_shell_can_use_the_inherited_prompt() {
    for (name, text) in THEMES {
        assert!(!text.contains("__sx_"), "{name} calls or defines a helper function");
        assert!(!text.contains("() {"), "{name} defines a function");
    }
    assert!(THEMES.iter().any(|(_, t)| t.contains(GIT) && t.contains(RC)));
}

/// Runs a script in a real bash and returns its stdout (without CR) and stderr. `None` when there
/// is no bash to run.
fn run_bash(script: &str) -> Option<(String, String)> {
    use std::io::Write;
    if !bash_at_least(3, 0) {
        return None;
    }
    let mut child = std::process::Command::new("bash")
        .arg("-s")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(script.as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    Some((String::from_utf8_lossy(&out.stdout).replace('\r', ""), String::from_utf8_lossy(&out.stderr).to_string()))
}

#[test]
fn a_shell_without_any_function_shows_the_branch_and_the_status_from_the_inherited_prompt() {
    if !bash_at_least(4, 4) {
        return;
    }
    // `sudo -i` and `su` keep the exported PS1 and drop the functions, which `env -i` does too.
    let script = format!(
        "{}cd \"$(mktemp -d)\" && git init -q -b trunk . 2>/dev/null\n\
         false\n\
         env -i PATH=\"$PATH\" PS1=\"$PS1\" \"$BASH\" --norc -i -c 'echo \"[${{PS1@P}}]\"'\n",
        theme("robbyrussell").unwrap()
    );
    let Some((out, err)) = run_bash(&script) else { return };
    assert!(!err.contains("command not found"), "{err}");
    assert!(out.contains("(trunk)"), "{out}");
}

#[test]
fn the_exit_status_survives_the_branch_part_of_the_prompt() {
    if !bash_at_least(4, 4) {
        return;
    }
    let script = format!("{}false\necho \"[${{PS1@P}}]\"\n", theme("robbyrussell").unwrap());
    let Some((out, err)) = run_bash(&script) else { return };
    assert!(!err.contains("command not found"), "{err}");
    assert!(out.contains("✗1"), "{out}");
}

#[test]
fn a_prompt_file_that_is_bash_is_used_as_it_is() {
    let bash = "alias x=y\nexport PS1='a> '\n";
    assert!(is_script(bash));
    assert_eq!(script_for(bash), bash);
    assert_eq!(file_ps1(bash).as_deref(), Some("a> "));
}

#[test]
fn the_preview_shows_a_directory_a_branch_and_a_failed_command() {
    let shown = text(&render_ps1(r"\W $(__sx_git)$(__sx_rc)"));
    assert_eq!(shown, ["app  (main) ✗1"]);
}

#[test]
fn every_built_in_theme_shows_the_user_and_the_host() {
    for (name, text) in THEMES {
        let ps1 = extract_ps1(text).unwrap();
        assert!(ps1.contains(r"\u"), "{name} has no user name: {ps1}");
        assert!(ps1.contains(r"\h"), "{name} has no host name: {ps1}");
    }
}
