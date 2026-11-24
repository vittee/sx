//! Profiles tab: profiles, the prompt theme picker and the snippet grid.

use super::*;

impl App {
    pub(super) fn profile_names(&self) -> Vec<String> {
        self.cfg.profiles.keys().cloned().collect()
    }

    pub(super) fn selected_profile(&self) -> Option<String> {
        self.profile_names().get(self.prof_state.selected()?).cloned()
    }

    /// Every prompt theme you can pick: the built-in ones, then your own `*.prompt` snippets.
    pub(super) fn theme_names(&self) -> Vec<String> {
        prompt::THEMES
            .iter()
            .map(|(n, _)| n.to_string())
            .chain(self.snippets.iter().filter(|s| config::is_prompt_snippet(s)).cloned())
            .collect()
    }

    /// Snippets a profile or host can send (everything but prompt themes), in grid order.
    pub(super) fn grid_items(&self) -> Vec<String> {
        config::sendable(&self.snippets)
    }

    /// Row in the prompt list for the selected profile: 0 = server's own prompt, then one per theme.
    pub(super) fn prompt_index(&self) -> usize {
        let theme = self.selected_profile().and_then(|p| self.cfg.profiles.get(&p)?.prompt.clone());
        let names = self.theme_names();
        theme.and_then(|t| names.iter().position(|n| *n == t)).map_or(0, |i| i + 1)
    }

    /// The row you are looking at: the one you moved to, or else the profile's own prompt.
    pub(super) fn prompt_cursor_row(&self) -> usize {
        match (&self.prompt_cursor, self.selected_profile()) {
            (Some((p, i)), Some(sel)) if *p == sel => (*i).min(self.theme_names().len()),
            _ => self.prompt_index(),
        }
    }

    /// Picks a prompt row for the selected profile and saves it.
    pub(super) fn set_prompt_index(&mut self, i: usize) {
        let Some(profile) = self.selected_profile() else { return };
        let theme = i.checked_sub(1).and_then(|t| self.theme_names().get(t).cloned());
        self.cfg.set_prompt(&profile, theme.as_deref());
        self.prompt_cursor = None;
        self.save_cfg();
    }

    /// Moves the cursor in the prompt list. It only shows the preview, `apply_prompt` saves it.
    pub(super) fn move_prompt(&mut self, delta: isize) {
        let Some(profile) = self.selected_profile() else { return };
        let last = self.theme_names().len() as isize;
        let next = (self.prompt_cursor_row() as isize + delta).clamp(0, last) as usize;
        self.prompt_cursor = Some((profile, next));
    }

    /// Uses the row under the cursor as the prompt of the selected profile.
    pub(super) fn apply_prompt(&mut self) {
        let row = self.prompt_cursor_row();
        self.set_prompt_index(row);
        let name = self.theme_names().get(row.wrapping_sub(1)).cloned();
        self.say(format!("prompt: {}", name.as_deref().unwrap_or("the server's own")));
    }

    /// Opens the selected prompt theme for editing in the Snippets tab. A built-in theme is first
    /// copied to `<name>.prompt` (or `my-prompt.prompt` for "server's own"), and the profile then
    /// uses the copy.
    pub(super) fn edit_prompt(&mut self) {
        let Some(profile) = self.selected_profile() else { return };
        let names = self.theme_names();
        let current = self.prompt_cursor_row().checked_sub(1).and_then(|i| names.get(i).cloned());
        let target = match current {
            Some(n) if config::is_prompt_snippet(&n) => n,
            other => {
                let base = other.clone().unwrap_or_else(|| "my-prompt".to_string());
                let text = config::theme_ps1(&self.dir, other.as_deref().unwrap_or(prompt::DEFAULT_THEME))
                    .map(|ps1| format!("{ps1}
"))
                    .unwrap_or_default();
                let mut name = format!("{base}.prompt");
                let mut n = 2;
                while self.snippets.contains(&name) {
                    name = format!("{base}-{n}.prompt");
                    n += 1;
                }
                if let Err(e) = config::write_snippet(&self.dir, &name, &text) {
                    return self.fail(e.to_string());
                }
                self.cfg.set_prompt(&profile, Some(&name));
                self.save_cfg();
                self.flush_editor();
                self.reload_snippets(Some(&name));
                self.say(format!("editing a copy as {name}; Esc saves"));
                name
            }
        };
        self.reload_snippets(Some(&target));
        self.switch_tab(Tab::Snippets);
        self.editor_focus = true;
    }

    pub(super) fn create_profile(&mut self, name: &str) {
        match self.cfg.add_profile(name.trim()) {
            Ok(()) => {
                self.save_cfg();
                let i = self.profile_names().iter().position(|p| p == name.trim()).unwrap_or(0);
                self.prof_state.select(Some(i));
                self.say(format!("created profile {}", name.trim()));
            }
            Err(e) => self.fail(e),
        }
    }

    pub(super) fn duplicate_profile(&mut self, from: &str, to: &str) {
        let to = to.trim();
        match self.cfg.duplicate_profile(from, to) {
            Ok(()) => {
                self.save_cfg();
                let i = self.profile_names().iter().position(|p| p == to).unwrap_or(0);
                self.prof_state.select(Some(i));
                self.say(format!("copied profile {from} to {to}"));
            }
            Err(e) => self.fail(e),
        }
    }

    pub(super) fn delete_profile(&mut self, name: &str) {
        match self.cfg.delete_profile(name) {
            Ok(()) => {
                self.save_cfg();
                self.prof_state.select(Some(0));
                self.say(format!("deleted profile {name}"));
            }
            Err(e) => self.fail(e),
        }
    }

    pub(super) fn toggle_checked(&mut self) {
        let items = self.grid_items();
        let (Some(profile), Some(snippet)) = (self.selected_profile(), self.chk_state.selected().and_then(|i| items.get(i).cloned())) else {
            return;
        };
        self.cfg.toggle_snippet(&profile, &snippet);
        self.save_cfg();
    }

    /// Turns the window title (`marks` false) or the prompt marks (`marks` true) on or off for the
    /// selected profile.
    pub(super) fn toggle_terminal(&mut self, marks: bool) {
        let Some(profile) = self.selected_profile() else { return };
        if let Some(p) = self.cfg.profiles.get_mut(&profile) {
            if marks {
                p.marks = !p.marks;
            } else {
                p.title = !p.title;
            }
            self.save_cfg();
        }
    }

    /// Shows grid item `i` in the Snippets tab, selected in the list and open in the editor.
    fn open_snippet(&mut self, i: usize) {
        let Some(name) = self.grid_items().get(i).cloned() else { return };
        self.switch_tab(Tab::Snippets);
        if let Some(at) = self.snippets.iter().position(|s| *s == name) {
            self.select_snippet(at);
        }
    }

    /// Encoded payload size of `profile`. Building it reads every snippet and gzips, so the result
    /// is kept until the config or a snippet changes (`save_cfg`, `flush_editor`, `reload_snippets`).
    pub(super) fn payload_size(&mut self, profile: &str) -> Option<usize> {
        if let Some((p, size)) = &self.payload_cache
            && p == profile
        {
            return *size;
        }
        let rc = config::build_rc(&self.dir, &self.cfg, profile);
        let size = payload::encode(&rc).ok().map(|s| s.len());
        self.payload_cache = Some((profile.to_string(), size));
        size
    }

    pub(super) fn on_key_profiles(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match self.prof_focus {
            ProfFocus::Snippets => {
                // The grid is filled column by column, so one step is the next item, which
                // is the top of the next column after the bottom of this one.
                let len = self.grid_items().len();
                let rows = self.hits.chk_rows.max(1);
                let sel = self.chk_state.selected().unwrap_or(0);
                match k.code {
                    KeyCode::Up => Self::move_sel(&mut self.chk_state, len, -1),
                    KeyCode::Down => Self::move_sel(&mut self.chk_state, len, 1),
                    // Right goes to the same row of the next column, or the last item when that
                    // column is shorter. The last column stays put.
                    KeyCode::Right if sel / rows < len.saturating_sub(1) / rows => {
                        self.chk_state.select(Some((sel + rows).min(len - 1)));
                    }
                    KeyCode::Char(' ') | KeyCode::Enter => self.toggle_checked(),
                    KeyCode::Left if sel < rows => self.prof_focus = ProfFocus::Prompt,
                    KeyCode::Left => Self::move_sel(&mut self.chk_state, len, -(rows as isize)),
                    KeyCode::Esc => self.prof_focus = ProfFocus::Prompt,
                    _ => {}
                }
                return;
            }
            ProfFocus::Prompt => {
                match k.code {
                    KeyCode::Up => self.move_prompt(-1),
                    KeyCode::Down => self.move_prompt(1),
                    KeyCode::Char('e') if ctrl => self.edit_prompt(),
                    KeyCode::Enter | KeyCode::Char(' ') => self.apply_prompt(),
                    KeyCode::Right if !self.grid_items().is_empty() => self.prof_focus = ProfFocus::Snippets,
                    KeyCode::Left | KeyCode::Esc => self.prof_focus = ProfFocus::List,
                    _ => {}
                }
                return;
            }
            ProfFocus::List => {}
        }
        if self.global_nav(k) {
            return;
        }
        let len = self.cfg.profiles.len();
        match k.code {
            KeyCode::Up => Self::move_sel(&mut self.prof_state, len, -1),
            KeyCode::Down => Self::move_sel(&mut self.prof_state, len, 1),
            KeyCode::Right | KeyCode::Enter => {
                self.prompt_cursor = None;
                self.prof_focus = ProfFocus::Prompt;
            }
            KeyCode::Char('n') if ctrl => self.mode = Mode::Input(InputKind::NewProfile, String::new()),
            KeyCode::Char('y') if ctrl => {
                if let Some(p) = self.selected_profile() {
                    self.mode = Mode::Input(InputKind::DuplicateProfile(p.clone()), format!("{p}-copy"));
                }
            }
            KeyCode::Char('d') if ctrl => {
                if let Some(p) = self.selected_profile() {
                    self.mode = Mode::Confirm(Pending::DeleteProfile(p));
                }
            }
            KeyCode::Char('g') if ctrl => {
                if let Some(p) = self.selected_profile() {
                    self.cfg.default_profile = p.clone();
                    self.save_cfg();
                    self.say(format!("{p} is now the default profile"));
                }
            }
            KeyCode::Esc => self.quit = true,
            _ => {}
        }
    }

    pub(super) fn mouse_profiles(&mut self, x: u16, y: u16, wheel: Option<isize>, click: bool) {
        let (plist, prompts, clist) = (self.hits.prof_list, self.hits.prompt_list, self.hits.chk_list);
        let term = self.hits.prof_term;
        if click && wheel.is_none() && inside(term, x, y) {
            return self.toggle_terminal(y > term.y);
        }
        if let Some(d) = wheel {
            if inside(clist, x, y) {
                let len = self.grid_items().len();
                Self::move_sel(&mut self.chk_state, len, d);
            } else if inside(prompts, x, y) {
                self.move_prompt(d);
            } else {
                let len = self.cfg.profiles.len();
                Self::move_sel(&mut self.prof_state, len, d);
            }
            return;
        }
        if !click {
            return;
        }
        if inside(plist, x, y) {
            let i = self.prof_state.offset() + (y - plist.y) as usize;
            if i < self.cfg.profiles.len() {
                self.prof_state.select(Some(i));
                self.prof_focus = ProfFocus::List;
            }
        } else if inside(prompts, x, y) {
            let i = self.prompt_state.offset() + (y - prompts.y) as usize;
            if i <= self.theme_names().len() {
                // the radio mark applies the row, the name only previews it
                if x < prompts.x + 4 {
                    self.set_prompt_index(i);
                } else if let Some(profile) = self.selected_profile() {
                    self.prompt_cursor = Some((profile, i));
                }
                self.prof_focus = ProfFocus::Prompt;
            }
        } else if inside(clist, x, y) {
            let rows = self.hits.chk_rows.max(1);
            let colw = self.hits.chk_colw.max(1);
            let col = ((x - clist.x) / colw) as usize;
            let row = self.chk_top + (y - clist.y) as usize;
            let i = col * rows + row;
            if row < rows && i < self.grid_items().len() {
                self.chk_state.select(Some(i));
                self.prof_focus = ProfFocus::Snippets;
                // the checkbox toggles the snippet, the name only selects it
                if (x - clist.x) % colw < 4 {
                    self.toggle_checked();
                } else if self.is_double_click(Tab::Profiles, i) {
                    self.open_snippet(i);
                }
            }
        }
    }

    pub(super) fn draw_profiles(&mut self, f: &mut Frame, area: Rect) {
        let [left_a, right] = Layout::horizontal([Constraint::Length(28), Constraint::Min(20)]).areas(area);
        let [left, term_a, size_a] =
            Layout::vertical([Constraint::Min(3), Constraint::Length(4), Constraint::Length(3)]).areas(left_a);

        let block = Block::bordered().title(" Profiles ");
        let prof_inner = block.inner(left);
        self.hits.prof_list = prof_inner;
        let items: Vec<ListItem> = self
            .cfg
            .profiles
            .keys()
            .map(|p| {
                let mark = if *p == self.cfg.default_profile { " *" } else { "" };
                ListItem::new(format!("{p}{mark}"))
            })
            .collect();
        let list = List::new(items).block(block).highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(list, left, &mut self.prof_state);
        let bar_a = Rect { x: left.x, width: left.width, ..prof_inner };
        Self::scrollbar(f, bar_a, self.cfg.profiles.len(), self.prof_state.offset());

        let profile = self.selected_profile();
        self.draw_terminal_switches(f, term_a, profile.as_deref());
        self.draw_payload_size(f, size_a, profile.as_deref());
        // "none" row + themes + borders, but on a short terminal leave room for the snippet grid
        // (the theme list scrolls) while keeping a two-line preview readable.
        let prompt_h = (self.theme_names().len() as u16 + 3).min(right.height.saturating_sub(5)).max(8);
        let [prompt_a, chk_a] = Layout::vertical([Constraint::Length(prompt_h), Constraint::Min(3)]).areas(right);
        self.draw_prompt_picker(f, prompt_a);

        let focused = self.prof_focus == ProfFocus::Snippets;
        let border = if focused { Color::Cyan } else { Color::Reset };
        let block = Block::bordered()
            .title(" Snippets ")
            .border_style(Style::new().fg(border));
        let inner = block.inner(chk_a);
        f.render_widget(block, chk_a);
        self.hits.chk_list = inner;

        // Grid, filled down each column then on to the next: as many columns as fit.
        let selected = profile.as_ref().and_then(|p| self.cfg.profiles.get(p)).cloned().unwrap_or_default();
        let items = self.grid_items();
        let name_w = items.iter().map(|s| s.chars().count()).max().unwrap_or(0);
        let colw = (name_w + 4 + 2).min(inner.width.max(1) as usize).max(1);
        let cols = (inner.width as usize / colw).max(1);
        // Each column is as tall as the pane, so the grid only gets wider when the items do not
        // fit. If even all the columns that fit are not enough, the grid scrolls down.
        let total_rows = inner.height.max(1).max(items.len().div_ceil(cols) as u16).min(items.len().max(1) as u16) as usize;
        self.hits.chk_rows = total_rows;
        self.hits.chk_colw = colw as u16;
        let height = inner.height as usize;
        let sel = self.chk_state.selected().unwrap_or(0).min(items.len().saturating_sub(1));
        let sel_row = sel % total_rows;
        if sel_row < self.chk_top {
            self.chk_top = sel_row;
        } else if height > 0 && sel_row >= self.chk_top + height {
            self.chk_top = sel_row + 1 - height;
        }
        self.chk_top = self.chk_top.min(total_rows.saturating_sub(height));
        let lines: Vec<Line> = (self.chk_top..total_rows.min(self.chk_top + height))
            .map(|r| {
                let spans = (0..cols)
                    .filter_map(|c| items.get(c * total_rows + r).map(|s| (c * total_rows + r, s)))
                    .flat_map(|(i, name)| {
                        let on = selected.snippets.contains(name);
                        let at_cursor = focused && i == sel;
                        let mark = Style::new().fg(if on { Color::Green } else { Color::DarkGray });
                        let kind = Style::new().fg(kind_color(name));
                        let (mark, text) = if at_cursor {
                            (mark.add_modifier(Modifier::REVERSED), kind.add_modifier(Modifier::REVERSED))
                        } else {
                            (mark, kind)
                        };
                        let room = colw - 4;
                        let label = format!("{:<w$}", truncate(name, room as u16), w = room);
                        [Span::styled(if on { "[x] " } else { "[ ] " }, mark), Span::styled(label, text)]
                    })
                    .collect::<Vec<_>>();
                Line::from(spans)
            })
            .collect();
        f.render_widget(Paragraph::new(lines), inner);
        let bar_a = Rect { x: chk_a.x, width: chk_a.width, ..inner };
        Self::scrollbar(f, bar_a, total_rows, self.chk_top);
    }

    /// The size of what a connect sends for the profile, red once it is over the limit.
    fn draw_payload_size(&mut self, f: &mut Frame, area: Rect, profile: Option<&str>) {
        let size = profile.and_then(|p| self.payload_size(p)).unwrap_or(0);
        let limit = payload::WARN_ENCODED_LEN;
        let color = match size {
            s if s >= limit => Color::Red,
            s if s * 4 >= limit * 3 => Color::Yellow,
            _ => Color::Green,
        };
        let block = Block::bordered().title(" Payload ");
        let inner = block.inner(area);
        f.render_widget(block, area);
        f.render_widget(Paragraph::new(Span::styled(format!("{size}/{limit} B"), Style::new().fg(color))), inner);
    }

    /// The two terminal integration switches of the selected profile, under the profile list.
    fn draw_terminal_switches(&mut self, f: &mut Frame, area: Rect, profile: Option<&str>) {
        let block = Block::bordered().title(" Terminal ");
        let inner = block.inner(area);
        f.render_widget(block, area);
        self.hits.prof_term = inner;
        let p = profile.and_then(|p| self.cfg.profiles.get(p)).cloned().unwrap_or_default();
        let row = |on: bool, label: &str| {
            let mark = Style::new().fg(if on { Color::Green } else { Color::DarkGray });
            Line::from(vec![Span::styled(if on { "[x] " } else { "[ ] " }, mark), Span::raw(label.to_string())])
        };
        let lines = vec![row(p.title, "Window title"), row(p.marks, "Prompt marks")];
        f.render_widget(Paragraph::new(lines), inner);
    }

    /// Radio list of built-in prompt themes for the selected profile, with a live preview.
    pub(super) fn draw_prompt_picker(&mut self, f: &mut Frame, area: Rect) {
        let focused = self.prof_focus == ProfFocus::Prompt;
        let border = if focused { Color::Cyan } else { Color::Reset };
        // Two sibling panes: the theme list, and the preview next to it.
        let [pick_a, prev_a] = Layout::horizontal([Constraint::Length(28), Constraint::Min(10)]).areas(area);
        let block = Block::bordered()
            .title(" Prompt ")
            .border_style(Style::new().fg(border));
        let list_a = block.inner(pick_a);
        f.render_widget(block, pick_a);
        self.hits.prompt_list = list_a;

        let current = self.prompt_index();
        let cursor = self.prompt_cursor_row();
        let theme_names = self.theme_names();
        let names = std::iter::once("(server's own)").chain(theme_names.iter().map(String::as_str));
        let items: Vec<ListItem> = names
            .enumerate()
            .map(|(i, name)| {
                let on = i == current;
                ListItem::new(Line::from(vec![
                    Span::styled(if on { "(•) " } else { "( ) " }, Style::new().fg(if on { Color::Green } else { Color::DarkGray })),
                    Span::raw(name),
                ]))
            })
            .collect();
        let hl = if focused { Style::new().add_modifier(Modifier::REVERSED) } else { Style::new() };
        self.prompt_state.select(Some(cursor));
        // Reserve the last column for the scrollbar so the selection bar never covers it.
        let total = theme_names.len() + 1;
        let rows_a = if total > list_a.height as usize {
            Rect { width: list_a.width.saturating_sub(1), ..list_a }
        } else {
            list_a
        };
        f.render_stateful_widget(List::new(items).highlight_style(hl), rows_a, &mut self.prompt_state);
        Self::scrollbar(f, list_a, total, self.prompt_state.offset());

        let shown = match cursor.checked_sub(1) {
            Some(t) => theme_names
                .get(t)
                .and_then(|n| config::theme_ps1(&self.dir, n))
                .map(|p| prompt::render_ps1(&p))
                .unwrap_or_default(),
            None => vec![Line::styled("the server keeps its own prompt", Style::new().add_modifier(Modifier::DIM))],
        };
        let title = if cursor == current { " Preview ".to_string() } else { " Preview (Enter to use it) ".to_string() };
        f.render_widget(Paragraph::new(shown).block(Block::bordered().title(title)), prev_a);
    }
}
