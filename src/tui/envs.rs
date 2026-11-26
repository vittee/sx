//! Envs tab: create environments and edit their label, color, patterns and features.

use super::*;
use config::Env;

/// Which pane of the Envs tab has the keyboard.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum EnvFocus {
    List,
    Fields,
}

/// The rows of the form. Rows 3 to 6 switch a feature, the others are text. The guarded commands
/// come last so they sit under the `guard` switch they belong to.
const FIELDS: [&str; 8] =
    ["label", "color", "host patterns", "badge", "title", "confirm", "guard", "guarded commands"];

/// Width of the name column of a form row, with its leading space. The checkbox follows it.
pub(super) const NAME_WIDTH: u16 = 18;

/// Whether a row of the form is typed into, not switched.
fn is_text(field: usize) -> bool {
    matches!(field, 0..=2 | 7)
}

/// The feature a switch row of the form controls.
fn field_feature(field: usize) -> &'static str {
    match field {
        3 => config::FEATURE_BADGE,
        4 => config::FEATURE_TITLE,
        5 => config::FEATURE_CONFIRM,
        _ => config::FEATURE_GUARD,
    }
}

/// The look of a badge: the color of the environment, with black or white text. A color that is not
/// recognized gives plain bold text.
pub(super) fn badge_style(color: &str) -> Style {
    match config::badge_colors(color) {
        Some((index, light)) => Style::new()
            .bg(match config::hex_rgb(color) {
                Some((r, g, b)) if config::terminal_truecolor() => Color::Rgb(r, g, b),
                _ => Color::Indexed(index),
            })
            .fg(if light { Color::Black } else { Color::White })
            .add_modifier(Modifier::BOLD),
        None => Style::new().add_modifier(Modifier::BOLD),
    }
}

impl App {
    pub(super) fn env_names(&self) -> Vec<String> {
        self.cfg.envs.keys().cloned().collect()
    }

    pub(super) fn selected_env(&self) -> Option<String> {
        self.env_names().get(self.env_state.selected()?).cloned()
    }

    /// Selects an environment by name and reads its field into the edit buffer.
    fn select_env(&mut self, name: &str) {
        let i = self.env_names().iter().position(|n| n == name).unwrap_or(0);
        self.env_state.select(Some(i));
        self.load_env_buf();
    }

    /// Copies the focused text field of the selected environment into the edit buffer.
    pub(super) fn load_env_buf(&mut self) {
        let text = self
            .selected_env()
            .and_then(|n| {
                let e = self.cfg.envs.get(&n)?;
                Some(match self.env_field {
                    0 => e.label.clone(),
                    1 => e.color.clone(),
                    2 => e.patterns.join(", "),
                    7 => e.guard.join(", "),
                    _ => String::new(),
                })
            })
            .unwrap_or_default();
        self.env_buf = Editor::from_text(&text);
        self.env_buf.end();
    }

    /// The text of the focused row, as typed.
    fn env_text(&self) -> &str {
        &self.env_buf.lines[0]
    }

    /// Writes the edit buffer into the selected environment and saves it.
    fn apply_env_buf(&mut self) {
        let Some(name) = self.selected_env() else { return };
        let buf = self.env_text().to_string();
        let field = self.env_field;
        let Some(env) = self.cfg.envs.get_mut(&name) else { return };
        match field {
            0 => env.label = buf,
            1 => env.color = buf,
            2 => {
                env.patterns =
                    buf.split(|c: char| c == ',' || c.is_whitespace()).filter(|s| !s.is_empty()).map(String::from).collect();
            }
            7 => env.guard = buf.split(',').map(str::trim).filter(|s| !s.is_empty()).map(String::from).collect(),
            _ => return,
        }
        self.save_cfg();
    }

    fn toggle_env_feature(&mut self, feature: &str) {
        let Some(name) = self.selected_env() else { return };
        if let Some(env) = self.cfg.envs.get_mut(&name) {
            match env.features.iter().position(|f| f == feature) {
                Some(i) => {
                    env.features.remove(i);
                }
                None => env.features.push(feature.to_string()),
            }
        }
        self.save_cfg();
    }

    /// Creates an environment. A name from the templates (`prod`, `staging`, ...) starts with that
    /// template's color and patterns, any other name starts blank.
    pub(super) fn create_env(&mut self, name: &str) {
        let name = name.trim();
        let env = config::env_templates()
            .into_iter()
            .find(|(n, _)| *n == name)
            .map(|(_, e)| e)
            .unwrap_or_else(|| Env::new("white", &[]));
        match self.cfg.add_env(name, env) {
            Ok(()) => {
                self.save_cfg();
                self.select_env(name);
                self.say(format!("created environment {name}"));
            }
            Err(e) => self.fail(e),
        }
    }

    pub(super) fn duplicate_env(&mut self, from: &str, to: &str) {
        let to = to.trim();
        match self.cfg.duplicate_env(from, to) {
            Ok(()) => {
                self.save_cfg();
                self.select_env(to);
                self.say(format!("copied environment {from} to {to}"));
            }
            Err(e) => self.fail(e),
        }
    }

    pub(super) fn rename_env(&mut self, from: &str, to: &str) {
        let to = to.trim();
        match self.cfg.rename_env(from, to) {
            Ok(()) => {
                self.save_cfg();
                self.select_env(to);
                self.say(format!("renamed environment {from} to {to}"));
            }
            Err(e) => self.fail(e),
        }
    }

    pub(super) fn delete_env(&mut self, name: &str) {
        self.cfg.delete_env(name);
        self.save_cfg();
        self.env_state.select(Some(0));
        self.load_env_buf();
        self.say(format!("deleted environment {name}, hosts that had it are untagged"));
    }

    fn focus_env_field(&mut self, field: usize) {
        self.env_field = field.min(FIELDS.len() - 1);
        self.env_focus = EnvFocus::Fields;
        self.load_env_buf();
    }

    pub(super) fn on_key_envs(&mut self, k: KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        if self.env_focus == EnvFocus::Fields {
            return self.on_key_env_fields(k);
        }
        if self.global_nav(k) {
            return;
        }
        let len = self.cfg.envs.len();
        match k.code {
            KeyCode::Up => {
                Self::move_sel(&mut self.env_state, len, -1);
                self.load_env_buf();
            }
            KeyCode::Down => {
                Self::move_sel(&mut self.env_state, len, 1);
                self.load_env_buf();
            }
            KeyCode::Right | KeyCode::Enter if len > 0 => self.focus_env_field(self.env_field),
            KeyCode::Char('n') if ctrl => self.mode = Mode::Input(InputKind::NewEnv, String::new()),
            KeyCode::Char('y') if ctrl => {
                if let Some(n) = self.selected_env() {
                    self.mode = Mode::Input(InputKind::DuplicateEnv(n.clone()), format!("{n}-copy"));
                }
            }
            KeyCode::Char('r') if ctrl => {
                if let Some(n) = self.selected_env() {
                    self.mode = Mode::Input(InputKind::RenameEnv(n.clone()), n);
                }
            }
            KeyCode::Char('d') if ctrl => {
                if let Some(n) = self.selected_env() {
                    self.mode = Mode::Confirm(Pending::Env(n));
                }
            }
            KeyCode::Esc => self.quit = true,
            _ => {}
        }
    }

    fn on_key_env_fields(&mut self, k: KeyEvent) {
        let text = is_text(self.env_field);
        if text && select_key(&mut self.env_buf, k, false) {
            return;
        }
        match k.code {
            KeyCode::Esc => self.env_focus = EnvFocus::List,
            KeyCode::Left if !text => self.env_focus = EnvFocus::List,
            KeyCode::Up | KeyCode::BackTab => self.focus_env_field(self.env_field.saturating_sub(1)),
            KeyCode::Down | KeyCode::Tab => self.focus_env_field(self.env_field + 1),
            KeyCode::Enter if text => self.focus_env_field(self.env_field + 1),
            KeyCode::Left if text => self.env_buf.left(),
            KeyCode::Right if text => self.env_buf.right(),
            KeyCode::Home if text => self.env_buf.home(),
            KeyCode::End if text => self.env_buf.end(),
            KeyCode::Char(c) if text && !k.modifiers.contains(KeyModifiers::CONTROL) => {
                let selected = self.env_buf.selected_cols(0).map_or(0, |(from, to)| to - from);
                if self.env_field == 0 && self.env_text().chars().count() - selected >= config::MAX_LABEL {
                    return;
                }
                self.env_buf.insert_char(c);
                self.apply_env_buf();
            }
            KeyCode::Backspace if text => {
                self.env_buf.backspace();
                self.apply_env_buf();
            }
            KeyCode::Delete if text => {
                self.env_buf.delete();
                self.apply_env_buf();
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                self.toggle_env_feature(field_feature(self.env_field));
            }
            _ => {}
        }
    }

    pub(super) fn mouse_envs(&mut self, x: u16, y: u16, wheel: Option<isize>, click: bool) {
        let (list, fields) = (self.hits.env_list, self.hits.env_fields);
        if let Some(d) = wheel {
            let len = self.cfg.envs.len();
            Self::move_sel(&mut self.env_state, len, d);
            self.load_env_buf();
            return;
        }
        if !click {
            return;
        }
        if inside(list, x, y) {
            let i = self.env_state.offset() + (y - list.y) as usize;
            if i < self.cfg.envs.len() {
                self.env_state.select(Some(i));
                self.env_focus = EnvFocus::List;
                self.load_env_buf();
            }
        } else if inside(fields, x, y) && self.selected_env().is_some() {
            let field = (y - fields.y) as usize;
            if field < FIELDS.len() {
                self.focus_env_field(field);
                if is_text(field) {
                    self.env_buf.click(x.saturating_sub(fields.x + NAME_WIDTH) as usize, 0);
                    self.text_drag = Some(TextDrag::Env);
                }
                // only the checkbox switches, a click on its text just focuses the row
                if !is_text(field) && (NAME_WIDTH..NAME_WIDTH + 3).contains(&(x - fields.x)) {
                    self.toggle_env_feature(field_feature(field));
                }
            }
        }
    }

    pub(super) fn draw_envs(&mut self, f: &mut Frame, area: Rect) {
        let [left, right] = Layout::horizontal([Constraint::Length(28), Constraint::Min(20)]).areas(area);

        let block = Block::bordered().title(" Environments ");
        let inner = block.inner(left);
        self.hits.env_list = inner;
        let items: Vec<ListItem> = self
            .cfg
            .envs
            .iter()
            .map(|(name, env)| {
                let badge = format!(" {} ", env.label_for(name));
                ListItem::new(Line::from(vec![Span::styled(badge, badge_style(&env.color)), Span::raw(format!(" {name}"))]))
            })
            .collect();
        let hl = if self.env_focus == EnvFocus::List { Modifier::REVERSED } else { Modifier::BOLD };
        let list = List::new(items).block(block).highlight_style(Style::new().add_modifier(hl));
        f.render_stateful_widget(list, left, &mut self.env_state);
        Self::scrollbar(f, Rect { x: left.x, width: left.width, ..inner }, self.cfg.envs.len(), self.env_state.offset());

        let focused = self.env_focus == EnvFocus::Fields;
        let border = if focused { Color::Cyan } else { Color::Reset };
        let Some(name) = self.selected_env() else {
            let hint = vec![
                Line::raw(""),
                Line::raw(" No environments yet. Press Ctrl-N to create one."),
                Line::styled(
                    format!(" Names like {} start from a template.", config::env_templates().iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")),
                    Style::new().add_modifier(Modifier::DIM),
                ),
            ];
            f.render_widget(Paragraph::new(hint).block(Block::bordered().title(" Environment ")), right);
            self.hits.env_fields = Rect::default();
            return;
        };
        let env = self.cfg.envs.get(&name).cloned().unwrap_or_else(|| Env::new("white", &[]));
        let block = Block::bordered().title(format!(" {name} ")).border_style(Style::new().fg(border));
        let inner = block.inner(right);
        f.render_widget(block, right);
        self.hits.env_fields = Rect { height: FIELDS.len() as u16, ..inner };

        let dim = Style::new().add_modifier(Modifier::DIM);
        let mut lines = Vec::new();
        let mut cursor = None;
        let room = inner.width.saturating_sub(NAME_WIDTH + 1) as usize;
        for (i, label) in FIELDS.iter().enumerate() {
            let at = focused && i == self.env_field;
            let key = Span::styled(
                format!(" {label:<w$}", w = NAME_WIDTH as usize - 1),
                if at { Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD) } else { dim },
            );
            let value = match i {
                0..=2 | 7 => {
                    let text = if at {
                        // the terminal's own cursor marks the position, a long text scrolls sideways
                        self.env_buf.keep_cursor_visible(room, 1);
                        cursor = Some((inner.x + NAME_WIDTH + (self.env_buf.col - self.env_buf.left) as u16, inner.y + i as u16));
                        self.env_text().chars().skip(self.env_buf.left).collect()
                    } else {
                        match i {
                            0 => env.label.clone(),
                            1 => env.color.clone(),
                            2 => env.patterns.join(", "),
                            _ => env.guard.join(", "),
                        }
                    };
                    let mut spans = vec![Span::raw(text)];
                    if at && let Some((from, to)) = self.env_buf.selected_cols(0) {
                        let left = self.env_buf.left;
                        spans = super::snippets::reversed_range(Line::from(spans), from.saturating_sub(left), to.saturating_sub(left)).spans;
                    }
                    if !at && i == 0 && env.label.is_empty() {
                        spans.push(Span::styled("(the name in capitals)", dim));
                    } else if !at && i == 7 && env.guard.is_empty() {
                        spans.push(Span::styled("(commands to ask about, separated by commas, * is any text)", dim));
                    } else if !at && i == 2 && env.patterns.is_empty() {
                        spans.push(Span::styled("(host names to suggest this environment for, * is any text)", dim));
                    }
                    if i == 1 && config::badge_colors(&env.color).is_none() {
                        spans.push(Span::styled("  not a color: a name, 0-255 or #rrggbb", Style::new().fg(Color::Red)));
                    }
                    spans
                }
                _ => {
                    let on = env.has(field_feature(i));
                    let mark = Style::new().fg(if on { Color::Green } else { Color::DarkGray });
                    let mark = if at { mark.add_modifier(Modifier::REVERSED) } else { mark };
                    let what = match i {
                        3 => "the label in the prompt, before the $ or #",
                        4 => "the label in the window title",
                        5 => "ask before connecting to a host of this environment",
                        _ => "ask before running a command that matches the guarded commands below",
                    };
                    vec![Span::styled(if on { "[x]" } else { "[ ]" }, mark), Span::styled(format!(" {what}"), dim)]
                }
            };
            let mut spans = vec![key];
            spans.extend(value);
            lines.push(Line::from(spans));
        }

        lines.push(Line::styled("─".repeat(inner.width as usize), dim));
        lines.push(Line::from(vec![
            Span::styled(format!(" {:<w$}", "preview", w = NAME_WIDTH as usize - 1), dim),
            Span::styled(format!(" {} ", env.label_for(&name)), badge_style(&env.color)),
            Span::raw(" me@host:~$"),
        ]));
        let cap = |hosts: Vec<String>| match hosts.len() {
            0 => "none".to_string(),
            n if n > 6 => format!("{} and {} more", hosts[..6].join(", "), n - 6),
            _ => hosts.join(", "),
        };
        let tagged: Vec<String> = self
            .hosts
            .iter()
            .filter(|h| self.cfg.env_of(&h.alias).is_some_and(|(n, _)| n == name))
            .map(|h| h.alias.clone())
            .collect();
        let suggested: Vec<String> = self
            .hosts
            .iter()
            .filter(|h| self.cfg.env_of(&h.alias).is_none() && self.cfg.suggest_env(&h.alias) == Some(name.as_str()))
            .map(|h| h.alias.clone())
            .collect();
        let name_col = |text: &str| Span::styled(format!(" {text:<w$}", w = NAME_WIDTH as usize - 1), dim);
        lines.push(Line::from(vec![name_col("tagged hosts"), Span::raw(cap(tagged))]));
        lines.push(Line::from(vec![
            name_col("matching hosts"),
            Span::raw(cap(suggested)),
            Span::styled("  (not tagged yet, the name fits a host pattern)", dim),
        ]));
        f.render_widget(Paragraph::new(lines), inner);
        if let (Some((x, y)), Mode::Normal) = (cursor, &self.mode) {
            f.set_cursor_position((x, y));
        }
    }
}
