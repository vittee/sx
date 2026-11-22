use super::*;

const PAYLOAD: &str = "H4sIAAAAAAAC/wExAM7/AAAA+/=";

#[test]
fn has_no_ssh_token_chars() {
    assert!(!remote_command("web-01", PAYLOAD).contains('%'));
    assert!(!remote_command("we%b'01", PAYLOAD).contains('%'));
}

#[test]
fn single_quotes_are_balanced_whatever_the_host() {
    // a pair each around: the connected printf, the bash -c script, the warning printf
    for host in ["web-01", "me@10.0.0.1", "a'b", "x\"; rm -rf ~ #", ""] {
        assert_eq!(remote_command(host, PAYLOAD).matches('\'').count(), 6, "{host}");
    }
}

#[test]
fn checks_run_before_bash_and_fall_back_to_the_login_shell() {
    let cmd = remote_command("h", PAYLOAD);
    let pos = |s: &str| cmd.find(s).unwrap_or_else(|| panic!("missing {s}"));
    assert!(pos("command -v bash") < pos("exec bash"));
    assert!(pos("base64 -d >/dev/null") < pos("exec bash"));
    assert!(pos("command -v gunzip") < pos("exec bash"));
    assert!(pos("exec bash") < pos("using the plain shell"));
    assert!(cmd.ends_with("exec \"$SHELL\" -l"));
}

/// Dev aid: `SX_DUMP_REMOTE=<file> cargo test dump_remote -- --ignored` writes the command for
/// `h` so it can be run under a real `sh` (see the manual checks in the README of this change).
#[test]
#[ignore]
fn dump_remote() {
    std::fs::write(std::env::var("SX_DUMP_REMOTE").unwrap(), remote_command("web-01", "PAYLOADPLACEHOLDER")).unwrap();
}

#[test]
fn announces_the_connection_with_the_host_before_starting_the_shell() {
    let cmd = remote_command("me@web-01", PAYLOAD);
    let say = cmd.find("connected to me@web-01").unwrap();
    assert!(say < cmd.find("exec bash").unwrap());
    // moves up one line, clears it, then writes the message
    assert!(cmd.starts_with("printf '\\033[1A\\r\\033[2K"));
}

#[test]
fn hostile_hosts_are_reduced_to_safe_characters() {
    assert_eq!(printable("web-01.example.com:22"), "web-01.example.com:22");
    assert_eq!(printable("a'b\"c;d$e\\f%g h"), "abcdefgh");
    assert_eq!(printable("[::1]"), "::1");
}

#[test]
fn the_payload_travels_in_the_command_and_no_environment_variable_is_needed() {
    let cmd = remote_command("h", PAYLOAD);
    assert!(cmd.contains(&format!("P={PAYLOAD}; ")));
    assert!(!cmd.contains('%'));
}

#[test]
fn the_payload_is_handed_to_the_shell_in_sx_rc_for_sx_sudo() {
    let cmd = remote_command("h", PAYLOAD);
    assert!(cmd.contains(&format!("P={PAYLOAD}; export SX_RC=$P; ")));
    assert!(cmd.find("export SX_RC").unwrap() < cmd.find("exec bash --rcfile").unwrap());
}
