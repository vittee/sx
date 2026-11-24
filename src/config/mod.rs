//! Config layout under `$SX_HOME` (default `~/.config/sx`):
//!   snippets/<name>.sh        rc pieces, one file each (edited from the TUI)
//!   snippets/<name>.vim       a vimrc, used by `vim`/`nvim` on the server (see [`SnippetKind`])
//!   snippets/<name>.tmux      a tmux config, used by `tmux` on the server
//!   snippets/<name>.inputrc   readline settings, applied with `bind` when the shell starts
//!   snippets/<name>.prompt    your own prompt theme, listed next to the built-in ones
//!   config.toml               profiles (which snippets), per-host preferences
//!   history.tsv               connection history (see `history.rs`)

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs, io};

use serde::{Deserialize, Serialize};

use crate::prompt;

mod envs;
mod presets;
mod snippets;

pub use envs::*;
pub use snippets::*;
use snippets::snippets_dir;

/// Host profile value meaning "plain ssh, send nothing".
pub const PROFILE_OFF: &str = "-";
const DEFAULT_PROFILE: &str = "default";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Profile {
    #[serde(default)]
    pub snippets: Vec<String>,
    /// Built-in prompt theme (see `prompt::THEMES`). `None` keeps the server's own prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Sets the terminal's window title on the server. On unless the profile turns it off.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub title: bool,
    /// Marks each prompt, command and output for the terminal (OSC 133). On unless turned off.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub marks: bool,
}

impl Default for Profile {
    fn default() -> Self {
        Profile { snippets: Vec::new(), prompt: None, title: true, marks: true }
    }
}

fn yes() -> bool {
    true
}

fn is_true(b: &bool) -> bool {
    *b
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct HostPrefs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub favorite: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_used: Option<u64>,
    /// Snippets sent to this host on top of its profile's.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub snippets: Vec<String>,
    /// Snippets of its profile that this host does not get.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub without: Vec<String>,
    /// The environment this host is tagged with (a key of `Config::envs`), such as `prod`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<String>,
    /// The default connection options, which replace the ones from `~/.ssh/config`: ssh `-l`,
    /// `-p`, `-i`, `-J`, and free-form extra arguments. Empty means no override.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub user: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub port: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub identity: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub jump: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub args: String,
    /// Named options of connection options, picked when connecting (for example `root`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub options: BTreeMap<String, ConnOptions>,
}

/// One group of connection options.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ConnOptions {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub user: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub port: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub identity: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub jump: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub args: String,
}

impl ConnOptions {
    /// True when any option is set.
    pub fn is_set(&self) -> bool {
        [&self.user, &self.port, &self.identity, &self.jump, &self.args].iter().any(|s| !s.trim().is_empty())
    }

    /// The ssh options that carry these settings. They go before the host, where they win over
    /// `~/.ssh/config` (ssh keeps the first value it sees for an option).
    pub fn ssh_args(&self) -> Vec<String> {
        let mut out = Vec::new();
        for (flag, value) in [("-l", &self.user), ("-p", &self.port), ("-i", &self.identity), ("-J", &self.jump)] {
            let value = value.trim();
            if !value.is_empty() {
                out.push(flag.to_string());
                out.push(value.to_string());
            }
        }
        out.extend(split_args(&self.args));
        out
    }

    /// A short form for lists: the ssh arguments, or `no options`.
    pub fn summary(&self) -> String {
        if self.is_set() { self.ssh_args().join(" ") } else { "no options".to_string() }
    }
}

impl HostPrefs {
    /// The default options: the options a plain connect uses.
    pub fn default_options(&self) -> ConnOptions {
        ConnOptions {
            user: self.user.clone(),
            port: self.port.clone(),
            identity: self.identity.clone(),
            jump: self.jump.clone(),
            args: self.args.clone(),
        }
    }

    pub fn set_default_options(&mut self, o: ConnOptions) {
        (self.user, self.port, self.identity, self.jump, self.args) = (o.user, o.port, o.identity, o.jump, o.args);
    }

    /// True when the default options or any named set has an option.
    pub fn has_overrides(&self) -> bool {
        self.default_options().is_set() || self.options.values().any(ConnOptions::is_set)
    }

    /// The ssh options of the default options.
    pub fn ssh_args(&self) -> Vec<String> {
        self.default_options().ssh_args()
    }
}

/// Splits like a shell would for simple cases: on white space, with single or double quotes
/// grouping words. Backslashes are kept as they are.
pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut started = false;
    for c in s.chars() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => cur.push(c),
            (None, '\'' | '"') => {
                quote = Some(c);
                started = true;
            }
            (None, c) if c.is_whitespace() => {
                if started {
                    out.push(std::mem::take(&mut cur));
                    started = false;
                }
            }
            (None, c) => {
                cur.push(c);
                started = true;
            }
        }
    }
    if started {
        out.push(cur);
    }
    out
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_default_sort(k: &SortKey) -> bool {
    *k == SortKey::Smart
}

/// How the host list is ordered. `Smart` = favorites first, then most recently used, then name.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SortKey {
    #[default]
    Smart,
    Host,
    Target,
    Profile,
    Env,
    Source,
    LastUsed,
}

impl SortKey {
    /// Next key in the Ctrl-S cycle.
    pub fn next(self) -> SortKey {
        match self {
            SortKey::Smart => SortKey::Host,
            SortKey::Host => SortKey::Target,
            SortKey::Target => SortKey::Profile,
            SortKey::Profile => SortKey::Env,
            SortKey::Env => SortKey::Source,
            SortKey::Source => SortKey::LastUsed,
            SortKey::LastUsed => SortKey::Smart,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    #[serde(default, skip_serializing_if = "is_default_sort")]
    pub sort: SortKey,
    #[serde(default, skip_serializing_if = "is_false")]
    pub sort_reverse: bool,
    /// Reverses the mouse wheel in the TUI, for a trackpad whose terminal reports the opposite way.
    #[serde(default, skip_serializing_if = "is_false")]
    pub invert_scroll: bool,
    #[serde(default = "default_profile_name")]
    pub default_profile: String,
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
    /// Environments such as `prod`, see [`Env`]. You create them in the Envs tab or here.
    #[serde(default)]
    pub envs: BTreeMap<String, Env>,
    #[serde(default)]
    pub hosts: BTreeMap<String, HostPrefs>,
}

fn default_profile_name() -> String {
    DEFAULT_PROFILE.to_string()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            sort: SortKey::default(),
            sort_reverse: false,
            invert_scroll: false,
            default_profile: default_profile_name(),
            profiles: BTreeMap::new(),
            envs: BTreeMap::new(),
            hosts: BTreeMap::new(),
        }
    }
}

/// `$SX_HOME`, else `~/.config/sx`.
pub fn config_dir() -> Option<PathBuf> {
    if let Some(p) = env::var_os("SX_HOME") {
        return Some(p.into());
    }
    let home = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".config").join("sx"))
}

pub fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")).map(PathBuf::from)
}

pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// Writes `path` through a temp file in the same directory and renames it into place, so a crash
/// or kill leaves either the old file or the new one, never a half-written one.
pub(super) fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".sx-tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && !name.starts_with('.')
}

impl Config {
    pub fn load(dir: &Path) -> io::Result<Config> {
        match fs::read_to_string(dir.join("config.toml")) {
            Ok(text) => toml::from_str(&text)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("config.toml: {e}"))),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
            Err(e) => Err(e),
        }
    }

    pub fn save(&self, dir: &Path) -> io::Result<()> {
        fs::create_dir_all(dir)?;
        let text = toml::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        write_atomic(&dir.join("config.toml"), text.as_bytes())
    }

    /// Effective profile for a host: `Some(name)`, or `None` when sx is switched off for it.
    pub fn profile_for(&self, host: &str) -> Option<&str> {
        let chosen = self.hosts.get(host).and_then(|h| h.profile.as_deref());
        match chosen {
            Some(PROFILE_OFF) => None,
            Some(name) if self.profiles.contains_key(name) => Some(name),
            _ => Some(self.default_profile.as_str()),
        }
    }

    /// All choosable profile values for a host: off first, then every profile.
    pub fn profile_choices(&self) -> Vec<String> {
        std::iter::once(PROFILE_OFF.to_string()).chain(self.profiles.keys().cloned()).collect()
    }

    /// Moves a host's profile to the next/previous choice.
    pub fn cycle_profile(&mut self, host: &str, forward: bool) {
        let choices = self.profile_choices();
        let current = match self.profile_for(host) {
            Some(name) => name.to_string(),
            None => PROFILE_OFF.to_string(),
        };
        let pos = choices.iter().position(|c| *c == current).unwrap_or(0);
        let next = if forward { (pos + 1) % choices.len() } else { (pos + choices.len() - 1) % choices.len() };
        self.hosts.entry(host.to_string()).or_default().profile = Some(choices[next].clone());
    }

    pub fn toggle_favorite(&mut self, host: &str) {
        let h = self.hosts.entry(host.to_string()).or_default();
        h.favorite = !h.favorite;
    }

    pub fn mark_used(&mut self, host: &str) {
        self.hosts.entry(host.to_string()).or_default().last_used = Some(now_secs());
    }

    /// Sets a profile's prompt theme (`None` = keep the server's prompt).
    pub fn set_prompt(&mut self, profile: &str, theme: Option<&str>) {
        if let Some(p) = self.profiles.get_mut(profile) {
            p.prompt = theme.map(String::from);
        }
    }

    pub fn toggle_snippet(&mut self, profile: &str, snippet: &str) {
        if let Some(p) = self.profiles.get_mut(profile) {
            if let Some(i) = p.snippets.iter().position(|s| s == snippet) {
                p.snippets.remove(i);
            } else {
                p.snippets.push(snippet.to_string());
            }
        }
    }

    pub fn add_profile(&mut self, name: &str) -> Result<(), String> {
        if !is_valid_name(name) || name == PROFILE_OFF {
            return Err("use letters, digits, - _ . only".into());
        }
        if self.profiles.contains_key(name) {
            return Err(format!("profile {name} exists"));
        }
        self.profiles.insert(name.to_string(), Profile::default());
        Ok(())
    }

    /// Copies a profile's snippets and prompt under a new name. Hosts keep pointing at the original.
    pub fn duplicate_profile(&mut self, from: &str, to: &str) -> Result<(), String> {
        let source = self.profiles.get(from).cloned().ok_or_else(|| format!("no profile {from}"))?;
        self.add_profile(to)?;
        self.profiles.insert(to.to_string(), source);
        Ok(())
    }

    pub fn delete_profile(&mut self, name: &str) -> Result<(), String> {
        if self.profiles.len() <= 1 {
            return Err("cannot delete the last profile".into());
        }
        self.profiles.remove(name);
        if self.default_profile == name {
            self.default_profile = self.profiles.keys().next().cloned().unwrap_or_default();
        }
        Ok(())
    }

    /// Drops a snippet name that no longer exists from every profile and host.
    pub fn forget_snippet(&mut self, snippet: &str) {
        for p in self.profiles.values_mut() {
            p.snippets.retain(|s| s != snippet);
            if p.prompt.as_deref() == Some(snippet) {
                p.prompt = None; // a deleted `.prompt` theme
            }
        }
        for h in self.hosts.values_mut() {
            h.snippets.retain(|s| s != snippet);
            h.without.retain(|s| s != snippet);
        }
    }

    /// Adds or removes a snippet from a host's own extras.
    pub fn toggle_host_snippet(&mut self, host: &str, snippet: &str) {
        let h = self.hosts.entry(host.to_string()).or_default();
        if let Some(i) = h.snippets.iter().position(|s| s == snippet) {
            h.snippets.remove(i);
        } else {
            h.snippets.push(snippet.to_string());
        }
    }

    /// Turns a snippet of the host's profile off for this host, or back on.
    pub fn toggle_host_without(&mut self, host: &str, snippet: &str) {
        let h = self.hosts.entry(host.to_string()).or_default();
        if let Some(i) = h.without.iter().position(|s| s == snippet) {
            h.without.remove(i);
        } else {
            h.without.push(snippet.to_string());
        }
    }

    /// The snippets of a host's profile that the host does not get.
    pub fn host_without(&self, host: &str) -> &[String] {
        self.hosts.get(host).map_or(&[], |h| h.without.as_slice())
    }

    /// A host's extra snippets.
    pub fn host_snippets(&self, host: &str) -> &[String] {
        self.hosts.get(host).map_or(&[], |h| h.snippets.as_slice())
    }
}

/// First run: create the presets (the aliases are on in the default profile, the rest off) and the default profile with
/// the default prompt theme.
/// Returns true if anything was created.
pub fn ensure_initialized(dir: &Path) -> io::Result<(Config, bool)> {
    let mut cfg = Config::load(dir)?;
    let mut changed = false;

    if !snippets_dir(dir).exists() {
        fs::create_dir_all(snippets_dir(dir))?;
        for (name, text) in presets::all() {
            write_snippet(dir, name, &text)?;
        }
        changed = true;
    }

    if cfg.profiles.is_empty() {
        cfg.profiles.insert(
            cfg.default_profile.clone(),
            Profile { snippets: vec!["20-aliases".to_string()], prompt: Some(prompt::DEFAULT_THEME.to_string()), ..Profile::default() },
        );
        changed = true;
    }
    if changed {
        cfg.save(dir)?;
    }
    Ok((cfg, changed))
}

/// Creates the presets (vim and shell) that are missing from `snippets/`, for a setup made before they
/// existed. Files that are there are left alone, and nothing is turned on in any profile.
/// Returns the names it created.
pub fn add_missing_presets(dir: &Path) -> io::Result<Vec<String>> {
    let have = list_snippets(dir);
    let mut added = Vec::new();
    for (name, text) in presets::all() {
        if !have.iter().any(|n| n == name) {
            write_snippet(dir, name, &text)?;
            added.push(name.to_string());
        }
    }
    Ok(added)
}

/// Replaces the presets in `snippets/` that are older than the built-in ones, and returns each one
/// with the version it had and the version it has now. A file without a `version` is not touched,
/// since it may be your own, and neither is one that is as new or newer. Your edits to a replaced
/// preset are lost. Nothing is turned on or off in any profile.
pub fn update_presets(dir: &Path) -> io::Result<Vec<(String, Version, Version)>> {
    let mut updated = Vec::new();
    for (name, text) in presets::all() {
        let Ok(old) = read_snippet(dir, name) else { continue };
        let Some(have) = parse_meta(name, &old).version else { continue };
        let new = presets::version_of(name);
        if have < new {
            write_snippet(dir, name, &text)?;
            updated.push((name.to_string(), have, new));
        }
    }
    Ok(updated)
}

#[cfg(test)]
mod tests;
