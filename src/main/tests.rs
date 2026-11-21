use super::*;

fn host(line: &str) -> Option<String> {
    let args: Vec<String> = line.split_whitespace().map(String::from).collect();
    injectable_host(&args)
}

fn inject(line: &str) -> bool {
    host(line).is_some()
}

fn cfg_with_options() -> config::Config {
    let mut cfg = config::Config::default();
    let prefs = cfg.hosts.entry("web".to_string()).or_default();
    prefs.port = "2200".to_string();
    prefs.options.insert("root".to_string(), config::ConnOptions { user: "root".to_string(), ..Default::default() });
    cfg
}

#[test]
fn named_options_are_picked_by_name_and_the_default_is_used_without_one() {
    let cfg = cfg_with_options();
    let root = pick_options(&cfg, "web", Some("root")).unwrap().unwrap();
    assert_eq!(root.ssh_args(), ["-l", "root"]);
    // `user@` is not part of the saved host name.
    assert_eq!(pick_options(&cfg, "me@web", None).unwrap().unwrap().ssh_args(), ["-p", "2200"]);
    assert!(pick_options(&cfg, "other", None).unwrap().is_none());
}

#[test]
fn an_unknown_options_name_lists_the_known_ones() {
    let cfg = cfg_with_options();
    let msg = pick_options(&cfg, "web", Some("nope")).unwrap_err();
    assert!(msg.contains("\"nope\"") && msg.contains("root"), "{msg}");
    assert!(pick_options(&cfg, "other", Some("x")).unwrap_err().contains("none saved"));
}

#[test]
fn saved_options_come_after_the_typed_arguments() {
    let o = config::ConnOptions { port: "2200".to_string(), ..Default::default() };
    let args = |s: &str| s.split_whitespace().map(String::from).collect::<Vec<_>>();
    assert_eq!(with_options(args("-p 22 web"), &o), args("-p 22 web -p 2200"));
    assert_eq!(with_options(args("web -- ls"), &o), args("web -p 2200 -- ls"));
}

#[test]
fn options_flag_is_taken_from_any_position() {
    let take = |line: &str| {
        let mut args: Vec<String> = line.split_whitespace().map(String::from).collect();
        let name = take_options(&mut args);
        (name, args.join(" "))
    };
    assert_eq!(take("--options root web"), (Ok(Some("root".to_string())), "web".to_string()));
    assert_eq!(take("-p 22 web --options root"), (Ok(Some("root".to_string())), "-p 22 web".to_string()));
    assert_eq!(take("-p 22 --options=root web"), (Ok(Some("root".to_string())), "-p 22 web".to_string()));
    assert_eq!(take("web"), (Ok(None), "web".to_string()));
    // After `--` the words belong to the remote command.
    assert_eq!(take("web -- ls --options x"), (Ok(None), "web -- ls --options x".to_string()));
    assert!(take("web --options").0.is_err());
}

#[test]
fn a_connection_from_the_command_line_sets_last_used() {
    let tmp = tempfile::tempdir().unwrap();
    config::Config::default().save(tmp.path()).unwrap();
    mark_used(tmp.path(), "me@web");
    let cfg = config::Config::load(tmp.path()).unwrap();
    assert!(cfg.hosts.get("web").and_then(|p| p.last_used).is_some(), "the user@ prefix is not part of the host");
    // No config yet: nothing is created.
    let empty = tempfile::tempdir().unwrap();
    mark_used(&empty.path().join("none"), "web");
    assert!(!empty.path().join("none").exists());
}

#[test]
fn version_line_names_the_crate_version() {
    assert_eq!(version_line(), format!("sx {}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn plain_login_injects() {
    assert!(inject("host"));
    assert!(inject("user@host"));
    assert!(inject("-p 2222 user@host"));
    assert!(inject("-p2222 -i key.pem host"));
    assert!(inject("-vvv -A host"));
    assert!(inject("-o StrictHostKeyChecking=no host"));
    assert!(inject("host -p 22"));
    assert!(inject("-- host"));
}

#[test]
fn returns_the_host() {
    assert_eq!(host("-p 2222 user@box").as_deref(), Some("user@box"));
    assert_eq!(host("-J jump -- target").as_deref(), Some("target"));
    assert_eq!(host("host -p 22").as_deref(), Some("host"));
}

#[test]
fn option_values_are_not_mistaken_for_host() {
    assert!(!inject("-p 22")); // no host at all
    assert!(inject("-J jump host"));
    assert!(inject("-L 8080:localhost:80 host"));
}

#[test]
fn remote_command_skips() {
    assert!(!inject("host ls"));
    assert!(!inject("-p 22 host uptime -a"));
    assert!(!inject("-- host ls"));
}

#[test]
fn no_shell_options_skip() {
    for line in ["-N host", "-T host", "-W h:22 host", "-O check host", "-V", "-G host", "-s host sftp", "-fN host", "-vN host"] {
        assert!(!inject(line), "{line}");
    }
}

#[test]
fn user_remote_command_skips() {
    assert!(!inject("-o RemoteCommand=top host"));
    assert!(!inject("-oRemoteCommand=top host"));
    assert!(!inject("-o remotecommand=top host"));
}

#[test]
fn ctrl_c_exit_is_an_interrupt_but_plain_130_is_not() {
    #[cfg(windows)]
    {
        use std::os::windows::process::ExitStatusExt;
        let killed = std::process::ExitStatus::from_raw(STATUS_CONTROL_C_EXIT as u32);
        assert_eq!(outcome_of(killed), SshOutcome::Interrupted);
        let remote_130 = std::process::ExitStatus::from_raw(130);
        assert_eq!(outcome_of(remote_130), SshOutcome::Exited(130));
        assert_eq!(outcome_of(std::process::ExitStatus::from_raw(255)), SshOutcome::Exited(255));
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(outcome_of(std::process::ExitStatus::from_raw(2)), SshOutcome::Interrupted); // SIGINT
        assert_eq!(outcome_of(std::process::ExitStatus::from_raw(130 << 8)), SshOutcome::Exited(130));
        assert_eq!(outcome_of(std::process::ExitStatus::from_raw(9)), SshOutcome::Exited(255)); // SIGKILL
    }
    assert_eq!(SshOutcome::Interrupted.code(), 130);
    assert_eq!(SshOutcome::Exited(7).code(), 7);
}

#[test]
fn no_args_skips() {
    assert!(!inject(""));
}

#[test]
fn profile_lookup_tries_exact_then_without_user() {
    let mut cfg = config::Config::default();
    cfg.profiles.insert("default".into(), config::Profile::default());
    cfg.hosts.entry("box".into()).or_default().profile = Some(config::PROFILE_OFF.into());
    assert_eq!(profile_for_target(&cfg, "box"), None);
    assert_eq!(profile_for_target(&cfg, "me@box"), None);
    assert_eq!(profile_for_target(&cfg, "other"), Some("default"));
}

/// Feeds our argv to the real `ssh -G` (prints parsed config, no connection):
/// proves ssh accepts the options and the quoting survives Command.
#[test]
fn real_ssh_parses_our_options() {
    let Ok(out) = Command::new("ssh")
        .arg("-G")
        .args(build_argv(vec!["testhost".into()], "UEFZTE9BRA=="))
        .output()
    else {
        return; // ssh not installed
    };
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(!text.contains("setenv"), "{text}");
    assert!(text.contains("requesttty force"), "{text}");
    assert!(
        text.lines().any(|l| l == format!("remotecommand {}", bootstrap::remote_command("testhost", "UEFZTE9BRA=="))),
        "{text}"
    );
}

#[test]
fn argv_puts_our_options_first() {
    let argv = build_argv(vec!["host".into()], "PAYLOAD");
    assert_eq!(argv[0], "-o");
    assert_eq!(argv[1], "RequestTTY=force");
    assert_eq!(argv[2], "-o");
    assert!(argv[3].starts_with("RemoteCommand="));
    assert!(argv[3].contains("P=PAYLOAD;"));
    assert_eq!(argv.last().unwrap(), "host");
}

#[test]
fn a_ctrl_c_seen_by_sx_is_an_interrupt_whatever_ssh_returns() {
    #[cfg(windows)]
    use std::os::windows::process::ExitStatusExt;
    #[cfg(unix)]
    use std::os::unix::process::ExitStatusExt;
    let status = |code: u32| std::process::ExitStatus::from_raw(if cfg!(unix) { code << 8 } else { code } as _);
    assert_eq!(finish(true, status(0)), SshOutcome::Interrupted);
    assert_eq!(finish(true, status(130)), SshOutcome::Interrupted);
    assert_eq!(finish(false, status(7)), SshOutcome::Exited(7));
    assert_eq!(finish(false, status(130)), SshOutcome::Exited(130));
}
