use super::hosts::{COL_ALIAS, COL_PROFILE, COL_SOURCE, COL_STAR, COL_TARGET, fuzzy, sort_key_at};
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
    let app = App::new(tmp.path().to_path_buf(), cfg, hosts);
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
fn click_profile_column_and_arrows_cycle_profile() {
    let (_t, mut app) = app();
    app.cfg.add_profile("work").unwrap();
    render(&mut app);
    let list = app.hits.hosts_list;
    click(&mut app, list.x + COL_STAR + COL_ALIAS + COL_TARGET + 1, list.y);
    assert_eq!(app.cfg.profile_for("alpha"), Some("work"));
    key(&mut app, KeyCode::Right); // work -> off
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
    key(&mut app, KeyCode::Char('n'));
    typed(&mut app, "30-git");
    key(&mut app, KeyCode::Enter);
    assert!(app.snippets.contains(&"30-git".to_string()));
    assert_eq!(app.editor_name.as_deref(), Some("30-git"));
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::Char('n'));
    typed(&mut app, "bad name");
    key(&mut app, KeyCode::Enter);
    assert!(app.status.as_ref().is_some_and(|s| s.1), "{:?}", app.status);
}

#[test]
fn delete_snippet_needs_confirmation_and_cleans_profiles() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Snippets);
    key(&mut app, KeyCode::Char('d'));
    key(&mut app, KeyCode::Char('n')); // cancel
    assert_eq!(app.snippets, ["20-aliases", "vim-basic.vim", "vim-dark.vim", "vim-indent-2.vim", "vim-indent-4.vim", "vim-indent-tabs.vim", "vim-keys.vim", "vim-mouse.vim", "vim-no-traces.vim", "vim-recommended.vim"]);
    key(&mut app, KeyCode::Char('d'));
    key(&mut app, KeyCode::Char('y'));
    assert_eq!(app.snippets, ["vim-basic.vim", "vim-dark.vim", "vim-indent-2.vim", "vim-indent-4.vim", "vim-indent-tabs.vim", "vim-keys.vim", "vim-mouse.vim", "vim-no-traces.vim", "vim-recommended.vim"]);
    assert!(app.cfg.profiles["default"].snippets.is_empty());
}

#[test]
fn duplicate_snippet_prompts_with_a_copy_name_and_copies_the_text() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Snippets);
    key(&mut app, KeyCode::Char('c'));
    assert!(matches!(&app.mode, Mode::Input(InputKind::DuplicateSnippet(f), b) if f == "20-aliases" && b == "20-aliases-copy"));
    assert!(render(&mut app).contains("copy of snippet 20-aliases as: 20-aliases-copy"));
    key(&mut app, KeyCode::Enter);
    assert_eq!(app.snippets, ["20-aliases", "20-aliases-copy", "vim-basic.vim", "vim-dark.vim", "vim-indent-2.vim", "vim-indent-4.vim", "vim-indent-tabs.vim", "vim-keys.vim", "vim-mouse.vim", "vim-no-traces.vim", "vim-recommended.vim"]);
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
    key(&mut app, KeyCode::Char('c'));
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
    key(&mut app, KeyCode::Char('c'));
    for _ in 0.."-copy".len() {
        key(&mut app, KeyCode::Backspace);
    }
    key(&mut app, KeyCode::Enter); // same name as the source
    assert!(app.status.as_ref().is_some_and(|s| s.1), "{:?}", app.status);
    key(&mut app, KeyCode::Char('c'));
    typed(&mut app, " bad");
    key(&mut app, KeyCode::Enter);
    assert!(app.status.as_ref().is_some_and(|s| s.1), "{:?}", app.status);
    key(&mut app, KeyCode::Char('c'));
    key(&mut app, KeyCode::Esc);
    assert_eq!(app.snippets, ["20-aliases", "vim-basic.vim", "vim-dark.vim", "vim-indent-2.vim", "vim-indent-4.vim", "vim-indent-tabs.vim", "vim-keys.vim", "vim-mouse.vim", "vim-no-traces.vim", "vim-recommended.vim"]);
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn duplicate_profile_copies_snippets_and_prompt_and_selects_the_copy() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    key(&mut app, KeyCode::Char('c'));
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
    key(&mut app, KeyCode::Char('c'));
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
    key(&mut app, KeyCode::Down); // the default theme -> the next one
    let at = prompt::THEMES.iter().position(|t| t.0 == prompt::DEFAULT_THEME).unwrap();
    let next = prompt::THEMES[at + 1].0;
    assert_eq!(prompt_of(&app, "default").as_deref(), Some(next));
    assert!(render(&mut app).contains(&format!("(•) {next}")));
    assert_eq!(Config::load(&app.dir).unwrap(), app.cfg); // saved immediately
    for _ in 0..20 {
        key(&mut app, KeyCode::Down); // stops at the last theme
    }
    assert_eq!(prompt_of(&app, "default").as_deref(), Some(prompt::THEMES.last().unwrap().0));
    for _ in 0..20 {
        key(&mut app, KeyCode::Up); // stops at "(server's own)"
    }
    assert_eq!(prompt_of(&app, "default"), None);
    assert!(render(&mut app).contains("the server keeps its own prompt"));
}

#[test]
fn clicking_a_theme_picks_it_and_wheel_moves_it() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    render(&mut app);
    let l = app.hits.prompt_list;
    click(&mut app, l.x + 2, l.y + 1); // first theme: minimal
    assert_eq!(prompt_of(&app, "default").as_deref(), Some("minimal"));
    assert_eq!(app.prof_focus, ProfFocus::Prompt);
    wheel(&mut app, l.x + 2, l.y + 1, false);
    assert_eq!(prompt_of(&app, "default").as_deref(), Some("green"));
    click(&mut app, l.x + 2, l.y); // "(server's own)"
    assert_eq!(prompt_of(&app, "default"), None);
}

#[test]
fn each_profile_keeps_its_own_prompt() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    key(&mut app, KeyCode::Char('n'));
    typed(&mut app, "work");
    key(&mut app, KeyCode::Enter); // selects the new profile, which has no prompt yet
    assert_eq!(prompt_of(&app, "work"), None);
    key(&mut app, KeyCode::Right);
    key(&mut app, KeyCode::Down); // minimal
    key(&mut app, KeyCode::Down); // green
    assert_eq!(prompt_of(&app, "work").as_deref(), Some("green"));
    assert_eq!(prompt_of(&app, "default").as_deref(), Some(prompt::DEFAULT_THEME));
    key(&mut app, KeyCode::Down);
    assert_eq!(prompt_of(&app, "work").as_deref(), Some("two-line"));
    assert_eq!(prompt_of(&app, "default").as_deref(), Some(prompt::DEFAULT_THEME)); // untouched
}

#[test]
fn prompt_choice_changes_the_payload() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    let with_theme = config::build_rc(&app.dir, &app.cfg, "default");
    assert!(with_theme.contains("PS1="));
    key(&mut app, KeyCode::Right);
    for _ in 0..10 {
        key(&mut app, KeyCode::Up);
    }
    assert!(!config::build_rc(&app.dir, &app.cfg, "default").contains("PS1="));
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
    key(&mut app, KeyCode::Char('n'));
    typed(&mut app, "work");
    key(&mut app, KeyCode::Enter);
    assert!(app.cfg.profiles.contains_key("work"));
    key(&mut app, KeyCode::Char('*'));
    assert_eq!(app.cfg.default_profile, "work");
    assert!(render(&mut app).contains("work *"));
    key(&mut app, KeyCode::Char('d'));
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
    assert!(app.hits.chk_cols > 1, "cols {}", app.hits.chk_cols);
    let screen = render(&mut app);
    let line = screen.lines().find(|l| l.contains("snip-00")).unwrap();
    assert!(line.contains("snip-01"), "first row holds several snippets: {line}");
}

#[test]
fn snippet_grid_navigates_by_row_and_column_and_clicks_hit_the_right_cell() {
    let (_t, mut app) = app();
    many_snippets(&mut app, 12);
    app.switch_tab(Tab::Profiles);
    render(&mut app);
    let cols = app.hits.chk_cols;
    key(&mut app, KeyCode::Right);
    key(&mut app, KeyCode::Right); // focus snippets
    assert_eq!(app.prof_focus, ProfFocus::Snippets);
    key(&mut app, KeyCode::Right);
    assert_eq!(app.chk_state.selected(), Some(1));
    key(&mut app, KeyCode::Down);
    assert_eq!(app.chk_state.selected(), Some(1 + cols));
    key(&mut app, KeyCode::Up);
    key(&mut app, KeyCode::Left);
    key(&mut app, KeyCode::Left); // first column: back to the prompt list
    assert_eq!(app.prof_focus, ProfFocus::Prompt);
    let c = app.hits.chk_list;
    let w = app.hits.chk_colw;
    click(&mut app, c.x + w + 1, c.y); // second cell of the first row
    assert_eq!(app.chk_state.selected(), Some(1));
    assert!(app.cfg.profiles["default"].snippets.contains(&"snip-01".to_string()));
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
    key(&mut app, KeyCode::Char('n'));
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
    key(&mut app, KeyCode::Char('e'));
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
    key(&mut app, KeyCode::Char('e'));
    assert_eq!(app.editor_name.as_deref(), Some("fino-2.prompt"));
    assert!(config::read_snippet(&app.dir, "fino.prompt").unwrap().starts_with("mine "));
}

#[test]
fn e_with_the_servers_own_prompt_starts_a_new_theme_from_the_default() {
    let (_t, mut app) = app();
    app.switch_tab(Tab::Profiles);
    key(&mut app, KeyCode::Right);
    app.set_prompt_index(0);
    key(&mut app, KeyCode::Char('e'));
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
    assert_eq!(sort_key_at(0), SortKey::Smart);
    assert_eq!(sort_key_at(COL_STAR), SortKey::Host);
    assert_eq!(sort_key_at(COL_STAR + COL_ALIAS), SortKey::Target);
    assert_eq!(sort_key_at(COL_STAR + COL_ALIAS + COL_TARGET), SortKey::Profile);
    assert_eq!(sort_key_at(COL_STAR + COL_ALIAS + COL_TARGET + COL_PROFILE), SortKey::Source);
    assert_eq!(sort_key_at(COL_STAR + COL_ALIAS + COL_TARGET + COL_PROFILE + COL_SOURCE), SortKey::LastUsed);
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
    let mut term = Terminal::new(TestBackend::new(100, 24)).unwrap();
    term.draw(|f| app.draw(f)).unwrap();
    let list = app.hits.snip_list;
    let buf = term.backend().buffer();
    for (i, name) in app.snippets.iter().enumerate() {
        let want = match name.as_str() {
            "20-aliases" => Color::Green,
            "t.tmux" => Color::Yellow,
            "i.inputrc" => Color::Blue,
            "p.prompt" => Color::Magenta,
            _ => Color::Cyan, // the vim presets
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
