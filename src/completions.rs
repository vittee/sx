//! Shell completion: `sx completions <shell>` prints a script, and the script calls the hidden
//! `sx __complete` for the candidates. Keeping the logic here means the scripts stay thin and the
//! names always come from the current ssh config and `config.toml`.

use crate::config::{self, Config};
use crate::ssh_config;
use std::process::ExitCode;

const FLAGS: [&str; 4] = ["--options", "--dry-run", "--version", "--help"];

const BASH: &str = include_str!("completions/sx.bash");
const ZSH: &str = include_str!("completions/sx.zsh");
const FISH: &str = include_str!("completions/sx.fish");
const POWERSHELL: &str = include_str!("completions/sx.ps1");
const NUSHELL: &str = include_str!("completions/sx.nu");

mod install;

/// A shell that sx has a completion script for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shell {
    Bash,
    Zsh,
    Fish,
    Powershell,
    Nushell,
}

impl Shell {
    fn parse(name: &str) -> Option<Shell> {
        match name {
            "bash" => Some(Shell::Bash),
            "zsh" => Some(Shell::Zsh),
            "fish" => Some(Shell::Fish),
            "powershell" | "pwsh" => Some(Shell::Powershell),
            "nushell" | "nu" => Some(Shell::Nushell),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
            Shell::Powershell => "PowerShell",
            Shell::Nushell => "nushell",
        }
    }

    fn script(self) -> &'static str {
        match self {
            Shell::Bash => BASH,
            Shell::Zsh => ZSH,
            Shell::Fish => FISH,
            Shell::Powershell => POWERSHELL,
            Shell::Nushell => NUSHELL,
        }
    }
}

/// What a wrong `sx completions` command prints, in the notation of `sx --help`.
const USAGE: &str = "\
usage: sx completions <bash|zsh|fish|powershell|nushell>
       sx completions --install [bash|zsh|fish|powershell|nushell] [--yes]";

/// `sx completions <shell>` prints the script, `sx completions --install [shell]` sets it up,
/// where `shell` is one of bash, zsh, fish, powershell and nushell.
pub fn print(rest: &[String]) -> ExitCode {
    if rest.iter().any(|a| a == "--install") {
        return install::run(rest);
    }
    match rest {
        [s] if let Some(shell) = Shell::parse(s) => {
            print!("{}", shell.script());
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// `sx __complete [--new] WORDS...`: the candidates for the last word, one per line.
/// `--new` means the cursor is after a space, so the word to complete is empty. The shells that
/// cannot pass an empty argument use it.
pub fn run(args: &[String]) -> ExitCode {
    let mut words = args.to_vec();
    if words.first().is_some_and(|a| a == "--new") {
        words.remove(0);
        words.push(String::new());
    }
    let hosts = config::home_dir().map(|h| ssh_config::load_hosts(&h)).unwrap_or_default();
    let hosts: Vec<String> = hosts.into_iter().map(|h| h.alias).collect();
    let cfg = config::config_dir().and_then(|d| Config::load(&d).ok()).unwrap_or_default();
    for c in candidates(&words, &hosts, &cfg) {
        println!("{c}");
    }
    ExitCode::SUCCESS
}

/// The first word that is not an option or an option's value: the host being connected to.
fn host_word(words: &[String]) -> Option<&str> {
    let mut i = 0;
    while i < words.len() {
        let w = words[i].as_str();
        if w == "--options" || takes_value(w) {
            i += 2;
        } else if w.starts_with('-') {
            i += 1;
        } else {
            return Some(w);
        }
    }
    None
}

/// A lone ssh option that is followed by its value, like `-p`.
fn takes_value(w: &str) -> bool {
    let mut c = w.chars();
    c.next() == Some('-') && c.next().is_some_and(|o| crate::OPTS_WITH_VALUE.contains(o)) && c.next().is_none()
}

fn starting_with(items: impl IntoIterator<Item = String>, prefix: &str) -> Vec<String> {
    let mut v: Vec<String> = items.into_iter().filter(|s| s.starts_with(prefix)).collect();
    v.sort();
    v.dedup();
    v
}

/// What fits the last of `words` (the word under the cursor, empty when none is typed yet).
pub fn candidates(words: &[String], hosts: &[String], cfg: &Config) -> Vec<String> {
    let Some((cur, before)) = words.split_last() else { return Vec::new() };
    if before.first().is_some_and(|w| w == "init") {
        return if before.len() == 1 { starting_with(["--presets".to_string()], cur) } else { Vec::new() };
    }
    let prev = before.last().map(String::as_str);
    if prev == Some("--options") {
        // The names saved for the host typed so far, else every name there is.
        let names = match host_word(before) {
            Some(host) => cfg.hosts.get(&crate::host_key(cfg, host)).into_iter().flat_map(|p| p.options.keys().cloned()).collect(),
            None => cfg.hosts.values().flat_map(|p| p.options.keys().cloned()).collect::<Vec<_>>(),
        };
        return starting_with(names, cur);
    }
    if prev.is_some_and(takes_value) {
        return Vec::new(); // a file, a port, a jump host: left to the shell
    }
    if cur.starts_with('-') {
        return starting_with(FLAGS.map(String::from), cur);
    }
    if host_word(before).is_some() {
        return Vec::new(); // the host is typed: the rest is a remote command
    }
    // `root@we` completes to `root@web`.
    let user = cur.find('@').map_or("", |i| &cur[..=i]);
    let mut all: Vec<String> = hosts.iter().cloned().chain(cfg.hosts.keys().cloned()).map(|h| format!("{user}{h}")).collect();
    if before.is_empty() {
        all.push("init".to_string());
    }
    starting_with(all, cur)
}

#[cfg(test)]
#[path = "completions/tests.rs"]
mod tests;
