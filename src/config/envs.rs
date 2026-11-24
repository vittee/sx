//! Environments (`prod`, `staging`, ...): a color, a label and a set of host name patterns, and what
//! the environment changes on the server. A host belongs to an environment when you tag it. The
//! patterns only suggest one.

use serde::{Deserialize, Serialize};

use super::Config;

/// Shows a colored label in the prompt, before its `$` or `#`.
pub const FEATURE_BADGE: &str = "badge";
/// Puts the label in the window and tab title.
pub const FEATURE_TITLE: &str = "title";
/// Asks before `sx` connects to a host of the environment. It is not on in a new environment.
pub const FEATURE_CONFIRM: &str = "confirm";
const DEFAULT_FEATURES: [&str; 2] = [FEATURE_BADGE, FEATURE_TITLE];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Env {
    /// Text of the badge and the title. Empty means the environment's name in capitals.
    #[serde(default)]
    pub label: String,
    /// A color name (black, red, green, yellow, blue, magenta, cyan, white), a number from 0 to 255,
    /// or `#rrggbb`, which is rounded to the nearest of the 256 colors.
    #[serde(default = "default_color")]
    pub color: String,
    /// Host name patterns that suggest this environment: `*` is any text and `?` is one character.
    #[serde(default)]
    pub patterns: Vec<String>,
    /// What the environment does: `badge` and `title` on the server, `confirm` before connecting.
    #[serde(default = "default_features")]
    pub features: Vec<String>,
}

fn default_color() -> String {
    "white".to_string()
}

fn default_features() -> Vec<String> {
    DEFAULT_FEATURES.iter().map(|s| s.to_string()).collect()
}

/// The most characters a label shows. The form stops accepting more.
pub const MAX_LABEL: usize = 16;

impl Env {
    /// A new environment with this color and patterns, the badge and the title on, and no confirmation.
    pub fn new(color: &str, patterns: &[&str]) -> Env {
        Env {
            label: String::new(),
            color: color.to_string(),
            patterns: patterns.iter().map(|s| s.to_string()).collect(),
            features: default_features(),
        }
    }

    pub fn has(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }

    /// The label to show for the environment called `name`: the label, else the name in capitals.
    /// Only plain characters are kept, since it ends up inside a prompt and a title.
    pub fn label_for(&self, name: &str) -> String {
        let text = if self.label.trim().is_empty() { name.to_uppercase() } else { self.label.clone() };
        let clean: String = text
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '/' | ':' | '(' | ')' | '[' | ']'))
            .take(MAX_LABEL)
            .collect();
        clean.trim().to_string()
    }

    /// Whether `host` matches one of the patterns.
    pub fn matches(&self, host: &str) -> bool {
        self.patterns.iter().any(|p| glob_match(p, host))
    }

    /// Bash that defines what `sx` puts in the prompt and the title: `__sx_env_badge` goes at the
    /// front of `PS1` and `__sx_env_label` in the title. Both stay empty when the feature is off.
    pub fn shell_vars(&self, name: &str, truecolor: bool) -> String {
        let label = self.label_for(name);
        let badge = match (self.has(FEATURE_BADGE), badge_sgr(&self.color, truecolor)) {
            (true, Some(sgr)) if !label.is_empty() => format!("\\[\\e[{sgr}m\\] {label} \\[\\e[0m\\] "),
            _ => String::new(),
        };
        let title = if self.has(FEATURE_TITLE) { label } else { String::new() };
        format!("__sx_env_badge='{badge}'\n__sx_env_label='{title}'\n")
    }
}

/// [`Env::shell_vars`] for a host's environment, or empty values for a host that has none.
pub fn env_shell_vars(env: Option<(&str, &Env)>, truecolor: bool) -> String {
    env.map_or_else(|| "__sx_env_badge=''\n__sx_env_label=''\n".to_string(), |(name, e)| e.shell_vars(name, truecolor))
}

/// Whether the terminal `sx` runs in shows 24-bit colors, so a `#rrggbb` color can be exact. The
/// terminal says so in `COLORTERM`, Windows Terminal sets `WT_SESSION`, and inside a local tmux
/// it is not trusted. `SX_TRUECOLOR=1` or `0` decides by hand.
pub fn truecolor_from(var: impl Fn(&str) -> Option<String>) -> bool {
    match var("SX_TRUECOLOR").as_deref() {
        Some("1") => return true,
        Some("0") => return false,
        _ => {}
    }
    if var("TMUX").is_some_and(|v| !v.is_empty()) {
        return false;
    }
    var("COLORTERM").is_some_and(|v| matches!(v.to_ascii_lowercase().as_str(), "truecolor" | "24bit"))
        || var("WT_SESSION").is_some_and(|v| !v.is_empty())
}

/// [`truecolor_from`] for this process, read once.
pub fn terminal_truecolor() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| truecolor_from(|k| std::env::var(k).ok()))
}

/// Starting points for a new environment, offered by the Envs tab. A config has none until you add one.
pub fn env_templates() -> Vec<(&'static str, Env)> {
    vec![
        (
            "prod",
            Env {
                features: vec![FEATURE_BADGE.into(), FEATURE_TITLE.into(), FEATURE_CONFIRM.into()],
                ..Env::new("red", &["*prod*", "*prd*"])
            },
        ),
        ("staging", Env::new("yellow", &["*stag*", "*stg*"])),
        ("uat", Env::new("magenta", &["*uat*"])),
        ("sit", Env::new("cyan", &["sit-*", "*-sit", "*-sit-*", "*.sit.*"])),
        ("dev", Env::new("green", &["dev-*", "*-dev", "*-dev-*", "*.dev.*"])),
    ]
}

/// `*` and `?` patterns, ignoring ASCII case.
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.to_ascii_lowercase().chars().collect();
    let t: Vec<char> = text.to_ascii_lowercase().chars().collect();
    let (mut pi, mut ti) = (0, 0);
    let mut star: Option<(usize, usize)> = None;
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some((pi, ti));
            pi += 1;
        } else if let Some((sp, st)) = star {
            pi = sp + 1;
            ti = st + 1;
            star = Some((sp, st + 1));
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|&c| c == '*')
}

const NAMES: [&str; 8] = ["black", "red", "green", "yellow", "blue", "magenta", "cyan", "white"];

/// The rgb of one of the 256 terminal colors. The first 16 are the common xterm values.
fn indexed_rgb(i: u8) -> (u8, u8, u8) {
    const BASE: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (205, 0, 0),
        (0, 205, 0),
        (205, 205, 0),
        (0, 0, 238),
        (205, 0, 205),
        (0, 205, 205),
        (229, 229, 229),
        (127, 127, 127),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (92, 92, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    const LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
    match i {
        0..=15 => BASE[i as usize],
        16..=231 => {
            let n = (i - 16) as usize;
            (LEVELS[n / 36], LEVELS[n / 6 % 6], LEVELS[n % 6])
        }
        _ => {
            let g = 8 + (i - 232) * 10;
            (g, g, g)
        }
    }
}

/// The rgb of a `#rrggbb` color, `None` for any other color.
pub fn hex_rgb(color: &str) -> Option<(u8, u8, u8)> {
    let c = color.trim().to_ascii_lowercase();
    let hex = c.strip_prefix('#').filter(|h| h.len() == 6 && h.is_ascii())?;
    let channel = |s: &str| u8::from_str_radix(s, 16).ok();
    Some((channel(&hex[0..2])?, channel(&hex[2..4])?, channel(&hex[4..6])?))
}

/// The color's index among the 256 terminal colors, and its rgb for picking readable text.
fn parse_color(color: &str) -> Option<(u8, (u8, u8, u8))> {
    let c = color.trim().to_ascii_lowercase();
    if let Some(i) = NAMES.iter().position(|n| *n == c) {
        return Some((i as u8, indexed_rgb(i as u8)));
    }
    if let Ok(n) = c.parse::<u8>() {
        return Some((n, indexed_rgb(n)));
    }
    let (r, g, b) = hex_rgb(&c)?;
    let level = |v: u8| ((v as u32 * 5 + 127) / 255) as u8;
    Some((16 + 36 * level(r) + 6 * level(g) + level(b), (r, g, b)))
}

/// The background color of a badge (an index of the 256 colors) and whether its text should be
/// black, because the color is light. `None` for a color that is not recognized.
pub fn badge_colors(color: &str) -> Option<(u8, bool)> {
    let (index, (r, g, b)) = parse_color(color)?;
    Some((index, (299 * r as u32 + 587 * g as u32 + 114 * b as u32) / 1000 > 140))
}

/// The SGR parameters of a badge: bold text, black or white by how light the color is, on the color.
/// `None` for a color that is not recognized. A `#rrggbb` color is exact with `truecolor`, else it
/// is the nearest of the 256 colors.
pub fn badge_sgr(color: &str, truecolor: bool) -> Option<String> {
    let (index, light) = badge_colors(color)?;
    let text = if light { 30 } else { 97 };
    if let (true, Some((r, g, b))) = (truecolor, hex_rgb(color)) {
        return Some(format!("1;{text};48;2;{r};{g};{b}"));
    }
    let named = NAMES.iter().any(|n| n.eq_ignore_ascii_case(color.trim()));
    Some(if named { format!("1;{text};4{index}") } else { format!("1;{text};48;5;{index}") })
}

impl Config {
    /// The environment a host is tagged with, when it is still defined.
    pub fn env_of(&self, host: &str) -> Option<(&str, &Env)> {
        let name = self.hosts.get(host)?.env.as_deref()?;
        self.envs.get_key_value(name).map(|(n, e)| (n.as_str(), e))
    }

    /// The label of the environment of `host` when that environment asks before connecting.
    pub fn confirm_label(&self, host: &str) -> Option<String> {
        let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
        self.env_of(host).filter(|(_, e)| e.has(FEATURE_CONFIRM)).map(|(n, e)| e.label_for(n))
    }

    /// The environment whose patterns match the host, for a suggestion. The first one in name order wins.
    pub fn suggest_env(&self, host: &str) -> Option<&str> {
        let host = host.rsplit_once('@').map_or(host, |(_, h)| h);
        self.envs.iter().find(|(_, e)| e.matches(host)).map(|(n, _)| n.as_str())
    }

    /// The tag choices for a host: no environment first, then every environment.
    pub fn env_choices(&self) -> Vec<Option<String>> {
        std::iter::once(None).chain(self.envs.keys().cloned().map(Some)).collect()
    }

    /// Tags a host with an environment, or removes the tag with `None`.
    pub fn set_env(&mut self, host: &str, env: Option<&str>) {
        self.hosts.entry(host.to_string()).or_default().env = env.map(String::from);
    }

    /// Adds an environment. The name follows the same rules as a profile name.
    pub fn add_env(&mut self, name: &str, env: Env) -> Result<(), String> {
        if !super::is_valid_name(name) {
            return Err("use letters, digits, - _ . only".into());
        }
        if self.envs.contains_key(name) {
            return Err(format!("environment {name} exists"));
        }
        self.envs.insert(name.to_string(), env);
        Ok(())
    }

    /// Copies an environment under a new name. Hosts keep their tag on the original.
    pub fn duplicate_env(&mut self, from: &str, to: &str) -> Result<(), String> {
        let source = self.envs.get(from).cloned().ok_or_else(|| format!("no environment {from}"))?;
        self.add_env(to, source)
    }

    /// Renames an environment, and the tag of every host that has it.
    pub fn rename_env(&mut self, from: &str, to: &str) -> Result<(), String> {
        if from == to {
            return Ok(());
        }
        let env = self.envs.get(from).cloned().ok_or_else(|| format!("no environment {from}"))?;
        self.add_env(to, env)?;
        self.envs.remove(from);
        for h in self.hosts.values_mut().filter(|h| h.env.as_deref() == Some(from)) {
            h.env = Some(to.to_string());
        }
        Ok(())
    }

    /// Deletes an environment and the tag of every host that has it.
    pub fn delete_env(&mut self, name: &str) {
        self.envs.remove(name);
        for h in self.hosts.values_mut().filter(|h| h.env.as_deref() == Some(name)) {
            h.env = None;
        }
    }

    /// Moves a host's tag to the next or previous choice.
    pub fn cycle_env(&mut self, host: &str, forward: bool) {
        let choices = self.env_choices();
        let current = self.hosts.get(host).and_then(|h| h.env.clone()).filter(|e| self.envs.contains_key(e));
        let pos = choices.iter().position(|c| *c == current).unwrap_or(0);
        let next = if forward { (pos + 1) % choices.len() } else { (pos + choices.len() - 1) % choices.len() };
        self.set_env(host, choices[next].as_deref());
    }
}

#[cfg(test)]
mod tests;
