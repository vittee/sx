//! The two-line footer: status or input prompt, and the key hints.

use super::*;

impl App {
    pub(super) fn draw_footer(&self, f: &mut Frame, area: Rect) {
        let [top, bottom] = Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
        let blink = self.flash.is_some_and(|t| t.elapsed() < Duration::from_millis(200));
        let ask = |text: String| {
            let style = Style::new().fg(Color::Yellow);
            Line::styled(text, if blink { style.add_modifier(Modifier::REVERSED) } else { style })
        };
        let first = match &self.mode {
            Mode::Input(kind, buf) => {
                let label = match kind {
                    InputKind::NewSnippet => "new snippet name (.vim .tmux .inputrc .prompt = other kinds)".to_string(),
                    InputKind::NewProfile => "new profile name".to_string(),
                    InputKind::DuplicateSnippet(from) => format!("copy of snippet {from} as"),
                    InputKind::DuplicateProfile(from) => format!("copy of profile {from} as"),
                    InputKind::NewEnv => format!(
                        "new environment name ({} start from a template)",
                        config::env_templates().iter().map(|(n, _)| *n).collect::<Vec<_>>().join(" ")
                    ),
                    InputKind::DuplicateEnv(from) => format!("copy of environment {from} as"),
                    InputKind::RenameEnv(from) => format!("rename environment {from} to"),
                };
                Line::from(vec![
                    Span::styled(format!(" {label}: "), Style::new().fg(Color::Cyan)),
                    Span::raw(buf.clone()),
                    Span::styled("▏", Style::new().fg(Color::Cyan)),
                ])
            }
            Mode::Confirm(Pending::Connect(args)) => {
                let host = crate::host_arg(args);
                let label = self.cfg.confirm_label(&host).unwrap_or_default();
                ask(format!(" Connect to {host} ({label})? (y/N)"))
            }
            Mode::Confirm(Pending::Unsaved(_)) => {
                let name = self.editor_name.as_deref().map(config::snippet_file).unwrap_or_default();
                ask(format!(" {name} has unsaved changes. Save? (y/n)"))
            }
            Mode::Confirm(p) => {
                let what = match p {
                    Pending::Snippet(n) => format!("snippet {n}"),
                    Pending::Profile(n) => format!("profile {n}"),
                    Pending::Env(n) => format!("environment {n}"),
                    Pending::Connect(_) | Pending::Unsaved(_) => String::new(),
                };
                ask(format!(" Delete {what}? (y/N)"))
            }
            Mode::HostSnippets(host) => {
                Line::styled(format!(" Extra snippets for {host}, on top of its profile"), Style::new().fg(Color::Cyan))
            }
            Mode::HostForm(_) | Mode::OptionsPicker(_) => match &self.status {
                Some((m, true)) => Line::styled(format!(" {m}"), Style::new().fg(Color::Red)),
                _ => Line::raw(""),
            },
            _ => match &self.status {
                Some((m, true)) => Line::styled(format!(" {m}"), Style::new().fg(Color::Red)),
                Some((m, false)) => Line::styled(format!(" {m}"), Style::new().fg(Color::Green)),
                None => Line::raw(""),
            },
        };
        f.render_widget(Paragraph::new(first), top);

        // (key, what it does): keys are highlighted, descriptions dimmed.
        let hints: &[(&str, &str)] = match (&self.mode, self.tab) {
            (Mode::Input(..), _) => &[("Enter", "create"), ("Esc", "cancel")],
            (Mode::HostSnippets(_), _) => &[("Up/Down", "choose"), ("Space", "toggle"), ("Esc", "close")],
            (Mode::HostForm(form), _) if form.is_naming() => &[("Enter", "add"), ("Esc", "cancel")],
            (Mode::HostForm(_), _) => &[
                ("Tab", "next field"),
                ("^←/→", "options"),
                ("^N", "new options"),
                ("^D", "delete options"),
                ("^S", "save"),
                ("Enter", "connect once"),
                ("Esc", "cancel"),
            ],
            (Mode::OptionsPicker(_), _) => &[("Up/Down", "choose"), ("Enter", "connect"), ("Esc", "close")],
            (_, Tab::Hosts) => &[
                ("Enter", "connect"),
                ("^P", "profile"),
                ("^T", "env"),
                ("^E", "extras"),
                ("^O", "opts"),
                ("^L", "pick"),
                ("^F", "fav"),
                ("^S/R", "sort"),
                ("Tab", "next"),
                ("Esc", "quit"),
            ],
            (_, Tab::Envs) if self.env_focus == envs::EnvFocus::Fields => {
                &[("Up/Down", "field"), ("type", "edit"), ("Space", "toggle"), ("Esc", "done")]
            }
            (_, Tab::Envs) => &[
                ("Up/Down", "choose"),
                ("Enter", "edit"),
                ("^N", "new"),
                ("^Y", "copy"),
                ("^R", "rename"),
                ("^D", "delete"),
                ("Tab", "next"),
                ("Esc", "quit"),
            ],
            (_, Tab::Snippets) if self.editor_focus => &[("Ctrl-S", "save"), ("Esc", "done")],
            (_, Tab::Snippets) => &[
                ("Up/Down", "choose"),
                ("Enter", "edit"),
                ("^N", "new"),
                ("^Y", "copy"),
                ("^D", "delete"),
                ("Tab", "next"),
                ("Esc", "quit"),
            ],
            (_, Tab::Profiles) if self.prof_focus == ProfFocus::Prompt => {
                &[("Up/Down", "preview"), ("Enter", "use"), ("Space", "in sx-theme"), ("^E", "edit theme"), ("→", "snippets"), ("←/Esc", "back")]
            }
            (_, Tab::Profiles) if self.prof_focus == ProfFocus::Snippets => {
                &[("Arrows", "choose"), ("Space", "toggle"), ("←/Esc", "back")]
            }
            (_, Tab::Profiles) => &[
                ("Up/Down", "choose"),
                ("→", "prompt"),
                ("^N", "new"),
                ("^Y", "copy"),
                ("^D", "delete"),
                ("^G", "default"),
                ("Tab", "next"),
                ("Esc", "quit"),
            ],
        };
        let key_style = Style::new().add_modifier(Modifier::BOLD);
        let desc_style = Style::new().add_modifier(Modifier::DIM);
        let toggle = ("F2", if self.mouse { "select text" } else { "mouse" });
        let mut hints = hints.to_vec();
        if !matches!(self.mode, Mode::Input(..) | Mode::HostSnippets(_) | Mode::HostForm(_) | Mode::OptionsPicker(_)) {
            hints.push(toggle);
        }
        // The pick key does nothing for a host without named options, so it is not lit up.
        let pick_off = matches!(self.mode, Mode::Normal) && self.tab == Tab::Hosts && !self.can_pick_options();
        let mut spans = vec![Span::raw(" ")];
        for (key, desc) in hints {
            let key_style = if pick_off && key == "^L" { desc_style } else { key_style };
            spans.push(Span::styled(key, key_style));
            spans.push(Span::styled(format!(" {desc} "), desc_style));
        }
        f.render_widget(Paragraph::new(Line::from(spans)), bottom);
    }
}
