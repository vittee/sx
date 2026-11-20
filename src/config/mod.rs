//! Config layout under `$SX_HOME` (default `~/.config/sx`):
//!
//!   snippets/<name>.sh   pieces of your rc, joined in name order
//!   config.toml          profiles (which snippets to send) and each host's profile

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::{env, fs, io};

use serde::{Deserialize, Serialize};

/// Host profile value meaning "plain ssh, send nothing".
pub const PROFILE_OFF: &str = "-";
const DEFAULT_PROFILE: &str = "default";

const SAMPLE_ALIASES: &str = "\
alias ll='ls -alF'
alias la='ls -A'
alias gs='git status'
";

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Profile {
    #[serde(default)]
    pub snippets: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct HostPrefs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
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
        Config { default_profile: default_profile_name(), profiles: BTreeMap::new(), hosts: BTreeMap::new() }
    }
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
    /// An unknown profile name falls back to the default profile.
    pub fn profile_for(&self, host: &str) -> Option<&str> {
        let chosen = self.hosts.get(host).and_then(|h| h.profile.as_deref());
        match chosen {
            Some(PROFILE_OFF) => None,
            Some(name) if self.profiles.contains_key(name) => Some(name),
            _ => Some(self.default_profile.as_str()),
        }
    }
}

/// Writes `path` through a temp file in the same directory and renames it into place, so a crash
/// or kill leaves either the old file or the new one, never a half-written one.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".sx-tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// `$SX_HOME`, else `~/.config/sx`.
pub fn config_dir() -> Option<PathBuf> {
    if let Some(p) = env::var_os("SX_HOME") {
        return Some(p.into());
    }
    let home = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".config").join("sx"))
}

fn snippets_dir(dir: &Path) -> PathBuf {
    dir.join("snippets")
}

/// Snippet names, sorted: `snippets/10-x.sh` is `10-x`. Prefix names with numbers to order them.
pub fn list_snippets(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(snippets_dir(dir))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if path.extension()?.to_str()? == "sh" {
                path.file_stem()?.to_str().map(String::from)
            } else {
                None
            }
        })
        .collect();
    names.sort();
    names
}

/// Builds the rc text sent to the server: the profile's snippets, in name order.
pub fn build_rc(dir: &Path, cfg: &Config, profile: &str) -> String {
    let Some(p) = cfg.profiles.get(profile) else { return String::new() };
    let mut out = String::new();
    for name in list_snippets(dir).into_iter().filter(|n| p.snippets.contains(n)) {
        if let Ok(text) = fs::read_to_string(snippets_dir(dir).join(format!("{name}.sh"))) {
            out.push_str(&format!("# --- {name} ---\n{text}"));
            if !text.ends_with('\n') {
                out.push('\n');
            }
        }
    }
    out
}

/// First run: create the sample snippet and the default profile that sends every snippet.
/// Returns the config and whether anything was created.
pub fn ensure_initialized(dir: &Path) -> io::Result<(Config, bool)> {
    let mut cfg = Config::load(dir)?;
    let mut changed = false;

    if !snippets_dir(dir).exists() {
        fs::create_dir_all(snippets_dir(dir))?;
        fs::write(snippets_dir(dir).join("20-aliases.sh"), SAMPLE_ALIASES)?;
        changed = true;
    }
    if cfg.profiles.is_empty() {
        cfg.profiles.insert(cfg.default_profile.clone(), Profile { snippets: list_snippets(dir) });
        changed = true;
    }
    if changed {
        cfg.save(dir)?;
    }
    Ok((cfg, changed))
}

#[cfg(test)]
mod tests;
