use super::*;

fn write(dir: &Path, file: &str, text: &str) {
    fs::create_dir_all(snippets_dir(dir)).unwrap();
    fs::write(snippets_dir(dir).join(file), text).unwrap();
}

fn profile(names: &[&str]) -> Profile {
    Profile { snippets: names.iter().map(|n| n.to_string()).collect() }
}

#[test]
fn snippets_are_listed_sorted_and_only_sh_files() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "20-b.sh", "");
    write(tmp.path(), "10-a.sh", "");
    write(tmp.path(), "notes.txt", "");
    assert_eq!(list_snippets(tmp.path()), ["10-a", "20-b"]);
}

#[test]
fn build_rc_sends_the_profiles_snippets_in_name_order_with_headers() {
    let tmp = tempfile::tempdir().unwrap();
    write(tmp.path(), "20-b.sh", "echo b");
    write(tmp.path(), "10-a.sh", "echo a\n");
    write(tmp.path(), "30-c.sh", "echo c\n");
    let mut cfg = Config::default();
    cfg.profiles.insert("p".into(), profile(&["20-b", "10-a"]));
    assert_eq!(build_rc(tmp.path(), &cfg, "p"), "# --- 10-a ---\necho a\n# --- 20-b ---\necho b\n");
}

#[test]
fn unknown_profile_or_no_snippets_gives_an_empty_rc() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cfg = Config::default();
    cfg.profiles.insert("p".into(), profile(&[]));
    assert_eq!(build_rc(tmp.path(), &cfg, "p"), "");
    assert_eq!(build_rc(tmp.path(), &cfg, "missing"), "");
}

#[test]
fn init_creates_the_sample_and_a_default_profile_once() {
    let tmp = tempfile::tempdir().unwrap();
    let (cfg, created) = ensure_initialized(tmp.path()).unwrap();
    assert!(created);
    assert_eq!(cfg.profiles["default"], profile(&["20-aliases"]));
    write(tmp.path(), "20-aliases.sh", "alias x=y\n");
    let (cfg, created) = ensure_initialized(tmp.path()).unwrap();
    assert!(!created);
    assert_eq!(build_rc(tmp.path(), &cfg, "default"), "# --- 20-aliases ---\nalias x=y\n");
}

#[test]
fn config_roundtrips_through_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cfg = Config::default();
    cfg.profiles.insert("work".into(), profile(&["10-a"]));
    cfg.hosts.entry("box".into()).or_default().profile = Some("work".into());
    cfg.save(tmp.path()).unwrap();
    assert_eq!(Config::load(tmp.path()).unwrap(), cfg);
}

#[test]
fn a_bad_config_file_is_an_error_not_a_reset() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("config.toml"), "profiles = [").unwrap();
    assert!(Config::load(tmp.path()).is_err());
}

#[test]
fn profile_for_uses_the_host_choice_then_the_default() {
    let mut cfg = Config::default();
    cfg.profiles.insert("default".into(), profile(&[]));
    cfg.profiles.insert("work".into(), profile(&[]));
    cfg.hosts.entry("a".into()).or_default().profile = Some("work".into());
    cfg.hosts.entry("b".into()).or_default().profile = Some(PROFILE_OFF.into());
    cfg.hosts.entry("c".into()).or_default().profile = Some("gone".into());
    assert_eq!(cfg.profile_for("a"), Some("work"));
    assert_eq!(cfg.profile_for("b"), None);
    assert_eq!(cfg.profile_for("c"), Some("default"));
    assert_eq!(cfg.profile_for("unlisted"), Some("default"));
}

#[test]
fn the_example_in_the_readme_parses() {
    let text = r#"
default_profile = "default"

[profiles.default]
snippets = ["10-prompt", "20-aliases"]

[profiles.work]
snippets = ["20-aliases"]

[hosts.box]
profile = "work"

[hosts.legacy]
profile = "-"       # plain ssh for this host
"#;
    let cfg: Config = toml::from_str(text).unwrap();
    assert_eq!(cfg.profile_for("box"), Some("work"));
    assert_eq!(cfg.profile_for("legacy"), None);
    assert_eq!(cfg.profile_for("other"), Some("default"));
}
