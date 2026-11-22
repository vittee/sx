use super::*;

fn setup() -> (tempfile::TempDir, Config) {
    let tmp = tempfile::tempdir().unwrap();
    let (cfg, created) = ensure_initialized(tmp.path()).unwrap();
    assert!(created);
    (tmp, cfg)
}

#[test]
fn first_run_creates_samples_and_default_profile() {
    let (tmp, cfg) = setup();
    assert_eq!(list_snippets(tmp.path()), ["20-aliases", "colorterm", "no-history", "vim-basic.vim", "vim-dark.vim", "vim-indent-2.vim", "vim-indent-4.vim", "vim-indent-tabs.vim", "vim-keys.vim", "vim-mouse.vim", "vim-no-traces.vim", "vim-recommended.vim"]);
    assert_eq!(cfg.profiles["default"].snippets, ["20-aliases"]);
    assert_eq!(cfg.profiles["default"].prompt.as_deref(), Some(prompt::DEFAULT_THEME));
    assert!(!ensure_initialized(tmp.path()).unwrap().1);
}

#[test]
fn default_theme_exists() {
    assert!(prompt::theme(prompt::DEFAULT_THEME).is_some());
}

#[test]
fn build_rc_orders_by_name_and_skips_unselected() {
    let (tmp, mut cfg) = setup();
    write_snippet(tmp.path(), "05-first", "echo first").unwrap();
    write_snippet(tmp.path(), "30-last", "echo last").unwrap();
    cfg.toggle_snippet("default", "05-first");
    cfg.toggle_snippet("default", "30-last");
    cfg.toggle_snippet("default", "20-aliases"); // off
    cfg.set_prompt("default", None);
    let rc = build_rc(tmp.path(), &cfg, "default");
    let (snippets, clip) = rc.split_once("# --- clip ---\n").unwrap();
    assert_eq!(snippets, "# --- 05-first ---\necho first\n# --- 30-last ---\necho last\n");
    assert!(clip.starts_with("clip() {"), "{clip}");
    assert!(clip.contains("# --- title ---\n__sx_tc='"), "{clip}");
}

#[test]
fn prompt_theme_is_appended_after_snippets() {
    let (tmp, mut cfg) = setup();
    write_snippet(tmp.path(), "99-mine", "export PS1='mine> '").unwrap();
    cfg.toggle_snippet("default", "99-mine");
    cfg.set_prompt("default", Some("minimal"));
    let rc = build_rc(tmp.path(), &cfg, "default");
    let theme = prompt::theme("minimal").unwrap();
    let rc = rc.split("# --- sx-theme ---").next().unwrap();
    assert!(rc.ends_with(&format!("# --- prompt: minimal ---\n{theme}")), "{rc}");
    assert!(rc.find("mine> ").unwrap() < rc.find("# --- prompt: minimal").unwrap());
}

#[test]
fn no_theme_means_no_ps1_and_unknown_theme_is_ignored() {
    let (tmp, mut cfg) = setup();
    cfg.set_prompt("default", None);
    assert!(!build_rc(tmp.path(), &cfg, "default").contains("export PS1"));
    cfg.set_prompt("default", Some("no-such-theme"));
    assert!(!build_rc(tmp.path(), &cfg, "default").contains("export PS1"));
}

#[test]
fn snippet_kinds_follow_the_name() {
    assert_eq!(snippet_kind("20-aliases"), SnippetKind::Shell);
    assert_eq!(snippet_kind("vimrc.vim"), SnippetKind::Vim);
    assert_eq!(snippet_kind("t.tmux"), SnippetKind::Tmux);
    assert_eq!(snippet_kind("r.inputrc"), SnippetKind::Inputrc);
    assert_eq!(snippet_kind("mine.prompt"), SnippetKind::Prompt);
    assert_eq!(snippet_kind("vim"), SnippetKind::Shell);
}

#[test]
fn non_shell_snippets_are_listed_by_full_name_and_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    write_snippet(tmp.path(), "20-aliases", "alias x=y\n").unwrap();
    write_snippet(tmp.path(), "vimrc.vim", "set number\n").unwrap();
    write_snippet(tmp.path(), "mine.prompt", "export PS1='x> '\n").unwrap();
    assert_eq!(list_snippets(tmp.path()), ["20-aliases", "mine.prompt", "vimrc.vim"]);
    assert_eq!(read_snippet(tmp.path(), "vimrc.vim").unwrap(), "set number\n");
    assert!(tmp.path().join("snippets").join("vimrc.vim").exists());
    delete_snippet(tmp.path(), "vimrc.vim").unwrap();
    assert_eq!(list_snippets(tmp.path()), ["20-aliases", "mine.prompt"]);
    assert_eq!(sendable(&list_snippets(tmp.path())), ["20-aliases"]);
}

#[test]
fn vim_tmux_and_inputrc_snippets_become_shell_without_writing_files() {
    let (tmp, mut cfg) = setup();
    write_snippet(tmp.path(), "vimrc.vim", "set number").unwrap();
    write_snippet(tmp.path(), "t.tmux", "set -g mouse on\n").unwrap();
    write_snippet(tmp.path(), "r.inputrc", "set bell-style none\n").unwrap();
    for n in ["vimrc.vim", "t.tmux", "r.inputrc"] {
        cfg.toggle_snippet("default", n);
    }
    cfg.set_prompt("default", None);
    let rc = build_rc(tmp.path(), &cfg, "default");
    assert!(rc.contains("__sx_vimrc() { cat <<'SX_EOF_VIM'\nset number\nSX_EOF_VIM\n}\n"), "{rc}");
    assert!(rc.contains("vim() { command vim -u <(__sx_vimrc) \"$@\"; }"), "{rc}");
    assert!(rc.contains("nvim() { command nvim -u <(__sx_vimrc) \"$@\"; }"), "{rc}");
    assert!(rc.contains("tmux() { command tmux -f <(__sx_tmuxconf) \"$@\"; }"), "{rc}");
    assert!(rc.contains("bind -f /dev/stdin <<'SX_EOF_INPUTRC'\nset bell-style none\nSX_EOF_INPUTRC\n"), "{rc}");
}

#[test]
fn heredoc_tag_never_collides_with_the_content() {
    let text = "a\nSX_EOF_VIM\nb\n";
    let wrapped = wrap_snippet(SnippetKind::Vim, text);
    assert!(wrapped.contains("<<'SX_EOF_VIM_'"), "{wrapped}");
    assert!(wrapped.contains("\nSX_EOF_VIM_\n}"), "{wrapped}");
}

#[test]
fn generated_shell_actually_works_in_bash() {
    // Runs only where bash exists (Linux, macOS, Git Bash); proves the wrapper parses and the
    // functions hand the config over through the process substitution.
    let Ok(bash) = std::process::Command::new("bash").arg("--version").output() else { return };
    if !bash.status.success() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let mut cfg = Config::default();
    cfg.profiles.insert("p".into(), Profile { snippets: vec!["v.vim".into()], prompt: None });
    write_snippet(tmp.path(), "v.vim", "set number\nset tw=80\n").unwrap();
    let rc = build_rc(tmp.path(), &cfg, "p");
    // Stand in for the real editor: `command vim -u FILE` prints FILE.
    let script = format!("command() {{ shift; [ \"$1\" = -u ] && cat \"$2\"; }}\n{rc}\nvim");
    // The script goes in on stdin: quoting a multi-line script as an argument is not reliable
    // when `bash` is a launcher such as WSL's bash.exe.
    use std::io::Write;
    let mut child = std::process::Command::new("bash")
        .arg("-s")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(script.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "status {:?}\nstderr: {}\nstdout: {}\nscript:\n{script}",
        out.status,
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout).replace('\r', ""), "set number\nset tw=80\n");
}

#[test]
fn host_extras_are_added_to_the_profile_and_survive_save_and_load() {
    let (tmp, mut cfg) = setup();
    write_snippet(tmp.path(), "05-proxy", "export http_proxy=x").unwrap();
    cfg.set_prompt("default", None);
    cfg.toggle_host_snippet("db1", "05-proxy");
    assert_eq!(cfg.host_snippets("db1"), ["05-proxy"]);
    let plain = build_rc(tmp.path(), &cfg, "default");
    let for_db1 = build_rc_for_host(tmp.path(), &cfg, "default", "db1");
    let for_other = build_rc_for_host(tmp.path(), &cfg, "default", "web");
    assert!(!plain.contains("http_proxy"));
    assert!(for_db1.contains("http_proxy"));
    assert!(for_db1.find("05-proxy").unwrap() < for_db1.find("20-aliases").unwrap(), "sorted by name");
    assert_eq!(for_other, plain);
    cfg.save(tmp.path()).unwrap();
    assert_eq!(Config::load(tmp.path()).unwrap().host_snippets("db1"), ["05-proxy"]);
    cfg.toggle_host_snippet("db1", "05-proxy");
    assert!(cfg.host_snippets("db1").is_empty());
}

#[test]
fn a_snippet_in_both_profile_and_host_is_sent_once() {
    let (tmp, mut cfg) = setup();
    cfg.toggle_host_snippet("h", "20-aliases");
    let rc = build_rc_for_host(tmp.path(), &cfg, "default", "h");
    assert_eq!(rc.matches("# --- 20-aliases ---").count(), 1);
}

#[test]
fn deleting_a_snippet_forgets_it_everywhere() {
    let (_tmp, mut cfg) = setup();
    cfg.toggle_host_snippet("h", "20-aliases");
    cfg.forget_snippet("20-aliases");
    assert!(cfg.host_snippets("h").is_empty());
    assert!(cfg.profiles["default"].snippets.is_empty());
}

#[test]
fn user_prompt_themes_are_used_and_a_missing_one_is_ignored() {
    let (tmp, mut cfg) = setup();
    write_snippet(tmp.path(), "mine.prompt", "export PS1='mine\\h> '\n").unwrap();
    cfg.toggle_snippet("default", "mine.prompt"); // never sent as a snippet
    cfg.set_prompt("default", Some("mine.prompt"));
    let rc = build_rc(tmp.path(), &cfg, "default");
    let rc = rc.split("# --- sx-theme ---").next().unwrap();
    assert!(rc.ends_with("# --- prompt: mine.prompt ---\nexport PS1='mine\\h> '\n"), "{rc}");
    assert_eq!(rc.matches("export PS1").count(), 1, "sent once, as the theme: {rc}");
    assert!(!rc.contains("# --- mine.prompt ---"), "not sent as a snippet: {rc}");
    cfg.set_prompt("default", Some("gone.prompt"));
    assert!(!build_rc(tmp.path(), &cfg, "default").contains("export PS1"));
}

#[test]
fn prompt_is_saved_per_profile() {
    let (tmp, mut cfg) = setup();
    cfg.add_profile("work").unwrap();
    cfg.set_prompt("work", Some("two-line"));
    cfg.save(tmp.path()).unwrap();
    let loaded = Config::load(tmp.path()).unwrap();
    assert_eq!(loaded.profiles["work"].prompt.as_deref(), Some("two-line"));
    assert_eq!(loaded.profiles["default"].prompt.as_deref(), Some(prompt::DEFAULT_THEME));
}

#[test]
fn old_config_without_prompt_field_still_loads() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("config.toml"), "[profiles.default]\nsnippets = [\"a\"]\n").unwrap();
    let cfg = Config::load(tmp.path()).unwrap();
    assert_eq!(cfg.profiles["default"].prompt, None);
}

#[test]
fn profile_for_falls_back_and_off_works() {
    let (_tmp, mut cfg) = setup();
    assert_eq!(cfg.profile_for("h"), Some("default"));
    cfg.hosts.entry("h".into()).or_default().profile = Some(PROFILE_OFF.into());
    assert_eq!(cfg.profile_for("h"), None);
    cfg.hosts.get_mut("h").unwrap().profile = Some("gone".into());
    assert_eq!(cfg.profile_for("h"), Some("default"));
}

#[test]
fn cycling_walks_off_then_profiles() {
    let (_tmp, mut cfg) = setup();
    cfg.add_profile("work").unwrap();
    // choices: "-", "default", "work"; host starts on default
    cfg.cycle_profile("h", true);
    assert_eq!(cfg.profile_for("h"), Some("work"));
    cfg.cycle_profile("h", true);
    assert_eq!(cfg.profile_for("h"), None);
    cfg.cycle_profile("h", false);
    assert_eq!(cfg.profile_for("h"), Some("work"));
}

#[test]
fn profile_rules() {
    let (_tmp, mut cfg) = setup();
    assert!(cfg.add_profile("bad name").is_err());
    assert!(cfg.add_profile("-").is_err());
    assert!(cfg.add_profile("default").is_err());
    assert!(cfg.delete_profile("default").is_err()); // last one
    cfg.add_profile("work").unwrap();
    cfg.delete_profile("default").unwrap();
    assert_eq!(cfg.default_profile, "work");
}

#[test]
fn duplicate_profile_copies_snippets_and_prompt_independently() {
    let (_tmp, mut cfg) = setup();
    cfg.set_prompt("default", Some("arrow"));
    cfg.duplicate_profile("default", "work").unwrap();
    assert_eq!(cfg.profiles["work"], cfg.profiles["default"]);
    cfg.toggle_snippet("work", "20-aliases");
    cfg.set_prompt("work", None);
    assert_eq!(cfg.profiles["default"].snippets, ["20-aliases"]);
    assert_eq!(cfg.profiles["default"].prompt.as_deref(), Some("arrow"));
    assert!(cfg.profiles["work"].snippets.is_empty());
}

#[test]
fn duplicate_profile_rejects_bad_targets() {
    let (_tmp, mut cfg) = setup();
    assert!(cfg.duplicate_profile("default", "default").is_err()); // exists
    assert!(cfg.duplicate_profile("default", "bad name").is_err());
    assert!(cfg.duplicate_profile("nope", "x").is_err());
    assert_eq!(cfg.profiles.len(), 1);
}

#[test]
fn config_roundtrips_through_toml() {
    let (tmp, mut cfg) = setup();
    cfg.toggle_favorite("srv");
    cfg.mark_used("srv");
    cfg.save(tmp.path()).unwrap();
    assert_eq!(Config::load(tmp.path()).unwrap(), cfg);
}

#[test]
fn bad_snippet_names_are_rejected() {
    let (tmp, _) = setup();
    assert!(write_snippet(tmp.path(), "../evil", "x").is_err());
    assert!(write_snippet(tmp.path(), "", "x").is_err());
    assert!(write_snippet(tmp.path(), ".hidden", "x").is_err());
}

#[test]
fn write_atomic_replaces_file_and_leaves_no_temp() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("config.toml");
    write_atomic(&path, b"one").unwrap();
    write_atomic(&path, b"two").unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), "two");
    let names: Vec<_> = fs::read_dir(tmp.path()).unwrap().flatten().map(|e| e.file_name()).collect();
    assert_eq!(names, vec![std::ffi::OsString::from("config.toml")]);
}

#[test]
fn a_prompt_snippet_with_only_the_ps1_string_is_wrapped_for_the_server() {
    let (tmp, mut cfg) = setup();
    write_snippet(tmp.path(), "mine.prompt", "\\u@\\h\\$ \n").unwrap();
    cfg.set_prompt("default", Some("mine.prompt"));
    let rc = build_rc(tmp.path(), &cfg, "default");
    let rc = rc.split("# --- sx-theme ---").next().unwrap();
    assert!(rc.ends_with("# --- prompt: mine.prompt ---\nexport PS1='\\u@\\h\\$ '\n"), "{rc}");
    assert_eq!(theme_ps1(tmp.path(), "mine.prompt").as_deref(), Some("\\u@\\h\\$ "));
    assert_eq!(theme_ps1(tmp.path(), "minimal").as_deref(), Some("\\u@\\h:\\w\\$ "));
}

#[test]
fn enabled_vim_snippets_are_joined_into_one_config_in_name_order() {
    let (tmp, mut cfg) = setup();
    write_snippet(tmp.path(), "a.vim", "set number").unwrap();
    write_snippet(tmp.path(), "b.vim", "set ruler
").unwrap();
    write_snippet(tmp.path(), "c.vim", "set off
").unwrap();
    cfg.toggle_snippet("default", "b.vim");
    cfg.toggle_snippet("default", "a.vim");
    let rc = build_rc(tmp.path(), &cfg, "default");
    assert_eq!(rc.matches("__sx_vimrc() {").count(), 1, "{rc}");
    assert!(rc.contains("# --- a.vim, b.vim ---
"), "{rc}");
    assert!(rc.contains("<<'SX_EOF_VIM'
set number
set ruler
SX_EOF_VIM
"), "{rc}");
    assert!(!rc.contains("set off"), "{rc}");
}

#[test]
fn the_vim_presets_are_off_by_default_and_every_name_is_valid() {
    let (tmp, cfg) = setup();
    assert_eq!(cfg.profiles["default"].snippets, ["20-aliases"]);
    for (name, text) in presets::vim_presets() {
        assert!(is_valid_name(name) && name.ends_with(".vim"), "{name}");
        assert!(text.starts_with("\" sx "), "{name}");
        assert_eq!(read_snippet(tmp.path(), name).unwrap(), text, "{name}");
    }
    let recommended = read_snippet(tmp.path(), "vim-recommended.vim").unwrap();
    assert!(recommended.starts_with(&read_snippet(tmp.path(), "vim-basic.vim").unwrap()));
}

#[test]
fn no_vim_preset_sets_hidden() {
    for (name, text) in presets::vim_presets() {
        assert!(!text.lines().any(|l| l.trim_start().starts_with("set hidden")), "{name}");
    }
}

#[test]
fn every_vim_preset_is_highlighted_character_for_character() {
    for (name, text) in presets::vim_presets() {
        let lines: Vec<String> = text.lines().map(String::from).collect();
        let kinds = crate::highlight::highlight_vim(&lines);
        for (l, k) in lines.iter().zip(&kinds) {
            assert_eq!(l.chars().count(), k.len(), "{name}: {l}");
        }
    }
}

#[test]
fn missing_presets_are_added_off_and_existing_files_are_kept() {
    let (tmp, cfg) = setup();
    assert!(add_missing_presets(tmp.path()).unwrap().is_empty());
    delete_snippet(tmp.path(), "vim-keys.vim").unwrap();
    write_snippet(tmp.path(), "vim-basic.vim", "set mine").unwrap();
    assert_eq!(add_missing_presets(tmp.path()).unwrap(), ["vim-keys.vim"]);
    assert_eq!(read_snippet(tmp.path(), "vim-basic.vim").unwrap(), "set mine");
    assert!(list_snippets(tmp.path()).contains(&"vim-keys.vim".to_string()));
    assert_eq!(Config::load(tmp.path()).unwrap(), cfg, "no profile changed");
}

#[test]
fn host_options_become_ssh_arguments_before_the_host() {
    let prefs = HostPrefs {
        user: "root".into(),
        port: "2222".into(),
        identity: " ~/.ssh/work ".into(),
        jump: "bastion".into(),
        args: "-o ForwardAgent=yes".into(),
        ..HostPrefs::default()
    };
    assert!(prefs.has_overrides());
    assert_eq!(
        prefs.ssh_args(),
        ["-l", "root", "-p", "2222", "-i", "~/.ssh/work", "-J", "bastion", "-o", "ForwardAgent=yes"]
    );
    assert!(!HostPrefs::default().has_overrides());
    assert!(HostPrefs::default().ssh_args().is_empty());
}

#[test]
fn extra_arguments_are_split_like_a_shell_would_for_simple_cases() {
    assert_eq!(split_args("  -v   -o  A=b "), ["-v", "-o", "A=b"]);
    assert_eq!(split_args(r#"-o "ProxyCommand ssh -W %h:%p jump" -L '8080:localhost:80'"#), [
        "-o",
        "ProxyCommand ssh -W %h:%p jump",
        "-L",
        "8080:localhost:80"
    ]);
    assert_eq!(split_args("a '' b"), ["a", "", "b"], "an empty quoted word is kept");
    assert!(split_args("   ").is_empty());
}

#[test]
fn host_options_are_saved_only_when_set() {
    let (tmp, mut cfg) = setup();
    cfg.hosts.entry("box".into()).or_default().user = "root".into();
    cfg.save(tmp.path()).unwrap();
    let text = std::fs::read_to_string(tmp.path().join("config.toml")).unwrap();
    assert!(text.contains("user = \"root\""), "{text}");
    assert!(!text.contains("port") && !text.contains("identity") && !text.contains("jump"), "{text}");
    assert_eq!(Config::load(tmp.path()).unwrap().hosts["box"].user, "root");
}

#[test]
fn named_option_sets_round_trip_and_old_configs_still_load() {
    let (tmp, mut cfg) = setup();
    let prefs = cfg.hosts.entry("box".into()).or_default();
    prefs.user = "me".into();
    prefs.options.insert("root".into(), ConnOptions { user: "root".into(), jump: "bastion".into(), ..ConnOptions::default() });
    cfg.save(tmp.path()).unwrap();
    let loaded = Config::load(tmp.path()).unwrap();
    assert_eq!(loaded.hosts["box"].options["root"].summary(), "-l root -J bastion");
    assert_eq!(loaded.hosts["box"].user, "me");
    assert!(loaded.hosts["box"].has_overrides());

    // A config saved before options existed has only the default options.
    std::fs::write(tmp.path().join("config.toml"), "[hosts.old]\nuser = \"root\"\nport = \"2200\"\n").unwrap();
    let old = Config::load(tmp.path()).unwrap();
    assert!(old.hosts["old"].options.is_empty());
    assert_eq!(old.hosts["old"].ssh_args(), ["-l", "root", "-p", "2200"]);
    assert_eq!(ConnOptions::default().summary(), "no options");
}

#[test]
fn sx_theme_switches_between_the_built_in_and_your_own_themes() {
    let (tmp, mut cfg) = setup();
    write_snippet(tmp.path(), "mine.prompt", "export PS1='mine> '\n").unwrap();
    let rc = build_rc(tmp.path(), &cfg, "default");
    assert!(rc.contains("sx-theme() {"), "{rc}");
    for (name, _) in prompt::THEMES {
        assert!(rc.contains(&format!("'{name}') cat <<")), "{name}");
    }
    assert!(rc.contains("'mine.prompt') cat <<"), "{rc}");
    assert!(rc.contains("complete -W '") && rc.contains("mine.prompt' sx-theme sxt"), "{rc}");
    assert!(rc.contains("alias sxt=sx-theme\n"), "{rc}");
    cfg.set_prompt("default", None);
    assert!(!build_rc(tmp.path(), &cfg, "default").contains("sx-theme"), "no theme, no switcher");
}

#[test]
fn clip_is_always_sent_and_writes_osc_52_to_the_terminal() {
    let (tmp, mut cfg) = setup();
    cfg.set_prompt("default", None);
    let rc = build_rc(tmp.path(), &cfg, "default");
    assert!(rc.contains("# --- clip ---\nclip() {"), "{rc}");
    assert!(rc.contains(r"\033]52;c;%s\a") && rc.contains(">/dev/tty"), "{rc}");
}

#[test]
fn the_title_hook_is_sent_last_and_only_once() {
    let (tmp, cfg) = setup();
    let rc = build_rc(tmp.path(), &cfg, "default");
    assert_eq!(rc.matches("__sx_title() {").count(), 1, "{rc}");
    assert!(rc.contains(r"\033]0;[sx] %s@%s: %s\a"), "{rc}");
    assert!(rc.find("# --- title ---").unwrap() > rc.find("# --- prompt: ").unwrap(), "{rc}");
    assert!(rc.contains("return $s; }"), "the hook must keep $?: {rc}");
    assert!(rc.contains(r#"PS1="$PS1\[\$($__sx_tc)\]\[\e]133;B\a\]""#), "the title goes last in PS1: {rc}");
    assert!(!rc.contains("__sx_t()") && !rc.contains("export -f"), "no function: sudo -i drops them: {rc}");
    assert!(rc.contains("eval \"$t\"; __sx_ps1\n}"), "sx-theme re-adds it and remembers the name: {rc}");
    assert!(rc.contains("'') __sx_theme_show; return;;") && rc.contains("-l|--list)"), "{rc}");
}

#[test]
fn the_theme_preview_drops_the_carriage_return_bash_puts_in_a_prompt_newline() {
    let (tmp, cfg) = setup();
    let rc = build_rc(tmp.path(), &cfg, "default");
    assert!(rc.contains(r"p=${p//[$'\001\002\r']/}"), "{rc}");
}

#[test]
fn the_prompt_is_marked_with_osc_133() {
    let (tmp, cfg) = setup();
    let rc = build_rc(tmp.path(), &cfg, "default");
    assert!(rc.contains(r"PS1='\[\e]133;A\a\]'$PS1"), "A starts the prompt: {rc}");
    assert!(rc.contains(r"\[\e]133;B\a\]"), "B ends it: {rc}");
    assert!(rc.contains(r"PS0=${PS0-}'\e]133;C\a'"), "C starts the output: {rc}");
    assert!(rc.contains(r"\033]133;D;%s\a"), "D ends the command: {rc}");
}

#[test]
fn colorterm_is_created_off_and_sets_truecolor_when_turned_on() {
    let (tmp, mut cfg) = setup();
    assert_eq!(read_snippet(tmp.path(), "colorterm").unwrap().lines().last(), Some("export COLORTERM=truecolor"));
    assert!(!build_rc(tmp.path(), &cfg, "default").contains("COLORTERM"), "off by default");
    cfg.toggle_snippet("default", "colorterm");
    assert!(build_rc(tmp.path(), &cfg, "default").contains("export COLORTERM=truecolor\n"));
    // `sx init --presets` brings it back for a setup that has no such file.
    fs::remove_file(snippets_dir(tmp.path()).join("colorterm.sh")).unwrap();
    assert_eq!(add_missing_presets(tmp.path()).unwrap(), ["colorterm"]);
}

#[test]
fn no_history_is_created_off_and_unsets_histfile_when_turned_on() {
    let (tmp, mut cfg) = setup();
    assert_eq!(read_snippet(tmp.path(), "no-history").unwrap().lines().last(), Some("unset HISTFILE"));
    assert!(!build_rc(tmp.path(), &cfg, "default").contains("unset HISTFILE"), "off by default");
    cfg.toggle_snippet("default", "no-history");
    assert!(build_rc(tmp.path(), &cfg, "default").contains("unset HISTFILE
"));
    fs::remove_file(snippets_dir(tmp.path()).join("no-history.sh")).unwrap();
    assert_eq!(add_missing_presets(tmp.path()).unwrap(), ["no-history"]);
}

#[test]
fn sx_sudo_is_always_sent_and_passes_the_payload_on_stdin_not_in_the_arguments() {
    let (tmp, cfg) = setup();
    let rc = build_rc(tmp.path(), &cfg, "default");
    let at = rc.find("# --- sx-sudo ---").expect("the sx-sudo block");
    let block = &rc[at..];
    assert!(block.contains("sx-sudo() {"), "{block}");
    // sudo logs its arguments, so the payload goes through a pipe
    assert!(block.contains("printf %s \"$__sx_payload\" | sudo -H -u \"$u\" bash -c"), "{block}");
    assert!(block.contains("-i <&1'") && !block.contains("</dev/tty"), "tmux needs the real tty name: {block}");
}

/// Runs `script` in an interactive bash that has loaded the rc of the default profile with a prompt
/// theme, as the server does. `None` when there is no bash. Returns stdout and stderr.
fn run_in_rc(script: &str) -> Option<(String, String)> {
    use std::io::Write;
    let (tmp, mut cfg) = setup();
    cfg.set_prompt("default", Some("fino"));
    let rc = build_rc(tmp.path(), &cfg, "default");
    let mut child = std::process::Command::new("bash")
        .arg("--norc")
        .arg("-i")
        .env("LANG", "C.UTF-8")
        .env("HOME", tmp.path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(format!("{rc}\n{script}").as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    Some((String::from_utf8_lossy(&out.stdout).replace('\r', ""), String::from_utf8_lossy(&out.stderr).to_string()))
}

#[test]
fn every_sx_command_answers_help_and_refuses_an_unknown_option() {
    let Some((out, _)) = run_in_rc("sx-theme --help
echo ---
sxt -h
echo ---
sx-sudo --help
echo ---
clip -h
echo ---
sx-theme -x
echo \"rc=$?\"
sx-sudo -x
echo \"rc=$?\"
clip --nope
echo \"rc=$?\"
") else { return };
    // The prompt marks around the output are not part of what is checked.
    assert_eq!(out.matches("usage: sx-theme [name]").count(), 2, "sx-theme and sxt: {out}");
    assert!(out.contains("-l, --list   list the theme names"), "{out}");
    assert!(out.contains("usage: sx-sudo [user]") && out.contains("like sudo -i"), "{out}");
    assert!(out.contains("usage: clip [text]") && out.contains("cat file | clip"), "{out}");
    assert_eq!(out.matches("rc=1").count(), 3, "an unknown option fails: {out}");
}

#[test]
fn the_title_runs_inside_ps1_with_no_function_and_keeps_the_status() {
    // `sudo -i` and tmux start shells that have none of our functions, so the title is plain code in PS1
    let script = "cd /tmp; false; __sx_title; echo \"status=$?\"; t=${PS1@P}; case $t in *'0;[sx] '*'@'*': /tmp'*) echo TITLE_OK;; *) echo TITLE_MISSING;; esac\n";
    let Some((out, err)) = run_in_rc(script) else { return };
    assert!(out.contains("TITLE_OK") && !out.contains("TITLE_MISSING"), "{out}");
    assert!(!err.contains("command not found"), "{err}");
}

#[test]
fn the_theme_preview_still_shows_a_demo_branch_and_status() {
    let Some((out, err)) = run_in_rc("sx-theme
") else { return };
    assert!(!err.contains("command not found"), "{err}");
    assert!(out.contains("(main)") && out.contains("✗1"), "{out}");
    // made-up names, so a long real hostname cannot break the boxes
    assert!(out.contains("you") && out.contains("server") && out.contains("~/app"), "{out}");
}
