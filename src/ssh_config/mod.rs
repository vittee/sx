//! Host discovery: `Host` entries in `~/.ssh/config` (following `Include`) and `~/.ssh/known_hosts`.

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Config,
    KnownHosts,
}

impl Source {
    /// Name shown in the host list.
    pub fn label(&self) -> &'static str {
        match self {
            Source::Config => "config",
            Source::KnownHosts => "known_hosts",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostEntry {
    pub alias: String,
    pub hostname: Option<String>,
    pub user: Option<String>,
    pub port: Option<String>,
    pub source: Source,
}

impl HostEntry {
    fn new(alias: &str, source: Source) -> Self {
        HostEntry { alias: alias.to_string(), hostname: None, user: None, port: None, source }
    }

    /// `user@hostname:port`, only the parts that are known.
    pub fn target(&self) -> String {
        let mut s = String::new();
        if let Some(u) = &self.user {
            s.push_str(u);
            s.push('@');
        }
        s.push_str(self.hostname.as_deref().unwrap_or(&self.alias));
        if let Some(p) = &self.port {
            s.push(':');
            s.push_str(p);
        }
        s
    }
}

fn is_pattern(s: &str) -> bool {
    s.contains(['*', '?', '!']) || s.is_empty()
}

fn split_keyword(line: &str) -> Option<(String, &str)> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let end = line.find(|c: char| c.is_whitespace() || c == '=')?;
    let value = line[end..].trim_start_matches(|c: char| c.is_whitespace() || c == '=').trim();
    Some((line[..end].to_ascii_lowercase(), value))
}

/// Parses one ssh config file's text. `ssh_dir` resolves relative `Include` paths.
fn parse_config(text: &str, ssh_dir: &Path, depth: u8, out: &mut Vec<HostEntry>) {
    // Indices into `out` for the current `Host` block (a block can name several hosts).
    let mut block: Vec<usize> = Vec::new();
    for line in text.lines() {
        let Some((key, value)) = split_keyword(line) else { continue };
        match key.as_str() {
            "host" => {
                block.clear();
                for pat in value.split_whitespace().map(|p| p.trim_matches('"')) {
                    if is_pattern(pat) {
                        continue;
                    }
                    if let Some(i) = out.iter().position(|h| h.alias == pat) {
                        block.push(i);
                    } else {
                        out.push(HostEntry::new(pat, Source::Config));
                        block.push(out.len() - 1);
                    }
                }
            }
            "match" => block.clear(),
            "hostname" | "user" | "port" => {
                for &i in &block {
                    let slot = match key.as_str() {
                        "hostname" => &mut out[i].hostname,
                        "user" => &mut out[i].user,
                        _ => &mut out[i].port,
                    };
                    slot.get_or_insert_with(|| value.to_string()); // ssh: first value wins
                }
            }
            "include" if depth < 4 => {
                for pat in value.split_whitespace() {
                    for path in expand_include(pat, ssh_dir) {
                        if let Ok(inc) = fs::read_to_string(&path) {
                            // Includes inside a Host block are rare; treat them as top level.
                            parse_config(&inc, ssh_dir, depth + 1, out);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn expand_include(pat: &str, ssh_dir: &Path) -> Vec<PathBuf> {
    let pat = pat.trim_matches('"');
    let full = if let Some(rest) = pat.strip_prefix("~/") {
        ssh_dir.parent().map_or_else(|| PathBuf::from(rest), |h| h.join(rest))
    } else if Path::new(pat).is_absolute() {
        PathBuf::from(pat)
    } else {
        ssh_dir.join(pat)
    };
    glob::glob(&full.to_string_lossy()).map(|paths| paths.flatten().collect()).unwrap_or_default()
}

/// Adds plain (unhashed) known_hosts names that are not already listed.
fn parse_known_hosts(text: &str, out: &mut Vec<HostEntry>) {
    for line in text.lines() {
        let first = line.split_whitespace().next().unwrap_or("");
        if first.is_empty() || first.starts_with(['#', '|', '@']) {
            continue;
        }
        for name in first.split(',') {
            let (host, port) = match name.strip_prefix('[').and_then(|r| r.split_once("]:")) {
                Some((h, p)) => (h, Some(p)),
                None => (name, None),
            };
            if is_pattern(host) || out.iter().any(|h| h.alias == host || h.hostname.as_deref() == Some(host)) {
                continue;
            }
            let mut entry = HostEntry::new(host, Source::KnownHosts);
            entry.port = port.map(String::from);
            out.push(entry);
        }
    }
}

/// Hosts from `<home>/.ssh/config` and `<home>/.ssh/known_hosts`. Missing files are fine.
pub fn load_hosts(home: &Path) -> Vec<HostEntry> {
    let ssh_dir = home.join(".ssh");
    let mut out = Vec::new();
    if let Ok(text) = fs::read_to_string(ssh_dir.join("config")) {
        parse_config(&text, &ssh_dir, 0, &mut out);
    }
    if let Ok(text) = fs::read_to_string(ssh_dir.join("known_hosts")) {
        parse_known_hosts(&text, &mut out);
    }
    out
}

#[cfg(test)]
mod tests;
