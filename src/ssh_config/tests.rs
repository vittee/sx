use super::*;

fn parse(text: &str) -> Vec<HostEntry> {
    let mut out = Vec::new();
    parse_config(text, Path::new("/nonexistent/.ssh"), 0, &mut out);
    out
}

#[test]
fn parses_hosts_and_settings() {
    let hosts = parse(
        "Host *\n  ServerAliveInterval 30\n\
         Host web db\n  HostName 10.0.0.5\n  User deploy\n  Port=2200\n\
         Host nas\n  hostname nas.lan\n",
    );
    assert_eq!(hosts.len(), 3);
    assert_eq!(hosts[0].alias, "web");
    assert_eq!(hosts[1].alias, "db");
    assert_eq!(hosts[1].target(), "deploy@10.0.0.5:2200");
    assert_eq!(hosts[2].target(), "nas.lan");
}

#[test]
fn skips_wildcards_and_match_blocks() {
    let hosts = parse("Host *.corp !bad ok\n  User a\nMatch host x\n  User b\n");
    assert_eq!(hosts.len(), 1);
    assert_eq!(hosts[0].user.as_deref(), Some("a"));
}

#[test]
fn known_hosts_adds_new_names_only() {
    let mut out = parse("Host web\n  HostName 10.0.0.5\n");
    parse_known_hosts(
        "10.0.0.5 ssh-ed25519 AAAA\n|1|hashed|x ssh-rsa AAAA\nfoo.example,1.2.3.4 ssh-rsa AAAA\n[bar]:2222 ssh-rsa AAAA\n# c\n",
        &mut out,
    );
    let names: Vec<_> = out.iter().map(|h| h.alias.as_str()).collect();
    assert_eq!(names, ["web", "foo.example", "1.2.3.4", "bar"]);
    assert_eq!(out[3].port.as_deref(), Some("2222"));
    assert_eq!(out[1].source, Source::KnownHosts);
}

#[test]
fn include_is_followed() {
    let tmp = tempfile::tempdir().unwrap();
    let ssh = tmp.path().join(".ssh");
    fs::create_dir_all(ssh.join("conf.d")).unwrap();
    fs::write(ssh.join("config"), "Include conf.d/*\nHost main\n").unwrap();
    fs::write(ssh.join("conf.d").join("a"), "Host included\n  User bob\n").unwrap();
    let hosts = load_hosts(tmp.path());
    let names: Vec<_> = hosts.iter().map(|h| h.alias.as_str()).collect();
    assert_eq!(names, ["included", "main"]);
    assert_eq!(hosts[0].user.as_deref(), Some("bob"));
}

#[test]
fn target_with_replaces_the_user_and_port() {
    let host = &parse("Host box\n  HostName 10.0.0.1\n  User me\n  Port 22\n")[0];
    assert_eq!(host.target(), "me@10.0.0.1:22");
    assert_eq!(host.target_with(Some("root"), None), "root@10.0.0.1:22");
    assert_eq!(host.target_with(None, Some("2222")), "me@10.0.0.1:2222");
    let bare = &parse("Host plain\n")[0];
    assert_eq!(bare.target_with(Some("root"), Some("2200")), "root@plain:2200");
}
