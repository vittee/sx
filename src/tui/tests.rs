use super::hosts::{COL_ALIAS, COL_PROFILE, COL_SOURCE, COL_STAR, COL_TARGET, Cols, fuzzy, sort_key_at};
use super::*;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::KeyEventState;

fn host(alias: &str, hostname: &str) -> HostEntry {
    HostEntry {
        alias: alias.into(),
        hostname: Some(hostname.into()),
        user: Some("me".into()),
        port: None,
        source: Source::Config,
    }
}

fn app() -> (tempfile::TempDir, App) {
    app_with(vec![host("alpha", "10.0.0.1"), host("beta", "10.0.0.2"), host("gamma", "10.0.0.3")])
}

fn app_with(hosts: Vec<HostEntry>) -> (tempfile::TempDir, App) {
    let tmp = tempfile::tempdir().unwrap();
    let (cfg, _) = config::ensure_initialized(tmp.path()).unwrap();
    let mut app = App::new(tmp.path().to_path_buf(), cfg, hosts);
    app.ssh_dir = None;
    (tmp, app)
}

fn ctrl(app: &mut App, c: char) {
    app.on_event(Event::Key(KeyEvent {
        code: KeyCode::Char(c),
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }));
}

/// Host aliases in display order.
fn order(app: &App) -> Vec<String> {
    app.visible().iter().map(|&i| app.hosts[i].alias.clone()).collect()
}

/// Clicks the header cell for `key` (x measured from the header's left edge).
fn click_header(app: &mut App, key: SortKey) {
    let head = app.hits.hosts_head;
    let x = match key {
        SortKey::Smart => 0,
        SortKey::Host => COL_STAR + 1,
        SortKey::Target => COL_STAR + COL_ALIAS + 1,
        SortKey::Profile => COL_STAR + COL_ALIAS + COL_TARGET + 1,
        SortKey::Source => COL_STAR + COL_ALIAS + COL_TARGET + COL_PROFILE + 1,
        SortKey::LastUsed => COL_STAR + COL_ALIAS + COL_TARGET + COL_PROFILE + COL_SOURCE + 1,
    };
    click(app, head.x + x, head.y);
}

fn sortable() -> (tempfile::TempDir, App) {
    // by host: amy, Bob, zed   by target: zed, Bob, amy
    app_with(vec![host("zed", "10.0.0.1"), host("amy", "10.0.0.9"), host("Bob", "10.0.0.5")])
}

fn render(app: &mut App) -> String {
    let mut term = Terminal::new(TestBackend::new(110, 30)).unwrap();
    term.draw(|f| app.draw(f)).unwrap();
    let buf = term.backend().buffer().clone();
    (0..buf.area.height)
        .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn key(app: &mut App, code: KeyCode) {
    app.on_event(Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }));
}

/// The screen with every run of spaces made one, to compare text that is padded into columns.
fn squash(screen: &str) -> String {
    screen.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// An app whose `~/.ssh` holds two private keys and some files that are not.
fn app_with_keys() -> (tempfile::TempDir, tempfile::TempDir, App) {
    let (t, mut a) = app();
    let ssh = tempfile::tempdir().unwrap();
    let w = |n: &str, c: &str| std::fs::write(ssh.path().join(n), c).unwrap();
    w("id_ed25519", "-----BEGIN OPENSSH PRIVATE KEY-----
");
    w("id_ed25519.pub", "ssh-ed25519 AAAA");
    w("work_key", "-----BEGIN RSA PRIVATE KEY-----
");
    w("legacy", "binary");
    w("legacy.pub", "ssh-rsa AAAA");
    w("config", "Host *");
    w("known_hosts", "x");
    a.ssh_dir = Some(ssh.path().to_path_buf());
    (t, ssh, a)
}

#[test]
fn ctrl_k_lists_the_private_keys_and_enter_takes_one() {
    let (_t, _s, mut a) = app_with_keys();
    ctrl(&mut a, 'o');
    key(&mut a, KeyCode::Down);
    key(&mut a, KeyCode::Down); // focus the identity field
    ctrl(&mut a, 'k');
    let screen = render(&mut a);
    for k in ["~/.ssh/id_ed25519", "~/.ssh/legacy", "~/.ssh/work_key"] {
        assert!(screen.contains(k), "missing {k}
{screen}");
    }
    for k in ["known_hosts", ".pub
", "~/.ssh/config"] {
        assert!(!screen.contains(k), "unexpected {k}
{screen}");
    }
    key(&mut a, KeyCode::Down);
    key(&mut a, KeyCode::Enter);
    assert_eq!(a.identity_value().as_deref(), Some("~/.ssh/legacy"));
    assert!(!render(&mut a).contains("work_key"), "list should be closed");
}

#[test]
fn typing_narrows_the_key_list_and_esc_closes_only_the_list() {
    let (_t, _s, mut a) = app_with_keys();
    ctrl(&mut a, 'o');
    key(&mut a, KeyCode::Down);
    key(&mut a, KeyCode::Down); // focus the identity field
    typed(&mut a, "wor");
    ctrl(&mut a, 'k');
    let screen = render(&mut a);
    assert!(screen.contains("work_key") && !screen.contains("~/.ssh/legacy"), "{screen}");
    key(&mut a, KeyCode::Esc);
    assert!(matches!(a.mode, Mode::HostForm(_)), "Esc closed the whole form");
    assert_eq!(a.identity_value().as_deref(), Some("wor"));
    // With the list closed, Esc leaves the form as before.
    key(&mut a, KeyCode::Esc);
    key(&mut a, KeyCode::Char('y'));
    assert!(matches!(a.mode, Mode::Normal));
}

#[test]
fn clicking_a_key_in_the_list_takes_it() {
    let (_t, _s, mut a) = app_with_keys();
    ctrl(&mut a, 'o');
    key(&mut a, KeyCode::Down);
    key(&mut a, KeyCode::Down); // focus the identity field
    ctrl(&mut a, 'k');
    render(&mut a);
    let (r, name) = a.hits.form_keylist[2].clone();
    click(&mut a, r.x + 1, r.y);
    assert_eq!(a.identity_value(), Some(name));
}

#[test]
fn ctrl_k_without_keys_says_so() {
    let (_t, mut a) = app();
    ctrl(&mut a, 'o');
    key(&mut a, KeyCode::Down);
    key(&mut a, KeyCode::Down); // focus the identity field
    ctrl(&mut a, 'k');
    assert!(matches!(&a.status, Some((m, true)) if m.contains("no private keys")), "{:?}", a.status);
}

#[test]
fn the_key_list_belongs_to_the_identity_field() {
    let (_t, _s, mut a) = app_with_keys();
    ctrl(&mut a, 'o');
    assert!(!render(&mut a).contains("^K"), "hint shown on the user field");
    ctrl(&mut a, 'k');
    assert!(a.hits.form_keylist.is_empty() && !render(&mut a).contains("work_key"), "^K worked on the user field");
    key(&mut a, KeyCode::Down);
    key(&mut a, KeyCode::Down);
    assert!(squash(&render(&mut a)).contains("^K pick key file"));
    key(&mut a, KeyCode::Down); // on to the jump host: the hint goes away again
    assert!(!render(&mut a).contains("^K"));
}

#[test]
fn the_hints_do_not_move_when_the_key_hint_appears() {
    let (_t, _s, mut a) = app_with_keys();
    ctrl(&mut a, 'o');
    let col = |screen: &str| screen.lines().find(|l| l.contains("delete")).and_then(|l| l.find("^D"));
    let before = col(&render(&mut a));
    key(&mut a, KeyCode::Down);
    key(&mut a, KeyCode::Down);
    let on_identity = render(&mut a);
    assert!(on_identity.contains("^K"));
    assert!(before.is_some() && col(&on_identity) == before, "^D moved: {before:?} -> {:?}", col(&on_identity));
}

#[test]
fn the_cursor_follows_a_host_that_moves_after_a_connection() {
    let (_t, mut a) = app();
    a.cfg.sort = SortKey::LastUsed;
    key(&mut a, KeyCode::Down);
    key(&mut a, KeyCode::Down); // gamma, the last row
    let alias = |a: &App| a.selected_host().map(|i| a.hosts[i].alias.clone());
    assert_eq!(alias(&a).as_deref(), Some("gamma"));
    a.note_connected("gamma");
    assert_eq!(a.visible().first().map(|&i| a.hosts[i].alias.as_str()), Some("gamma"), "gamma should be on top");
    assert_eq!(alias(&a).as_deref(), Some("gamma"));
    assert_eq!(a.host_state.selected(), Some(0));
}

#[test]
fn columns_grow_with_long_names_when_the_terminal_is_wide() {
    let long = "a-very-long-host-name-for-a-production-database-server";
    let (_t, mut a) = app_with(vec![host(long, "10.0.0.1"), host("beta", "10.0.0.2")]);
    let mut term = Terminal::new(TestBackend::new(200, 30)).unwrap();
    term.draw(|f| a.draw(f)).unwrap();
    let buf = term.backend().buffer().clone();
    let screen: String = (0..buf.area.height)
        .map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(screen.contains(long), "wide terminal should show the whole name\n{screen}");
    assert!(a.hits.cols.alias > COL_ALIAS && a.hits.cols.alias >= long.len() as u16);
    // At 110 columns only the spare room is shared, so the name is cut but the host column still grows.
    let (_t2, mut b) = app_with(vec![host(long, "10.0.0.1")]);
    render(&mut b);
    assert!(b.hits.cols.alias > COL_ALIAS && b.hits.cols.alias < long.len() as u16);
    // Nothing is taken from the narrow terminal: it keeps the smallest widths.
    assert_eq!(Cols::fit(60, [60, 60, 60]), Cols::default());
}

#[test]
fn spare_width_is_shared_in_proportion_to_what_each_column_lacks() {
    let min = Cols::default();
    // Room for 15 more characters only: the host column lacks 20, the target 10, the profile nothing.
    let width = COL_STAR + COL_SOURCE + 12 + min.alias + min.target + min.profile + 15;
    let c = Cols::fit(width, [min.alias + 20, min.target + 10, 0]);
    assert!(c.alias > min.alias && c.target > min.target && c.profile == min.profile, "{c:?}");
    assert!(c.alias + c.target + c.profile <= min.alias + min.target + min.profile + 15);
    assert!(c.alias - min.alias > c.target - min.target);
    // Plenty of room: every column fits its longest entry, and no more.
    let c = Cols::fit(300, [min.alias + 20, min.target + 10, 0]);
    assert_eq!((c.alias, c.target, c.profile), (min.alias + 20, min.target + 10, min.profile));
}

#[test]
fn title_shows_the_version() {
    let (_t, mut a) = app();
    let screen = render(&mut a);
    assert!(screen.contains(&format!(" sx {} ", env!("CARGO_PKG_VERSION"))), "{screen}");
}

fn typed(app: &mut App, s: &str) {
    s.chars().for_each(|c| key(app, KeyCode::Char(c)));
}

fn click(app: &mut App, x: u16, y: u16) {
    app.on_event(Event::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    }));
}

fn wheel(app: &mut App, x: u16, y: u16, up: bool) {
    app.on_event(Event::Mouse(MouseEvent {
        kind: if up { MouseEventKind::ScrollUp } else { MouseEventKind::ScrollDown },
        column: x,
        row: y,
        modifiers: KeyModifiers::NONE,
    }));
}

#[test]
fn hosts_tab_lists_hosts_and_columns() {
    let (_t, mut app) = app();
    let screen = render(&mut app);
    for s in ["Hosts", "Snippets", "Profiles", "alpha", "me@10.0.0.2", "default", "3 hosts"] {
        assert!(screen.contains(s), "missing {s:?}\n{screen}");
    }
}

#[test]
fn search_filters_and_enter_connects() {
    let (_t, mut app) = app();
    typed(&mut app, "bet");
    render(&mut app);
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.pending_connect, Some(vec!["beta".to_string()]));
}

#[test]
fn no_match_connects_to_typed_args() {
    let (_t, mut app) = app();
    typed(&mut app, "-p 2200 me@new.host");
    render(&mut app);
    assert!(render(&mut app).contains("No match"));
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.pending_connect, Some(vec!["-p".into(), "2200".into(), "me@new.host".into()]));
}

#[test]
fn click_selects_double_click_connects() {
    let (_t, mut app) = app();
    render(&mut app);
    let list = app.hits.hosts_list;
    let x = list.x + COL_STAR + 3; // on the host column
    click(&mut app, x, list.y + 1);
    assert_eq!(app.host_state.selected(), Some(1));
    assert!(app.pending_connect.is_none());
    click(&mut app, x, list.y + 1);
    assert_eq!(app.pending_connect, Some(vec!["beta".to_string()]));
}

#[test]
fn click_star_toggles_favorite_and_sorts_first() {
    let (_t, mut app) = app();
    render(&mut app);
    let list = app.hits.hosts_list;
    click(&mut app, list.x, list.y + 2); // star column of gamma
    assert!(app.cfg.hosts["gamma"].favorite);
    let screen = render(&mut app);
    assert!(screen.contains('★'));
    assert_eq!(app.hosts[app.visible()[0]].alias, "gamma");
}

#[test]
fn click_profile_column_and_ctrl_p_cycle_profile() {
    let (_t, mut app) = app();
    app.cfg.add_profile("work").unwrap();
    render(&mut app);
    let list = app.hits.hosts_list;
    click(&mut app, list.x + COL_STAR + COL_ALIAS + COL_TARGET + 1, list.y);
    assert_eq!(app.cfg.profile_for("alpha"), Some("work"));
    key(&mut app, KeyCode::Right); // an arrow key must not change the profile
    key(&mut app, KeyCode::Left);
    assert_eq!(app.cfg.profile_for("alpha"), Some("work"));
    ctrl(&mut app, 'p'); // work -> off
    assert_eq!(app.cfg.profile_for("alpha"), None);
    assert!(render(&mut app).contains("off"));
    // persisted
    assert_eq!(Config::load(&app.dir).unwrap().profile_for("alpha"), None);
}

#[test]
fn wheel_moves_selection() {
    let (_t, mut app) = app();
    render(&mut app);
    wheel(&mut app, 5, 10, false);
    wheel(&mut app, 5, 10, false);
    assert_eq!(app.host_state.selected(), Some(2));
    wheel(&mut app, 5, 10, true);
    assert_eq!(app.host_state.selected(), Some(1));
}

#[test]
fn invert_scroll_reverses_the_wheel_and_is_saved() {
    let (_t, mut app) = app();
    app.cfg.invert_scroll = true;
    app.cfg.save(&app.dir).unwrap();
    render(&mut app);
    wheel(&mut app, 5, 10, true); // up now moves the selection down
    wheel(&mut app, 5, 10, true);
    assert_eq!(app.host_state.selected(), Some(2));
    wheel(&mut app, 5, 10, false);
    assert_eq!(app.host_state.selected(), Some(1));
    assert!(Config::load(&app.dir).unwrap().invert_scroll);
}

#[test]
fn clicking_tabs_switches_screens() {
    let (_t, mut app) = app();
    render(&mut app);
    let r = app.hits.tabs[1];
    click(&mut app, r.x + 1, r.y);
    assert_eq!(app.tab, Tab::Snippets);
    let screen = render(&mut app);
    assert!(screen.contains("20-aliases.sh"), "{screen}");
    let r = app.hits.tabs[2];
    click(&mut app, r.x + 1, r.y);
    assert_eq!(app.tab, Tab::Profiles);
    let screen = render(&mut app);
    assert!(screen.contains("payload ") && screen.contains("Prompt"), "{screen}");
}

#[test]
fn tab_key_cycles_tabs() {
    let (_t, mut app) = app();
    key(&mut app, KeyCode::Tab);
    key(&mut app, KeyCode::Tab);
    assert_eq!(app.tab, Tab::Profiles);
    key(&mut app, KeyCode::Tab);
    assert_eq!(app.tab, Tab::Hosts);
    key(&mut app, KeyCode::BackTab);
    assert_eq!(app.tab, Tab::Profiles);
}

#[test]
fn editing_a_snippet_autosaves_on_esc() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Snippets);
    render(&mut app);
    key(&mut app, KeyCode::Enter); // focus editor on 20-aliases
    assert!(app.editor_focus);
    typed(&mut app, "alias zz='ls'");
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Esc);
    assert!(!app.editor_focus);
    let saved = config::read_snippet(&app.dir, "20-aliases").unwrap();
    assert!(saved.starts_with("alias zz='ls'\n"), "{saved}");
}

#[test]
fn clicking_in_editor_focuses_and_places_cursor() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Snippets);
    render(&mut app);
    let ed = app.hits.editor;
    click(&mut app, ed.x + 4, ed.y);
    assert!(app.editor_focus);
    assert_eq!((app.editor.row, app.editor.col), (0, 4));
}

#[test]
fn new_snippet_flow_and_validation() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Snippets);
    ctrl(&mut app, 'n');
    typed(&mut app, "30-git");
    key(&mut app, KeyCode::Enter);
    assert!(app.snippets.contains(&"30-git".to_string()));
    assert_eq!(app.editor_name.as_deref(), Some("30-git"));
    key(&mut app, KeyCode::Esc);
    ctrl(&mut app, 'n');
    typed(&mut app, "bad name");
    key(&mut app, KeyCode::Enter);
    assert!(app.status.as_ref().is_some_and(|s| s.1), "{:?}", app.status);
}

#[test]
fn delete_snippet_needs_confirmation_and_cleans_profiles() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Snippets);
    ctrl(&mut app, 'd');
    key(&mut app, KeyCode::Char('n')); // cancel
    assert_eq!(app.snippets, ["20-aliases", "cmd-colors", "colorterm", "completion-colors.inputrc", "dir-keys", "history-search.inputrc", "keys-fix.inputrc", "man-colors", "menu-complete.inputrc", "no-bell.inputrc", "no-history", "sudo-esc.inputrc", "utf8", "vim-basic.vim", "vim-dark.vim", "vim-indent-2.vim", "vim-indent-4.vim", "vim-indent-tabs.vim", "vim-keys.vim", "vim-mouse.vim", "vim-no-traces.vim", "vim-recommended.vim"]);
    ctrl(&mut app, 'd');
    key(&mut app, KeyCode::Char('y'));
    assert_eq!(app.snippets, ["cmd-colors", "colorterm", "completion-colors.inputrc", "dir-keys", "history-search.inputrc", "keys-fix.inputrc", "man-colors", "menu-complete.inputrc", "no-bell.inputrc", "no-history", "sudo-esc.inputrc", "utf8", "vim-basic.vim", "vim-dark.vim", "vim-indent-2.vim", "vim-indent-4.vim", "vim-indent-tabs.vim", "vim-keys.vim", "vim-mouse.vim", "vim-no-traces.vim", "vim-recommended.vim"]);
    assert!(app.cfg.profiles["default"].snippets.is_empty());
}

#[test]
fn duplicate_snippet_prompts_with_a_copy_name_and_copies_the_text() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Snippets);
    ctrl(&mut app, 'y');
    assert!(matches!(&app.mode, Mode::Input(InputKind::DuplicateSnippet(f), b) if f == "20-aliases" && b == "20-aliases-copy"));
    assert!(render(&mut app).contains("copy of snippet 20-aliases as: 20-aliases-copy"));
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.snippets, ["20-aliases", "20-aliases-copy", "cmd-colors", "colorterm", "completion-colors.inputrc", "dir-keys", "history-search.inputrc", "keys-fix.inputrc", "man-colors", "menu-complete.inputrc", "no-bell.inputrc", "no-history", "sudo-esc.inputrc", "utf8", "vim-basic.vim", "vim-dark.vim", "vim-indent-2.vim", "vim-indent-4.vim", "vim-indent-tabs.vim", "vim-keys.vim", "vim-mouse.vim", "vim-no-traces.vim", "vim-recommended.vim"]);
    assert_eq!(app.editor_name.as_deref(), Some("20-aliases-copy"));
    assert_eq!(
        config::read_snippet(&app.dir, "20-aliases-copy").unwrap(),
        config::read_snippet(&app.dir, "20-aliases").unwrap()
    );
    // not added to any profile automatically
    assert_eq!(app.cfg.profiles["default"].snippets, ["20-aliases"]);
}

#[test]
fn duplicate_snippet_name_can_be_edited_and_copies_unsaved_edits() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Snippets);
    key(&mut app, KeyCode::Enter); // edit
    typed(&mut app, "# edited\n");
    key(&mut app, KeyCode::Enter);
    key(&mut app, KeyCode::Esc); // autosave, back to the list
    ctrl(&mut app, 'y');
    for _ in 0.."-copy".len() {
        key(&mut app, KeyCode::Backspace);
    }
    typed(&mut app, "-v2");
    key(&mut app, KeyCode::Enter);
    let copy = config::read_snippet(&app.dir, "20-aliases-v2").unwrap();
    assert!(copy.contains("# edited"), "{copy}");
}

#[test]
fn duplicate_snippet_rejects_existing_or_invalid_names_and_can_be_cancelled() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Snippets);
    ctrl(&mut app, 'y');
    for _ in 0.."-copy".len() {
        key(&mut app, KeyCode::Backspace);
    }
    key(&mut app, KeyCode::Enter); // same name as the source
    assert!(app.status.as_ref().is_some_and(|s| s.1), "{:?}", app.status);
    ctrl(&mut app, 'y');
    typed(&mut app, " bad");
    key(&mut app, KeyCode::Enter);
    assert!(app.status.as_ref().is_some_and(|s| s.1), "{:?}", app.status);
    ctrl(&mut app, 'y');
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.snippets, ["20-aliases", "cmd-colors", "colorterm", "completion-colors.inputrc", "dir-keys", "history-search.inputrc", "keys-fix.inputrc", "man-colors", "menu-complete.inputrc", "no-bell.inputrc", "no-history", "sudo-esc.inputrc", "utf8", "vim-basic.vim", "vim-dark.vim", "vim-indent-2.vim", "vim-indent-4.vim", "vim-indent-tabs.vim", "vim-keys.vim", "vim-mouse.vim", "vim-no-traces.vim", "vim-recommended.vim"]);
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn duplicate_profile_copies_snippets_and_prompt_and_selects_the_copy() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    ctrl(&mut app, 'y');
    assert!(matches!(&app.mode, Mode::Input(InputKind::DuplicateProfile(f), b) if f == "default" && b == "default-copy"));
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.cfg.profiles["default-copy"], app.cfg.profiles["default"]);
    assert_eq!(app.selected_profile().as_deref(), Some("default-copy"));
    assert_eq!(app.cfg.default_profile, "default"); // the copy is not made default
    assert_eq!(Config::load(&app.dir).unwrap(), app.cfg);
    let screen = render(&mut app);
    assert!(screen.contains("payload ") && screen.contains("(•) fino"), "{screen}");
}

#[test]
fn duplicate_profile_rejects_existing_name() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    ctrl(&mut app, 'y');
    for _ in 0.."-copy".len() {
        key(&mut app, KeyCode::Backspace);
    }
    key(&mut app, KeyCode::Enter);
    assert!(app.status.as_ref().is_some_and(|s| s.1), "{:?}", app.status);
    assert_eq!(app.cfg.profiles.len(), 1);
}

#[test]
fn copy_key_does_nothing_while_typing_in_the_editor_or_picker() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Snippets);
    key(&mut app, KeyCode::Enter); // editor focus
    key(&mut app, KeyCode::Char('c'));
    assert!(matches!(app.mode, Mode::Normal));
    assert!(app.editor.text().starts_with('c'));
    app.switch_tab(Tab::Profiles);
    key(&mut app, KeyCode::Right); // prompt picker focus
    key(&mut app, KeyCode::Char('c'));
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn snippets_tab_has_no_prompt_gallery_any_more() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Snippets);
    key(&mut app, KeyCode::Char('g'));
    assert!(matches!(app.mode, Mode::Normal));
    assert!(!render(&mut app).contains("gallery"));
}

#[test]
fn profiles_toggle_snippets_with_keyboard_and_mouse() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    render(&mut app);
    let c = app.hits.chk_list;
    click(&mut app, c.x + 1, c.y); // 20-aliases off
    assert!(app.cfg.profiles["default"].snippets.is_empty());
    assert_eq!(app.prof_focus, ProfFocus::Snippets);
    key(&mut app, KeyCode::Char(' ')); // toggles again (snippets focus)
    assert_eq!(app.cfg.profiles["default"].snippets, ["20-aliases"]);
    assert_eq!(Config::load(&app.dir).unwrap(), app.cfg);
}

fn prompt_of(app: &App, profile: &str) -> Option<String> {
    app.cfg.profiles[profile].prompt.clone()
}

#[test]
fn prompt_picker_lists_themes_with_preview() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    let screen = render(&mut app);
    for s in ["Prompt","(server's own)", "minimal", "two-line", "timestamp", "(•) fino", "you at server"] {
        assert!(screen.contains(s), "missing {s:?}\n{screen}");
    }
}

#[test]
fn arrow_keys_choose_and_save_a_prompt() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    key(&mut app, KeyCode::Right); // focus the prompt picker
    assert_eq!(app.prof_focus, ProfFocus::Prompt);
    let at = prompt::THEMES.iter().position(|t| t.0 == prompt::DEFAULT_THEME).unwrap();
    let next = prompt::THEMES[at + 1].0;
    key(&mut app, KeyCode::Down); // the default theme -> the next one: only a preview
    assert_eq!(prompt_of(&app, "default").as_deref(), Some(prompt::DEFAULT_THEME), "moving does not apply");
    let screen = render(&mut app);
    assert!(screen.contains(&format!("(•) {}", prompt::DEFAULT_THEME)) && screen.contains("Enter to use it"), "{screen}");
    assert_eq!(Config::load(&app.dir).unwrap(), app.cfg);
    key(&mut app, KeyCode::Enter); // now it is saved
    assert_eq!(prompt_of(&app, "default").as_deref(), Some(next));
    assert!(render(&mut app).contains(&format!("(•) {next}")));
    assert_eq!(Config::load(&app.dir).unwrap(), app.cfg);
    for _ in 0..20 {
        key(&mut app, KeyCode::Down); // stops at the last theme
    }
    assert_eq!(prompt_of(&app, "default").as_deref(), Some(next));
    key(&mut app, KeyCode::Enter);
    assert_eq!(prompt_of(&app, "default").as_deref(), Some(prompt::THEMES.last().unwrap().0));
    for _ in 0..20 {
        key(&mut app, KeyCode::Up); // stops at "(server's own)"
    }
    assert!(render(&mut app).contains("the server keeps its own prompt"), "the preview follows the cursor");
    assert!(prompt_of(&app, "default").is_some());
    key(&mut app, KeyCode::Enter);
    assert_eq!(prompt_of(&app, "default"), None);
}

#[test]
fn clicking_a_theme_picks_it_and_wheel_moves_it() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    render(&mut app);
    let l = app.hits.prompt_list;
    click(&mut app, l.x + 8, l.y + 1); // the name of the first theme: a preview only
    assert_eq!(prompt_of(&app, "default").as_deref(), Some(prompt::DEFAULT_THEME));
    assert_eq!(app.prompt_cursor_row(), 1);
    assert_eq!(app.prof_focus, ProfFocus::Prompt);
    wheel(&mut app, l.x + 8, l.y + 1, false);
    assert_eq!(app.prompt_cursor_row(), 2);
    assert_eq!(prompt_of(&app, "default").as_deref(), Some(prompt::DEFAULT_THEME));
    click(&mut app, l.x + 1, l.y + 1); // the radio mark applies the row
    assert_eq!(prompt_of(&app, "default").as_deref(), Some("minimal"));
    click(&mut app, l.x + 1, l.y); // "(server's own)"
    assert_eq!(prompt_of(&app, "default"), None);
}

#[test]
fn each_profile_keeps_its_own_prompt() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    ctrl(&mut app, 'n');
    typed(&mut app, "work");
    key(&mut app, KeyCode::Enter); // selects the new profile, which has no prompt yet
    assert_eq!(prompt_of(&app, "work"), None);
    key(&mut app, KeyCode::Right);
    key(&mut app, KeyCode::Down); // minimal
    key(&mut app, KeyCode::Down); // green
    key(&mut app, KeyCode::Enter);
    assert_eq!(prompt_of(&app, "work").as_deref(), Some("green"));
    assert_eq!(prompt_of(&app, "default").as_deref(), Some(prompt::DEFAULT_THEME));
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Enter);
    assert_eq!(prompt_of(&app, "work").as_deref(), Some("two-line"));
    assert_eq!(prompt_of(&app, "default").as_deref(), Some(prompt::DEFAULT_THEME)); // untouched
}

#[test]
fn prompt_choice_changes_the_payload() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    let with_theme = config::build_rc(&app.dir, &app.cfg, "default");
    assert!(with_theme.contains("export PS1="));
    key(&mut app, KeyCode::Right);
    for _ in 0..10 {
        key(&mut app, KeyCode::Up);
    }
    assert!(config::build_rc(&app.dir, &app.cfg, "default").contains("export PS1="), "moving does not apply");
    key(&mut app, KeyCode::Enter);
    assert!(!config::build_rc(&app.dir, &app.cfg, "default").contains("export PS1="));
}

#[test]
fn focus_moves_between_panes_with_left_right_esc() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    assert_eq!(app.prof_focus, ProfFocus::List);
    key(&mut app, KeyCode::Right);
    assert_eq!(app.prof_focus, ProfFocus::Prompt);
    key(&mut app, KeyCode::Right);
    assert_eq!(app.prof_focus, ProfFocus::Snippets);
    key(&mut app, KeyCode::Left);
    assert_eq!(app.prof_focus, ProfFocus::Prompt);
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.prof_focus, ProfFocus::List);
}

#[test]
fn profile_create_default_delete() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    ctrl(&mut app, 'n');
    typed(&mut app, "work");
    key(&mut app, KeyCode::Enter);
    assert!(app.cfg.profiles.contains_key("work"));
    ctrl(&mut app, 'g');
    assert_eq!(app.cfg.default_profile, "work");
    assert!(render(&mut app).contains("work *"));
    ctrl(&mut app, 'd');
    key(&mut app, KeyCode::Char('y'));
    assert!(!app.cfg.profiles.contains_key("work"));
    assert_eq!(app.cfg.default_profile, "default");
}

#[test]
fn payload_size_is_a_compact_readout_not_a_bar() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    let screen = render(&mut app);
    let size = app.payload_size("default").unwrap();
    assert!(size > 0);
    assert!(screen.contains(&format!("payload {size}/{} B", payload::WARN_ENCODED_LEN)), "{screen}");
    assert!(!screen.contains("Payload size"), "{screen}");
}

fn many_snippets(app: &mut App, n: usize) {
    app.snippets = (0..n).map(|i| format!("snip-{i:02}")).collect();
}

#[test]
fn profile_snippets_fill_several_columns() {
    let (_t, mut app) = app();
    many_snippets(&mut app, 6);
    app.switch_tab(Tab::Profiles);
    render(&mut app);
    assert!(app.hits.chk_rows < 6, "rows {}", app.hits.chk_rows);
    let screen = render(&mut app);
    let line = screen.lines().find(|l| l.contains("snip-00")).unwrap();
    assert!(line.contains(&format!("snip-{:02}", app.hits.chk_rows)), "the second column starts after the first: {line}");
}

/// 12 snippets on the Profiles tab with the focus on the snippet grid, and the rows of one column.
fn grid_app() -> (tempfile::TempDir, App, usize) {
    let (t, mut app) = app();
    many_snippets(&mut app, 12);
    app.switch_tab(Tab::Profiles);
    render(&mut app);
    key(&mut app, KeyCode::Right);
    key(&mut app, KeyCode::Right); // focus snippets
    assert_eq!(app.prof_focus, ProfFocus::Snippets);
    let rows = app.hits.chk_rows;
    assert!((2..12).contains(&rows), "several columns of several rows: {rows}");
    assert_eq!(rows, app.hits.chk_list.height as usize, "a column is as tall as the pane");
    (t, app, rows)
}

#[test]
fn the_snippet_grid_runs_down_each_column_and_on_to_the_next() {
    let (_t, mut app, rows) = grid_app();
    key(&mut app, KeyCode::Down);
    assert_eq!(app.chk_state.selected(), Some(1));
    key(&mut app, KeyCode::Up);
    key(&mut app, KeyCode::Up); // the very top stays
    assert_eq!(app.chk_state.selected(), Some(0));
    for _ in 0..rows {
        key(&mut app, KeyCode::Down); // past the bottom of the column: the top of the next one
    }
    assert_eq!(app.chk_state.selected(), Some(rows));
    key(&mut app, KeyCode::Up); // past the top: the bottom of the previous column
    assert_eq!(app.chk_state.selected(), Some(rows - 1));
    for _ in 0..30 {
        key(&mut app, KeyCode::Down); // the last item stays
    }
    assert_eq!(app.chk_state.selected(), Some(11));
}

#[test]
fn left_and_right_change_the_column_and_keep_the_row() {
    let (_t, mut app, rows) = grid_app();
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Right);
    assert_eq!(app.chk_state.selected(), Some(rows + 1));
    key(&mut app, KeyCode::Left);
    assert_eq!(app.chk_state.selected(), Some(1));
    key(&mut app, KeyCode::Left); // the first column: back to the prompt list
    assert_eq!(app.prof_focus, ProfFocus::Prompt);
    // the last column can be shorter: Right goes to its last item, and then stays
    key(&mut app, KeyCode::Right);
    app.chk_state.select(Some(rows - 1));
    for _ in 0..12 {
        key(&mut app, KeyCode::Right);
    }
    assert_eq!(app.chk_state.selected(), Some(11));
}

#[test]
fn only_the_checkbox_toggles_a_snippet_and_the_name_selects_it() {
    let (_t, mut app, rows) = grid_app();
    let c = app.hits.chk_list;
    let w = app.hits.chk_colw;
    let on = |app: &App, n: &str| app.cfg.profiles["default"].snippets.contains(&n.to_string());
    click(&mut app, c.x + w + 6, c.y); // the name, first row of the second column
    assert_eq!(app.chk_state.selected(), Some(rows));
    assert!(!on(&app, &format!("snip-{rows:02}")), "the name does not toggle");
    click(&mut app, c.x + w + 1, c.y); // its checkbox
    assert!(on(&app, &format!("snip-{rows:02}")));
    click(&mut app, c.x + w + 6, c.y + 1);
    assert_eq!(app.chk_state.selected(), Some(rows + 1));
    assert!(!on(&app, &format!("snip-{:02}", rows + 1)));
}

#[test]
fn the_wheel_moves_one_snippet_and_crosses_to_the_next_column() {
    let (_t, mut app, rows) = grid_app();
    let c = app.hits.chk_list;
    app.chk_state.select(Some(rows - 1));
    wheel(&mut app, c.x + 3, c.y, false);
    assert_eq!(app.chk_state.selected(), Some(rows), "down from the bottom: the top of the next column");
    wheel(&mut app, c.x + 3, c.y, true);
    assert_eq!(app.chk_state.selected(), Some(rows - 1), "and back");
}

#[test]
fn f2_toggles_mouse_capture_and_the_footer_says_how() {
    let (_t, mut app) = app();
    assert!(app.mouse);
    assert!(render(&mut app).contains("F2 select text"));
    key(&mut app, KeyCode::F(2));
    assert!(!app.mouse);
    let screen = render(&mut app);
    assert!(screen.contains("mouse off") && screen.contains("F2 mouse"), "{screen}");
    key(&mut app, KeyCode::F(2));
    assert!(app.mouse);
}

#[test]
fn f2_works_in_every_tab_and_while_typing() {
    let (_t, mut app) = app();
    for tab in [Tab::Hosts, Tab::Snippets, Tab::Profiles] {
        app.switch_tab(tab);
        key(&mut app, KeyCode::F(2));
        key(&mut app, KeyCode::F(2));
        assert!(app.mouse, "{tab:?}");
    }
    app.switch_tab(Tab::Snippets);
    ctrl(&mut app, 'n');
    key(&mut app, KeyCode::F(2));
    assert!(!app.mouse);
    assert!(matches!(app.mode, Mode::Input(..)), "typing is not interrupted");
}

#[test]
fn history_pane_lists_the_selected_hosts_recent_sessions() {
    let (_t, mut app) = app();
    let now = config::now_secs();
    for (host, when, secs, outcome) in
        [("alpha", now - 7200, 185, "ok"), ("beta", now - 60, 9, "failed"), ("alpha", now - 30, 12, "exit 2")]
    {
        let e = history::Entry { when, host: host.into(), secs, outcome: outcome.into() };
        history::append(&app.dir, &e).unwrap();
    }
    let screen = render(&mut app); // alpha is first in the list
    assert!(screen.contains("History: alpha (2 sessions)"), "{screen}");
    assert!(screen.contains("3m 05s") && screen.contains("exit 2"), "{screen}");
    assert!(!screen.contains("failed"), "beta's session is not shown for alpha: {screen}");
    // newest first
    assert!(screen.find("exit 2").unwrap() < screen.find("3m 05s").unwrap(), "{screen}");
    key(&mut app, KeyCode::Down);
    let screen = render(&mut app);
    assert!(screen.contains("History: beta (1 session)") && screen.contains("failed"), "{screen}");
    key(&mut app, KeyCode::Down);
    assert!(render(&mut app).contains("no sessions yet"));
}

#[test]
fn history_pane_hides_on_a_short_terminal() {
    let (_t, mut app) = app();
    let mut term = Terminal::new(TestBackend::new(110, 16)).unwrap();
    term.draw(|f| app.draw(f)).unwrap();
    let text: String = term.backend().buffer().content().iter().map(|c| c.symbol()).collect();
    assert!(!text.contains("History"), "no room for it");
}

#[test]
fn ctrl_e_picks_extra_snippets_for_one_host() {
    let (_t, mut app) = app();
    config::write_snippet(&app.dir, "05-proxy", "export p=1").unwrap();
    app.reload_snippets(None);
    ctrl(&mut app, 'e');
    assert!(matches!(app.mode, Mode::HostSnippets(ref h) if h == "alpha"));
    let screen = render(&mut app);
    assert!(screen.contains("alpha + profile default"), "{screen}");
    assert!(screen.contains("05-proxy") && screen.contains("20-aliases (in profile)") || screen.contains("(in profile)"), "{screen}");
    key(&mut app, KeyCode::Char(' ')); // first row: 05-proxy
    assert_eq!(app.cfg.host_snippets("alpha"), ["05-proxy"]);
    assert_eq!(Config::load(&app.dir).unwrap().host_snippets("alpha"), ["05-proxy"]);
    // typing does not leak into the search box, Esc closes
    key(&mut app, KeyCode::Char('x'));
    assert!(app.filter.is_empty());
    key(&mut app, KeyCode::Esc);
    assert!(matches!(app.mode, Mode::Normal));
    assert!(render(&mut app).contains("default+1"), "profile column shows the extra");
    // other hosts are untouched
    assert!(app.cfg.host_snippets("beta").is_empty());
}

#[test]
fn host_extras_popup_works_with_the_mouse() {
    let (_t, mut app) = app();
    config::write_snippet(&app.dir, "05-proxy", "x").unwrap();
    app.reload_snippets(None);
    ctrl(&mut app, 'e');
    render(&mut app);
    let list = app.hits.popup_list;
    click(&mut app, list.x + 2, list.y);
    assert_eq!(app.cfg.host_snippets("alpha"), ["05-proxy"]);
    click(&mut app, 0, 0); // outside: closes
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn ctrl_e_does_nothing_without_a_host() {
    let (_t, mut app) = app_with(vec![]);
    ctrl(&mut app, 'e');
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn prompt_snippets_are_themes_not_snippets_to_send() {
    let (_t, mut app) = app();
    config::write_snippet(&app.dir, "mine.prompt", "export PS1='mine\\h> '\n").unwrap();
    app.reload_snippets(None);
    assert!(app.theme_names().contains(&"mine.prompt".to_string()));
    assert!(!app.grid_items().contains(&"mine.prompt".to_string()));
    app.switch_tab(Tab::Profiles);
    // pick it with the keyboard: it is the last row, so the list scrolls down to it
    key(&mut app, KeyCode::Right);
    for _ in 0..40 {
        key(&mut app, KeyCode::Down);
    }
    key(&mut app, KeyCode::Enter);
    assert_eq!(prompt_of(&app, "default").as_deref(), Some("mine.prompt"));
    let screen = render(&mut app);
    assert!(screen.contains("(•) mine.prompt"), "listed and selected in the prompt picker: {screen}");
    assert!(screen.contains("mine"), "preview shows the user's theme: {screen}");
}

#[test]
fn e_copies_a_builtin_theme_and_opens_it_in_the_editor() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    key(&mut app, KeyCode::Right); // prompt list, "fino" is selected
    ctrl(&mut app, 'e');
    assert_eq!(app.tab, Tab::Snippets);
    assert!(app.editor_focus);
    assert_eq!(app.editor_name.as_deref(), Some("fino.prompt"));
    // the copy holds the PS1 string alone
    let ps1 = prompt::extract_ps1(prompt::theme("fino").unwrap()).unwrap();
    assert_eq!(config::read_snippet(&app.dir, "fino.prompt").unwrap(), format!("{ps1}\n"));
    assert_eq!(prompt_of(&app, "default").as_deref(), Some("fino.prompt"));
    // editing and leaving saves, and the profile sends the edited prompt
    typed(&mut app, "mine ");
    key(&mut app, KeyCode::Esc);
    assert!(config::build_rc(&app.dir, &app.cfg, "default").contains("export PS1='mine "));
    // a second copy of the same theme does not overwrite the first
    app.switch_tab(Tab::Profiles);
    key(&mut app, KeyCode::Right);
    app.set_prompt_index(app.theme_names().iter().position(|n| n == "fino").unwrap() + 1);
    ctrl(&mut app, 'e');
    assert_eq!(app.editor_name.as_deref(), Some("fino-2.prompt"));
    assert!(config::read_snippet(&app.dir, "fino.prompt").unwrap().starts_with("mine "));
}

#[test]
fn e_with_the_servers_own_prompt_starts_a_new_theme_from_the_default() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    key(&mut app, KeyCode::Right);
    app.set_prompt_index(0);
    ctrl(&mut app, 'e');
    assert_eq!(app.editor_name.as_deref(), Some("my-prompt.prompt"));
    let ps1 = prompt::extract_ps1(prompt::theme(prompt::DEFAULT_THEME).unwrap()).unwrap();
    assert_eq!(config::read_snippet(&app.dir, "my-prompt.prompt").unwrap(), format!("{ps1}\n"));
}

#[test]
fn deleting_a_user_theme_resets_profiles_that_used_it() {
    let (_t, mut app) = app();
    config::write_snippet(&app.dir, "mine.prompt", "export PS1='x'\n").unwrap();
    app.reload_snippets(None);
    app.cfg.set_prompt("default", Some("mine.prompt"));
    app.delete_snippet("mine.prompt");
    assert_eq!(prompt_of(&app, "default"), None);
}

#[test]
fn non_shell_snippets_are_not_colored_as_bash() {
    let (_t, mut app) = app();
    config::write_snippet(&app.dir, "t.tmux", "\" a vim comment\nset number\n").unwrap();
    app.reload_snippets(Some("t.tmux"));
    app.switch_tab(Tab::Snippets);
    let mut term = Terminal::new(TestBackend::new(100, 20)).unwrap();
    term.draw(|f| app.draw(f)).unwrap();
    let ed = app.hits.editor;
    let buf = term.backend().buffer();
    // `set` is not a bash builtin here, and the quote must not turn the line green
    assert_eq!(buf[(ed.x, ed.y)].fg, Color::Reset);
    assert_eq!(buf[(ed.x + 3, ed.y)].fg, Color::Reset);
}

#[test]
fn editor_colors_bash_syntax_and_survives_horizontal_scroll() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Snippets);
    app.editor.lines = vec!["# note".into(), "alias ll='ls -l' # x".into()];
    let mut term = Terminal::new(TestBackend::new(100, 20)).unwrap();
    term.draw(|f| app.draw(f)).unwrap();
    let ed = app.hits.editor;
    let buf = term.backend().buffer();
    let fg = |x: u16, y: u16| buf[(ed.x + x, ed.y + y)].fg;
    assert_eq!(fg(0, 0), Color::DarkGray, "comment");
    assert_eq!(fg(0, 1), Color::Blue, "builtin alias");
    assert_eq!(fg(6, 1), Color::Cyan, "assignment name ll");
    assert_eq!(fg(9, 1), Color::Green, "string");
    // scrolled right: the same characters keep their color
    (app.editor.row, app.editor.col, app.editor.left) = (1, 12, 9);
    term.draw(|f| app.draw(f)).unwrap();
    let buf = term.backend().buffer();
    assert_eq!(buf[(ed.x, ed.y + 1)].fg, Color::Green);
    assert_eq!(buf[(ed.x, ed.y + 1)].symbol(), "'");
}

#[test]
fn selection_highlight_does_not_cover_the_scrollbar() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    key(&mut app, KeyCode::Right); // focus the prompt list so the selection is highlighted
    let mut term = Terminal::new(TestBackend::new(100, 18)).unwrap();
    term.draw(|f| app.draw(f)).unwrap();
    let list = app.hits.prompt_list;
    let x = list.x + list.width - 1;
    let buf = term.backend().buffer();
    let mut bar_cells = 0;
    for y in list.y..list.y + list.height {
        let cell = &buf[(x, y)];
        if cell.symbol() != " " {
            bar_cells += 1;
        }
        assert!(!cell.modifier.contains(Modifier::REVERSED), "row {y} highlights the scrollbar column");
    }
    assert!(bar_cells > 0, "expected a scrollbar in the last column");
}

#[test]
fn long_lists_get_a_scrollbar_and_short_ones_do_not() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    // the theme picker is taller than a short terminal
    let mut term = Terminal::new(TestBackend::new(100, 18)).unwrap();
    term.draw(|f| app.draw(f)).unwrap();
    let text: String = term.backend().buffer().content().iter().map(|c| c.symbol()).collect();
    assert!(text.contains('█') || text.contains('▐') || text.contains('║'), "no scrollbar thumb: {text}");
    let mut term = Terminal::new(TestBackend::new(100, 60)).unwrap();
    app.switch_tab(Tab::Hosts);
    term.draw(|f| app.draw(f)).unwrap();
    let text: String = term.backend().buffer().content().iter().map(|c| c.symbol()).collect();
    assert!(!text.contains('█'), "scrollbar on a list that fits");
}

#[test]
fn tiny_terminal_does_not_panic() {
    let (_t, mut app) = app();
    for tab in [Tab::Hosts, Tab::Snippets, Tab::Profiles] {
        app.switch_tab(tab);
        let mut term = Terminal::new(TestBackend::new(20, 6)).unwrap();
        term.draw(|f| app.draw(f)).unwrap();
    }
}

#[test]
fn default_sort_is_favorites_then_recent_then_name() {
    let (_t, mut app) = sortable();
    assert_eq!(order(&app), ["amy", "Bob", "zed"]);
    app.cfg.mark_used("zed");
    app.cfg.toggle_favorite("Bob");
    assert_eq!(order(&app), ["Bob", "zed", "amy"]);
}

#[test]
fn header_click_sorts_then_reverses() {
    let (_t, mut app) = sortable();
    render(&mut app);
    click_header(&mut app, SortKey::Target);
    assert_eq!(order(&app), ["zed", "Bob", "amy"]);
    click_header(&mut app, SortKey::Target);
    assert_eq!(order(&app), ["amy", "Bob", "zed"]);
    click_header(&mut app, SortKey::Host);
    assert_eq!(order(&app), ["amy", "Bob", "zed"]); // case-insensitive, ascending again
    click_header(&mut app, SortKey::Host);
    assert_eq!(order(&app), ["zed", "Bob", "amy"]);
    click_header(&mut app, SortKey::Smart);
    assert_eq!(app.cfg.sort, SortKey::Smart);
    assert!(!app.cfg.sort_reverse);
}

#[test]
fn ctrl_s_cycles_and_ctrl_r_reverses() {
    let (_t, mut app) = sortable();
    ctrl(&mut app, 's');
    assert_eq!(app.cfg.sort, SortKey::Host);
    ctrl(&mut app, 's');
    assert_eq!(app.cfg.sort, SortKey::Target);
    assert_eq!(order(&app), ["zed", "Bob", "amy"]);
    ctrl(&mut app, 'r');
    assert_eq!(order(&app), ["amy", "Bob", "zed"]);
    ctrl(&mut app, 's'); // moving to the next key resets the direction
    assert_eq!((app.cfg.sort, app.cfg.sort_reverse), (SortKey::Profile, false));
    ctrl(&mut app, 's');
    assert_eq!(app.cfg.sort, SortKey::Source);
    ctrl(&mut app, 's');
    assert_eq!(app.cfg.sort, SortKey::LastUsed);
    ctrl(&mut app, 's');
    assert_eq!(app.cfg.sort, SortKey::Smart); // full cycle
}

#[test]
fn last_used_sorts_newest_first_and_never_used_stay_last() {
    let (_t, mut app) = sortable();
    app.cfg.hosts.entry("zed".into()).or_default().last_used = Some(100);
    app.cfg.hosts.entry("Bob".into()).or_default().last_used = Some(200);
    app.cfg.sort = SortKey::LastUsed;
    assert_eq!(order(&app), ["Bob", "zed", "amy"]);
    app.cfg.sort_reverse = true;
    assert_eq!(order(&app), ["zed", "Bob", "amy"]); // oldest first, never-used still last
}

#[test]
fn profile_sort_groups_by_profile_name() {
    let (_t, mut app) = sortable();
    app.cfg.add_profile("work").unwrap();
    app.cfg.hosts.entry("amy".into()).or_default().profile = Some("work".into());
    app.cfg.hosts.entry("Bob".into()).or_default().profile = Some(config::PROFILE_OFF.into());
    app.cfg.sort = SortKey::Profile;
    // "default" < "off" < "work"
    assert_eq!(order(&app), ["zed", "Bob", "amy"]);
}

#[test]
fn sort_keeps_the_selected_host_selected() {
    let (_t, mut app) = sortable();
    render(&mut app);
    key(&mut app, KeyCode::Down); // Bob
    assert_eq!(app.hosts[app.selected_host().unwrap()].alias, "Bob");
    click_header(&mut app, SortKey::Target);
    assert_eq!(app.hosts[app.selected_host().unwrap()].alias, "Bob");
    ctrl(&mut app, 'r');
    assert_eq!(app.hosts[app.selected_host().unwrap()].alias, "Bob");
}

#[test]
fn sort_choice_is_saved_and_restored() {
    let (_t, mut app) = sortable();
    render(&mut app);
    click_header(&mut app, SortKey::Target);
    click_header(&mut app, SortKey::Target);
    let loaded = Config::load(&app.dir).unwrap();
    assert_eq!((loaded.sort, loaded.sort_reverse), (SortKey::Target, true));
    let text = std::fs::read_to_string(app.dir.join("config.toml")).unwrap();
    assert!(text.contains("sort = \"target\""), "{text}");
}

#[test]
fn default_sort_is_not_written_to_config() {
    let (_t, app) = sortable();
    app.cfg.save(&app.dir).unwrap();
    let text = std::fs::read_to_string(app.dir.join("config.toml")).unwrap();
    assert!(!text.contains("sort"), "{text}");
}

#[test]
fn header_shows_arrow_on_active_column() {
    let (_t, mut app) = sortable();
    let screen = render(&mut app);
    assert!(!screen.contains('▲') && !screen.contains('▼'), "{screen}");
    click_header(&mut app, SortKey::Host);
    let screen = render(&mut app);
    assert!(screen.contains("host ▲"), "{screen}");
    click_header(&mut app, SortKey::Host);
    assert!(render(&mut app).contains("host ▼"));
    click_header(&mut app, SortKey::LastUsed);
    assert!(render(&mut app).contains("last used ▼")); // newest first is descending
}

#[test]
fn header_click_does_not_connect_or_select() {
    let (_t, mut app) = sortable();
    render(&mut app);
    click_header(&mut app, SortKey::Host);
    click_header(&mut app, SortKey::Host);
    assert!(app.pending_connect.is_none());
}

#[test]
fn sort_key_columns_map_correctly() {
    let c = Cols::default();
    assert_eq!(sort_key_at(0, c), SortKey::Smart);
    assert_eq!(sort_key_at(COL_STAR, c), SortKey::Host);
    assert_eq!(sort_key_at(COL_STAR + COL_ALIAS, c), SortKey::Target);
    assert_eq!(sort_key_at(COL_STAR + COL_ALIAS + COL_TARGET, c), SortKey::Profile);
    assert_eq!(sort_key_at(COL_STAR + COL_ALIAS + COL_TARGET + COL_PROFILE, c), SortKey::Source);
    assert_eq!(sort_key_at(COL_STAR + COL_ALIAS + COL_TARGET + COL_PROFILE + COL_SOURCE, c), SortKey::LastUsed);
    // A wider host column pushes the others right.
    let wide = Cols { alias: 40, ..c };
    assert_eq!(sort_key_at(COL_STAR + COL_ALIAS, wide), SortKey::Host);
    assert_eq!(sort_key_at(COL_STAR + 40, wide), SortKey::Target);
}

/// Dev aid: `cargo test dump_screens -- --ignored --nocapture` prints every tab.
#[test]
#[ignore]
fn dump_screens() {
    let (_t, mut app) = app();
    app.cfg.toggle_favorite("beta");
    app.cfg.mark_used("gamma");
    for tab in [Tab::Hosts, Tab::Snippets, Tab::Profiles] {
        app.switch_tab(tab);
        println!("{}\n{}", "=".repeat(110), render(&mut app));
    }
    app.switch_tab(Tab::Snippets);
    key(&mut app, KeyCode::Char('g'));
    println!("{}\n{}", "=".repeat(110), render(&mut app));
}

#[test]
fn helpers() {
    assert!(fuzzy("abc", "a-big-cat"));
    assert!(!fuzzy("acb", "abc"));
    assert_eq!(truncate("abcdef", 4), "abc…");
    assert_eq!(truncate("ab", 4), "ab  ");
}

#[test]
fn editor_title_shows_the_real_file_name_for_every_kind() {
    let (_t, mut app) = app();
    for (name, shown) in [("20-aliases", "20-aliases.sh"), ("v.vim", "v.vim"), ("t.tmux", "t.tmux"), ("i.inputrc", "i.inputrc"), ("p.prompt", "p.prompt")] {
        config::write_snippet(&app.dir, name, "x\n").unwrap();
        app.reload_snippets(Some(name));
        app.switch_tab(Tab::Snippets);
        let mut term = Terminal::new(TestBackend::new(100, 20)).unwrap();
        term.draw(|f| app.draw(f)).unwrap();
        let screen: String = term.backend().buffer().content().iter().map(|c| c.symbol()).collect();
        assert!(screen.contains(&format!(" {shown} ")), "{name}: {screen}");
        assert!(!screen.contains(&format!("{shown}.sh")), "{name}");
    }
}

#[test]
fn prompt_themes_are_colored_as_bash() {
    let (_t, mut app) = app();
    config::write_snippet(&app.dir, "mine.prompt", "# note
export PS1='x'
").unwrap();
    app.reload_snippets(Some("mine.prompt"));
    app.switch_tab(Tab::Snippets);
    let mut term = Terminal::new(TestBackend::new(100, 20)).unwrap();
    term.draw(|f| app.draw(f)).unwrap();
    let ed = app.hits.editor;
    let buf = term.backend().buffer();
    assert_eq!(buf[(ed.x, ed.y)].fg, Color::DarkGray, "comment");
}

#[test]
fn a_plain_ps1_prompt_file_gets_a_preview_but_no_bash_colors() {
    let (_t, mut app) = app();
    config::write_snippet(&app.dir, "p.prompt", "# not a comment\n").unwrap();
    app.reload_snippets(Some("p.prompt"));
    app.switch_tab(Tab::Snippets);
    let mut term = Terminal::new(TestBackend::new(100, 20)).unwrap();
    term.draw(|f| app.draw(f)).unwrap();
    let ed = app.hits.editor;
    let buf = term.backend().buffer();
    assert_eq!(buf[(ed.x, ed.y)].fg, Color::Reset, "a # is part of the string, not a comment");
    app.editor.lines = vec![r"\u@\h".into()];
    term.draw(|f| app.draw(f)).unwrap();
    let buf = term.backend().buffer();
    assert_ne!(buf[(ed.x, ed.y)].fg, Color::Reset, "prompt escapes are colored");
    assert_eq!(buf[(ed.x + 2, ed.y)].fg, Color::Reset, "the @ is plain");
    let text: String = buf.content().iter().map(|c| c.symbol()).collect();
    assert!(text.contains("Prompt preview"));
}

#[test]
fn vim_snippets_are_colored_as_vim_script() {
    let (_t, mut app) = app();
    config::write_snippet(&app.dir, "v.vim", "\" note
set number
").unwrap();
    app.reload_snippets(Some("v.vim"));
    app.switch_tab(Tab::Snippets);
    let mut term = Terminal::new(TestBackend::new(100, 20)).unwrap();
    term.draw(|f| app.draw(f)).unwrap();
    let ed = app.hits.editor;
    let buf = term.backend().buffer();
    assert_eq!(buf[(ed.x, ed.y)].fg, Color::DarkGray, "comment");
    assert_eq!(buf[(ed.x, ed.y + 1)].fg, Color::Magenta, "set");
    assert_eq!(buf[(ed.x + 4, ed.y + 1)].fg, Color::Cyan, "option name");
}

#[test]
fn snippet_names_are_colored_by_kind() {
    let (_t, mut app) = app();
    for n in ["t.tmux", "i.inputrc", "p.prompt"] {
        config::write_snippet(&app.dir, n, "x").unwrap();
    }
    app.reload_snippets(None);
    app.switch_tab(Tab::Snippets);
    let mut term = Terminal::new(TestBackend::new(100, 40)).unwrap();
    term.draw(|f| app.draw(f)).unwrap();
    let list = app.hits.snip_list;
    let buf = term.backend().buffer();
    for (i, name) in app.snippets.iter().enumerate() {
        let want = match name.as_str() {
            "t.tmux" => Color::Yellow,
            n if n.ends_with(".inputrc") => Color::LightBlue,
            "p.prompt" => Color::Magenta,
            n if n.ends_with(".vim") => Color::Cyan, // the vim presets
            _ => Color::Green, // shell snippets
        };
        assert_eq!(buf[(list.x, list.y + i as u16)].fg, want, "{name}");
    }
}

#[test]
fn the_host_list_shows_where_each_host_came_from_and_can_sort_by_it() {
    let mut known = host("kh", "10.0.0.7");
    known.source = Source::KnownHosts;
    let (_t, mut app) = app_with(vec![known, host("zed", "10.0.0.1"), host("amy", "10.0.0.2")]);
    let screen = render(&mut app);
    assert!(screen.contains("source"), "{screen}");
    let row = |name: &str| screen.lines().find(|l| l.contains(name)).unwrap().to_string();
    assert!(row("kh").contains("known_hosts"), "{screen}");
    assert!(row("zed").contains("config") && !row("zed").contains("known_hosts"), "{screen}");
    click_header(&mut app, SortKey::Source);
    assert_eq!(app.cfg.sort, SortKey::Source);
    assert_eq!(order(&app), ["amy", "zed", "kh"], "config hosts first, then by name");
    click_header(&mut app, SortKey::Source);
    assert_eq!(order(&app), ["kh", "zed", "amy"], "reversed");
}

#[test]
fn ctrl_o_edits_the_connection_options_of_a_host() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    let screen = render(&mut app);
    assert!(screen.contains("alpha: connection options"));
    let screen = squash(&screen);
    for hint in ["Tab next field", "^N new", "^R rename", "^D delete", "Enter connect once", "^S save all", "Esc cancel"] {
        assert!(screen.contains(hint), "{hint} missing: {screen}");
    }
    typed(&mut app, "root");
    key(&mut app, KeyCode::Tab);
    typed(&mut app, "22x22"); // the port takes digits only
    key(&mut app, KeyCode::Tab);
    typed(&mut app, "~/.ssh/work");
    key(&mut app, KeyCode::Tab);
    typed(&mut app, "bastion");
    key(&mut app, KeyCode::Tab);
    typed(&mut app, "-o ForwardAgent=yes");
    ctrl(&mut app, 's'); // save

    let p = &app.cfg.hosts["alpha"];
    assert_eq!((p.user.as_str(), p.port.as_str(), p.identity.as_str()), ("root", "2222", "~/.ssh/work"));
    assert_eq!((p.jump.as_str(), p.args.as_str()), ("bastion", "-o ForwardAgent=yes"));
    assert_eq!(Config::load(&app.dir).unwrap().hosts["alpha"], *p, "saved");
    let screen = render(&mut app);
    assert!(!screen.contains("connection options"), "form closed: {screen}");
    assert!(screen.contains("root@10.0.0.1:2222"), "the target shows the user and port: {screen}");

    key(&mut app, KeyCode::Enter);
    assert_eq!(
        app.pending_connect,
        Some(
            ["-l", "root", "-p", "2222", "-i", "~/.ssh/work", "-J", "bastion", "-o", "ForwardAgent=yes", "alpha"]
                .map(String::from)
                .to_vec()
        )
    );
}

#[test]
fn the_options_form_shows_the_ssh_config_values_and_esc_discards() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    let screen = render(&mut app);
    assert!(screen.contains("me"), "the user from ~/.ssh/config is the hint: {screen}");
    typed(&mut app, "root");
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::Char('y')); // discard
    assert!(app.cfg.hosts.get("alpha").is_none_or(|p| p.user.is_empty()));
    assert!(!render(&mut app).contains("connection options"));
}

#[test]
fn esc_asks_before_throwing_away_unsaved_changes() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    key(&mut app, KeyCode::Esc);
    assert!(!render(&mut app).contains("connection options"), "nothing to lose: closes at once");

    ctrl(&mut app, 'o');
    new_options(&mut app, "root");
    let screen = render(&mut app);
    assert!(screen.contains("(unsaved)"), "{screen}");
    key(&mut app, KeyCode::Esc);
    let screen = render(&mut app);
    assert!(screen.contains("discard the changes") && screen.contains("keep editing"), "{screen}");
    key(&mut app, KeyCode::Char('n')); // keep editing
    assert!(render(&mut app).contains("connection options"));
    assert!(!render(&mut app).contains("discard the changes"));

    key(&mut app, KeyCode::Esc);
    ctrl(&mut app, 's'); // save from the prompt
    assert!(app.cfg.hosts["alpha"].options.contains_key("root"));
    assert!(!render(&mut app).contains("connection options"));

    // Typing and then undoing the typing leaves nothing to lose.
    ctrl(&mut app, 'o');
    typed(&mut app, "x");
    key(&mut app, KeyCode::Backspace);
    key(&mut app, KeyCode::Esc);
    assert!(!render(&mut app).contains("connection options"));
}

#[test]
fn a_click_outside_asks_too_and_a_click_on_the_hint_discards() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    typed(&mut app, "root");
    render(&mut app);
    click(&mut app, 0, 0);
    assert!(render(&mut app).contains("discard the changes"), "asked instead of closing");
    let (r, _) = app.hits.form_keys.iter().find(|(_, k)| k.code == KeyCode::Char('y')).copied().unwrap();
    click(&mut app, r.x, r.y);
    assert!(!render(&mut app).contains("connection options"));
    assert!(app.cfg.hosts.get("alpha").is_none_or(|p| p.user.is_empty()));
}

#[test]
fn a_bad_port_is_refused_and_an_emptied_form_clears_the_saved_options() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    typed(&mut app, "root");
    ctrl(&mut app, 's');
    assert_eq!(app.cfg.hosts["alpha"].user, "root");

    ctrl(&mut app, 'o');
    key(&mut app, KeyCode::Tab);
    typed(&mut app, "99999");
    ctrl(&mut app, 's');
    let screen = render(&mut app);
    assert!(screen.contains("port must be"), "{screen}");
    assert!(screen.contains("connection options"), "the form stays open: {screen}");
    assert!(app.cfg.hosts["alpha"].port.is_empty());
    ctrl(&mut app, 'u'); // clear the port
    key(&mut app, KeyCode::BackTab); // back to the user
    ctrl(&mut app, 'u');
    ctrl(&mut app, 's');
    assert!(!app.cfg.hosts["alpha"].has_overrides());
    assert!(render(&mut app).contains("uses ~/.ssh/config again"));
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.pending_connect, Some(vec!["alpha".to_string()]), "plain again");
}

#[test]
fn enter_in_the_form_connects_once_without_saving() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    typed(&mut app, "root");
    key(&mut app, KeyCode::Tab);
    typed(&mut app, "2200");
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.pending_connect, Some(["-l", "root", "-p", "2200", "alpha"].map(String::from).to_vec()));
    assert!(!render(&mut app).contains("connection options"), "the form closed");
    assert!(app.cfg.hosts.get("alpha").is_none_or(|p| !p.has_overrides()), "nothing saved");
    assert!(Config::load(&app.dir).unwrap().hosts.get("alpha").is_none_or(|p| !p.has_overrides()));

    // The next plain connect has no trace of it.
    app.pending_connect = None;
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.pending_connect, Some(vec!["alpha".to_string()]));
}

#[test]
fn a_one_off_connect_builds_on_the_saved_options_only_through_the_form() {
    let (_t, mut app) = app();
    app.cfg.hosts.entry("alpha".into()).or_default().user = "admin".into();
    ctrl(&mut app, 'o');
    assert!(render(&mut app).contains("admin"), "the form starts from the saved options");
    key(&mut app, KeyCode::Tab);
    typed(&mut app, "2200");
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.pending_connect, Some(["-l", "admin", "-p", "2200", "alpha"].map(String::from).to_vec()));
    assert!(app.cfg.hosts["alpha"].port.is_empty(), "the port was for this time only");
    assert_eq!(app.cfg.hosts["alpha"].user, "admin", "the saved user is unchanged");
}

#[test]
fn a_typed_user_at_host_picks_the_matching_host_and_the_user() {
    let (_t, mut app) = app();
    typed(&mut app, "root@bet");
    let screen = render(&mut app);
    assert!(screen.contains("connect as root"), "{screen}");
    assert_eq!(app.visible().len(), 1, "only beta matches `bet`");
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.pending_connect, Some(["-l", "root", "beta"].map(String::from).to_vec()));

    // The typed user wins over the host's saved one: ssh keeps the first -l.
    app.cfg.hosts.entry("beta".into()).or_default().user = "admin".into();
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.pending_connect, Some(["-l", "root", "-l", "admin", "beta"].map(String::from).to_vec()));
}

#[test]
fn text_with_spaces_or_no_user_is_not_a_user_at_host_search() {
    let (_t, mut app) = app();
    typed(&mut app, "-p 22 me@new.host");
    assert_eq!(app.filter_parts().0, None);
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.pending_connect, Some(["-p", "22", "me@new.host"].map(String::from).to_vec()));
    let (_t2, mut app2) = app_with(vec![host("alpha", "10.0.0.1")]);
    typed(&mut app2, "@alpha");
    assert_eq!(app2.filter_parts().0, None, "an empty user is no user");
}

fn new_options(app: &mut App, name: &str) {
    ctrl(app, 'n');
    typed(app, name);
    key(app, KeyCode::Enter);
}

#[test]
fn named_sets_are_added_edited_saved_and_connect_with_the_picker() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    typed(&mut app, "me");
    new_options(&mut app, "root");
    assert!(render(&mut app).contains("root"), "the new options are shown and selected");
    ctrl(&mut app, 'u'); // the copy of the default options: change the user
    typed(&mut app, "root");
    key(&mut app, KeyCode::Tab);
    typed(&mut app, "2200");
    new_options(&mut app, "deploy");
    ctrl(&mut app, 'u');
    typed(&mut app, "deploy");
    key(&mut app, KeyCode::Tab);
    key(&mut app, KeyCode::Tab);
    key(&mut app, KeyCode::Tab);
    typed(&mut app, "bastion");
    ctrl(&mut app, 's');

    let p = &app.cfg.hosts["alpha"];
    assert_eq!(p.user, "me");
    assert_eq!(p.options["root"].user, "root");
    assert_eq!(p.options["root"].port, "2200");
    assert_eq!((p.options["deploy"].user.as_str(), p.options["deploy"].jump.as_str()), ("deploy", "bastion"));
    assert_eq!(Config::load(&app.dir).unwrap().hosts["alpha"], *p, "saved");

    // A plain Enter still uses the default options.
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.pending_connect, Some(["-l", "me", "alpha"].map(String::from).to_vec()));

    // Ctrl-L picks one of the options.
    app.pending_connect = None;
    ctrl(&mut app, 'l');
    let screen = render(&mut app);
    assert!(screen.contains("connect to alpha with") && screen.contains("deploy") && screen.contains("-l root -p 2200"), "{screen}");
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.pending_connect, Some(["-l", "root", "-p", "2200", "alpha"].map(String::from).to_vec()));
    assert!(!render(&mut app).contains("connect to alpha with"), "the picker closed");
}

#[test]
fn left_and_right_switch_sets_and_keep_the_edits() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    typed(&mut app, "me");
    new_options(&mut app, "root");
    ctrl(&mut app, 'u');
    typed(&mut app, "root");
    key(&mut app, KeyCode::Left); // back to default
    assert!(render(&mut app).contains("me"));
    key(&mut app, KeyCode::Right); // and to root again
    key(&mut app, KeyCode::Enter); // connect with the options on screen
    assert_eq!(app.pending_connect, Some(["-l", "root", "alpha"].map(String::from).to_vec()));
    // New named options that are used right away are kept, but the default ones were not saved.
    let p = &app.cfg.hosts["alpha"];
    assert_eq!(p.options["root"].user, "root");
    assert_eq!(p.user, "", "the unsaved default stays unsaved");
    assert_eq!(Config::load(&app.dir).unwrap().hosts["alpha"].options["root"].user, "root");
}

#[test]
fn connecting_with_edited_saved_options_does_not_change_them() {
    let (_t, mut app) = app();
    app.cfg.hosts.entry("alpha".into()).or_default().options.insert(
        "root".into(),
        config::ConnOptions { user: "root".into(), ..Default::default() },
    );
    ctrl(&mut app, 'o');
    key(&mut app, KeyCode::Right);
    key(&mut app, KeyCode::Tab);
    typed(&mut app, "2200");
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.pending_connect, Some(["-l", "root", "-p", "2200", "alpha"].map(String::from).to_vec()));
    assert_eq!(app.cfg.hosts["alpha"].options["root"].port, "", "an edit of saved options stays ad hoc");
}

#[test]
fn the_picker_moves_when_its_title_bar_is_dragged() {
    let (_t, mut app) = app();
    app.cfg.hosts.entry("alpha".into()).or_default().options.insert("root".into(), Default::default());
    ctrl(&mut app, 'l');
    render(&mut app);
    let before = app.hits.picker_area;
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), before.x + 4, before.y);
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), before.x + 9, before.y + 3);
    mouse(&mut app, MouseEventKind::Up(MouseButton::Left), before.x + 9, before.y + 3);
    render(&mut app);
    let after = app.hits.picker_area;
    assert_eq!((after.x, after.y), (before.x + 5, before.y + 3));
    assert!(matches!(app.mode, Mode::OptionsPicker(_)), "still open");
    key(&mut app, KeyCode::Esc);
    ctrl(&mut app, 'l');
    render(&mut app);
    assert_eq!(app.hits.picker_area, before, "centred again on reopening");
}

#[test]
fn the_options_form_is_wide_enough_for_its_key_hints() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    let screen = squash(&render(&mut app));
    for hint in [
        "Enter connect once Esc cancel Tab next field",
        "←/→ switch ^N new ^R rename ^D delete",
        "^S save all ^U clear field",
    ] {
        assert!(screen.contains(hint), "{hint}: {screen}");
    }
    let area = app.hits.form_area;
    assert!(app.hits.form_keys.iter().all(|(r, _)| r.right() < area.right()), "a hint sticks out of the form");
    assert!(app.hits.form_keys.len() >= 9);
}

#[test]
fn a_set_can_be_deleted_and_its_name_must_be_new() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    new_options(&mut app, "root");
    ctrl(&mut app, 'n');
    typed(&mut app, "root");
    key(&mut app, KeyCode::Enter);
    assert!(render(&mut app).contains("already exists"));
    key(&mut app, KeyCode::Esc); // cancels the name, not the form
    assert!(render(&mut app).contains("connection options"));
    new_options(&mut app, "default");
    assert!(render(&mut app).contains("already exists"));
    key(&mut app, KeyCode::Esc);
    ctrl(&mut app, 'd');
    ctrl(&mut app, 's');
    assert!(app.cfg.hosts["alpha"].options.is_empty(), "root deleted");
}

#[test]
fn the_picker_needs_named_sets() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'l');
    assert!(render(&mut app).contains("no named options"));
    assert!(app.pending_connect.is_none());
}

#[test]
fn a_bad_port_in_any_set_stops_the_save() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    new_options(&mut app, "root");
    key(&mut app, KeyCode::Tab);
    typed(&mut app, "99999");
    key(&mut app, KeyCode::Left); // the bad value stays in the root set
    ctrl(&mut app, 's');
    let screen = render(&mut app);
    assert!(screen.contains("root: port must be"), "{screen}");
    assert!(app.cfg.hosts.get("alpha").is_none_or(|p| p.options.is_empty()), "nothing saved");
}

#[test]
fn the_options_form_takes_clicks_on_fields_sets_and_key_hints() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    typed(&mut app, "me");
    render(&mut app);

    // Add a set with the clickable hint, then name it.
    let (r, _) = app.hits.form_keys.iter().find(|(_, k)| k.code == KeyCode::Char('n')).copied().unwrap();
    click(&mut app, r.x, r.y);
    typed(&mut app, "root");
    key(&mut app, KeyCode::Enter);
    render(&mut app);
    assert_eq!(app.hits.form_options.len(), 2);

    // A click on a field focuses it.
    let f = app.hits.form_fields;
    click(&mut app, f.x + 20, f.y + 3); // Jump host
    typed(&mut app, "bastion");
    // A click on a set name switches to it.
    let (default_tab, _) = app.hits.form_options[0].clone();
    click(&mut app, default_tab.x, default_tab.y);
    assert!(render(&mut app).contains("me"));
    let (root_tab, _) = app.hits.form_options[1].clone();
    click(&mut app, root_tab.x, root_tab.y);
    assert!(render(&mut app).contains("bastion"), "the edit stayed in the root set");

    // The Enter hint connects once.
    render(&mut app);
    let (r, _) = app.hits.form_keys.iter().find(|(_, k)| k.code == KeyCode::Enter).copied().unwrap();
    click(&mut app, r.x, r.y);
    assert_eq!(app.pending_connect, Some(["-l", "me", "-J", "bastion", "alpha"].map(String::from).to_vec()));
}

#[test]
fn a_click_outside_the_options_form_closes_it_and_the_wheel_moves_the_focus() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    render(&mut app);
    app.on_event(Event::Mouse(MouseEvent {
        kind: MouseEventKind::ScrollDown,
        column: 40,
        row: 12,
        modifiers: KeyModifiers::NONE,
    }));
    typed(&mut app, "root");
    key(&mut app, KeyCode::Char('s'));
    ctrl(&mut app, 's');
    assert_eq!(app.cfg.hosts["alpha"].port, "", "the wheel moved to the port, which takes digits only");
    assert_eq!(app.cfg.hosts["alpha"].user, "", "nothing typed went to the user");

    ctrl(&mut app, 'o');
    render(&mut app);
    click(&mut app, 0, 0);
    assert!(!render(&mut app).contains("connection options"));
}

#[test]
fn the_set_picker_connects_on_a_click_and_closes_on_an_outside_click() {
    let (_t, mut app) = app();
    app.cfg.hosts.entry("alpha".into()).or_default().options.insert(
        "root".into(),
        config::ConnOptions { user: "root".into(), ..Default::default() },
    );
    ctrl(&mut app, 'l');
    render(&mut app);
    click(&mut app, 0, 0);
    assert!(!render(&mut app).contains("connect to alpha with"));
    assert!(app.pending_connect.is_none());

    ctrl(&mut app, 'l');
    render(&mut app);
    let list = app.hits.picker_list;
    click(&mut app, list.x + 2, list.y + 1); // the second row: root
    assert_eq!(app.pending_connect, Some(["-l", "root", "alpha"].map(String::from).to_vec()));
}

fn mouse(app: &mut App, kind: MouseEventKind, x: u16, y: u16) {
    app.on_event(Event::Mouse(MouseEvent { kind, column: x, row: y, modifiers: KeyModifiers::NONE }));
}

#[test]
fn the_options_popup_moves_when_its_title_bar_is_dragged() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    render(&mut app);
    let before = app.hits.form_area;
    let (x, y) = (before.x + 5, before.y);
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), x, y);
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), x + 6, y + 2);
    mouse(&mut app, MouseEventKind::Up(MouseButton::Left), x + 6, y + 2);
    render(&mut app);
    let after = app.hits.form_area;
    assert_eq!((after.x, after.y), (before.x + 6, before.y + 2));
    assert!(matches!(app.mode, Mode::HostForm(_)), "the drag did not close it");

    // A click on a field still works at the new place, and a drag far away stops at the edge.
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), after.x + 5, after.y);
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), 500, 500);
    mouse(&mut app, MouseEventKind::Up(MouseButton::Left), 500, 500);
    render(&mut app);
    let edge = app.hits.form_area;
    assert!(edge.right() <= 110 && edge.bottom() <= 40, "{edge:?}");

    // A drag that starts elsewhere moves nothing, and reopening centres it again.
    mouse(&mut app, MouseEventKind::Down(MouseButton::Left), edge.x + 3, edge.y + 2);
    mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), edge.x, edge.y);
    key(&mut app, KeyCode::Esc);
    ctrl(&mut app, 'o');
    render(&mut app);
    assert_eq!(app.hits.form_area, before);
}

#[test]
fn many_option_names_scroll_around_the_current_one() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    for i in 1..=12 {
        new_options(&mut app, &format!("deploy-{i:02}"));
    }
    let screen = render(&mut app);
    let area = app.hits.form_area;
    assert!(screen.contains("deploy-12") && screen.contains('‹'), "the newest is shown with names hidden before it: {screen}");
    assert!(!screen.contains("deploy-01"), "{screen}");
    assert!(app.hits.form_options.iter().all(|(r, _)| r.right() < area.right()), "a name sticks out of the form");

    // Stepping left keeps the current name in view and brings in the ones at the other side.
    for _ in 0..11 {
        key(&mut app, KeyCode::Left);
        let screen = render(&mut app);
        let cur = app.hits.form_options.iter().any(|(_, n)| *n == app.current_options_name());
        assert!(cur, "the current name is not shown: {screen}");
    }
    let screen = render(&mut app);
    assert!(screen.contains("deploy-01") && screen.contains('›') && !screen.contains('‹'), "{screen}");

    // The arrows are clickable and step one name.
    let (r, k) = app.hits.form_keys.iter().find(|(_, k)| k.code == KeyCode::Right).copied().unwrap();
    assert_eq!(k.code, KeyCode::Right);
    let before = app.current_options_name();
    click(&mut app, r.x, r.y);
    assert_ne!(app.current_options_name(), before);
    assert!(matches!(app.mode, Mode::HostForm(_)), "still open");
}

#[test]
fn a_name_longer_than_the_row_is_still_reachable() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    new_options(&mut app, &"x".repeat(80));
    let screen = render(&mut app);
    assert!(screen.contains("xxxxxxxx"), "{screen}");
    key(&mut app, KeyCode::Left);
    key(&mut app, KeyCode::Right);
    assert!(matches!(app.mode, Mode::HostForm(_)));
}

#[test]
fn ctrl_r_renames_the_current_named_options() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    ctrl(&mut app, 'r');
    assert!(render(&mut app).contains("cannot be renamed"), "the default options keep their name");
    typed(&mut app, "x"); // the key went to a field, not to a name prompt
    key(&mut app, KeyCode::Backspace);

    new_options(&mut app, "root");
    ctrl(&mut app, 'u');
    typed(&mut app, "root");
    ctrl(&mut app, 'r');
    let screen = render(&mut app);
    assert!(screen.contains("Rename to") && screen.contains("root"), "prefilled with the old name: {screen}");
    for _ in 0..4 {
        key(&mut app, KeyCode::Backspace);
    }
    typed(&mut app, "admin");
    key(&mut app, KeyCode::Enter);
    ctrl(&mut app, 's');

    let p = &app.cfg.hosts["alpha"];
    assert!(!p.options.contains_key("root"), "the old name is gone");
    assert_eq!(p.options["admin"].user, "root", "the values moved with the name");
}

#[test]
fn a_rename_to_a_taken_name_is_refused_and_esc_cancels_it() {
    let (_t, mut app) = app();
    ctrl(&mut app, 'o');
    new_options(&mut app, "root");
    new_options(&mut app, "deploy");
    ctrl(&mut app, 'r');
    for _ in 0.."deploy".len() {
        key(&mut app, KeyCode::Backspace);
    }
    typed(&mut app, "root");
    key(&mut app, KeyCode::Enter);
    assert!(render(&mut app).contains("already exist"));
    key(&mut app, KeyCode::Esc); // cancels the rename, not the form
    let screen = render(&mut app);
    assert!(screen.contains("connection options") && !screen.contains("Rename to"), "{screen}");
    ctrl(&mut app, 's');
    let names: Vec<_> = app.cfg.hosts["alpha"].options.keys().cloned().collect();
    assert_eq!(names, ["deploy", "root"]);
}

/// True when the cell at the start of `text` on the screen is drawn bold.
fn bold_at(app: &mut App, text: &str) -> bool {
    let mut term = Terminal::new(TestBackend::new(110, 30)).unwrap();
    term.draw(|f| app.draw(f)).unwrap();
    let buf = term.backend().buffer().clone();
    for y in 0..buf.area.height {
        let row: Vec<&str> = (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect();
        let line = row.concat();
        if let Some(at) = line.find(text) {
            let x = line[..at].chars().count() as u16;
            return buf[(x, y)].modifier.contains(Modifier::BOLD);
        }
    }
    panic!("{text} not on screen");
}

#[test]
fn the_pick_hint_lights_up_only_when_the_host_has_named_options() {
    let (_t, mut app) = app();
    assert!(!app.can_pick_options());
    assert!(!bold_at(&mut app, "^L"), "nothing to pick: the key is dim");
    assert!(bold_at(&mut app, "^O"), "the other keys stay lit");

    app.cfg.hosts.entry("alpha".into()).or_default().options.insert("root".into(), Default::default());
    assert!(app.can_pick_options());
    assert!(bold_at(&mut app, "^L"), "named options exist: the key is lit");

    // Another host has none.
    key(&mut app, KeyCode::Down);
    assert!(!app.can_pick_options());
    assert!(!bold_at(&mut app, "^L"));
}

#[test]
fn the_list_pages_use_ctrl_keys_and_plain_letters_do_nothing() {
    let (_tmp, mut app) = app();
    for tab in [Tab::Snippets, Tab::Profiles] {
        app.switch_tab(tab);
        for c in ['n', 'c', 'd', 'r', 'e', 'y', '*'] {
            key(&mut app, KeyCode::Char(c));
            assert!(matches!(app.mode, Mode::Normal), "plain {c} must not act on {tab:?}");
        }
        ctrl(&mut app, 'n');
        assert!(matches!(app.mode, Mode::Input(..)), "Ctrl-N creates on {tab:?}");
        key(&mut app, KeyCode::Esc);
    }
}

#[test]
fn the_footer_shows_tab_on_every_page_and_esc_goes_back_one_pane_in_profiles() {
    let (_t, mut app) = app();
    let hosts = render(&mut app);
    assert!(hosts.contains("Tab next") && hosts.contains("^S/R sort") && hosts.contains("Esc quit") && hosts.contains("F2"), "{hosts}");
    app.switch_tab(Tab::Profiles);
    key(&mut app, KeyCode::Right);
    assert_eq!(app.prof_focus, ProfFocus::Prompt);
    assert!(render(&mut app).contains("←/Esc back"));
    key(&mut app, KeyCode::Right);
    assert_eq!(app.prof_focus, ProfFocus::Snippets);
    assert!(render(&mut app).contains("←/Esc back"));
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.prof_focus, ProfFocus::Prompt, "Esc goes back one pane, like Left");
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.prof_focus, ProfFocus::List);
}
