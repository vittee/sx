//! Config layout under `$SX_HOME` (default `~/.config/sx`):
//!   snippets/<name>.sh        rc pieces, one file each (edited from the TUI)
//!   snippets/<name>.vim       a vimrc, used by `vim`/`nvim` on the server (see [`SnippetKind`])
//!   snippets/<name>.tmux      a tmux config, used by `tmux` on the server
//!   snippets/<name>.inputrc   readline settings, loaded with `bind -f`
//!   snippets/<name>.prompt    your own prompt theme, listed next to the built-in ones
//!   config.toml               profiles (which snippets), per-host preferences
//!   history.tsv               connection history (see `history.rs`)

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs, io};

use serde::{Deserialize, Serialize};

use crate::prompt;

mod presets;
mod snippets;

pub use snippets::*;
use snippets::snippets_dir;

/// Host profile value meaning "plain ssh, send nothing".
pub const PROFILE_OFF: &str = "-";
const DEFAULT_PROFILE: &str = "default";

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Profile {
    #[serde(default)]
    pub snippets: Vec<String>,
    /// Built-in prompt theme (see `prompt::THEMES`). `None` keeps the server's own prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
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
            SortKey::Profile => SortKey::Source,
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
    #[serde(default = "default_profile_name")]
    pub default_profile: String,
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
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
            default_profile: default_profile_name(),
            profiles: BTreeMap::new(),
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

const SAMPLE_ALIASES: &str = "\
alias ll='ls -alF'
alias la='ls -A'
alias gs='git status'
";

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

    /// A host's extra snippets.
    pub fn host_snippets(&self, host: &str) -> &[String] {
        self.hosts.get(host).map_or(&[], |h| h.snippets.as_slice())
    }
}

/// First run: create the sample aliases (on), the vim presets (off) and the default profile with
/// the default prompt theme.
/// Returns true if anything was created.
pub fn ensure_initialized(dir: &Path) -> io::Result<(Config, bool)> {
    let mut cfg = Config::load(dir)?;
    let mut changed = false;

    if !snippets_dir(dir).exists() {
        fs::create_dir_all(snippets_dir(dir))?;
        write_snippet(dir, "20-aliases", SAMPLE_ALIASES)?;
        for (name, text) in presets::vim_presets() {
            write_snippet(dir, name, &text)?;
        }
        changed = true;
    }

    if cfg.profiles.is_empty() {
        cfg.profiles.insert(
            cfg.default_profile.clone(),
            Profile { snippets: vec!["20-aliases".to_string()], prompt: Some(prompt::DEFAULT_THEME.to_string()) },
        );
        changed = true;
    }
    if changed {
        cfg.save(dir)?;
    }
    Ok((cfg, changed))
}

/// Creates the vim presets that are missing from `snippets/`, for a setup made before they
/// existed. Files that are there are left alone, and nothing is turned on in any profile.
/// Returns the names it created.
pub fn add_missing_presets(dir: &Path) -> io::Result<Vec<String>> {
    let have = list_snippets(dir);
    let mut added = Vec::new();
    for (name, text) in presets::vim_presets() {
        if !have.iter().any(|n| n == name) {
            write_snippet(dir, name, &text)?;
            added.push(name.to_string());
        }
    }
    Ok(added)
}

#[cfg(test)]
mod tests;
