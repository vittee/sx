//! Per-host connection options: a small form for the user, port, identity file, jump host and
//! extra ssh arguments. A host has a default options and any number of named ones, for example `root`
//! or `deploy via bastion`. They are stored in `config.toml` and passed to ssh when connecting, so
//! `~/.ssh/config` is never changed.

use super::*;
use crate::config::ConnOptions;
use std::collections::BTreeMap;

const LABELS: [&str; 5] = ["User", "Port", "Identity file", "Jump host", "Extra ssh args"];
const EXAMPLES: [&str; 5] = ["", "", "~/.ssh/id_ed25519", "bastion.example.com", ""];
const PORT: usize = 1;
const IDENTITY: usize = 2;
const JUMP: usize = 3;
/// Rows of the pick list (keys or hosts) shown at once.
const KEY_ROWS: usize = 5;
/// The third hint row. Its last hint shows only on the identity and jump host fields, but the columns
/// are sized for the longest of it, so the hints of the rows above do not move.
const THIRD_ROW: [(&str, &str); 3] = [("^S", "save all"), ("^U", "clear field"), ("^K", "pick jump host")];
const PICK_KEY: (&str, &str) = ("^K", "pick key file");
const LABEL_W: usize = 16;
const DEFAULT: &str = "default";
/// The colour of the options form, its picker, and the target of a host that has options.
pub(super) const ACCENT: Color = Color::LightMagenta;

/// The form for one host. An empty field means "use `~/.ssh/config`".
#[derive(Clone, Debug)]
pub(super) struct HostForm {
    host: String,
    default: ConnOptions,
    options: BTreeMap<String, ConnOptions>,
    /// The options being edited: `None` is the default one.
    current: Option<String>,
    /// The fields of the options being edited. They go back into `default` or `options` on a switch.
    values: [String; 5],
    /// What `~/.ssh/config` says, shown dim in an empty field.
    hints: [String; 5],
    focus: usize,
    /// The name being typed for new options.
    naming: Option<String>,
    /// What was saved when the form opened, to tell whether there is anything to lose.
    saved: (ConnOptions, BTreeMap<String, ConnOptions>),
    /// Asking whether to discard unsaved changes.
    confirming: bool,
    /// The name being typed is a new name for the current options, not for new options.
    renaming: bool,
    /// The private keys found in `~/.ssh`, as `~/.ssh/name`.
    keys: Vec<String>,
    /// The other hosts, to pick a jump host from.
    jumps: Vec<String>,
    /// The row chosen in the open pick list (Ctrl-K), counted in `key_matches`.
    key_pick: Option<usize>,
}

/// The private keys in `dir`: files that start with a PEM/OpenSSH header or have a `.pub` beside them.
fn scan_keys(dir: &std::path::Path) -> Vec<String> {
    use std::io::Read;
    const SKIP: [&str; 5] = ["known_hosts", "known_hosts.old", "authorized_keys", "config", "environment"];
    let mut keys: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let meta = e.metadata().ok()?;
            if !meta.is_file() || meta.len() > 16 * 1024 || name.ends_with(".pub") || SKIP.contains(&name.as_str()) {
                return None;
            }
            let mut head = [0u8; 10];
            let pem = std::fs::File::open(e.path()).and_then(|mut f| f.read_exact(&mut head)).is_ok() && head == *b"-----BEGIN";
            (pem || dir.join(format!("{name}.pub")).is_file()).then(|| format!("~/.ssh/{name}"))
        })
        .collect();
    keys.sort();
    keys
}

/// Words in a host name that suggest a jump host.
const JUMP_WORDS: [&str; 7] = ["bastion", "jump", "gateway", "gw", "proxy", "hop", "bridge"];

/// The other hosts, the likely jump hosts first: those that saved options already use as a jump
/// host, then those with a name like `bastion`. The rest keep the list order.
fn jump_candidates(hosts: &[HostEntry], cfg: &config::Config, this: &str) -> Vec<String> {
    let mut used = std::collections::HashSet::new();
    for p in cfg.hosts.values() {
        for o in std::iter::once(p.default_options()).chain(p.options.values().cloned()) {
            for j in o.jump.split(',') {
                let j = j.trim();
                let j = j.rsplit_once('@').map_or(j, |(_, h)| h);
                used.insert(j.rsplit_once(':').map_or(j, |(h, _)| h).to_string());
            }
        }
    }
    let rank = |h: &HostEntry| {
        let name = h.alias.to_lowercase();
        if used.contains(&h.alias) || h.hostname.as_ref().is_some_and(|n| used.contains(n)) {
            0
        } else if JUMP_WORDS.iter().any(|w| name.split(|c: char| !c.is_alphanumeric()).any(|t| t == *w)) {
            1
        } else {
            2
        }
    };
    let mut others: Vec<&HostEntry> = hosts.iter().filter(|h| h.alias != this).collect();
    others.sort_by_key(|h| rank(h));
    others.into_iter().map(|h| h.alias.clone()).collect()
}

fn to_values(o: &ConnOptions) -> [String; 5] {
    [o.user.clone(), o.port.clone(), o.identity.clone(), o.jump.clone(), o.args.clone()]
}

fn from_values(v: &[String; 5]) -> ConnOptions {
    let [user, port, identity, jump, args] = v.each_ref().map(|s| s.trim().to_string());
    ConnOptions { user, port, identity, jump, args }
}

impl HostForm {
    /// What the focused field can be picked from: keys for the identity file, hosts for the jump host.
    fn choices(&self) -> &[String] {
        match self.focus {
            IDENTITY => &self.keys,
            JUMP => &self.jumps,
            _ => &[],
        }
    }

    /// The choices that fit what is typed in the focused field.
    fn key_matches(&self) -> Vec<&String> {
        let typed = self.values[self.focus].trim().to_lowercase();
        self.choices().iter().filter(|k| k.to_lowercase().contains(&typed)).collect()
    }

    /// Puts the chosen key or host into the focused field and closes the list.
    fn take_key(&mut self, key: &str) {
        self.values[self.focus] = key.to_string();
        self.key_pick = None;
    }

    fn new(host: &str, prefs: Option<&config::HostPrefs>, entry: &HostEntry, keys: Vec<String>, jumps: Vec<String>) -> HostForm {
        let p = prefs.cloned().unwrap_or_default();
        let mut hints: [String; 5] = EXAMPLES.map(|e| e.to_string());
        hints[0] = entry.user.clone().unwrap_or_default();
        hints[PORT] = entry.port.clone().unwrap_or_default();
        let default = p.default_options();
        HostForm {
            host: host.to_string(),
            values: to_values(&default),
            default,
            options: p.options.clone(),
            current: None,
            hints,
            focus: 0,
            naming: None,
            saved: (p.default_options(), p.options.clone()),
            confirming: false,
            renaming: false,
            keys,
            jumps,
            key_pick: None,
        }
    }

    /// Writes the fields back into the options being edited.
    fn commit(&mut self) {
        let o = from_values(&self.values);
        match &self.current {
            None => self.default = o,
            Some(name) => {
                self.options.insert(name.clone(), o);
            }
        }
    }

    /// The options names in display order: the default one first.
    fn names(&self) -> Vec<Option<String>> {
        std::iter::once(None).chain(self.options.keys().cloned().map(Some)).collect()
    }

    fn switch_to(&mut self, to: Option<String>) {
        self.commit();
        self.values = match &to {
            None => to_values(&self.default),
            Some(name) => to_values(&self.options[name]),
        };
        self.current = to;
        self.focus = 0;
    }

    fn step_options(&mut self, delta: isize) {
        let names = self.names();
        let at = names.iter().position(|n| *n == self.current).unwrap_or(0) as isize;
        let next = (at + delta).rem_euclid(names.len() as isize) as usize;
        self.switch_to(names[next].clone());
    }

    fn add_options(&mut self, name: &str) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("give the options a name".to_string());
        }
        if name == DEFAULT || self.options.contains_key(name) {
            return Err(format!("a options named {name} already exists"));
        }
        self.commit();
        // The new options start as a copy of the one being edited, to change a value or two.
        let copy = from_values(&self.values);
        self.options.insert(name.to_string(), copy);
        self.switch_to(Some(name.to_string()));
        Ok(())
    }

    /// Gives the current options a new name. The default options cannot be renamed.
    fn rename_options(&mut self, name: &str) -> Result<(), String> {
        let name = name.trim();
        let Some(old) = self.current.clone() else {
            return Err("the default options cannot be renamed".to_string());
        };
        if name.is_empty() {
            return Err("give the options a name".to_string());
        }
        if name != old && (name == DEFAULT || self.options.contains_key(name)) {
            return Err(format!("options named {name} already exist"));
        }
        self.commit();
        if let Some(o) = self.options.remove(&old) {
            self.options.insert(name.to_string(), o);
        }
        self.current = Some(name.to_string());
        Ok(())
    }

    fn delete_options(&mut self) {
        if let Some(name) = self.current.take() {
            self.options.remove(&name);
            self.values = to_values(&self.default);
            self.focus = 0;
        }
    }

    /// True when the form holds changes that are not saved.
    fn is_dirty(&self) -> bool {
        let mut now = self.clone();
        now.commit();
        (now.default, now.options) != self.saved
    }

    /// Esc or a click outside: closes the form, or asks first when there are unsaved changes.
    /// True when the form should close.
    fn want_close(&mut self) -> bool {
        if self.is_dirty() {
            self.confirming = true;
            return false;
        }
        true
    }

    /// The name of the options being edited, `None` for the default ones.
    #[cfg(test)]
    pub(super) fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    pub(super) fn is_naming(&self) -> bool {
        self.naming.is_some()
    }

    /// Checks every field of the options, or says what is wrong.
    fn validate(o: &ConnOptions, label: &str) -> Result<(), String> {
        if !o.port.is_empty() && !o.port.parse::<u16>().is_ok_and(|p| p > 0) {
            return Err(format!("{label}: port must be a number from 1 to 65535"));
        }
        if o.user.chars().any(char::is_whitespace) {
            return Err(format!("{label}: user cannot contain spaces"));
        }
        Ok(())
    }

    /// The options being edited, checked.
    fn current_options(&self) -> Result<ConnOptions, String> {
        let o = from_values(&self.values);
        Self::validate(&o, self.current.as_deref().unwrap_or(DEFAULT))?;
        Ok(o)
    }
}

/// The choice of named options when connecting to a host (Ctrl-L).
#[derive(Clone, Debug)]
pub(super) struct OptionsPicker {
    host: String,
    /// `None` is the default options.
    names: Vec<Option<String>>,
    sel: usize,
}

impl App {
    /// Opens the form for the selected host (Ctrl-O).
    pub(super) fn open_host_options(&mut self) {
        if let Some(i) = self.selected_host() {
            let entry = &self.hosts[i];
            let keys = self.ssh_dir.as_deref().map(scan_keys).unwrap_or_default();
            let jumps = jump_candidates(&self.hosts, &self.cfg, &entry.alias);
            let form = HostForm::new(&entry.alias, self.cfg.hosts.get(&entry.alias), entry, keys, jumps);
            self.popup_offset = (0, 0);
            self.mode = Mode::HostForm(Box::new(form));
        }
    }

    /// True when the selected host has named options to pick from.
    pub(super) fn can_pick_options(&self) -> bool {
        self.selected_host()
            .and_then(|i| self.cfg.hosts.get(&self.hosts[i].alias))
            .is_some_and(|p| !p.options.is_empty())
    }

    /// Opens the choice of named options for the selected host (Ctrl-L).
    pub(super) fn open_options_picker(&mut self) {
        let Some(i) = self.selected_host() else { return };
        self.popup_offset = (0, 0);
        let host = self.hosts[i].alias.clone();
        let prefs = self.cfg.hosts.get(&host);
        if prefs.is_none_or(|p| p.options.is_empty()) {
            self.say(format!("{host} has no named options: Ctrl-O adds them"));
            return;
        }
        let names = std::iter::once(None).chain(prefs.into_iter().flat_map(|p| p.options.keys().cloned().map(Some))).collect();
        self.mode = Mode::OptionsPicker(Box::new(OptionsPicker { host, names, sel: 0 }));
    }

    pub(super) fn on_key_options_picker(&mut self, k: KeyEvent, mut pick: OptionsPicker) {
        let n = pick.names.len();
        match k.code {
            KeyCode::Esc | KeyCode::Char('q') => return,
            KeyCode::Up => pick.sel = (pick.sel + n - 1) % n,
            KeyCode::Down => pick.sel = (pick.sel + 1) % n,
            KeyCode::Enter => {
                self.connect_with_options(&pick.host, pick.names[pick.sel].as_deref());
                return;
            }
            _ => {}
        }
        self.mode = Mode::OptionsPicker(Box::new(pick));
    }

    /// Connects with saved options, `None` for the default one.
    pub(super) fn connect_with_options(&mut self, host: &str, named: Option<&str>) {
        let Some(i) = self.hosts.iter().position(|h| h.alias == host) else { return };
        let prefs = self.cfg.hosts.get(host).map(|p| match named {
            None => p.default_options(),
            Some(name) => p.options.get(name).cloned().unwrap_or_default(),
        });
        self.pending_connect = Some(self.connect_args(i, prefs.as_ref()));
    }

    pub(super) fn on_key_host_form(&mut self, k: KeyEvent, mut form: HostForm) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        if form.confirming {
            match k.code {
                KeyCode::Char('y') if !ctrl => return, // discard
                KeyCode::Char('s') if ctrl => match self.save_host_form(&mut form) {
                    Ok(()) => return,
                    Err(msg) => {
                        self.fail(msg);
                        form.confirming = false;
                    }
                },
                KeyCode::Char('n') | KeyCode::Esc => form.confirming = false,
                _ => {}
            }
            self.mode = Mode::HostForm(Box::new(form));
            return;
        }
        if let Some(mut name) = form.naming.take() {
            match k.code {
                KeyCode::Esc => form.renaming = false,
                KeyCode::Enter => {
                    let done = if form.renaming { form.rename_options(&name) } else { form.add_options(&name) };
                    match done {
                        Ok(()) => form.renaming = false,
                        Err(msg) => {
                            self.fail(msg);
                            form.naming = Some(name);
                        }
                    }
                }
                KeyCode::Backspace => {
                    name.pop();
                    form.naming = Some(name);
                }
                KeyCode::Char(c) if !ctrl => {
                    name.push(c);
                    form.naming = Some(name);
                }
                _ => form.naming = Some(name),
            }
            self.mode = Mode::HostForm(Box::new(form));
            return;
        }
        // The open key list takes the arrows, Enter and Esc. Other keys act as usual and close it.
        let was_open = form.key_pick.is_some();
        if let Some(sel) = form.key_pick {
            let matches: Vec<String> = form.key_matches().into_iter().cloned().collect();
            let m = matches.len();
            match k.code {
                KeyCode::Esc => form.key_pick = None,
                KeyCode::Up if m > 0 => form.key_pick = Some((sel + m - 1) % m),
                KeyCode::Down if m > 0 => form.key_pick = Some((sel + 1) % m),
                KeyCode::Enter if m > 0 => form.take_key(&matches[sel.min(m - 1)]),
                _ => {}
            }
            if matches!(k.code, KeyCode::Esc | KeyCode::Up | KeyCode::Down | KeyCode::Enter) {
                self.mode = Mode::HostForm(Box::new(form));
                return;
            }
        }
        let (field, before) = (form.focus, form.values[form.focus].clone());
        let n = LABELS.len();
        match k.code {
            KeyCode::Esc if form.want_close() => return,
            KeyCode::Esc => {}
            KeyCode::Enter => {
                // Connect with the options being edited this time only: nothing is saved.
                match form.current_options() {
                    Ok(o) => {
                        // Options created and used right away are meant to be kept.
                        if let Some(name) = form.current.as_ref().filter(|n| !form.saved.1.contains_key(*n)) {
                            self.cfg.hosts.entry(form.host.clone()).or_default().options.insert(name.clone(), o.clone());
                            self.save_cfg();
                        }
                        if let Some(i) = self.hosts.iter().position(|h| h.alias == form.host) {
                            self.pending_connect = Some(self.connect_args(i, Some(&o)));
                        }
                        return;
                    }
                    Err(msg) => self.fail(msg),
                }
            }
            KeyCode::Char('s') if ctrl => match self.save_host_form(&mut form) {
                Ok(()) => return,
                Err(msg) => self.fail(msg),
            },
            KeyCode::Char('n') if ctrl => form.naming = Some(String::new()),
            KeyCode::Char('d') if ctrl => form.delete_options(),
            KeyCode::Char('k') if ctrl && matches!(form.focus, IDENTITY | JUMP) => {
                let (none, no_match) = if form.focus == IDENTITY {
                    ("no private keys found in ~/.ssh", "no key in ~/.ssh matches what is typed")
                } else {
                    ("there are no other hosts to pick from", "no host matches what is typed")
                };
                if form.choices().is_empty() {
                    self.fail(none.to_string());
                } else if form.key_matches().is_empty() {
                    self.fail(no_match.to_string());
                } else {
                    form.key_pick = Some(0);
                }
            }
            KeyCode::Char('r') if ctrl => match form.current.clone() {
                Some(name) => {
                    form.naming = Some(name);
                    form.renaming = true;
                }
                None => self.fail("the default options cannot be renamed".to_string()),
            },
            KeyCode::Left => form.step_options(-1),
            KeyCode::Right => form.step_options(1),
            KeyCode::Tab | KeyCode::Down => form.focus = (form.focus + 1) % n,
            KeyCode::BackTab | KeyCode::Up => form.focus = (form.focus + n - 1) % n,
            KeyCode::Backspace => {
                form.values[form.focus].pop();
            }
            KeyCode::Char('u') if ctrl => form.values[form.focus].clear(),
            KeyCode::Char(c) if !ctrl && (form.focus != PORT || c.is_ascii_digit()) => {
                form.values[form.focus].push(c);
            }
            _ => {}
        }
        if was_open {
            // Typing narrows the list and goes back to its first row. Any other key closes it.
            let typed = matches!(k.code, KeyCode::Backspace | KeyCode::Char(_)) && form.values[field] != before;
            form.key_pick = (typed && form.focus == field && !form.key_matches().is_empty()).then_some(0);
        }
        self.mode = Mode::HostForm(Box::new(form));
    }

    /// Saves the default options and the named options of the form.
    fn save_host_form(&mut self, form: &mut HostForm) -> Result<(), String> {
        form.commit();
        HostForm::validate(&form.default, DEFAULT)?;
        for (name, o) in &form.options {
            HostForm::validate(o, name)?;
        }
        let prefs = self.cfg.hosts.entry(form.host.clone()).or_default();
        prefs.set_default_options(form.default.clone());
        prefs.options = form.options.clone();
        let any = prefs.has_overrides() || !prefs.options.is_empty();
        self.save_cfg();
        self.say(if any {
            format!("saved the options of {}", form.host)
        } else {
            format!("{} uses ~/.ssh/config again", form.host)
        });
        Ok(())
    }

    /// The popup rectangle: centred, then moved by the drag offset and kept inside the body.
    pub(super) fn dragged_popup_area(&mut self, body: Rect, w: u16, h: u16) -> Rect {
        let mut area = Self::popup_area(body, w, h);
        let dx = self.popup_offset.0.clamp(body.x as i32 - area.x as i32, body.right() as i32 - area.right() as i32);
        let dy = self.popup_offset.1.clamp(body.y as i32 - area.y as i32, body.bottom() as i32 - area.bottom() as i32);
        self.popup_offset = (dx, dy);
        area.x = (area.x as i32 + dx) as u16;
        area.y = (area.y as i32 + dy) as u16;
        area
    }

    fn popup_area(body: Rect, w: u16, h: u16) -> Rect {
        let (w, h) = (w.min(body.width), h.min(body.height));
        Rect::new(body.x + (body.width - w) / 2, body.y + (body.height - h) / 2, w, h)
    }

    pub(super) fn draw_host_form(&mut self, f: &mut Frame, body: Rect, form: &HostForm) {
        let area = self.dragged_popup_area(body, 90, LABELS.len() as u16 + 7);
        f.render_widget(ratatui::widgets::Clear, area);
        let block = Block::bordered()
            .title(format!(" {}: connection options{} ", form.host, if form.is_dirty() { " (unsaved)" } else { "" }))
            .border_style(Style::new().fg(ACCENT));
        let inner = block.inner(area);
        f.render_widget(block, area);

        let dim = Style::new().add_modifier(Modifier::DIM);
        let accent = Style::new().fg(ACCENT);
        let cursor = Span::styled("▏", accent);

        // The options row: every name, the one being edited highlighted.
        let mut options_tabs = Vec::new();
        let mut arrows = Vec::new();
        let mut options_row = vec![Span::styled(format!(" {:<LABEL_W$}", "Options Name"), dim)];
        if let Some(name) = &form.naming {
            options_row = vec![Span::styled(format!(" {:<LABEL_W$}", if form.renaming { "Rename to" } else { "New options" }), accent.add_modifier(Modifier::BOLD))];
            options_row.push(Span::raw(name.clone()));
            options_row.push(cursor.clone());
        } else {
            let names = form.names();
            let labels: Vec<String> = names.iter().map(|n| n.clone().unwrap_or_else(|| DEFAULT.to_string())).collect();
            let widths: Vec<u16> = labels.iter().map(|l| l.chars().count() as u16 + 2).collect();
            let at = names.iter().position(|n| *n == form.current).unwrap_or(0);
            let avail = inner.width.saturating_sub(1 + LABEL_W as u16);
            let (first, last) = names_window(&widths, at, avail);
            let mut x = inner.x + 1 + LABEL_W as u16;
            if first > 0 {
                arrows.push((Rect::new(x, inner.y, 1, 1), KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)));
                options_row.push(Span::styled("‹ ", accent.add_modifier(Modifier::BOLD)));
                x += 2;
            }
            for i in first..=last {
                let style = if names[i] == form.current { accent.add_modifier(Modifier::BOLD | Modifier::REVERSED) } else { dim };
                options_tabs.push((Rect::new(x, inner.y, widths[i], 1), names[i].clone()));
                x += widths[i] + 1;
                options_row.push(Span::styled(format!(" {} ", labels[i]), style));
                options_row.push(Span::raw(" "));
            }
            if last + 1 < names.len() {
                arrows.push((Rect::new(x, inner.y, 1, 1), KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)));
                options_row.push(Span::styled("›", accent.add_modifier(Modifier::BOLD)));
            }
        }

        let mut lines = vec![Line::from(options_row)];
        lines.extend(LABELS.iter().enumerate().map(|(i, label)| {
            let focused = i == form.focus && form.naming.is_none();
            let label_style = if focused { accent.add_modifier(Modifier::BOLD) } else { Style::new() };
            let mut spans = vec![Span::styled(format!(" {label:<LABEL_W$}"), label_style)];
            let value = &form.values[i];
            if value.is_empty() {
                if focused {
                    spans.push(cursor.clone());
                }
                spans.push(Span::styled(form.hints[i].clone(), dim));
            } else {
                spans.push(Span::raw(value.clone()));
                if focused {
                    spans.push(cursor.clone());
                }
            }
            Line::from(spans)
        }));
        lines.push(Line::raw(""));
        let keys: [&[(&str, &str)]; 3] = if form.confirming {
            [&[("y", "discard the changes"), ("n", "keep editing"), ("^S", "save")], &[], &[]]
        } else if form.naming.is_some() {
            [&[("Enter", if form.renaming { "rename" } else { "add" }), ("Esc", "cancel")], &[], &[]]
        } else {
            // The pick list belongs to the identity and jump host fields, so its hint shows only there.
            let third: &[(&str, &str)] = match form.focus {
                IDENTITY => &[THIRD_ROW[0], THIRD_ROW[1], PICK_KEY],
                JUMP => &THIRD_ROW,
                _ => &THIRD_ROW[..2],
            };
            [
                &[("Enter", "connect once"), ("Esc", "cancel"), ("Tab", "next field")],
                &[("←/→", "switch"), ("^N", "new"), ("^R", "rename"), ("^D", "delete")],
                third,
            ]
        };
        // The hints sit in columns: the keys of a column are padded to one width, so the
        // descriptions line up, and every column starts after the widest cell of the one before.
        let mut sized: Vec<&[(&str, &str)]> = keys.to_vec();
        if !form.confirming && form.naming.is_none() {
            sized[2] = &THIRD_ROW;
        }
        let columns = sized.iter().map(|r| r.len()).max().unwrap_or(0);
        let key_w: Vec<usize> = (0..columns)
            .map(|c| sized.iter().filter_map(|r| r.get(c)).map(|(k, _)| k.chars().count()).max().unwrap_or(0))
            .collect();
        let cell_w: Vec<usize> = (0..columns)
            .map(|c| sized.iter().filter_map(|r| r.get(c)).map(|(_, w)| w.chars().count()).max().unwrap_or(0) + key_w[c] + 1)
            .collect();
        let mut key_hits = Vec::new();
        for row in keys {
            let y = inner.y + lines.len() as u16;
            let mut x = inner.x + 1;
            let mut spans = vec![Span::raw(" ")];
            for (c, (key, what)) in row.iter().enumerate() {
                let w = (key.chars().count() + 1 + what.chars().count()) as u16;
                key_hits.push((Rect::new(x, y, w, 1), hint_key(key)));
                spans.push(Span::styled(format!("{key:<w$}", w = key_w[c]), accent.add_modifier(Modifier::BOLD)));
                spans.push(Span::styled(format!(" {what:<w$}  ", w = cell_w[c] - key_w[c] - 1), dim));
                x += cell_w[c] as u16 + 2;
            }
            lines.push(Line::from(spans));
        }
        self.hits.form_area = area;
        self.hits.form_fields = Rect::new(inner.x, inner.y + 1, inner.width, LABELS.len() as u16);
        self.hits.form_options = options_tabs;
        key_hits.extend(arrows);
        self.hits.form_keys = key_hits;
        f.render_widget(Paragraph::new(lines), inner);

        // The pick list drops down under the focused field, over the rows below it.
        self.hits.form_keylist.clear();
        if let Some(sel) = form.key_pick {
            let matches = form.key_matches();
            let x = inner.x + 1 + LABEL_W as u16;
            let list = Rect::new(x, inner.y + 2 + form.focus as u16, inner.width.saturating_sub(x - inner.x + 1), matches.len().min(KEY_ROWS) as u16);
            if !matches.is_empty() && list.width > 0 {
                let sel = sel.min(matches.len() - 1);
                let first = (sel + 1).saturating_sub(KEY_ROWS);
                f.render_widget(ratatui::widgets::Clear, list);
                let rows: Vec<Line> = (first..matches.len().min(first + KEY_ROWS))
                    .map(|i| {
                        let style = if i == sel { accent.add_modifier(Modifier::BOLD | Modifier::REVERSED) } else { Style::new() };
                        Line::styled(truncate(&format!(" {}", matches[i]), list.width), style)
                    })
                    .collect();
                for (r, i) in (first..matches.len().min(first + KEY_ROWS)).enumerate() {
                    self.hits.form_keylist.push((Rect::new(list.x, list.y + r as u16, list.width, 1), matches[i].clone()));
                }
                f.render_widget(Paragraph::new(rows), list);
            }
        }
    }

    pub(super) fn draw_set_picker(&mut self, f: &mut Frame, body: Rect, pick: &OptionsPicker) {
        let area = self.dragged_popup_area(body, 70, pick.names.len() as u16 + 2);
        self.hits.picker_area = area;
        f.render_widget(ratatui::widgets::Clear, area);
        let block = Block::bordered()
            .title(format!(" connect to {} with ", pick.host))
            .border_style(Style::new().fg(ACCENT));
        let inner = block.inner(area);
        f.render_widget(block, area);
        self.hits.picker_list = inner;
        let prefs = self.cfg.hosts.get(&pick.host);
        let rows: Vec<ListItem> = pick
            .names
            .iter()
            .map(|n| {
                let (label, opts) = match n {
                    None => (DEFAULT, prefs.map(|p| p.default_options())),
                    Some(name) => (name.as_str(), prefs.and_then(|p| p.options.get(name).cloned())),
                };
                let summary = opts.unwrap_or_default().summary();
                ListItem::new(Line::from(vec![
                    Span::raw(format!(" {label:<LABEL_W$}")),
                    Span::styled(summary, Style::new().add_modifier(Modifier::DIM)),
                ]))
            })
            .collect();
        let mut state = ListState::default().with_selected(Some(pick.sel));
        let list = List::new(rows).highlight_style(Style::new().add_modifier(Modifier::REVERSED));
        f.render_stateful_widget(list, inner, &mut state);
    }
}

/// The names that fit in `avail` columns around the current one, as `(first, last)` indexes.
/// Names that are left out are marked with `‹` and `›`, which take two columns each.
fn names_window(widths: &[u16], at: usize, avail: u16) -> (usize, usize) {
    let used = |a: usize, b: usize| {
        let names: u16 = widths[a..=b].iter().sum::<u16>() + (b - a) as u16;
        names + if a > 0 { 2 } else { 0 } + if b + 1 < widths.len() { 2 } else { 0 }
    };
    let (mut a, mut b) = (at, at);
    loop {
        let mut grew = false;
        if b + 1 < widths.len() && used(a, b + 1) <= avail {
            b += 1;
            grew = true;
        }
        if a > 0 && used(a - 1, b) <= avail {
            a -= 1;
            grew = true;
        }
        if !grew {
            return (a, b);
        }
    }
}

/// The key a clickable hint stands for.
fn hint_key(label: &str) -> KeyEvent {
    let plain = |c| KeyEvent::new(c, KeyModifiers::NONE);
    let ctrl = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL);
    match label {
        "Tab" => plain(KeyCode::Tab),
        "←/→" => plain(KeyCode::Right),
        "Enter" => plain(KeyCode::Enter),
        "Esc" => plain(KeyCode::Esc),
        "y" | "n" => plain(KeyCode::Char(label.chars().next().unwrap_or(' '))),
        _ => ctrl(label.trim_start_matches('^').chars().next().unwrap_or(' ').to_ascii_lowercase()),
    }
}

impl App {
    /// A click or the wheel on the connection options form. A click outside closes it.
    pub(super) fn on_mouse_host_form(&mut self, mut form: HostForm, (x, y): (u16, u16), wheel: Option<isize>, click: bool) {
        let n = LABELS.len() as isize;
        if let Some(sel) = form.key_pick {
            // The open key list: a click on a row takes it, the wheel moves along it.
            if click && let Some((_, key)) = self.hits.form_keylist.iter().find(|(r, _)| inside(*r, x, y)) {
                form.take_key(&key.clone());
                self.mode = Mode::HostForm(Box::new(form));
                return;
            }
            if let Some(d) = wheel {
                let m = form.key_matches().len() as isize;
                if m > 0 {
                    form.key_pick = Some((sel as isize + d).rem_euclid(m) as usize);
                }
                self.mode = Mode::HostForm(Box::new(form));
                return;
            }
        }
        if let Some(d) = wheel {
            if form.naming.is_none() {
                form.focus = (form.focus as isize + d).rem_euclid(n) as usize;
            }
        } else if let Some((_, key)) = self.hits.form_keys.iter().find(|(r, _)| inside(*r, x, y)) {
            return self.on_key_host_form(*key, form);
        } else if !inside(self.hits.form_area, x, y) {
            if click && !form.confirming && !form.want_close() {
                // unsaved changes: the form asks before it closes
            } else if click && !form.confirming {
                return; // a click outside closes the form
            }
        } else if click && form.naming.is_none() {
            form.key_pick = None;
            if let Some((_, name)) = self.hits.form_options.iter().find(|(r, _)| inside(*r, x, y)) {
                form.switch_to(name.clone());
            } else if inside(self.hits.form_fields, x, y) {
                form.focus = (y - self.hits.form_fields.y) as usize;
            }
        }
        self.mode = Mode::HostForm(Box::new(form));
    }

    /// A click or the wheel on the options picker: a click connects, one outside closes it.
    pub(super) fn on_mouse_options_picker(&mut self, mut pick: OptionsPicker, (x, y): (u16, u16), wheel: Option<isize>, click: bool) {
        let n = pick.names.len() as isize;
        if let Some(d) = wheel {
            pick.sel = (pick.sel as isize + d).rem_euclid(n) as usize;
        } else if click {
            let list = self.hits.picker_list;
            if !inside(list, x, y) {
                return;
            }
            let i = (y - list.y) as usize;
            if i < pick.names.len() {
                self.connect_with_options(&pick.host, pick.names[i].as_deref());
                return;
            }
        }
        self.mode = Mode::OptionsPicker(Box::new(pick));
    }
}

impl App {
    /// Dragging the title bar of the options popup, the picker or the extras popup moves it. True when the event was part of that.
    pub(super) fn drag_options_popup(&mut self, m: &MouseEvent) -> bool {
        let area = match self.mode {
            Mode::HostForm(_) => self.hits.form_area,
            Mode::OptionsPicker(_) => self.hits.picker_area,
            Mode::HostSnippets(_) => self.hits.extras_area,
            _ => {
                self.popup_drag = None;
                return false;
            }
        };
        let (x, y) = (m.column, m.row);
        match m.kind {
            MouseEventKind::Down(MouseButton::Left) if y == area.y && inside(area, x, y) => {
                self.popup_drag = Some((x, y));
                true
            }
            MouseEventKind::Drag(MouseButton::Left) if let Some((px, py)) = self.popup_drag => {
                self.popup_offset.0 += x as i32 - px as i32;
                self.popup_offset.1 += y as i32 - py as i32;
                self.popup_drag = Some((x, y));
                true
            }
            MouseEventKind::Up(MouseButton::Left) => self.popup_drag.take().is_some(),
            _ => false,
        }
    }
}

#[cfg(test)]
impl App {
    pub(super) fn identity_value(&self) -> Option<String> {
        match &self.mode {
            Mode::HostForm(form) => Some(form.values[IDENTITY].clone()),
            _ => None,
        }
    }

    pub(super) fn jump_value(&self) -> Option<String> {
        match &self.mode {
            Mode::HostForm(form) => Some(form.values[JUMP].clone()),
            _ => None,
        }
    }

    pub(super) fn current_options_name(&self) -> Option<String> {
        match &self.mode {
            Mode::HostForm(form) => form.current().map(String::from),
            _ => None,
        }
    }
}
