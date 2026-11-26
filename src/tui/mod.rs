//! Terminal UI: Hosts (pick and connect), Snippets (edit rc pieces), Profiles (choose snippets), Envs (prod, staging, ...).

use std::cmp::Ordering;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::crossterm::cursor::Show;
use ratatui::crossterm::execute;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs};
use ratatui::{DefaultTerminal, Frame};

use crate::config::{self, Config, SortKey};
use crate::editor::{Editor, Move};
use crate::highlight;
use crate::history;
use crate::log;
use crate::SshOutcome;
use crate::payload;
use crate::prompt;
use crate::ssh_config::{self, HostEntry, Source};

const TAB_TITLES: [&str; 4] = ["Hosts", "Snippets", "Profiles", "Envs"];
const DOUBLE_CLICK: Duration = Duration::from_millis(400);

/// The text boxes that a mouse drag can select in.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TextDrag {
    Snippet,
    Search,
    Env,
    Form,
}

/// The cursor move that a key makes in a text box. A box of one line has no move by rows.
fn move_of(code: KeyCode, multiline: bool) -> Option<Move> {
    Some(match code {
        KeyCode::Left => Move::Left,
        KeyCode::Right => Move::Right,
        KeyCode::Home => Move::Home,
        KeyCode::End => Move::End,
        KeyCode::Up if multiline => Move::Rows(-1),
        KeyCode::Down if multiline => Move::Rows(1),
        KeyCode::PageUp if multiline => Move::Rows(-10),
        KeyCode::PageDown if multiline => Move::Rows(10),
        _ => return None,
    })
}

/// Selecting in a text box, the same in all of them: `Shift` with a move key extends the selection
/// and `Ctrl-A` selects all. True when the key was one of these.
fn select_key(e: &mut Editor, k: KeyEvent, multiline: bool) -> bool {
    if k.code == KeyCode::Char('a') && k.modifiers.contains(KeyModifiers::CONTROL) {
        e.select_all();
        return true;
    }
    match move_of(k.code, multiline) {
        Some(mv) if k.modifiers.contains(KeyModifiers::SHIFT) => {
            e.extend(mv);
            true
        }
        _ => false,
    }
}

/// The keys of a one-line text box, the same in all of them: the cursor moves, selecting, typing
/// and deleting. True when the key was one of these.
fn edit_key(e: &mut Editor, k: KeyEvent) -> bool {
    if select_key(e, k, false) {
        return true;
    }
    match k.code {
        KeyCode::Backspace => e.backspace(),
        KeyCode::Delete => e.delete(),
        KeyCode::Char(c) if !k.modifiers.contains(KeyModifiers::CONTROL) => e.insert_char(c),
        code => match move_of(code, false) {
            Some(mv) => e.go(mv),
            None => return false,
        },
    }
    true
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tab {
    Hosts,
    Snippets,
    Profiles,
    Envs,
}

impl Tab {
    fn index(self) -> usize {
        self as usize
    }

    fn from_index(i: usize) -> Tab {
        [Tab::Hosts, Tab::Snippets, Tab::Profiles, Tab::Envs][i % 4]
    }
}

#[derive(Clone, Debug)]
enum InputKind {
    NewSnippet,
    NewProfile,
    /// Holds the name of the snippet being copied.
    DuplicateSnippet(String),
    /// Holds the name of the profile being copied.
    DuplicateProfile(String),
    NewEnv,
    /// Holds the name of the environment being copied.
    DuplicateEnv(String),
    /// Holds the name of the environment being renamed.
    RenameEnv(String),
}

#[derive(Clone, Debug)]
enum Pending {
    Snippet(String),
    Profile(String),
    Env(String),
    /// Holds the ssh arguments of the connection to make.
    Connect(Vec<String>),
}

#[derive(Clone, Debug)]
enum Mode {
    Normal,
    Input(InputKind, String),
    Confirm(Pending),
    /// Choosing the extra snippets of one host (holds its name in `config.toml`).
    HostSnippets(String),
    /// Editing the connection options of one host.
    HostForm(Box<HostForm>),
    /// Choosing one of a host's named options to connect with.
    OptionsPicker(Box<OptionsPicker>),
}

/// Which pane of the Profiles tab has the keyboard.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ProfFocus {
    List,
    Prompt,
    Snippets,
}

#[derive(Default)]
struct Hits {
    tabs: Vec<Rect>,
    hosts_head: Rect,
    /// The text row of the search box.
    search: Rect,
    hosts_list: Rect,
    snip_list: Rect,
    editor: Rect,
    prof_list: Rect,
    /// The two terminal switches under the profile list: the title row, then the marks row.
    prof_term: Rect,
    env_list: Rect,
    env_fields: Rect,
    prompt_list: Rect,
    chk_list: Rect,
    /// Snippet checklist grid, filled column by column: items per column and width of one cell.
    chk_rows: usize,
    chk_colw: u16,
    /// Rows of the per-host snippet popup.
    popup_list: Rect,
    /// The connection options form: its frame, the rows of its fields, the options names and the
    /// clickable key hints.
    form_area: Rect,
    form_fields: Rect,
    form_options: Vec<(Rect, Option<String>)>,
    form_keys: Vec<(Rect, KeyEvent)>,
    /// Widths of the host list's adaptive columns, as last drawn.
    cols: hosts::Cols,
    /// The open list of private keys under the identity field: row, key path.
    form_keylist: Vec<(Rect, String)>,
    /// The options picker: its frame and its rows.
    picker_area: Rect,
    extras_area: Rect,
    picker_list: Rect,
}

pub struct App {
    dir: PathBuf,
    cfg: Config,
    hosts: Vec<HostEntry>,
    tab: Tab,
    mode: Mode,
    /// The search box: one line, edited with the same editor as the snippets.
    filter: Editor,
    host_state: ListState,
    snippets: Vec<String>,
    snip_state: ListState,
    editor: Editor,
    editor_name: Option<String>,
    editor_focus: bool,
    prof_state: ListState,
    prompt_state: ListState,
    /// The row you are looking at in the prompt list, with the profile it belongs to. It is only a
    /// preview until `Enter` applies it. `None`: show the profile's own prompt.
    prompt_cursor: Option<(String, usize)>,
    chk_state: ListState,
    /// First visible grid row of the snippet checklist.
    chk_top: usize,
    /// Selection in the per-host snippet popup.
    host_chk: ListState,
    /// Recent sessions of the selected host: (host, newest first, total count). Cleared after a
    /// connection so the list is read again.
    history_cache: Option<(String, Vec<history::Entry>, usize)>,
    /// Payload size of one profile, so drawing does not rebuild it every frame. Cleared on change.
    payload_cache: Option<(String, Option<usize>)>,
    prof_focus: ProfFocus,
    /// The Envs tab: the selected environment, which pane has the keyboard, the focused form row and
    /// the text being typed into it.
    env_state: ListState,
    env_focus: envs::EnvFocus,
    env_field: usize,
    env_buf: Editor,
    /// Mouse capture on. Off lets the terminal select text; toggled with F2.
    mouse: bool,
    status: Option<(String, bool)>,
    quit: bool,
    hits: Hits,
    last_click: Option<(Instant, Tab, usize)>,
    /// ssh arguments to run once the event handler returns (needs the terminal).
    pending_connect: Option<Vec<String>>,
    /// How far the options popup or the picker was dragged from the centre, and where a drag last was.
    popup_offset: (i32, i32),
    popup_drag: Option<(u16, u16)>,
    /// The text box that a drag of the mouse is selecting in, from the click that started it.
    text_drag: Option<TextDrag>,
    /// Where the private keys offered for the identity file are looked up (`~/.ssh`).
    ssh_dir: Option<PathBuf>,
}

fn inside(r: Rect, x: u16, y: u16) -> bool {
    x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

fn truncate(s: &str, w: u16) -> String {
    let w = w as usize;
    if s.chars().count() <= w {
        format!("{s:<w$}")
    } else {
        let mut t: String = s.chars().take(w.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

impl App {
    pub fn new(dir: PathBuf, cfg: Config, hosts: Vec<HostEntry>) -> App {
        let snippets = config::list_snippets(&dir);
        let mut app = App {
            dir,
            cfg,
            hosts,
            tab: Tab::Hosts,
            mode: Mode::Normal,
            filter: Editor::from_text(""),
            host_state: ListState::default().with_selected(Some(0)),
            snippets,
            snip_state: ListState::default().with_selected(Some(0)),
            editor: Editor::from_text(""),
            editor_name: None,
            editor_focus: false,
            prof_state: ListState::default().with_selected(Some(0)),
            prompt_state: ListState::default().with_selected(Some(0)),
            prompt_cursor: None,
            chk_state: ListState::default().with_selected(Some(0)),
            chk_top: 0,
            host_chk: ListState::default().with_selected(Some(0)),
            history_cache: None,
            payload_cache: None,
            mouse: true,
            prof_focus: ProfFocus::List,
            env_state: ListState::default().with_selected(Some(0)),
            env_focus: envs::EnvFocus::List,
            env_field: 0,
            env_buf: Editor::from_text(""),
            status: None,
            quit: false,
            hits: Hits::default(),
            last_click: None,
            pending_connect: None,
            popup_offset: (0, 0),
            popup_drag: None,
            text_drag: None,
            ssh_dir: config::home_dir().map(|h| h.join(".ssh")),
        };
        // Hosts used before but missing from ssh config/known_hosts still show up.
        let known: Vec<String> = app.hosts.iter().map(|h| h.alias.clone()).collect();
        for name in app.cfg.hosts.keys().filter(|k| !known.contains(k)).cloned().collect::<Vec<_>>() {
            app.hosts.push(HostEntry {
                alias: name,
                hostname: None,
                user: None,
                port: None,
                source: Source::KnownHosts,
            });
        }
        app.load_editor();
        app.load_env_buf();
        app
    }


    fn say(&mut self, msg: impl Into<String>) {
        self.status = Some((msg.into(), false));
    }

    fn fail(&mut self, msg: impl Into<String>) {
        self.status = Some((msg.into(), true));
    }

    fn save_cfg(&mut self) {
        self.payload_cache = None;
        if let Err(e) = self.cfg.save(&self.dir) {
            self.fail(format!("cannot save config: {e}"));
        }
    }

    fn is_double_click(&mut self, tab: Tab, idx: usize) -> bool {
        let now = Instant::now();
        let double = self.last_click.is_some_and(|(t, tb, i)| tb == tab && i == idx && now - t < DOUBLE_CLICK);
        self.last_click = if double { None } else { Some((now, tab, idx)) };
        double
    }

    fn move_sel(state: &mut ListState, len: usize, delta: isize) {
        if len == 0 {
            state.select(None);
            return;
        }
        let cur = state.selected().unwrap_or(0) as isize;
        state.select(Some((cur + delta).clamp(0, len as isize - 1) as usize));
    }

    fn switch_tab(&mut self, tab: Tab) {
        self.flush_editor();
        self.editor_focus = false;
        self.prof_focus = ProfFocus::List;
        self.prompt_cursor = None;
        self.env_focus = envs::EnvFocus::List;
        self.tab = tab;
    }



    pub fn on_event(&mut self, ev: Event) {
        match ev {
            Event::Key(k) if k.kind == KeyEventKind::Press => self.on_key(k),
            Event::Mouse(m) => self.on_mouse(m),
            _ => {}
        }
    }

    fn on_key(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && k.code == KeyCode::Char('c') {
            self.flush_editor();
            self.quit = true;
            return;
        }
        if k.code == KeyCode::F(2) {
            self.mouse = !self.mouse;
            let msg = if self.mouse { "mouse on" } else { "mouse off: select text with the mouse, F2 to turn it back on" };
            self.say(msg.to_string());
            return;
        }
        match std::mem::replace(&mut self.mode, Mode::Normal) {
            Mode::Input(kind, buf) => self.on_key_input(k, kind, buf),
            Mode::Confirm(p) => self.on_key_confirm(k, p),
            Mode::HostSnippets(host) => self.on_key_host_snippets(k, host),
            Mode::HostForm(form) => self.on_key_host_form(k, *form),
            Mode::OptionsPicker(pick) => self.on_key_options_picker(k, *pick),
            Mode::Normal => {
                self.status = None;
                match self.tab {
                    Tab::Hosts => self.on_key_hosts(k, ctrl),
                    Tab::Snippets => self.on_key_snippets(k, ctrl),
                    Tab::Profiles => self.on_key_profiles(k),
                    Tab::Envs => self.on_key_envs(k),
                }
            }
        }
    }

    fn global_nav(&mut self, k: KeyEvent) -> bool {
        match k.code {
            KeyCode::Tab => self.switch_tab(Tab::from_index(self.tab.index() + 1)),
            KeyCode::BackTab => self.switch_tab(Tab::from_index(self.tab.index() + TAB_TITLES.len() - 1)),
            _ => return false,
        }
        true
    }

    fn on_key_input(&mut self, k: KeyEvent, kind: InputKind, mut buf: String) {
        match k.code {
            KeyCode::Esc => {}
            KeyCode::Enter => match kind {
                InputKind::NewSnippet => self.create_snippet(&buf),
                InputKind::NewProfile => self.create_profile(&buf),
                InputKind::DuplicateSnippet(from) => self.duplicate_snippet(&from, &buf),
                InputKind::DuplicateProfile(from) => self.duplicate_profile(&from, &buf),
                InputKind::NewEnv => self.create_env(&buf),
                InputKind::DuplicateEnv(from) => self.duplicate_env(&from, &buf),
                InputKind::RenameEnv(from) => self.rename_env(&from, &buf),
            },
            KeyCode::Backspace => {
                buf.pop();
                self.mode = Mode::Input(kind, buf);
            }
            KeyCode::Char(c) => {
                buf.push(c);
                self.mode = Mode::Input(kind, buf);
            }
            _ => self.mode = Mode::Input(kind, buf),
        }
    }

    fn on_key_confirm(&mut self, k: KeyEvent, pending: Pending) {
        if !matches!(k.code, KeyCode::Char('y' | 'Y')) {
            return self.say("cancelled");
        }
        match pending {
            Pending::Snippet(n) => self.delete_snippet(&n),
            Pending::Profile(n) => self.delete_profile(&n),
            Pending::Env(n) => self.delete_env(&n),
            Pending::Connect(args) => self.pending_connect = Some(args),
        }
    }

    /// Selects with the mouse in the box that the drag started in. True when a drag was handled.
    fn drag_text(&mut self, x: u16, y: u16) -> bool {
        // A position outside the text counts as one column or line beyond its nearest edge.
        let beyond = |v: u16, start: u16, len: u16| -> isize {
            if v < start {
                -1
            } else if v >= start.saturating_add(len) {
                len as isize
            } else {
                (v - start) as isize
            }
        };
        if !matches!(self.mode, Mode::Normal) && self.text_drag != Some(TextDrag::Form) {
            return false;
        }
        match self.text_drag {
            Some(TextDrag::Form) => self.drag_form_text(x),
            Some(TextDrag::Snippet) => {
                let a = self.hits.editor;
                self.editor.drag(beyond(x, a.x, a.width), beyond(y, a.y, a.height));
            }
            Some(TextDrag::Search) => {
                let a = self.hits.search;
                self.filter.drag(beyond(x, a.x + 3, a.width.saturating_sub(3)), 0);
            }
            Some(TextDrag::Env) => {
                let a = self.hits.env_fields;
                self.env_buf.drag(beyond(x, a.x + envs::NAME_WIDTH, a.width.saturating_sub(envs::NAME_WIDTH)), 0);
            }
            None => return false,
        }
        true
    }

    fn on_mouse(&mut self, m: MouseEvent) {
        let (x, y) = (m.column, m.row);
        if matches!(m.kind, MouseEventKind::Up(MouseButton::Left)) {
            self.text_drag = None;
        }
        let flip = if self.cfg.invert_scroll { -1 } else { 1 };
        let wheel = match m.kind {
            MouseEventKind::ScrollUp => Some(-flip),
            MouseEventKind::ScrollDown => Some(flip),
            _ => None,
        };
        if self.drag_options_popup(&m) {
            return;
        }
        let click = matches!(m.kind, MouseEventKind::Down(MouseButton::Left));
        if click {
            self.text_drag = None;
        }
        if matches!(m.kind, MouseEventKind::Drag(MouseButton::Left)) && self.drag_text(x, y) {
            return;
        }
        if !click && wheel.is_none() {
            return;
        }

        if let Mode::HostSnippets(host) = self.mode.clone() {
            let len = self.grid_items().len();
            if let Some(d) = wheel {
                Self::move_sel(&mut self.host_chk, len, d);
            } else if inside(self.hits.popup_list, x, y) {
                let i = self.host_chk.offset() + (y - self.hits.popup_list.y) as usize;
                if i < len {
                    self.host_chk.select(Some(i));
                    self.toggle_host_extra(&host);
                }
            } else {
                self.mode = Mode::Normal; // a click outside closes the popup
            }
            return;
        }
        match std::mem::replace(&mut self.mode, Mode::Normal) {
            Mode::HostForm(form) => return self.on_mouse_host_form(*form, (x, y), wheel, click),
            Mode::OptionsPicker(pick) => return self.on_mouse_options_picker(*pick, (x, y), wheel, click),
            other => self.mode = other,
        }
        if !matches!(self.mode, Mode::Normal) {
            return;
        }

        if click && let Some(i) = self.hits.tabs.iter().position(|r| inside(*r, x, y)) {
            return self.switch_tab(Tab::from_index(i));
        }
        match self.tab {
            Tab::Hosts => self.mouse_hosts(x, y, wheel, click),
            Tab::Snippets => self.mouse_snippets(x, y, wheel, click),
            Tab::Profiles => self.mouse_profiles(x, y, wheel, click),
            Tab::Envs => self.mouse_envs(x, y, wheel, click),
        }
    }


    pub fn draw(&mut self, f: &mut Frame) {
        let area = f.area();
        let [tabs_a, body, foot] =
            Layout::vertical([Constraint::Length(3), Constraint::Min(3), Constraint::Length(2)]).areas(area);
        self.draw_tabs(f, tabs_a);
        match self.tab {
            Tab::Hosts => self.draw_hosts(f, body),
            Tab::Snippets => self.draw_snippets(f, body),
            Tab::Profiles => self.draw_profiles(f, body),
            Tab::Envs => self.draw_envs(f, body),
        }
        if let Mode::HostForm(form) = &mut self.mode {
            form.keep_visible(hostedit::text_width(body.width));
        }
        match self.mode.clone() {
            Mode::HostSnippets(host) => self.draw_host_snippets(f, body, &host),
            Mode::HostForm(form) => self.draw_host_form(f, body, &form),
            Mode::OptionsPicker(pick) => self.draw_set_picker(f, body, &pick),
            _ => {}
        }
        self.draw_footer(f, foot);
    }

    fn draw_tabs(&mut self, f: &mut Frame, area: Rect) {
        let block = Block::bordered().title(format!(" sx {} ", env!("CARGO_PKG_VERSION")));
        let inner = block.inner(area);
        let tabs = Tabs::new(TAB_TITLES)
            .select(self.tab.index())
            .highlight_style(Style::new().fg(Color::Black).bg(Color::Cyan))
            .block(block);
        f.render_widget(tabs, area);
        // Tabs draws " title " cells separated by one divider column.
        self.hits.tabs.clear();
        let mut x = inner.x;
        for t in TAB_TITLES {
            let w = t.len() as u16 + 2;
            self.hits.tabs.push(Rect::new(x, inner.y, w, 1));
            x += w + 1;
        }
    }

    /// Vertical scrollbar along the right edge of `area`, only when `total` rows do not fit.
    fn scrollbar(f: &mut Frame, area: Rect, total: usize, offset: usize) {
        let visible = area.height as usize;
        if total <= visible || visible == 0 {
            return;
        }
        let mut state = ScrollbarState::new(total - visible + 1).position(offset).viewport_content_length(visible);
        let bar = Scrollbar::new(ScrollbarOrientation::VerticalRight).begin_symbol(None).end_symbol(None);
        f.render_stateful_widget(bar, area, &mut state);
    }

}

mod envs;
mod footer;
mod hostedit;
mod hosts;
use hostedit::{HostForm, OptionsPicker};
mod profiles;
mod snippets;
use snippets::kind_color;
mod terminal;

pub use terminal::run;

#[cfg(test)]
mod tests;
