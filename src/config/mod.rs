//! Config layout under `$SX_HOME` (default `~/.config/sx`):
//!
//!   snippets/<name>.sh   pieces of your rc, joined in name order

use std::path::{Path, PathBuf};
use std::{env, fs, io};

const SAMPLE_ALIASES: &str = "\
alias ll='ls -alF'
alias la='ls -A'
alias gs='git status'
";

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

/// Builds the rc text sent to the server: every snippet, in name order.
pub fn build_rc(dir: &Path) -> String {
    let mut out = String::new();
    for name in list_snippets(dir) {
        if let Ok(text) = fs::read_to_string(snippets_dir(dir).join(format!("{name}.sh"))) {
            out.push_str(&format!("# --- {name} ---\n{text}"));
            if !text.ends_with('\n') {
                out.push('\n');
            }
        }
    }
    out
}

/// First run: create the sample snippet. Returns true if anything was created.
pub fn ensure_initialized(dir: &Path) -> io::Result<bool> {
    if snippets_dir(dir).exists() {
        return Ok(false);
    }
    fs::create_dir_all(snippets_dir(dir))?;
    fs::write(snippets_dir(dir).join("20-aliases.sh"), SAMPLE_ALIASES)?;
    Ok(true)
}

#[cfg(test)]
mod tests;
