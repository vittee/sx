use super::*;

fn kinds(src: &str) -> Vec<(String, Kind)> {
    let lines: Vec<String> = src.lines().map(String::from).collect();
    let hl = highlight(&lines);
    let mut res = Vec::new();
    for (l, k) in lines.iter().zip(hl) {
        let chars: Vec<char> = l.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let mut j = i;
            while j < chars.len() && k[j] == k[i] {
                j += 1;
            }
            let text: String = chars[i..j].iter().collect();
            if !text.trim().is_empty() {
                res.push((text.trim().to_string(), k[i]));
            }
            i = j;
        }
    }
    res
}

fn has(src: &str, text: &str, kind: Kind) -> bool {
    kinds(src).iter().any(|(t, k)| t == text && *k == kind)
}

#[test]
fn one_kind_per_char() {
    let lines = vec!["echo \"héllo $USER\" # ✓".to_string(), String::new()];
    let hl = highlight(&lines);
    assert_eq!(hl[0].len(), lines[0].chars().count());
    assert!(hl[1].is_empty());
}

#[test]
fn comments() {
    assert!(has("# hello world", "# hello world", Kind::Comment));
    assert!(has("ls # trailing", "# trailing", Kind::Comment));
    // # inside a word or a string or $# is not a comment
    assert!(has("echo a#b", "a#b", Kind::Plain));
    assert!(!kinds("echo \"# no\"").iter().any(|(_, k)| *k == Kind::Comment));
    assert!(has("echo $#", "$#", Kind::Var));
}

#[test]
fn strings_and_expansions() {
    assert!(has("echo 'a $b'", "'a $b'", Kind::Str));
    let k = kinds("echo \"hi $USER and ${HOME}\"");
    assert!(k.contains(&("$USER".to_string(), Kind::Var)));
    assert!(k.contains(&("${HOME}".to_string(), Kind::Var)));
    assert!(k.iter().any(|(t, kind)| *kind == Kind::Str && t.starts_with("\"hi")));
}

#[test]
fn quote_can_span_lines() {
    let k = kinds("echo 'one\ntwo # not a comment\nthree' ls");
    assert!(!k.iter().any(|(_, kind)| *kind == Kind::Comment), "{k:?}");
    assert!(k.contains(&("two # not a comment".to_string(), Kind::Str)));
    // after the quote closes we are back in code
    assert!(k.iter().any(|(t, kind)| t.contains("ls") && *kind == Kind::Plain));
}

#[test]
fn keywords_builtins_functions() {
    let src = "if [ -f x ]; then\n  export A=1\nfi\nmyfn() {\n  cd /tmp\n}\nfunction other { :; }";
    for kw in ["if", "then", "fi"] {
        assert!(has(src, kw, Kind::Keyword), "{kw}");
    }
    assert!(has(src, "export", Kind::Builtin));
    assert!(has(src, "cd", Kind::Builtin));
    assert!(has(src, "myfn", Kind::Func));
    assert!(has(src, "other", Kind::Func));
    assert!(has(src, "A", Kind::Var), "assignment name");
}

#[test]
fn command_arguments_are_not_builtins() {
    // `echo` is a builtin, but `cd` as an argument is plain
    let k = kinds("echo cd");
    assert!(k.contains(&("echo".into(), Kind::Builtin)));
    assert!(k.contains(&("cd".into(), Kind::Plain)));
}

#[test]
fn alias_lines_and_operators() {
    let k = kinds("alias ll='ls -alF' && ls | wc -l > out");
    assert!(k.contains(&("alias".into(), Kind::Builtin)));
    assert!(k.contains(&("ll".into(), Kind::Var)));
    assert!(k.contains(&("'ls -alF'".into(), Kind::Str)));
    assert!(k.iter().any(|(t, kind)| t.contains("&&") && *kind == Kind::Op));
    assert!(k.iter().any(|(t, kind)| t == "|" && *kind == Kind::Op));
}

#[test]
fn prompt_theme_snippets_highlight_without_panicking() {
    for (name, text) in crate::prompt::THEMES {
        let lines: Vec<String> = text.lines().map(String::from).collect();
        let hl = highlight(&lines);
        assert_eq!(hl.len(), lines.len(), "{name}");
        for (l, k) in lines.iter().zip(&hl) {
            assert_eq!(l.chars().count(), k.len(), "{name}");
        }
    }
}

#[test]
fn ps1_parts_get_their_own_kinds() {
    let src = r"\[\e[1;32m\]\u@\h\[\e[0m\]:\w$(__sx_git) \$ ";
    let k = &highlight_ps1(&[src.to_string()])[0];
    let chars: Vec<char> = src.chars().collect();
    assert_eq!(k.len(), chars.len());
    let at = |needle: &str| {
        let n: Vec<char> = needle.chars().collect();
        let p = chars.windows(n.len()).position(|w| w == &n[..]).unwrap();
        k[p..p + n.len()].to_vec()
    };
    assert!(at(r"\[").iter().all(|x| *x == Kind::Comment));
    assert!(at(r"\e[1;32m").iter().all(|x| *x == Kind::Number));
    assert!(at(r"\u").iter().all(|x| *x == Kind::Keyword));
    assert!(at(r"\h").iter().all(|x| *x == Kind::Keyword));
    assert!(at("$(__sx_git)").iter().all(|x| *x == Kind::Var));
    assert!(at(r"\$").iter().all(|x| *x == Kind::Keyword));
    assert_eq!(at("@"), [Kind::Plain]);
}

#[test]
fn ps1_highlighting_covers_every_character_even_when_cut_short() {
    for src in [r"\", r"\e[", r"\e[1;3", "$(", "$", r"\033[0m", r"a\"] {
        let k = &highlight_ps1(&[src.to_string()])[0];
        assert_eq!(k.len(), src.chars().count(), "{src}");
    }
}

fn vim(src: &str) -> Vec<Kind> {
    highlight_vim(&[src.to_string()]).remove(0)
}

fn vim_kind_of(src: &str, needle: &str) -> Vec<Kind> {
    let chars: Vec<char> = src.chars().collect();
    let n: Vec<char> = needle.chars().collect();
    let p = chars.windows(n.len()).position(|w| w == &n[..]).unwrap();
    vim(src)[p..p + n.len()].to_vec()
}

#[test]
fn vim_comment_lines_and_trailing_comments() {
    assert!(vim("\" a comment").iter().all(|k| *k == Kind::Comment));
    assert!(vim("  \" indented")[2..].iter().all(|k| *k == Kind::Comment));
    assert!(vim_kind_of("set number \" trailing", "\" trailing").iter().all(|k| *k == Kind::Comment));
}

#[test]
fn vim_set_lines() {
    let src = "set shiftwidth=2 nocompatible";
    assert!(vim_kind_of(src, "set").iter().all(|k| *k == Kind::Keyword));
    assert!(vim_kind_of(src, "shiftwidth").iter().all(|k| *k == Kind::Var));
    assert_eq!(vim_kind_of(src, "="), [Kind::Op]);
    assert_eq!(vim_kind_of(src, "2"), [Kind::Number]);
    assert!(vim_kind_of(src, "nocompatible").iter().all(|k| *k == Kind::Var));
    let src = "set listchars+=tab:>-";
    assert_eq!(vim_kind_of(src, "+="), [Kind::Op, Kind::Op]);
    assert!(vim_kind_of(src, "tab:").iter().all(|k| *k == Kind::Plain), "values are plain");
}

#[test]
fn vim_maps_keys_and_strings() {
    let src = "silent! nnoremap <silent> <leader>w :w<CR>";
    assert!(vim_kind_of(src, "silent!").iter().all(|k| *k == Kind::Keyword));
    assert!(vim_kind_of(src, "nnoremap").iter().all(|k| *k == Kind::Keyword));
    assert!(vim_kind_of(src, "<silent>").iter().all(|k| *k == Kind::Op));
    assert!(vim_kind_of(src, "<CR>").iter().all(|k| *k == Kind::Op));
    let src = "let mapleader = ' '";
    assert!(vim_kind_of(src, "mapleader").iter().all(|k| *k == Kind::Var));
    assert!(vim_kind_of(src, "' '").iter().all(|k| *k == Kind::Str));
    let src = "echo \"hi\" \" note";
    assert!(vim_kind_of(src, "\"hi\"").iter().all(|k| *k == Kind::Str));
    assert!(vim_kind_of(src, "\" note").iter().all(|k| *k == Kind::Comment));
}

#[test]
fn vim_highlighting_covers_every_character() {
    for src in ["", "\"", "'", "<", "<a", "set", "set a=", "silent!", "if has('clipboard')", "x <"] {
        assert_eq!(vim(src).len(), src.chars().count(), "{src}");
    }
}

fn inputrc(src: &str) -> Vec<Kind> {
    highlight_inputrc(&[src.to_string()]).remove(0)
}

fn inputrc_kind_of(src: &str, needle: &str) -> Vec<Kind> {
    let chars: Vec<char> = src.chars().collect();
    let n: Vec<char> = needle.chars().collect();
    let p = chars.windows(n.len()).position(|w| w == &n[..]).unwrap();
    inputrc(src)[p..p + n.len()].to_vec()
}

#[test]
fn inputrc_comments_and_directives() {
    assert!(inputrc("# sx keys").iter().all(|k| *k == Kind::Comment));
    assert!(inputrc("  # indented")[2..].iter().all(|k| *k == Kind::Comment));
    let src = "$if mode=emacs";
    assert!(inputrc_kind_of(src, "$if").iter().all(|k| *k == Kind::Keyword));
    assert!(inputrc_kind_of(src, "mode").iter().all(|k| *k == Kind::Var));
    assert_eq!(inputrc_kind_of(src, "="), [Kind::Op]);
    let src = "$if version >= 7.0";
    assert!(inputrc_kind_of(src, "version").iter().all(|k| *k == Kind::Var));
    assert!(inputrc_kind_of(src, ">=").iter().all(|k| *k == Kind::Op));
    assert!(inputrc_kind_of(src, "7.0").iter().all(|k| *k == Kind::Number));
    assert!(inputrc("$endif").iter().all(|k| *k == Kind::Keyword));
    assert!(inputrc("$else").iter().all(|k| *k == Kind::Keyword));
}

#[test]
fn inputrc_set_lines() {
    let src = "set colored-stats on";
    assert!(inputrc_kind_of(src, "set").iter().all(|k| *k == Kind::Keyword));
    assert!(inputrc_kind_of(src, "colored-stats").iter().all(|k| *k == Kind::Var));
    assert!(inputrc_kind_of(src, "on").iter().all(|k| *k == Kind::Builtin));
    assert!(inputrc_kind_of("set completion-query-items 200", "200").iter().all(|k| *k == Kind::Number));
    assert!(inputrc_kind_of("set keymap vi-insert", "vi-insert").iter().all(|k| *k == Kind::Plain));
    // `settings` is not a `set` line
    assert!(inputrc("settings: x")[..8].iter().all(|k| *k == Kind::Var));
}

#[test]
fn inputrc_key_bindings() {
    let src = r#""\e[1;5C": forward-word"#;
    assert_eq!(inputrc(src)[0], Kind::Str);
    assert!(inputrc_kind_of(src, r"\e").iter().all(|k| *k == Kind::Op));
    assert_eq!(inputrc_kind_of(src, "[1;5C"), [Kind::Str; 5]);
    assert_eq!(inputrc_kind_of(src, ":"), [Kind::Op]);
    assert!(inputrc_kind_of(src, "forward-word").iter().all(|k| *k == Kind::Func));
    let src = r#""\C-x\C-r": re-read-init-file"#;
    assert!(inputrc_kind_of(src, r"\C-x").iter().all(|k| *k == Kind::Op));
    assert!(inputrc_kind_of(src, r"\C-r").iter().all(|k| *k == Kind::Op));
    assert!(inputrc_kind_of("Control-o: kill-line", "Control-o").iter().all(|k| *k == Kind::Var));
    assert!(inputrc_kind_of(r#""\M-\C-a": x"#, r"\M-\C-a").iter().all(|k| *k == Kind::Op));
    assert!(inputrc_kind_of(r#""\x1b[A": x"#, r"\x1b").iter().all(|k| *k == Kind::Op));
}

#[test]
fn inputrc_macros_are_strings_with_escapes() {
    let src = r#""\e\e": "\C-asudo \C-e""#;
    let k = inputrc(src);
    let at = src.find("sudo").unwrap();
    assert_eq!(&k[at..at + 5], [Kind::Str; 5]);
    assert!(inputrc_kind_of(src, r"\C-a").iter().all(|k| *k == Kind::Op));
    assert_eq!(*k.last().unwrap(), Kind::Str);
}

#[test]
fn inputrc_highlighting_covers_every_character_even_when_cut_short() {
    for src in ["", "$", "$if", "set", "set ", r#"""#, r#""\"#, r#""\e"#, r#""\C-"#, r#""\M-\"#, r#""a": "#, ":", "x:", r#""\x"#, "# ", "$include /etc/inputrc"] {
        assert_eq!(inputrc(src).len(), src.chars().count(), "{src}");
    }
}

fn front(src: &str) -> Vec<Vec<Kind>> {
    let lines: Vec<String> = src.lines().map(String::from).collect();
    with_front_matter(&lines, highlight)
}

#[test]
fn front_matter_has_its_own_colors() {
    let k = front("---\ndescription: Short aliases\nversion: 2.10\n---\nalias ll=ls\n");
    assert!(k[0].iter().all(|&x| x == Kind::Op) && k[3].iter().all(|&x| x == Kind::Op));
    assert_eq!(&k[1][..11], &[Kind::Keyword; 11]);
    assert_eq!(k[1][11], Kind::Op);
    assert!(k[1][13..].iter().all(|&x| x == Kind::Str));
    assert!(k[2][9..].iter().all(|&x| x == Kind::Number), "{:?}", k[2]);
    assert_eq!(k[4][0], Kind::Builtin, "the body is highlighted as shell: {:?}", k[4]);
}

#[test]
fn the_body_is_highlighted_on_its_own_after_the_front_matter() {
    // an open quote in the front matter must not run into the body
    let k = front("---\ndescription: it's\n---\necho hi # note\n");
    assert_eq!(k[3][0], Kind::Builtin);
    assert!(k[3][8..].iter().all(|&x| x == Kind::Comment));
}

#[test]
fn text_that_only_looks_like_front_matter_is_not_colored_as_one() {
    let unclosed = front("---\ndescription: x\necho hi\n");
    assert_eq!(unclosed, highlight(&["---".into(), "description: x".into(), "echo hi".into()]));
    let later = front("echo hi\n---\na: b\n---\n");
    assert_eq!(later.len(), 4);
    assert_eq!(front_matter_len(&["echo".to_string()]), 0);
    assert_eq!(front_matter_len(&[]), 0);
}

#[test]
fn every_character_of_front_matter_and_body_gets_a_kind() {
    let src = "---\ndescription: \"Quoted: yes\"\n# a note\nversion: 1\nbad line\n---\nalias a='b'\n";
    let lines: Vec<String> = src.lines().map(String::from).collect();
    for (l, k) in lines.iter().zip(with_front_matter(&lines, highlight)) {
        assert_eq!(l.chars().count(), k.len(), "{l}");
    }
}

#[test]
fn the_search_text_colors_options_numbers_and_the_user() {
    let k = highlight_search("-p 2222 root@web1 db");
    assert_eq!(&k[..2], &[Kind::Keyword; 2]);
    assert_eq!(k[2], Kind::Plain);
    assert_eq!(&k[3..7], &[Kind::Number; 4]);
    assert_eq!(&k[8..12], &[Kind::Var; 4]);
    assert_eq!(k[12], Kind::Op);
    assert_eq!(&k[13..17], &[Kind::Plain; 4]);
    assert_eq!(&k[18..], &[Kind::Plain; 2]);
    assert!(highlight_search("").is_empty());
    assert_eq!(highlight_search("@web"), [Kind::Plain; 4], "no user, no color");
    assert_eq!(highlight_search("web-1"), [Kind::Plain; 5], "a dash inside a name is not an option");
    assert_eq!(highlight_search("é@x").len(), 3);
}
