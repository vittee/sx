mod bootstrap;
mod completions;
mod config;
mod editor;
mod help;
mod highlight;
mod history;
mod log;
mod payload;
mod prompt;
mod ssh_config;
mod tui;

use std::env;
use std::process::{Command, ExitCode};
use std::sync::atomic::{AtomicBool, Ordering};

/// The help with the config directory sx uses right now, and why it picked that one.
fn help_text() -> String {
    let dir = match config::config_dir() {
        Some(d) => {
            let why = if env::var_os("SX_HOME").is_some() { "$SX_HOME" } else { "default, set $SX_HOME to change it" };
            format!("{} ({why})", d.display())
        }
        None => "not found: set $SX_HOME (HOME and USERPROFILE are unset)".to_string(),
    };
    HELP.replace("{config}", &dir)
}

const HELP: &str = "\
sx: run ssh with your own shell rc (prompt, aliases) on the server.

Usage:
  sx                                                          open the TUI (hosts, snippets, profiles)
  sx [ssh args] <host>                                        like ssh, but the remote bash loads your rc
  sx <host> --options <name>                                  use the host's saved options <name> (anywhere in the arguments)
  sx init [--presets]                                         create the sample snippets and default profile,
                                                              --presets adds the vim presets that are missing, switched off
  sx --dry-run [ssh args] <host>                              print the ssh command instead of running it
  sx completions <bash|zsh|fish|powershell|nushell>           print the completion script
  sx completions --install [bash|zsh|fish|powershell|nushell] [--yes]
                                                              set the completion up for your shell (asks first)
  sx --version                                                print the sx version (ssh's own -V shows the ssh version)
  sx --help

Config: {config}
";

/// ssh options that take a value.
const OPTS_WITH_VALUE: &str = "BbcDEeFIiJLlmOopQRSWw";
/// ssh options after which an interactive remote shell is not wanted.
const OPTS_NO_SHELL: &str = "NWOQVGsTfn";

fn version_line() -> String {
    format!("sx {}", env!("CARGO_PKG_VERSION"))
}

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let mut dry_run = false;

    match args.first().map(String::as_str) {
        None => return tui::run(),
        Some("init") => return init(&args[1..]),
        Some("completions") => return completions::print(&args[1..]),
        Some("__complete") => return completions::run(&args[1..]),
        Some("--help") => {
            println!("{}", help::paint(&help_text(), log::stdout_color()));
            return ExitCode::SUCCESS;
        }
        Some("--version") => {
            println!("{}", version_line());
            return ExitCode::SUCCESS;
        }
        Some("--dry-run") => {
            dry_run = true;
            args.remove(0);
        }
        _ => {}
    }

    let options = match take_options(&mut args) {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("sx: {msg}");
            return ExitCode::from(2);
        }
    };
    let host = host_arg(&args);
    match saved_options(&host, options.as_deref()) {
        Ok(Some(o)) => args = with_options(args, &o),
        Ok(None) => {}
        Err(msg) => {
            eprintln!("sx: {msg}");
            return ExitCode::from(2);
        }
    }
    let ssh_args = prepare_args(args);
    if dry_run {
        println!("ssh {}", ssh_args.iter().map(|a| format!("{a:?}")).collect::<Vec<_>>().join(" "));
        return ExitCode::SUCCESS;
    }
    if let Some(dir) = config::config_dir() {
        mark_used(&dir, &host);
    }
    // `as u8` would wrap a Windows status such as 0x100 to 0 (success).
    ExitCode::from(u8::try_from(run_and_record(&ssh_args, &host).code()).unwrap_or(1))
}

/// Removes `--options NAME` (or `--options=NAME`) from anywhere in `args`, up to a `--`, and returns
/// the name. The last one wins. ssh has no such option, so it cannot be mistaken for one of its own.
fn take_options(args: &mut Vec<String>) -> Result<Option<String>, String> {
    let mut name = None;
    let mut i = 0;
    while i < args.len() && args[i] != "--" {
        if args[i] == "--options" {
            if i + 1 >= args.len() || args[i + 1] == "--" {
                return Err("--options needs a name".to_string());
            }
            name = Some(args.remove(i + 1));
            args.remove(i);
        } else if let Some(n) = args[i].strip_prefix("--options=") {
            name = Some(n.to_string());
            args.remove(i);
        } else {
            i += 1;
        }
    }
    Ok(name)
}

/// Records that `host` was connected to, so "last used" in the TUI counts connections made from the
/// command line too. A missing or unreadable config is left alone.
fn mark_used(dir: &std::path::Path, host: &str) {
    // Writing a config here would make `sx init` think the sample snippets exist already.
    if host.is_empty() || !dir.join("config.toml").is_file() {
        return;
    }
    let Ok(mut cfg) = config::Config::load(dir) else { return };
    let key = host_key(&cfg, host);
    cfg.mark_used(&key);
    if let Err(e) = cfg.save(dir) {
        log::warn(format!("cannot save the last used time: {e}"));
    }
}

/// The saved options for `host`: the named ones for `name`, else the default ones.
/// `None` when the host has nothing saved and no name was asked for.
fn saved_options(host: &str, name: Option<&str>) -> Result<Option<config::ConnOptions>, String> {
    if host.is_empty() {
        return Ok(None);
    }
    let cfg = config::config_dir().and_then(|d| config::Config::load(&d).ok()).unwrap_or_default();
    pick_options(&cfg, host, name)
}

fn pick_options(cfg: &config::Config, host: &str, name: Option<&str>) -> Result<Option<config::ConnOptions>, String> {
    let key = host_key(cfg, host);
    let prefs = cfg.hosts.get(&key);
    match name {
        None => Ok(prefs.map(config::HostPrefs::default_options).filter(config::ConnOptions::is_set)),
        Some(name) => match prefs.and_then(|p| p.options.get(name)) {
            Some(o) => Ok(Some(o.clone())),
            None => {
                let have: Vec<&str> = prefs.into_iter().flat_map(|p| p.options.keys().map(String::as_str)).collect();
                let have = if have.is_empty() { "none saved".to_string() } else { have.join(", ") };
                Err(format!("{key} has no options named {name:?} ({have})"))
            }
        },
    }
}

/// Adds the options after the arguments the user typed, so ssh, which keeps the first value of an
/// option, prefers what was typed. They go before a `--`, after which ssh reads a command.
fn with_options(mut args: Vec<String>, o: &config::ConnOptions) -> Vec<String> {
    let at = args.iter().position(|a| a == "--").unwrap_or(args.len());
    args.splice(at..at, o.ssh_args());
    args
}

/// How an `ssh` run ended.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SshOutcome {
    Exited(i32),
    /// Killed by Ctrl-C (SIGINT, or Windows STATUS_CONTROL_C_EXIT).
    Interrupted,
}

impl SshOutcome {
    /// Process exit code: 130 for an interrupt, as shells report it.
    pub fn code(self) -> i32 {
        match self {
            SshOutcome::Exited(c) => c,
            SshOutcome::Interrupted => 130,
        }
    }
}

/// Windows exit status of a process ended by Ctrl-C.
const STATUS_CONTROL_C_EXIT: i32 = 0xC000_013Au32 as i32;

fn outcome_of(status: std::process::ExitStatus) -> SshOutcome {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if status.signal() == Some(2) {
            return SshOutcome::Interrupted;
        }
    }
    match status.code() {
        Some(STATUS_CONTROL_C_EXIT) => SshOutcome::Interrupted,
        Some(c) => SshOutcome::Exited(c),
        None => SshOutcome::Exited(255), // killed by some other signal
    }
}

/// Runs `ssh` and adds the session (host, duration, how it ended) to the connection history.
/// `host` is what the user typed; the history is keyed by [`host_key`] so it lines up with the
/// host list.
pub fn run_and_record(args: &[String], host: &str) -> SshOutcome {
    let started = std::time::Instant::now();
    let when = config::now_secs();
    let outcome = run_ssh(args);
    if let Some(dir) = config::config_dir() {
        let key = config::Config::load(&dir).map_or_else(|_| host.to_string(), |c| host_key(&c, host));
        let entry = history::Entry {
            when,
            host: key,
            secs: started.elapsed().as_secs(),
            outcome: history::outcome_text(outcome == SshOutcome::Interrupted, outcome.code()),
        };
        let _ = history::append(&dir, &entry);
    }
    outcome
}

/// A Ctrl-C that `sx` saw itself is an abort, whatever exit status ssh reports for it.
fn finish(ctrl_c: bool, status: std::process::ExitStatus) -> SshOutcome {
    if ctrl_c { SshOutcome::Interrupted } else { outcome_of(status) }
}

/// Set by the Ctrl-C handler of the TUI. Ctrl-C reaches `sx` itself only while ssh has not yet put
/// the terminal into raw mode, that is, while it is still connecting. That is an abort, whatever
/// exit status this platform's ssh reports for it.
pub static CTRL_C: AtomicBool = AtomicBool::new(false);

/// Runs `ssh` with inherited stdio.
pub fn run_ssh(args: &[String]) -> SshOutcome {
    CTRL_C.store(false, Ordering::Relaxed);
    match Command::new("ssh").args(args).status() {
        Ok(status) => finish(CTRL_C.swap(false, Ordering::Relaxed), status),
        Err(e) => {
            log::error(format!("cannot run ssh: {e}"));
            SshOutcome::Exited(127)
        }
    }
}

fn init(rest: &[String]) -> ExitCode {
    let presets = match rest {
        [] => false,
        [flag] if flag == "--presets" => true,
        _ => {
            log::error("usage: sx init [--presets]");
            return ExitCode::FAILURE;
        }
    };
    let Some(dir) = config::config_dir() else {
        log::error("cannot find home directory");
        return ExitCode::FAILURE;
    };
    match config::ensure_initialized(&dir) {
        Ok((_, true)) => {
            log::ok_out(format!("created snippets and default profile in {}", dir.display()));
            log::info("tip: sx completions --install sets up shell completion");
        }
        Ok((_, false)) => log::ok_out(format!("{} already set up, left unchanged", dir.display())),
        Err(e) => {
            log::error(e.to_string());
            return ExitCode::FAILURE;
        }
    }
    if presets {
        match config::add_missing_presets(&dir) {
            Ok(added) if added.is_empty() => log::ok_out("no vim preset is missing"),
            Ok(added) => log::ok_out(format!("added {} (off until you turn them on in Profiles)", added.join(", "))),
            Err(e) => {
                log::error(e.to_string());
                return ExitCode::FAILURE;
            }
        }
    }
    ExitCode::SUCCESS
}

/// Adds the sx options to `args` when this is a plain interactive login to a host whose profile is on.
/// Problems (missing config, empty rc) fall back to plain ssh with a message on stderr.
pub fn prepare_args(args: Vec<String>) -> Vec<String> {
    let Some(host) = injectable_host(&args) else { return args };
    match load_payload(&host) {
        Ok(Some(payload)) => build_argv(args, &payload),
        Ok(None) => args,
        Err(msg) => {
            log::warn(format!("{msg}; running plain ssh"));
            args
        }
    }
}

/// Encoded payload for `host`, or `None` when its profile is off or has nothing in it.
fn load_payload(host: &str) -> Result<Option<String>, String> {
    let dir = config::config_dir().ok_or("cannot find home directory")?;
    let (cfg, created) = config::ensure_initialized(&dir).map_err(|e| e.to_string())?;
    if created {
        log::info(format!("created sample snippets in {}", dir.display()));
    }
    let Some(profile) = profile_for_target(&cfg, host) else { return Ok(None) };
    let rc = config::build_rc_for_host(&dir, &cfg, profile, &host_key(&cfg, host));
    if rc.trim().is_empty() {
        return Ok(None);
    }
    let encoded = payload::encode(&rc).map_err(|e| format!("cannot encode rc: {e}"))?;
    if encoded.len() > payload::WARN_ENCODED_LEN {
        log::warn(format!(
            "payload is {} bytes encoded (over {}); the ssh command line may be too long",
            encoded.len(),
            payload::WARN_ENCODED_LEN
        ));
    }
    Ok(Some(encoded))
}

/// The name a host has in `config.toml`: as typed if known there, else without a `user@` prefix.
pub fn host_key(cfg: &config::Config, host: &str) -> String {
    if cfg.hosts.contains_key(host) {
        return host.to_string();
    }
    host.rsplit_once('@').map_or(host, |(_, h)| h).to_string()
}

/// Looks up the profile by the host exactly as typed, then without a `user@` prefix.
pub fn profile_for_target<'a>(cfg: &'a config::Config, host: &str) -> Option<&'a str> {
    cfg.profile_for(&host_key(cfg, host))
}

/// Our options go first: ssh keeps the first value it sees for an option.
fn build_argv(args: Vec<String>, payload: &str) -> Vec<String> {
    let host = injectable_host(&args).unwrap_or_default();
    let mut argv = vec![
        "-o".to_string(),
        "RequestTTY=force".to_string(),
        "-o".to_string(),
        format!("RemoteCommand={}", bootstrap::remote_command(&host, payload)),
    ];
    argv.extend(args);
    argv
}

/// The host to log in to when this is a plain interactive login: a host, no remote command,
/// and no option that rules out a shell or sets RemoteCommand itself. `None` otherwise.
pub fn injectable_host(args: &[String]) -> Option<String> {
    let mut host: Option<&str> = None;
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--" {
            // Next is the host (if none yet), anything after that is a command.
            let rest = &args[i + 1..];
            return match (host, rest) {
                (Some(h), []) => Some(h.to_string()),
                (None, [h]) => Some(h.clone()),
                _ => None,
            };
        }
        if let Some(cluster) = arg.strip_prefix('-').filter(|c| !c.is_empty()) {
            let chars: Vec<char> = cluster.chars().collect();
            for (j, &c) in chars.iter().enumerate() {
                if OPTS_NO_SHELL.contains(c) {
                    return None;
                }
                if OPTS_WITH_VALUE.contains(c) {
                    let value: String = if j + 1 < chars.len() {
                        chars[j + 1..].iter().collect()
                    } else {
                        i += 1;
                        args.get(i).cloned().unwrap_or_default()
                    };
                    if c == 'o' && value.trim_start().to_ascii_lowercase().starts_with("remotecommand") {
                        return None;
                    }
                    break;
                }
            }
        } else if host.is_some() {
            return None; // remote command
        } else {
            host = Some(arg);
        }
        i += 1;
    }
    host.map(String::from)
}

/// Best guess at the host in an ssh argument list, for bookkeeping (last used).
pub fn host_arg(args: &[String]) -> String {
    injectable_host(args).or_else(|| args.last().cloned()).unwrap_or_default()
}

#[cfg(test)]
#[path = "main/tests.rs"]
mod tests;
