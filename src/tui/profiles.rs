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

    /// Picks a prompt row for the selected profile and saves it.
    pub(super) fn set_prompt_index(&mut self, i: usize) {
        let Some(profile) = self.selected_profile() else { return };
        let theme = i.checked_sub(1).and_then(|t| self.theme_names().get(t).cloned());
        self.cfg.set_prompt(&profile, theme.as_deref());
        self.save_cfg();
    }

    pub(super) fn move_prompt(&mut self, delta: isize) {
        let last = self.theme_names().len() as isize;
        let next = (self.prompt_index() as isize + delta).clamp(0, last) as usize;
        self.set_prompt_index(next);
    }

    /// Opens the selected prompt theme for editing in the Snippets tab. A built-in theme is first
    /// copied to `<name>.prompt` (or `my-prompt.prompt` for "server's own"), and the profile then
    /// uses the copy.
    pub(super) fn edit_prompt(&mut self) {
        let Some(profile) = self.selected_profile() else { return };
        let names = self.theme_names();
        let current = self.prompt_index().checked_sub(1).and_then(|i| names.get(i).cloned());
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
        match self.prof_focus {
            ProfFocus::Snippets => {
                let len = self.grid_items().len();
                let cols = self.hits.chk_cols.max(1);
                let step = cols as isize;
                let sel = self.chk_state.selected().unwrap_or(0);
                let on_last_row = sel / cols >= len.saturating_sub(1) / cols;
                match k.code {
                    KeyCode::Up => Self::move_sel(&mut self.chk_state, len, -step),
                    // Down stays put on the last row instead of jumping to the last cell.
                    KeyCode::Down if !on_last_row => Self::move_sel(&mut self.chk_state, len, step),
                    KeyCode::Right => Self::move_sel(&mut self.chk_state, len, 1),
                    KeyCode::Char(' ') | KeyCode::Enter => self.toggle_checked(),
                    KeyCode::Left if sel.is_multiple_of(cols) => self.prof_focus = ProfFocus::Prompt,
                    KeyCode::Left => Self::move_sel(&mut self.chk_state, len, -1),
                    KeyCode::Esc => self.prof_focus = ProfFocus::List,
                    _ => {}
                }
                return;
            }
            ProfFocus::Prompt => {
                match k.code {
                    KeyCode::Up => self.move_prompt(-1),
                    KeyCode::Down => self.move_prompt(1),
                    KeyCode::Char('e') => self.edit_prompt(),
                    KeyCode::Right | KeyCode::Enter if !self.grid_items().is_empty() => {
                        self.prof_focus = ProfFocus::Snippets;
                    }
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
            KeyCode::Right | KeyCode::Enter => self.prof_focus = ProfFocus::Prompt,
            KeyCode::Char('n') => self.mode = Mode::Input(InputKind::NewProfile, String::new()),
            KeyCode::Char('c') => {
                if let Some(p) = self.selected_profile() {
                    self.mode = Mode::Input(InputKind::DuplicateProfile(p.clone()), format!("{p}-copy"));
                }
            }
            KeyCode::Char('d') => {
                if let Some(p) = self.selected_profile() {
                    self.mode = Mode::Confirm(Pending::DeleteProfile(p));
                }
            }
            KeyCode::Char('*') => {
                if let Some(p) = self.selected_profile() {
                    self.cfg.default_profile = p.clone();
                    self.save_cfg();
                    self.say(format!("{p} is now the default profile"));
                }
            }
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            _ => {}
        }
    }

    pub(super) fn mouse_profiles(&mut self, x: u16, y: u16, wheel: Option<isize>, click: bool) {
        let (plist, prompts, clist) = (self.hits.prof_list, self.hits.prompt_list, self.hits.chk_list);
        if let Some(d) = wheel {
            if inside(clist, x, y) {
                let len = self.grid_items().len();
                Self::move_sel(&mut self.chk_state, len, d * self.hits.chk_cols.max(1) as isize);
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
                self.set_prompt_index(i);
                self.prof_focus = ProfFocus::Prompt;
            }
        } else if inside(clist, x, y) {
            let cols = self.hits.chk_cols.max(1);
            let col = ((x - clist.x) / self.hits.chk_colw.max(1)) as usize;
            let i = (self.chk_top + (y - clist.y) as usize) * cols + col;
            if col < cols && i < self.grid_items().len() {
                self.chk_state.select(Some(i));
                self.prof_focus = ProfFocus::Snippets;
                self.toggle_checked();
            }
        }
    }

    pub(super) fn draw_profiles(&mut self, f: &mut Frame, area: Rect) {
        let [left, right] = Layout::horizontal([Constraint::Length(28), Constraint::Min(20)]).areas(area);

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
        // "none" row + themes + borders, but on a short terminal leave room for the snippet grid
        // (the theme list scrolls) while keeping a two-line preview readable.
        let prompt_h = (self.theme_names().len() as u16 + 3).min(right.height.saturating_sub(5)).max(8);
        let [prompt_a, chk_a] = Layout::vertical([Constraint::Length(prompt_h), Constraint::Min(3)]).areas(right);
        self.draw_prompt_picker(f, prompt_a);

        // Payload size is a compact readout on the bottom border, red once over the limit.
        let size = profile.as_deref().and_then(|p| self.payload_size(p)).unwrap_or(0);
        let limit = payload::WARN_ENCODED_LEN;
        let color = match size {
            s if s >= limit => Color::Red,
            s if s * 4 >= limit * 3 => Color::Yellow,
            _ => Color::Green,
        };
        let readout = Line::from(vec![
            Span::raw(" payload "),
            Span::styled(format!("{size}/{limit} B"), Style::new().fg(color)),
            Span::raw(" "),
        ]);

        let focused = self.prof_focus == ProfFocus::Snippets;
        let border = if focused { Color::Cyan } else { Color::Reset };
        // Bottom border, left-aligned so it stays near the list instead of the far corner.
        let block = Block::bordered()
            .title(" Snippets ")
            .title_bottom(readout)
            .border_style(Style::new().fg(border));
        let inner = block.inner(chk_a);
        f.render_widget(block, chk_a);
        self.hits.chk_list = inner;

        // Grid, filled left to right then down: as many columns as fit.
        let selected = profile.as_ref().and_then(|p| self.cfg.profiles.get(p)).cloned().unwrap_or_default();
        let items = self.grid_items();
        let name_w = items.iter().map(|s| s.chars().count()).max().unwrap_or(0);
        let colw = (name_w + 4 + 2).min(inner.width.max(1) as usize).max(1);
        let cols = (inner.width as usize / colw).max(1);
        self.hits.chk_cols = cols;
        self.hits.chk_colw = colw as u16;
        let total_rows = items.len().div_ceil(cols);
        let height = inner.height as usize;
        let sel = self.chk_state.selected().unwrap_or(0).min(items.len().saturating_sub(1));
        let sel_row = sel / cols;
        if sel_row < self.chk_top {
            self.chk_top = sel_row;
        } else if height > 0 && sel_row >= self.chk_top + height {
            self.chk_top = sel_row + 1 - height;
        }
        self.chk_top = self.chk_top.min(total_rows.saturating_sub(height));
        let lines: Vec<Line> = (self.chk_top..total_rows.min(self.chk_top + height))
            .map(|r| {
                let spans = (0..cols)
                    .filter_map(|c| items.get(r * cols + c).map(|s| (r * cols + c, s)))
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
        self.prompt_state.select(Some(current));
        // Reserve the last column for the scrollbar so the selection bar never covers it.
        let total = theme_names.len() + 1;
        let rows_a = if total > list_a.height as usize {
            Rect { width: list_a.width.saturating_sub(1), ..list_a }
        } else {
            list_a
        };
        f.render_stateful_widget(List::new(items).highlight_style(hl), rows_a, &mut self.prompt_state);
        Self::scrollbar(f, list_a, total, self.prompt_state.offset());

        let shown = match current.checked_sub(1) {
            Some(t) => theme_names
                .get(t)
                .and_then(|n| config::theme_ps1(&self.dir, n))
                .map(|p| prompt::render_ps1(&p))
                .unwrap_or_default(),
            None => vec![Line::styled("the server keeps its own prompt", Style::new().add_modifier(Modifier::DIM))],
        };
        f.render_widget(Paragraph::new(shown).block(Block::bordered().title(" Preview ")), prev_a);
    }
}
