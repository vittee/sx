use super::*;

fn host(line: &str) -> Option<String> {
    let args: Vec<String> = line.split_whitespace().map(String::from).collect();
    injectable_host(&args)
}

fn inject(line: &str) -> bool {
    host(line).is_some()
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
fn no_args_skips() {
    assert!(!inject(""));
}

/// Feeds our argv to the real `ssh -G` (prints parsed config, no connection):
/// proves ssh accepts the options and the quoting survives Command.
#[test]
fn real_ssh_parses_our_options() {
    let Ok(out) = Command::new("ssh")
        .arg("-G")
        .args(build_argv(vec!["testhost".into()], "testhost", "UEFZTE9BRA=="))
        .output()
    else {
        return; // ssh not installed
    };
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("setenv LC_MONETARY=UEFZTE9BRA=="), "{text}");
    assert!(text.contains("requesttty force"), "{text}");
    assert!(
        text.lines().any(|l| l == format!("remotecommand {}", bootstrap::remote_command("testhost"))),
        "{text}"
    );
}

#[test]
fn argv_puts_our_options_first() {
    let argv = build_argv(vec!["host".into()], "host", "PAYLOAD");
    assert_eq!(argv[0], "-o");
    assert_eq!(argv[1], "SetEnv=LC_MONETARY=PAYLOAD");
    assert_eq!(argv[3], "RequestTTY=force");
    assert!(argv[5].starts_with("RemoteCommand="));
    assert_eq!(argv.last().unwrap(), "host");
}
