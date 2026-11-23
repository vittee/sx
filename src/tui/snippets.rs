//! Snippets tab: the snippet list and the bash editor.

use super::*;

pub(super) fn kind_style(k: highlight::Kind) -> Style {
    use highlight::Kind::*;
    match k {
        Plain => Style::new(),
        Comment => Style::new().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        Str => Style::new().fg(Color::Green),
        Var => Style::new().fg(Color::Cyan),
        Keyword => Style::new().fg(Color::Magenta).add_modifier(Modifier::BOLD),
        Builtin => Style::new().fg(Color::Blue),
        Func => Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        Op => Style::new().fg(Color::Red),
        Number => Style::new().fg(Color::Yellow),
    }
}

/// The part of `text` visible in a view starting at column `left` with `width` columns,
/// as spans grouped by highlight kind.
pub(super) fn highlighted_line(text: &str, kinds: &[highlight::Kind], left: usize, width: usize) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut run = String::new();
    let mut run_kind = highlight::Kind::Plain;
    for (c, k) in text.chars().zip(kinds.iter().copied()).skip(left).take(width) {
        if k != run_kind && !run.is_empty() {
            spans.push(Span::styled(std::mem::take(&mut run), kind_style(run_kind)));
        }
        run_kind = k;
        run.push(c);
    }
    if !run.is_empty() {
        spans.push(Span::styled(run, kind_style(run_kind)));
    }
    Line::from(spans)
}

/// Color of a snippet name by its kind: shell green, vim cyan, tmux yellow, inputrc light blue, prompt
/// magenta.
pub(super) fn kind_color(name: &str) -> Color {
    match config::snippet_kind(name) {
        config::SnippetKind::Shell => Color::Green,
        config::SnippetKind::Vim => Color::Cyan,
        config::SnippetKind::Tmux => Color::Yellow,
        config::SnippetKind::Inputrc => Color::LightBlue,
        config::SnippetKind::Prompt => Color::Magenta,
    }
}

impl App {
    pub(super) fn selected_snippet(&self) -> Option<String> {
        self.snippets.get(self.snip_state.selected()?).cloned()
    }

    pub(super) fn load_editor(&mut self) {
        match self.selected_snippet() {
            Some(name) => {
                let text = config::read_snippet(&self.dir, &name).unwrap_or_default();
                self.editor = Editor::from_text(&text);
                self.editor_name = Some(name);
            }
            None => {
                self.editor = Editor::from_text("");
                self.editor_name = None;
            }
        }
    }

    /// Saves the open snippet if it changed.
    pub(super) fn flush_editor(&mut self) {
        let Some(name) = self.editor_name.clone() else { return };
        if !self.editor.dirty {
            return;
        }
        match config::write_snippet(&self.dir, &name, &self.editor.text()) {
            Ok(()) => {
                self.payload_cache = None;
                self.editor.dirty = false;
                self.say(format!("saved {name}"));
            }
            Err(e) => self.fail(format!("cannot save {name}: {e}")),
        }
    }

    pub(super) fn select_snippet(&mut self, i: usize) {
        if self.snip_state.selected() == Some(i) && self.editor_name.is_some() {
            return;
        }
        self.flush_editor();
        self.snip_state.select(Some(i));
        self.load_editor();
    }

    pub(super) fn reload_snippets(&mut self, select: Option<&str>) {
        self.payload_cache = None;
        self.snippets = config::list_snippets(&self.dir);
        let i = select.and_then(|n| self.snippets.iter().position(|s| s == n)).unwrap_or(0);
        self.snip_state.select(if self.snippets.is_empty() { None } else { Some(i.min(self.snippets.len() - 1)) });
        self.editor_name = None;
        self.load_editor();
    }

    pub(super) fn create_snippet(&mut self, name: &str) {
        self.flush_editor();
        let name = name.trim().trim_end_matches(".sh");
        if self.snippets.iter().any(|s| s == name) {
            return self.fail(format!("snippet {name} exists"));
        }
        match config::write_snippet(&self.dir, name, "") {
            Ok(()) => {
                self.reload_snippets(Some(name));
                self.editor_focus = true;
                self.say(format!("created {name}; add it to a profile in the Profiles tab"));
            }
            Err(e) => self.fail(e.to_string()),
        }
    }

    pub(super) fn duplicate_snippet(&mut self, from: &str, to: &str) {
        self.flush_editor();
        let to = to.trim().trim_end_matches(".sh");
        if self.snippets.iter().any(|s| s == to) {
            return self.fail(format!("snippet {to} exists"));
        }
        let copied = config::read_snippet(&self.dir, from).and_then(|text| config::write_snippet(&self.dir, to, &text));
        match copied {
            Ok(()) => {
                self.reload_snippets(Some(to));
                self.say(format!("copied {from} to {to}; add it to a profile in the Profiles tab"));
            }
            Err(e) => self.fail(e.to_string()),
        }
    }

    pub(super) fn delete_snippet(&mut self, name: &str) {
        match config::delete_snippet(&self.dir, name) {
            Ok(()) => {
                self.cfg.forget_snippet(name);
                self.save_cfg();
                self.editor_name = None;
                self.reload_snippets(None);
                self.say(format!("deleted {name}"));
            }
            Err(e) => self.fail(e.to_string()),
        }
    }

    pub(super) fn on_key_snippets(&mut self, k: KeyEvent, ctrl: bool) {
        if self.editor_focus {
            return self.on_key_editor(k, ctrl);
        }
        if self.global_nav(k) {
            return;
        }
        let len = self.snippets.len();
        match k.code {
            KeyCode::Up | KeyCode::Down => {
                let mut st = self.snip_state;
                Self::move_sel(&mut st, len, if k.code == KeyCode::Up { -1 } else { 1 });
                if let Some(i) = st.selected() {
                    self.select_snippet(i);
                }
            }
            KeyCode::Enter | KeyCode::Right if self.editor_name.is_some() => self.editor_focus = true,
            KeyCode::Char('n') if ctrl => self.mode = Mode::Input(InputKind::NewSnippet, String::new()),
            KeyCode::Char('y') if ctrl => {
                if let Some(name) = self.selected_snippet() {
                    self.mode = Mode::Input(InputKind::DuplicateSnippet(name.clone()), format!("{name}-copy"));
                }
            }
            KeyCode::Char('d') if ctrl => {
                if let Some(name) = self.selected_snippet() {
                    self.mode = Mode::Confirm(Pending::DeleteSnippet(name));
                }
            }
            KeyCode::Esc => {
                self.flush_editor();
                self.quit = true;
            }
            _ => {}
        }
    }

    pub(super) fn on_key_editor(&mut self, k: KeyEvent, ctrl: bool) {
        let e = &mut self.editor;
        match k.code {
            KeyCode::Esc => {
                self.editor_focus = false;
                self.flush_editor();
            }
            KeyCode::Char('s') if ctrl => self.flush_editor(),
            KeyCode::Char(c) if !ctrl => e.insert_char(c),
            KeyCode::Enter => e.newline(),
            KeyCode::Tab => e.insert_str("  "),
            KeyCode::Backspace => e.backspace(),
            KeyCode::Delete => e.delete(),
            KeyCode::Left => e.left(),
            KeyCode::Right => e.right(),
            KeyCode::Up => e.move_rows(-1),
            KeyCode::Down => e.move_rows(1),
            KeyCode::PageUp => e.move_rows(-10),
            KeyCode::PageDown => e.move_rows(10),
            KeyCode::Home => e.home(),
            KeyCode::End => e.end(),
            _ => {}
        }
    }

    pub(super) fn mouse_snippets(&mut self, x: u16, y: u16, wheel: Option<isize>, click: bool) {
        let (list, ed) = (self.hits.snip_list, self.hits.editor);
        if let Some(d) = wheel {
            if inside(ed, x, y) {
                let h = ed.height as usize;
                self.editor.scroll(d * 3, h);
            } else {
                let mut st = self.snip_state;
                Self::move_sel(&mut st, self.snippets.len(), d);
                if let Some(i) = st.selected() {
                    self.select_snippet(i);
                }
            }
            return;
        }
        if !click {
            return;
        }
        if inside(list, x, y) {
            let i = self.snip_state.offset() + (y - list.y) as usize;
            if i < self.snippets.len() {
                self.select_snippet(i);
                self.editor_focus = false;
            }
        } else if inside(ed, x, y) && self.editor_name.is_some() {
            self.editor_focus = true;
            self.editor.click((x - ed.x) as usize, (y - ed.y) as usize);
        }
    }

    pub(super) fn draw_snippets(&mut self, f: &mut Frame, area: Rect) {
        let [left, right] = Layout::horizontal([Constraint::Length(28), Constraint::Min(20)]).areas(area);

        let block = Block::bordered().title(" Snippets ");
        let inner = block.inner(left);
        self.hits.snip_list = inner;
        let items: Vec<ListItem> = self.snippets.iter().map(|s| ListItem::new(s.clone()).style(Style::new().fg(kind_color(s)))).collect();
        let list = List::new(items)
            .block(block)
            .highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(list, left, &mut self.snip_state);
        let bar_a = Rect { x: left.x, width: left.width, ..inner };
        Self::scrollbar(f, bar_a, self.snippets.len(), self.snip_state.offset());

        let text = self.editor.text();
        let is_prompt = self.editor_name.as_deref().is_some_and(config::is_prompt_snippet);
        let ps1 = if is_prompt { prompt::file_ps1(&text) } else { prompt::extract_ps1(&text) };
        let preview_h = ps1.as_ref().map_or(0, |p| prompt::render_ps1(p).len() as u16 + 2);
        let [ed_a, prev_a] =
            Layout::vertical([Constraint::Min(5), Constraint::Length(preview_h)]).areas(right);

        let title = match &self.editor_name {
            Some(n) if self.editor.dirty => format!(" {} (modified) ", config::snippet_file(n)),
            Some(n) => format!(" {} ", config::snippet_file(n)),
            None => " no snippet: press Ctrl-N to create one ".to_string(),
        };
        let border = if self.editor_focus { Color::Cyan } else { Color::Reset };
        let block = Block::bordered().title(title).border_style(Style::new().fg(border));
        let ed_inner = block.inner(ed_a);
        f.render_widget(block, ed_a);
        self.hits.editor = ed_inner;

        self.editor.keep_cursor_visible(ed_inner.width as usize, ed_inner.height as usize);
        // Only bash gets bash colors. A prompt theme is bash when it has a `PS1=` line, otherwise it
        // is just the PS1 string and gets its own colors, and so does vim script. Tmux and inputrc text
        // stays plain.
        let is_shell = self.editor_name.as_deref().is_none_or(|n| match config::snippet_kind(n) {
            config::SnippetKind::Shell => true,
            config::SnippetKind::Prompt => prompt::is_script(&text),
            _ => false,
        });
        let is_ps1 = self.editor_name.as_deref().is_some_and(|n| {
            config::snippet_kind(n) == config::SnippetKind::Prompt && !prompt::is_script(&text)
        });
        let is_vim = self.editor_name.as_deref().is_some_and(|n| config::snippet_kind(n) == config::SnippetKind::Vim);
        let is_inputrc = self.editor_name.as_deref().is_some_and(|n| config::snippet_kind(n) == config::SnippetKind::Inputrc);
        let kinds = if is_shell {
            highlight::highlight(&self.editor.lines)
        } else if is_vim {
            highlight::highlight_vim(&self.editor.lines)
        } else if is_inputrc {
            highlight::highlight_inputrc(&self.editor.lines)
        } else if is_ps1 {
            highlight::highlight_ps1(&self.editor.lines)
        } else {
            self.editor.lines.iter().map(|l| vec![highlight::Kind::Plain; l.chars().count()]).collect()
        };
        let (left, width) = (self.editor.left, ed_inner.width as usize);
        let lines: Vec<Line> = self
            .editor
            .lines
            .iter()
            .zip(&kinds)
            .skip(self.editor.top)
            .take(ed_inner.height as usize)
            .map(|(l, k)| highlighted_line(l, k, left, width))
            .collect();
        f.render_widget(Paragraph::new(lines), ed_inner);
        let bar_a = Rect { x: ed_a.x, width: ed_a.width, ..ed_inner };
        Self::scrollbar(f, bar_a, self.editor.lines.len(), self.editor.top);
        if self.editor_focus && matches!(self.mode, Mode::Normal) {
            let cx = ed_inner.x + (self.editor.col - self.editor.left) as u16;
            let cy = ed_inner.y + (self.editor.row - self.editor.top) as u16;
            f.set_cursor_position((cx, cy));
        }

        if let Some(ps1) = ps1 {
            let block = Block::bordered().title(" Prompt preview ");
            let inner = block.inner(prev_a);
            f.render_widget(block, prev_a);
            f.render_widget(Paragraph::new(prompt::render_ps1(&ps1)), inner);
        }
    }
}
