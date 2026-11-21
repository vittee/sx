use super::*;
use crate::config::ConnOptions;

fn cfg() -> Config {
    let mut cfg = Config::default();
    let web = cfg.hosts.entry("web".to_string()).or_default();
    web.options.insert("root".to_string(), ConnOptions::default());
    web.options.insert("deploy".to_string(), ConnOptions::default());
    cfg.hosts.entry("db".to_string()).or_default().options.insert("admin".to_string(), ConnOptions::default());
    cfg
}

fn hosts() -> Vec<String> {
    ["web", "web2", "bastion"].map(String::from).to_vec()
}

fn complete(line: &str) -> Vec<String> {
    // A trailing space means the cursor starts a new, empty word.
    let mut words: Vec<String> = line.split_whitespace().map(String::from).collect();
    if line.ends_with(' ') || line.is_empty() {
        words.push(String::new());
    }
    candidates(&words, &hosts(), &cfg())
}

#[test]
fn the_first_word_is_a_host_a_subcommand_or_a_flag() {
    assert_eq!(complete(""), ["bastion", "db", "init", "web", "web2"]);
    assert_eq!(complete("we"), ["web", "web2"]);
    assert_eq!(complete("i"), ["init"]);
    assert_eq!(complete("--o"), ["--options"]);
    assert_eq!(complete("--"), ["--dry-run", "--help", "--options", "--version"]);
}

#[test]
fn hosts_come_from_ssh_and_from_the_saved_config_without_repeats() {
    assert!(complete("b").contains(&"bastion".to_string()));
    assert_eq!(complete("d"), ["db"], "a host only in config.toml still completes");
    assert_eq!(complete("web"), ["web", "web2"]);
}

#[test]
fn a_typed_user_stays_in_front_of_the_host() {
    assert_eq!(complete("root@we"), ["root@web", "root@web2"]);
}

#[test]
fn options_names_follow_the_host_when_it_is_known() {
    assert_eq!(complete("web --options "), ["deploy", "root"]);
    assert_eq!(complete("web --options r"), ["root"]);
    assert_eq!(complete("-p 22 me@web --options "), ["deploy", "root"]);
    assert_eq!(complete("bastion --options "), Vec::<String>::new());
}

#[test]
fn options_names_before_the_host_list_every_name() {
    assert_eq!(complete("--options "), ["admin", "deploy", "root"]);
}

#[test]
fn nothing_is_offered_where_the_shell_should_take_over() {
    assert!(complete("-p ").is_empty(), "the value of an ssh option");
    assert!(complete("-i ").is_empty());
    assert!(complete("web ").is_empty(), "a remote command follows the host");
    assert!(complete("web ls ").is_empty());
}

#[test]
fn init_completes_its_flag_only() {
    assert_eq!(complete("init "), ["--presets"]);
    assert_eq!(complete("init --p"), ["--presets"]);
    assert!(complete("init --presets ").is_empty());
}

#[test]
fn a_flag_after_the_host_still_completes() {
    assert_eq!(complete("web --o"), ["--options"]);
}

#[test]
fn every_shell_has_a_script() {
    for script in [BASH, ZSH, FISH, POWERSHELL, NUSHELL] {
        assert!(script.contains("__complete"), "script does not call sx __complete");
    }
}

mod install_tests {
    use super::super::install::{Action, Dirs, detect, plan};
    use super::super::Shell;
    use std::fs;

    fn dirs(root: &std::path::Path) -> Dirs {
        Dirs {
            home: root.to_path_buf(),
            zdotdir: None,
            xdg_config: Some(root.join("xdg")),
            appdata: Some(root.join("appdata")),
            ps_profile: Some(root.join("profile.ps1")),
        }
    }

    fn apply_all(actions: &[Action]) {
        for a in actions.iter().filter(|a| !a.is_done()) {
            a.apply().unwrap();
        }
    }

    #[test]
    fn bash_gets_one_line_in_bashrc_and_a_second_install_changes_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let d = dirs(tmp.path());
        fs::write(tmp.path().join(".bashrc"), "alias ll='ls -l'").unwrap(); // no final newline
        let actions = plan(Shell::Bash, &d).unwrap();
        assert!(actions.iter().all(|a| !a.is_done()));
        apply_all(&actions);
        let text = fs::read_to_string(tmp.path().join(".bashrc")).unwrap();
        assert!(text.starts_with("alias ll='ls -l'\n"), "{text:?}");
        assert_eq!(text.matches("sx completions bash").count(), 1, "{text:?}");
        assert!(plan(Shell::Bash, &d).unwrap().iter().all(Action::is_done));
        apply_all(&plan(Shell::Bash, &d).unwrap());
        assert_eq!(fs::read_to_string(tmp.path().join(".bashrc")).unwrap(), text);
    }

    #[test]
    fn zsh_follows_zdotdir() {
        let tmp = tempfile::tempdir().unwrap();
        let mut d = dirs(tmp.path());
        d.zdotdir = Some(tmp.path().join("zdot"));
        apply_all(&plan(Shell::Zsh, &d).unwrap());
        let text = fs::read_to_string(tmp.path().join("zdot").join(".zshrc")).unwrap();
        assert!(text.contains("source <(sx completions zsh)"));
        assert!(!tmp.path().join(".zshrc").exists());
    }

    #[test]
    fn fish_gets_its_own_file_and_no_rc_edit() {
        let tmp = tempfile::tempdir().unwrap();
        let d = dirs(tmp.path());
        let actions = plan(Shell::Fish, &d).unwrap();
        assert_eq!(actions.len(), 1);
        apply_all(&actions);
        let file = tmp.path().join("xdg").join("fish").join("completions").join("sx.fish");
        assert_eq!(fs::read_to_string(file).unwrap(), Shell::Fish.script());
    }

    #[test]
    fn nushell_gets_the_script_and_a_use_line() {
        let tmp = tempfile::tempdir().unwrap();
        let d = dirs(tmp.path());
        let dir = d.nushell();
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("config.nu"), "$env.config = {}\n").unwrap();
        apply_all(&plan(Shell::Nushell, &d).unwrap());
        assert_eq!(fs::read_to_string(dir.join("sx-completions.nu")).unwrap(), Shell::Nushell.script());
        let config = fs::read_to_string(dir.join("config.nu")).unwrap();
        assert!(config.starts_with("$env.config = {}\n") && config.contains("use sx-completions.nu *"), "{config:?}");
        assert!(plan(Shell::Nushell, &d).unwrap().iter().all(Action::is_done));
    }

    #[test]
    fn powershell_needs_its_profile_path() {
        let tmp = tempfile::tempdir().unwrap();
        let mut d = dirs(tmp.path());
        apply_all(&plan(Shell::Powershell, &d).unwrap());
        assert!(fs::read_to_string(tmp.path().join("profile.ps1")).unwrap().contains("Invoke-Expression"));
        d.ps_profile = None;
        assert!(plan(Shell::Powershell, &d).is_err());
    }

    #[test]
    fn the_running_shell_is_found_from_the_environment() {
        assert_eq!(detect(Some("/usr/bin/zsh"), false), Some(Shell::Zsh));
        assert_eq!(detect(Some("/bin/bash"), false), Some(Shell::Bash));
        assert_eq!(detect(Some("/usr/bin/fish"), false), Some(Shell::Fish));
        assert_eq!(detect(None, true), Some(Shell::Nushell));
        assert_eq!(detect(Some("/bin/bash"), true), None, "both are inherited, so this is a guess");
        assert_eq!(detect(Some("/usr/bin/pwsh"), false), Some(Shell::Powershell));
        assert_eq!(detect(Some("/bin/unknown"), false), if cfg!(windows) { Some(Shell::Powershell) } else { None });
    }
}
