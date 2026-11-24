use super::*;

/// A config with every template added.
fn templated() -> Config {
    let mut cfg = Config::default();
    for (name, env) in env_templates() {
        cfg.add_env(name, env).unwrap();
    }
    cfg
}

#[test]
fn patterns_use_star_and_question_mark_and_ignore_case() {
    assert!(glob_match("*prod*", "web-PROD-1"));
    assert!(glob_match("web-??", "web-01"));
    assert!(!glob_match("web-??", "web-001"));
    assert!(glob_match("*", ""));
    assert!(glob_match("a*b*c", "a--b--c"));
    assert!(!glob_match("a*b*c", "a--b--d"));
    assert!(!glob_match("prod", "production"));
}

#[test]
fn the_template_patterns_suggest_an_environment_from_the_host_name() {
    let cfg = templated();
    assert_eq!(cfg.suggest_env("web-prod-1"), Some("prod"));
    assert_eq!(cfg.suggest_env("me@db-prd"), Some("prod"));
    assert_eq!(cfg.suggest_env("api.stg.example.com"), Some("staging"));
    assert_eq!(cfg.suggest_env("web-dev-1"), Some("dev"));
    assert_eq!(cfg.suggest_env("sit-db"), Some("sit"));
    assert_eq!(cfg.suggest_env("uat1"), Some("uat"));
    // "dev" and "sit" are only whole words in a name, so these are not suggestions
    assert_eq!(cfg.suggest_env("device-7"), None);
    assert_eq!(cfg.suggest_env("website"), None);
}

#[test]
fn colors_are_a_name_a_number_or_a_hex_value() {
    assert_eq!(badge_sgr("red", false).as_deref(), Some("1;97;41"));
    assert_eq!(badge_sgr("Yellow", false).as_deref(), Some("1;30;43"));
    assert_eq!(badge_sgr("208", false).as_deref(), Some("1;30;48;5;208"));
    assert_eq!(badge_sgr("17", false).as_deref(), Some("1;97;48;5;17"));
    assert_eq!(badge_sgr("#ff0000", false).as_deref(), Some("1;97;48;5;196"));
    assert_eq!(badge_sgr("#ffffff", false).as_deref(), Some("1;30;48;5;231"));
    for bad in ["", "pink", "256", "#12345", "#gggggg"] {
        assert_eq!(badge_sgr(bad, false), None, "{bad}");
    }
}

#[test]
fn the_label_is_the_name_in_capitals_and_keeps_only_plain_characters() {
    let mut env = Env::new("red", &[]);
    assert_eq!(env.label_for("prod"), "PROD");
    env.label = "  EU prod  ".to_string();
    assert_eq!(env.label_for("prod"), "EU prod");
    env.label = "a'b\"c\\d$e%f;g`h".to_string();
    assert_eq!(env.label_for("prod"), "abcdefgh");
    env.label = "x".repeat(40);
    assert_eq!(env.label_for("prod").len(), 16);
}

#[test]
fn the_shell_variables_follow_the_features() {
    let mut env = Env::new("red", &[]);
    let both = env.shell_vars("prod", false);
    assert_eq!(both, "__sx_env_badge='\\[\\e[1;97;41m\\] PROD \\[\\e[0m\\] '\n__sx_env_label='PROD'\n");
    env.features = vec![FEATURE_TITLE.to_string()];
    assert_eq!(env.shell_vars("prod", false), "__sx_env_badge=''\n__sx_env_label='PROD'\n");
    env.features = vec![FEATURE_BADGE.to_string()];
    assert!(env.shell_vars("prod", false).ends_with("__sx_env_label=''\n"));
    env.features.clear();
    assert_eq!(env.shell_vars("prod", false), env_shell_vars(None, false));
    env.features = default_features();
    env.color = "no-such-color".to_string();
    assert!(env.shell_vars("prod", false).starts_with("__sx_env_badge=''\n"), "an unknown color shows no badge");
}

#[test]
fn a_host_is_tagged_and_the_tag_cycles_through_the_environments() {
    let mut cfg = templated();
    assert!(cfg.env_of("web1").is_none());
    cfg.set_env("web1", Some("prod"));
    assert_eq!(cfg.env_of("web1").map(|(n, _)| n), Some("prod"));
    cfg.set_env("web1", Some("gone"));
    assert!(cfg.env_of("web1").is_none(), "a tag for an environment that is not defined is ignored");

    cfg.set_env("web2", None);
    let names: Vec<_> = cfg.envs.keys().cloned().collect();
    for name in &names {
        cfg.cycle_env("web2", true);
        assert_eq!(cfg.hosts["web2"].env.as_deref(), Some(name.as_str()));
    }
    cfg.cycle_env("web2", true);
    assert_eq!(cfg.hosts["web2"].env, None, "after the last environment comes none");
    cfg.cycle_env("web2", false);
    assert_eq!(cfg.hosts["web2"].env.as_deref(), names.last().map(String::as_str));
}

#[test]
fn environments_load_from_toml_and_a_new_config_has_none() {
    let cfg: Config = toml::from_str("").unwrap();
    assert!(cfg.envs.is_empty(), "a new config has no environments");

    let cfg: Config = toml::from_str(
        "[envs.qa]\ncolor = \"#00aaff\"\nlabel = \"QA\"\npatterns = [\"qa-*\"]\nfeatures = [\"title\"]\n\n[hosts.qa-1]\nenv = \"qa\"\n",
    )
    .unwrap();
    assert_eq!(cfg.envs.keys().collect::<Vec<_>>(), ["qa"]);
    assert_eq!(cfg.envs["qa"].features, ["title"]);
    assert_eq!(cfg.env_of("qa-1").map(|(n, _)| n), Some("qa"));
    assert_eq!(cfg.suggest_env("qa-9"), Some("qa"));

    let cfg: Config = toml::from_str("[envs.qa]\n").unwrap();
    assert_eq!(cfg.envs["qa"].color, "white");
    assert_eq!(cfg.envs["qa"].features, ["badge", "title"], "a missing field has its default");

    let text = toml::to_string(&Config::default()).unwrap();
    assert_eq!(toml::from_str::<Config>(&text).unwrap(), Config::default());
}

#[test]
fn environments_are_added_copied_renamed_and_deleted_with_their_tags() {
    let mut cfg = Config::default();
    cfg.add_env("qa", Env::new("blue", &["qa-*"])).unwrap();
    assert!(cfg.add_env("qa", Env::new("red", &[])).unwrap_err().contains("exists"));
    assert!(cfg.add_env("bad name", Env::new("red", &[])).is_err());
    assert!(cfg.add_env("", Env::new("red", &[])).is_err());

    cfg.duplicate_env("qa", "qa2").unwrap();
    assert_eq!(cfg.envs["qa2"], cfg.envs["qa"]);
    assert!(cfg.duplicate_env("nope", "x").is_err());

    cfg.set_env("qa-1", Some("qa"));
    cfg.rename_env("qa", "test").unwrap();
    assert!(!cfg.envs.contains_key("qa"));
    assert_eq!(cfg.hosts["qa-1"].env.as_deref(), Some("test"), "the tag follows the rename");
    assert!(cfg.rename_env("test", "qa2").is_err(), "the new name is taken");
    assert!(cfg.envs.contains_key("test"), "a failed rename changes nothing");

    cfg.delete_env("test");
    assert_eq!(cfg.hosts["qa-1"].env, None);
    assert!(cfg.env_of("qa-1").is_none());
}

#[test]
fn a_new_config_has_no_environments_and_cycling_does_nothing() {
    let mut cfg = Config::default();
    assert!(cfg.envs.is_empty());
    cfg.cycle_env("web1", true);
    assert!(cfg.env_of("web1").is_none());
    assert!(env_templates().iter().any(|(n, _)| *n == "prod"));
}

#[test]
fn confirm_is_a_feature_that_is_off_unless_asked_for() {
    let mut cfg = Config::default();
    cfg.add_env("qa", Env::new("blue", &[])).unwrap();
    cfg.add_env("danger", Env { features: vec![FEATURE_CONFIRM.to_string()], label: "LIVE".into(), ..Env::new("red", &[]) }).unwrap();
    cfg.set_env("qa-1", Some("qa"));
    cfg.set_env("web", Some("danger"));
    assert_eq!(cfg.confirm_label("qa-1"), None, "a new environment does not ask");
    assert_eq!(cfg.confirm_label("web").as_deref(), Some("LIVE"));
    assert_eq!(cfg.confirm_label("root@web").as_deref(), Some("LIVE"), "a user in front of the host is ignored");
    assert_eq!(cfg.confirm_label("untagged"), None);
    let prod = env_templates().into_iter().find(|(n, _)| *n == "prod").unwrap().1;
    assert!(prod.has(FEATURE_CONFIRM), "the prod template asks, the others do not");
    assert!(env_templates().iter().filter(|(n, _)| *n != "prod").all(|(_, e)| !e.has(FEATURE_CONFIRM)));
    assert!(!Env::new("red", &[]).has(FEATURE_CONFIRM));
}

#[test]
fn a_hex_color_is_exact_on_a_truecolor_terminal_and_nearest_of_256_otherwise() {
    assert_eq!(badge_sgr("#ff8800", true).as_deref(), Some("1;30;48;2;255;136;0"));
    assert_eq!(badge_sgr("#102030", true).as_deref(), Some("1;97;48;2;16;32;48"));
    assert!(badge_sgr("#ff8800", false).unwrap().contains("48;5;"));
    // a name or a number is the terminal's own color, truecolor or not
    assert_eq!(badge_sgr("red", true).as_deref(), Some("1;97;41"));
    assert_eq!(badge_sgr("208", true).as_deref(), Some("1;30;48;5;208"));
    assert_eq!(badge_sgr("nope", true), None);
    let mut env = Env::new("#00aaff", &[]);
    env.features = default_features();
    assert!(env.shell_vars("qa", true).contains("48;2;0;170;255"));
    assert!(env.shell_vars("qa", false).contains("48;5;"));
}

#[test]
fn the_terminal_is_truecolor_when_it_says_so() {
    let with = |pairs: &'static [(&'static str, &'static str)]| {
        truecolor_from(move |k| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| v.to_string()))
    };
    assert!(!with(&[]));
    assert!(with(&[("COLORTERM", "truecolor")]));
    assert!(with(&[("COLORTERM", "24bit")]));
    assert!(!with(&[("COLORTERM", "yes")]));
    assert!(with(&[("WT_SESSION", "1234")]));
    assert!(!with(&[("COLORTERM", "truecolor"), ("TMUX", "/tmp/tmux-1/default,1,0")]), "a local tmux is not trusted");
    assert!(with(&[("SX_TRUECOLOR", "1")]));
    assert!(!with(&[("SX_TRUECOLOR", "0"), ("COLORTERM", "truecolor")]));
    assert!(with(&[("SX_TRUECOLOR", "1"), ("TMUX", "x")]), "the switch decides");
}
