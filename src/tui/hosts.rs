//! Hosts tab: the host list with sorting, favorites, per-host snippets and the history pane.

use super::*;

// Column widths in the hosts list. The host, target and profile columns have these as their
// smallest width and grow with their content when the terminal is wide enough, see `Cols`.
pub(super) const COL_STAR: u16 = 2;

pub(super) const COL_ALIAS: u16 = 22;

pub(super) const COL_TARGET: u16 = 30;

pub(super) const COL_PROFILE: u16 = 12;

/// Wide enough for `known_hosts`.
pub(super) const COL_SOURCE: u16 = 12;

const COL_USED: u16 = 12;

/// The environment (`prod`, ...). A label longer than this is cut.
pub(super) const COL_ENV: u16 = 10;

/// The widths of the columns that adapt to the terminal and the hosts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Cols {
    pub alias: u16,
    pub target: u16,
    pub profile: u16,
}

impl Default for Cols {
    fn default() -> Cols {
        Cols { alias: COL_ALIAS, target: COL_TARGET, profile: COL_PROFILE }
    }
}

impl Cols {
    /// Starts from the smallest widths and shares the room left over in `width` among the columns
    /// whose longest entry (`want`, with two spaces of gap) does not fit yet, each in proportion to what it lacks.
    pub(super) fn fit(width: u16, want: [u16; 3]) -> Cols {
        let min = Cols::default();
        let min = [min.alias, min.target, min.profile];
        let room = width.saturating_sub(COL_STAR + COL_ENV + COL_SOURCE + COL_USED);
        let extra = u32::from(room.saturating_sub(min.iter().sum()));
        let lack: Vec<u32> = (0..3).map(|i| u32::from(want[i].saturating_sub(min[i]))).collect();
        let total: u32 = lack.iter().sum();
        let grant = |i: usize| (lack[i] * extra.min(total) / total.max(1)) as u16;
        Cols { alias: min[0] + grant(0), target: min[1] + grant(1), profile: min[2] + grant(2) }
    }
}

/// Which sort a click at `col` (relative to the header's left edge) selects.
pub(super) fn sort_key_at(col: u16, cols: Cols) -> SortKey {
    let host = COL_STAR;
    let target = host + cols.alias;
    let profile = target + cols.target;
    let env = profile + cols.profile;
    let source = env + COL_ENV;
    if col < host {
        SortKey::Smart
    } else if col < target {
        SortKey::Host
    } else if col < profile {
        SortKey::Target
    } else if col < env {
        SortKey::Profile
    } else if col < source {
        SortKey::Env
    } else if col < source + COL_SOURCE {
        SortKey::Source
    } else {
        SortKey::LastUsed
    }
}

/// Sessions shown in the history pane.
pub(super) const HISTORY_ROWS: usize = 5;

/// `2h ago    3m 12s   ok`, with the outcome colored.
pub(super) fn history_line(e: &history::Entry) -> Line<'static> {
    let color = match e.outcome.as_str() {
        "ok" => Color::Green,
        "failed" => Color::Red,
        _ => Color::Yellow,
    };
    Line::from(vec![
        Span::raw(format!(" {:<10}", ago(e.when))),
        Span::styled(format!("{:<9}", history::format_secs(e.secs)), Style::new().add_modifier(Modifier::DIM)),
        Span::styled(e.outcome.clone(), Style::new().fg(color)),
    ])
}

pub(super) fn ago(secs: u64) -> String {
    let d = config::now_secs().saturating_sub(secs);
    match d {
        0..=59 => "just now".into(),
        60..=3599 => format!("{}m ago", d / 60),
        3600..=86399 => format!("{}h ago", d / 3600),
        _ => format!("{}d ago", d / 86400),
    }
}

/// Subsequence match: every char of `needle` appears in `hay` in order (case-insensitive).
pub(super) fn fuzzy(needle: &str, hay: &str) -> bool {
    fuzzy_positions(needle, hay).is_some()
}

/// The char positions in `hay` that a subsequence match of `needle` uses, taking the first
/// occurrence of each char in turn. `None` when `needle` does not match.
pub(super) fn fuzzy_positions(needle: &str, hay: &str) -> Option<Vec<usize>> {
    let same = |a: char, b: char| a == b || a.to_lowercase().eq(b.to_lowercase());
    let mut at = hay.chars().enumerate();
    needle.chars().map(|n| at.find(|&(_, h)| same(h, n)).map(|(i, _)| i)).collect()
}

/// `text` cut to `width` columns, as spans where the chars at `marked` (char positions in `text`) stand out.
fn marked_spans(text: &str, width: u16, marked: &[usize], base: Style) -> Vec<Span<'static>> {
    let shown = truncate(text, width);
    let cut = text.chars().count() > width as usize;
    let stand_out = base.patch(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD).remove_modifier(Modifier::DIM));
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut run = String::new();
    let mut run_on = false;
    for (i, c) in shown.chars().enumerate() {
        let on = marked.contains(&i) && !(cut && i + 1 == width as usize);
        if on != run_on && !run.is_empty() {
            spans.push(Span::styled(std::mem::take(&mut run), if run_on { stand_out } else { base }));
        }
        run_on = on;
        run.push(c);
    }
    if !run.is_empty() {
        spans.push(Span::styled(run, if run_on { stand_out } else { base }));
    }
    spans
}

/// The tallest the extra snippets popup gets, borders included. Longer lists scroll.
const MAX_POPUP_HEIGHT: u16 = 16;

impl App {
    /// Indices into `self.hosts` that match the filter, in the configured sort order.
    pub(super) fn visible(&self) -> Vec<usize> {
        let (_, needle) = self.filter_parts();
        let mut idx: Vec<usize> = (0..self.hosts.len())
            .filter(|&i| {
                let h = &self.hosts[i];
                fuzzy(needle, &format!("{} {}", h.alias, self.target_of(h)))
            })
            .collect();
        idx.sort_by(|&a, &b| self.compare(a, b));
        idx
    }

    /// The search text as `(user, host part)`: `root@web` means "connect to the host matching `web`
    /// as `root`". Text with spaces is ssh arguments, not a host search, and has no user.
    pub(super) fn filter_parts(&self) -> (Option<&str>, &str) {
        match self.filter_text().split_once('@') {
            Some((user, rest)) if !user.is_empty() && !self.filter_text().contains(char::is_whitespace) => (Some(user), rest),
            _ => (None, self.filter_text()),
        }
    }

    /// `user@host:port` with the user and port the host's options replace.
    pub(super) fn target_of(&self, h: &HostEntry) -> String {
        match self.cfg.hosts.get(&h.alias) {
            Some(p) => h.target_with(Some(p.user.trim()).filter(|s| !s.is_empty()), Some(p.port.trim()).filter(|s| !s.is_empty())),
            None => h.target(),
        }
    }

    /// Orders two hosts by the configured key. Hosts never used always sort last under "last used".
    pub(super) fn compare(&self, a: usize, b: usize) -> Ordering {
        let (ha, hb) = (&self.hosts[a], &self.hosts[b]);
        let (pa, pb) = (self.cfg.hosts.get(&ha.alias), self.cfg.hosts.get(&hb.alias));
        let by_name = || ha.alias.to_lowercase().cmp(&hb.alias.to_lowercase());
        let used = |p: Option<&config::HostPrefs>| p.and_then(|p| p.last_used);
        let profile = |h: &HostEntry| self.cfg.profile_for(&h.alias).unwrap_or("off").to_string();
        // Hosts without an environment come after the ones that have one.
        let env_rank = |h: &HostEntry| {
            let tag = self.cfg.env_of(&h.alias).map(|(n, _)| n.to_string());
            (tag.is_none(), tag.unwrap_or_default())
        };
        let reverse = self.cfg.sort_reverse;
        let directed = |o: Ordering| if reverse { o.reverse() } else { o };

        match self.cfg.sort {
            SortKey::Smart => {
                let fav = |p: Option<&config::HostPrefs>| p.is_some_and(|p| p.favorite);
                fav(pb)
                    .cmp(&fav(pa))
                    .then(used(pb).cmp(&used(pa)))
                    .then_with(by_name)
            }
            SortKey::Host => directed(by_name()),
            SortKey::Target => directed(
                self.target_of(ha).to_lowercase().cmp(&self.target_of(hb).to_lowercase()).then_with(by_name),
            ),
            SortKey::Profile => directed(profile(ha).cmp(&profile(hb)).then_with(by_name)),
            SortKey::Env => directed(env_rank(ha).cmp(&env_rank(hb)).then_with(by_name)),
            SortKey::Source => directed(ha.source.label().cmp(hb.source.label()).then_with(by_name)),
            SortKey::LastUsed => match (used(pa), used(pb)) {
                (Some(x), Some(y)) => directed(y.cmp(&x)).then_with(by_name), // newest first
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => by_name(),
            },
        }
    }

    /// True when the list currently runs from high to low (newest first, or reversed).
    pub(super) fn sort_descending(&self) -> bool {
        (self.cfg.sort == SortKey::LastUsed) != self.cfg.sort_reverse
    }

    /// Applies a sort change, saves it, and keeps the same host selected.
    pub(super) fn change_sort(&mut self, f: impl FnOnce(&mut Config)) {
        self.keep_host_selected(|app| {
            f(&mut app.cfg);
            app.save_cfg();
        });
    }

    /// Runs `f`, which may reorder the list, and leaves the cursor on the host it was on.
    pub(super) fn keep_host_selected(&mut self, f: impl FnOnce(&mut App)) {
        let keep = self.selected_host().map(|i| self.hosts[i].alias.clone());
        f(self);
        let vis = self.visible();
        let pos = keep
            .and_then(|alias| vis.iter().position(|&i| self.hosts[i].alias == alias))
            .unwrap_or(0);
        self.host_state.select(Some(pos));
    }

    /// Records a connection to `host`. Under "last used" the host moves to the top, and the cursor goes with it.
    pub(super) fn note_connected(&mut self, host: &str) {
        self.keep_host_selected(|app| {
            app.cfg.mark_used(host);
            app.save_cfg();
        });
    }

    /// Header click: pick a column, or reverse it when it is already the sort column.
    pub(super) fn click_sort(&mut self, key: SortKey) {
        self.change_sort(|cfg| {
            if cfg.sort == key && key != SortKey::Smart {
                cfg.sort_reverse = !cfg.sort_reverse;
            } else {
                cfg.sort = key;
                cfg.sort_reverse = false;
            }
        });
    }

    pub(super) fn cycle_sort(&mut self) {
        self.change_sort(|cfg| {
            cfg.sort = cfg.sort.next();
            cfg.sort_reverse = false;
        });
    }

    pub(super) fn reverse_sort(&mut self) {
        self.change_sort(|cfg| cfg.sort_reverse = !cfg.sort_reverse);
    }

    pub(super) fn selected_host(&self) -> Option<usize> {
        let vis = self.visible();
        self.host_state.selected().and_then(|s| vis.get(s).copied())
    }

    /// The ssh arguments for host `i`, with `opts` as its connection options.
    pub(super) fn connect_args(&self, i: usize, opts: Option<&config::ConnOptions>) -> Vec<String> {
        let h = &self.hosts[i];
        // A user typed as `root@web1` wins over the host's options: ssh keeps the first value.
        let mut args = Vec::new();
        if let (Some(user), _) = self.filter_parts() {
            args.extend(["-l".to_string(), user.to_string()]);
        }
        if let Some(opts) = opts {
            args.extend(opts.ssh_args());
        }
        if let (Some(p), true) = (&h.port, h.source == Source::KnownHosts) {
            args.extend(["-p".to_string(), p.clone()]);
        }
        args.push(h.alias.clone());
        args
    }

    pub(super) fn connect_selected(&mut self) {
        let args = if let Some(i) = self.selected_host() {
            self.connect_args(i, self.cfg.hosts.get(&self.hosts[i].alias).map(|p| p.default_options()).as_ref())
        } else if !self.filter_text().trim().is_empty() {
            self.filter_text().split_whitespace().map(String::from).collect() // typed ssh arguments
        } else {
            return;
        };
        self.request_connect(args);
    }

    /// Connects, after a yes when the host's environment asks first.
    pub(super) fn request_connect(&mut self, args: Vec<String>) {
        if self.cfg.confirm_label(&crate::host_arg(&args)).is_some() {
            self.mode = Mode::Confirm(Pending::Connect(args));
        } else {
            self.pending_connect = Some(args);
        }
    }

    pub(super) fn cycle_profile_of_selected(&mut self, forward: bool) {
        if let Some(i) = self.selected_host() {
            let alias = self.hosts[i].alias.clone();
            self.cfg.cycle_profile(&alias, forward);
            self.save_cfg();
        }
    }

    /// Tags the selected host with an environment. An untagged host with a suggestion (its name
    /// matches a pattern) takes the suggestion, otherwise the tag moves to the next environment.
    pub(super) fn tag_env_of_selected(&mut self) {
        if let Some(i) = self.selected_host() {
            let alias = self.hosts[i].alias.clone();
            match self.cfg.suggest_env(&alias).map(String::from).filter(|_| self.cfg.env_of(&alias).is_none()) {
                Some(name) => self.cfg.set_env(&alias, Some(&name)),
                None if self.cfg.envs.is_empty() => return self.fail("no environments yet: create one in the Envs tab"),
                None => self.cfg.cycle_env(&alias, true),
            }
            self.save_cfg();
        }
    }

    /// Opens the popup for choosing the extra snippets of the selected host.
    pub(super) fn open_host_snippets(&mut self) {
        if let Some(i) = self.selected_host() {
            self.host_chk.select(Some(0));
            self.popup_offset = (0, 0);
            self.mode = Mode::HostSnippets(self.hosts[i].alias.clone());
        }
    }

    pub(super) fn on_key_host_snippets(&mut self, k: KeyEvent, host: String) {
        let items = self.grid_items();
        match k.code {
            KeyCode::Esc | KeyCode::Char('q') => return,
            KeyCode::Up => Self::move_sel(&mut self.host_chk, items.len(), -1),
            KeyCode::Down => Self::move_sel(&mut self.host_chk, items.len(), 1),
            KeyCode::Char(' ') | KeyCode::Enter => self.toggle_host_extra(&host),
            _ => {}
        }
        self.mode = Mode::HostSnippets(host);
    }

    /// The snippets of the profile a host uses.
    fn host_profile_snippets(&self, host: &str) -> Vec<String> {
        let profile = self.cfg.profile_for(host);
        profile.and_then(|p| self.cfg.profiles.get(p)).map(|p| p.snippets.clone()).unwrap_or_default()
    }

    pub(super) fn toggle_host_extra(&mut self, host: &str) {
        let items = self.grid_items();
        if let Some(name) = self.host_chk.selected().and_then(|i| items.get(i)) {
            // A snippet of the profile is turned off for this host, anything else is an extra.
            if self.host_profile_snippets(host).contains(name) {
                self.cfg.toggle_host_without(host, name);
            } else {
                self.cfg.toggle_host_snippet(host, name);
            }
            self.save_cfg();
        }
    }

    pub(super) fn toggle_favorite_of_selected(&mut self) {
        if let Some(i) = self.selected_host() {
            let alias = self.hosts[i].alias.clone();
            self.cfg.toggle_favorite(&alias);
            self.save_cfg();
        }
    }

    /// The text in the search box.
    pub(super) fn filter_text(&self) -> &str {
        &self.filter.lines[0]
    }

    /// Replaces the search text, with the cursor at its end.
    pub(super) fn set_filter(&mut self, text: &str) {
        self.filter = Editor::from_text(text);
        self.filter.end();
    }

    pub(super) fn on_key_hosts(&mut self, k: KeyEvent, ctrl: bool) {
        if self.global_nav(k) {
            return;
        }
        let before = self.filter_text().to_string();
        if common_key(&mut self.filter, k, false) {
            if self.filter_text() != before {
                self.host_state.select(Some(0));
            }
            return;
        }
        let len = self.visible().len();
        match k.code {
            KeyCode::Up => Self::move_sel(&mut self.host_state, len, -1),
            KeyCode::Down => Self::move_sel(&mut self.host_state, len, 1),
            KeyCode::PageUp => Self::move_sel(&mut self.host_state, len, -10),
            KeyCode::PageDown => Self::move_sel(&mut self.host_state, len, 10),
            KeyCode::Home => self.host_state.select(Some(0)),
            KeyCode::End => Self::move_sel(&mut self.host_state, len, len as isize),
            KeyCode::Enter => self.connect_selected(),
            KeyCode::Char('f') if ctrl => self.toggle_favorite_of_selected(),
            KeyCode::Char('p') if ctrl => self.cycle_profile_of_selected(true),
            KeyCode::Char('e') if ctrl => self.open_host_snippets(),
            KeyCode::Char('t') if ctrl => self.tag_env_of_selected(),
            KeyCode::Char('o') if ctrl => self.open_host_options(),
            KeyCode::Char('l') if ctrl => self.open_options_picker(),
            KeyCode::Char('s') if ctrl => self.cycle_sort(),
            KeyCode::Char('r') if ctrl => self.reverse_sort(),
            KeyCode::Left => self.filter.left(),
            KeyCode::Right => self.filter.right(),
            KeyCode::Char(c) if !ctrl => {
                self.filter.insert_char(c);
                self.host_state.select(Some(0));
            }
            KeyCode::Backspace => {
                self.filter.backspace();
                self.host_state.select(Some(0));
            }
            KeyCode::Delete => {
                self.filter.delete();
                self.host_state.select(Some(0));
            }
            KeyCode::Esc => {
                if self.filter_text().is_empty() {
                    self.quit = true;
                } else {
                    self.set_filter("");
                    self.host_state.select(Some(0));
                }
            }
            _ => {}
        }
    }

    pub(super) fn mouse_hosts(&mut self, x: u16, y: u16, wheel: Option<isize>, click: bool) {
        let len = self.visible().len();
        if let Some(d) = wheel {
            return Self::move_sel(&mut self.host_state, len, d);
        }
        let (head, list) = (self.hits.hosts_head, self.hits.hosts_list);
        if click && inside(self.hits.search, x, y) {
            // the text starts after the ` > ` prompt
            self.filter.click(x.saturating_sub(self.hits.search.x + 3) as usize, 0);
            self.text_drag = Some(TextDrag::Search);
            return;
        }
        if click && inside(head, x, y) {
            return self.click_sort(sort_key_at(x - head.x, self.hits.cols));
        }
        if !click || !inside(list, x, y) {
            return;
        }
        let i = self.host_state.offset() + (y - list.y) as usize;
        if i >= len {
            return;
        }
        self.host_state.select(Some(i));
        let col = x - list.x;
        let cols = self.hits.cols;
        if col < COL_STAR {
            self.toggle_favorite_of_selected();
        } else if (COL_STAR + cols.alias + cols.target..COL_STAR + cols.alias + cols.target + cols.profile).contains(&col) {
            self.cycle_profile_of_selected(true);
        } else if (COL_STAR + cols.alias + cols.target + cols.profile..COL_STAR + cols.alias + cols.target + cols.profile + COL_ENV).contains(&col) {
            self.tag_env_of_selected();
        } else if self.is_double_click(Tab::Hosts, i) {
            self.connect_selected();
        }
    }

    /// Popup over the host list: the snippets this host gets in addition to its profile's.
    pub(super) fn draw_host_snippets(&mut self, f: &mut Frame, body: Rect, host: &str) {
        let items = self.grid_items();
        let from_profile = self.host_profile_snippets(host);
        let w = body.width.min(56);
        // A long list scrolls inside the popup instead of making it taller.
        let h = (items.len() as u16 + 2).clamp(5, MAX_POPUP_HEIGHT).min(body.height);
        let area = self.dragged_popup_area(body, w, h);
        self.hits.extras_area = area;
        f.render_widget(ratatui::widgets::Clear, area);
        let block = Block::bordered()
            .title(" Snippets for this host ")
            .border_style(Style::new().fg(Color::Cyan));
        let inner = block.inner(area);
        f.render_widget(block, area);
        self.hits.popup_list = inner;
        if items.is_empty() {
            f.render_widget(Paragraph::new("no snippets yet: create one in the Snippets tab"), inner);
            return;
        }
        let extras = self.cfg.host_snippets(host).to_vec();
        let without = self.cfg.host_without(host).to_vec();
        let rows: Vec<ListItem> = items
            .iter()
            .map(|name| {
                let via_profile = from_profile.contains(name);
                let skipped = via_profile && without.contains(name);
                let on = (via_profile && !skipped) || extras.contains(name);
                let spans = vec![
                    Span::styled(if on { "[x] " } else { "[ ] " }, Style::new().fg(if on { Color::Green } else { Color::DarkGray })),
                    Span::styled(name.clone(), Style::new().fg(kind_color(name))),
                ];
                ListItem::new(Line::from(spans))
            })
            .collect();
        if self.host_chk.selected().is_none_or(|s| s >= items.len()) {
            self.host_chk.select(Some(0));
        }
        let rows_a = if items.len() > inner.height as usize { Rect { width: inner.width.saturating_sub(1), ..inner } } else { inner };
        f.render_stateful_widget(
            List::new(rows).highlight_style(Style::new().add_modifier(Modifier::REVERSED)),
            rows_a,
            &mut self.host_chk,
        );
        Self::scrollbar(f, inner, items.len(), self.host_chk.offset());
    }

    pub(super) fn draw_hosts(&mut self, f: &mut Frame, area: Rect) {
        let [search_a, rest] = Layout::vertical([Constraint::Length(3), Constraint::Min(2)]).areas(area);
        // The history pane needs room: header + 5 sessions + borders, and still a usable list.
        let hist_h = if rest.height >= 16 { HISTORY_ROWS as u16 + 2 } else { 0 };
        let [list_a, hist_a] = Layout::vertical([Constraint::Min(2), Constraint::Length(hist_h)]).areas(rest);
        let vis = self.visible();
        if hist_h > 0 {
            self.draw_history(f, hist_a);
        }
        // The terminal's own cursor marks the position, as in the snippet editor, so the text is
        // drawn as it is. A text wider than the box scrolls sideways.
        self.filter.keep_cursor_visible(search_a.width.saturating_sub(5) as usize, 1);
        let kinds = highlight::highlight_search(self.filter_text());
        let width = search_a.width.saturating_sub(5) as usize;
        let mut spans = vec![Span::styled(" > ", Style::new().fg(Color::Cyan))];
        let mut line = super::snippets::highlighted_line(self.filter_text(), &kinds, self.filter.left, width);
        if let Some((from, to)) = self.filter.selected_cols(0) {
            let left = self.filter.left;
            line = super::snippets::reversed_range(line, from.saturating_sub(left), to.saturating_sub(left));
        }
        spans.extend(line.spans);
        let search = Paragraph::new(Line::from(spans))
        .block(Block::bordered().title(match self.filter_parts().0 {
            Some(user) => format!(" Search ({} hosts, connect as {user}) ", vis.len()),
            None => format!(" Search ({} hosts) ", vis.len()),
        }));
        f.render_widget(search, search_a);
        self.hits.search = Rect { x: search_a.x + 1, y: search_a.y + 1, width: search_a.width.saturating_sub(2), height: 1 };
        if matches!(self.mode, Mode::Normal) {
            let col = (self.filter.col - self.filter.left) as u16;
            f.set_cursor_position((self.hits.search.x + 3 + col, self.hits.search.y));
        }

        let block = Block::bordered();
        let inner = block.inner(list_a);
        f.render_widget(block, list_a);
        let [head_a, rows_a] = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).areas(inner);
        self.hits.hosts_list = rows_a;
        self.hits.hosts_head = head_a;
        self.hits.cols = self.fit_cols(rows_a.width);
        f.render_widget(Paragraph::new(self.header_line()), head_a);

        if vis.is_empty() {
            let msg = if self.filter_text().trim().is_empty() {
                "No hosts found in ~/.ssh/config or known_hosts. Type a host and press Enter.".to_string()
            } else {
                format!("No match. Press Enter to run: ssh {}", self.filter_text().trim())
            };
            f.render_widget(Paragraph::new(msg).style(Style::new().fg(Color::Yellow)), rows_a);
            return;
        }

        let cols = self.hits.cols;
        let needle = self.filter_parts().1.to_string();
        let items: Vec<ListItem> = vis
            .iter()
            .map(|&i| {
                let h = &self.hosts[i];
                let prefs = self.cfg.hosts.get(&h.alias);
                let fav = prefs.is_some_and(|p| p.favorite);
                let base = self.cfg.profile_for(&h.alias).unwrap_or("off");
                let off = base == "off";
                let extras = self.cfg.host_snippets(&h.alias).len();
                let profile = &Self::profile_text(base, extras, self.cfg.host_without(&h.alias).len());
                let used = prefs.and_then(|p| p.last_used).map(ago).unwrap_or_default();
                // Cyan marks a host whose connection options are set (Ctrl-O).
                let target_style = if prefs.is_some_and(|p| p.has_overrides()) {
                    Style::new().fg(super::hostedit::ACCENT)
                } else {
                    Style::new().add_modifier(Modifier::DIM)
                };
                // The chars the search matched, in the alias and in the target after it.
                let target = self.target_of(h);
                let hit = fuzzy_positions(&needle, &format!("{} {target}", h.alias)).unwrap_or_default();
                let alias_len = h.alias.chars().count();
                let in_alias: Vec<usize> = hit.iter().copied().filter(|&i| i < alias_len).collect();
                let in_target: Vec<usize> = hit.iter().filter(|&&i| i > alias_len).map(|i| i - alias_len - 1).collect();
                let mut spans = vec![Span::styled(truncate(if fav { "★" } else { "" }, COL_STAR), Style::new().fg(Color::Yellow))];
                spans.extend(marked_spans(&h.alias, cols.alias, &in_alias, Style::new()));
                spans.extend(marked_spans(&target, cols.target, &in_target, target_style));
                spans.extend([
                    Span::styled(
                        truncate(profile, cols.profile),
                        Style::new().fg(if off { Color::DarkGray } else { Color::Green }),
                    ),
                    self.env_cell(&h.alias),
                    Span::styled(truncate(h.source.label(), COL_SOURCE), Style::new().add_modifier(Modifier::DIM)),
                    Span::styled(used, Style::new().add_modifier(Modifier::DIM)),
                ]);
                ListItem::new(Line::from(spans))
            })
            .collect();
        let list = List::new(items).highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        if self.host_state.selected().is_none_or(|s| s >= vis.len()) {
            self.host_state.select(Some(0));
        }
        f.render_stateful_widget(list, rows_a, &mut self.host_state);
        // Draw over the right border, skipping the header row.
        let bar_a = Rect { x: list_a.x, width: list_a.width, ..rows_a };
        Self::scrollbar(f, bar_a, vis.len(), self.host_state.offset());
    }

    /// Recent sessions of the selected host, newest first.
    pub(super) fn draw_history(&mut self, f: &mut Frame, area: Rect) {
        let alias = self.selected_host().map(|i| self.hosts[i].alias.clone());
        if let Some(a) = &alias
            && self.history_cache.as_ref().is_none_or(|(h, _, _)| h != a)
        {
            let rows = history::for_host(&self.dir, a, HISTORY_ROWS);
            self.history_cache = Some((a.clone(), rows, history::count_for_host(&self.dir, a)));
        }
        let (title, lines): (String, Vec<Line>) = match (&alias, &self.history_cache) {
            (Some(a), Some((h, rows, total))) if h == a => {
                let title = format!(" History: {a} ({total} session{}) ", if *total == 1 { "" } else { "s" });
                let lines = if rows.is_empty() {
                    vec![Line::styled("no sessions yet", Style::new().add_modifier(Modifier::DIM))]
                } else {
                    rows.iter().map(history_line).collect()
                };
                (title, lines)
            }
            _ => (" History ".to_string(), vec![Line::styled("select a host", Style::new().add_modifier(Modifier::DIM))]),
        };
        f.render_widget(Paragraph::new(lines).block(Block::bordered().title(title)), area);
    }

    /// The environment of a host as a colored badge. An untagged host whose name matches a pattern
    /// shows the suggestion in dim text with a `?`, which Ctrl-T accepts.
    fn env_cell(&self, alias: &str) -> Span<'static> {
        if let Some((name, env)) = self.cfg.env_of(alias) {
            let text = truncate(&format!(" {} ", env.label_for(name)), COL_ENV);
            let style = super::envs::badge_style(&env.color);
            let pad = " ".repeat(COL_ENV.saturating_sub(text.chars().count() as u16 + 1) as usize);
            return Span::styled(format!("{text}{pad}"), style);
        }
        match self.cfg.suggest_env(alias) {
            Some(name) => Span::styled(truncate(&format!("{name}?"), COL_ENV), Style::new().add_modifier(Modifier::DIM)),
            None => Span::raw(truncate("", COL_ENV)),
        }
    }

    /// `base`, or `base+N` when the host has N extra snippets.
    fn profile_text(base: &str, extras: usize, without: usize) -> String {
        let mut text = base.to_string();
        if base != "off" {
            if extras > 0 {
                text.push_str(&format!("+{extras}"));
            }
            if without > 0 {
                text.push_str(&format!("-{without}"));
            }
        }
        text
    }

    /// The widths of the host, target and profile columns for a list `width` wide. They follow the
    /// longest entry over all hosts, so they stay put while the search narrows the list.
    pub(super) fn fit_cols(&self, width: u16) -> Cols {
        let chars = |s: &str| s.chars().count() as u16 + 2;
        let mut want = [0u16; 3];
        for h in &self.hosts {
            let base = self.cfg.profile_for(&h.alias).unwrap_or("off");
            let profile = Self::profile_text(base, self.cfg.host_snippets(&h.alias).len(), self.cfg.host_without(&h.alias).len());
            want[0] = want[0].max(chars(&h.alias));
            want[1] = want[1].max(chars(&self.target_of(h)));
            want[2] = want[2].max(chars(&profile));
        }
        Cols::fit(width, want)
    }

    /// Column headers; the active sort column is highlighted and carries an arrow.
    pub(super) fn header_line(&self) -> Line<'static> {
        let arrow = if self.sort_descending() { " ▼" } else { " ▲" };
        let cell = |key: SortKey, label: &str, width: u16| {
            let active = self.cfg.sort == key;
            let text = if active && key != SortKey::Smart { format!("{label}{arrow}") } else { label.to_string() };
            let style = if active {
                Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::new().add_modifier(Modifier::DIM)
            };
            Span::styled(truncate(&text, width), style)
        };
        Line::from(vec![
            cell(SortKey::Smart, "★", COL_STAR),
            cell(SortKey::Host, "host", self.hits.cols.alias),
            cell(SortKey::Target, "target", self.hits.cols.target),
            cell(SortKey::Profile, "profile", self.hits.cols.profile),
            cell(SortKey::Env, "env", COL_ENV),
            cell(SortKey::Source, "source", COL_SOURCE),
            cell(SortKey::LastUsed, "last used", COL_USED),
        ])
    }
}
