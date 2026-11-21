mod bootstrap;
mod config;
mod editor;
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

const HELP: &str = "\
sx: run ssh with your own shell rc (prompt, aliases) on the server.

usage:
  sx                     open the TUI (hosts, snippets, profiles)
  sx [ssh args] host     like ssh, but the remote bash loads your rc
  sx init                create the sample snippets and default profile
  sx init --presets      add the vim presets that are missing, switched off
  sx --dry-run [...]     print the ssh command instead of running it
  sx --version           print the sx version (ssh's own -V shows the ssh version)
  sx --help

config: $SX_HOME, else ~/.config/sx (snippets/*.sh, config.toml)
sx skips injection (plain ssh) when you pass a remote command, -N, -T, -W, -O, -Q, -V, -G, -s,
-f, -n, or -o RemoteCommand, or when the host's profile is off.";

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
        Some("--help") => {
            println!("{HELP}");
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

    let host = host_arg(&args);
    let ssh_args = prepare_args(args);
    if dry_run {
        println!("ssh {}", ssh_args.iter().map(|a| format!("{a:?}")).collect::<Vec<_>>().join(" "));
        return ExitCode::SUCCESS;
    }
    // `as u8` would wrap a Windows status such as 0x100 to 0 (success).
    ExitCode::from(u8::try_from(run_and_record(&ssh_args, &host).code()).unwrap_or(1))
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
        Ok((_, true)) => log::ok_out(format!("created snippets and default profile in {}", dir.display())),
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
