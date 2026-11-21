mod bootstrap;
mod config;
mod payload;

use std::env;
use std::process::{Command, ExitCode};

const HELP: &str = "\
sx: run ssh with your own shell rc (prompt, aliases) on the server.

usage:
  sx [ssh args] host     like ssh, but the remote bash loads your rc
  sx init                create the sample snippet
  sx --dry-run [...]     print the ssh command instead of running it
  sx --help

config: $SX_HOME, else ~/.config/sx (snippets/*.sh, config.toml)
sx skips injection (plain ssh) when you pass a remote command, -N, -T, -W, -O, -Q, -V, -G, -s,
-f, -n, or -o RemoteCommand, or when the host's profile is off.";

/// ssh options that take a value.
const OPTS_WITH_VALUE: &str = "BbcDEeFIiJLlmOopQRSWw";
/// ssh options after which an interactive remote shell is not wanted.
const OPTS_NO_SHELL: &str = "NWOQVGsTfn";

fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let mut dry_run = false;

    match args.first().map(String::as_str) {
        Some("--help") | None => {
            println!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Some("init") => return init(),
        Some("--dry-run") => {
            dry_run = true;
            args.remove(0);
        }
        _ => {}
    }

    let given = args.len();
    let ssh_args = prepare_args(args);
    if dry_run {
        println!("ssh {}", ssh_args.iter().map(|a| format!("{a:?}")).collect::<Vec<_>>().join(" "));
        return ExitCode::SUCCESS;
    }
    if ssh_args.len() > given {
        // The remote side rewrites this line to "connected to <host>" once the login succeeds.
        let host = injectable_host(&ssh_args[ssh_args.len() - given..]).unwrap_or_default();
        eprintln!("sx: connecting to {host} ... (Ctrl-C to abort)");
    }
    ExitCode::from(run_ssh(&ssh_args))
}

/// Runs `ssh` with inherited stdio and returns its exit code (255 when it was killed by a signal).
fn run_ssh(args: &[String]) -> u8 {
    match Command::new("ssh").args(args).status() {
        Ok(status) => status.code().and_then(|c| u8::try_from(c).ok()).unwrap_or(255),
        Err(e) => {
            eprintln!("sx: cannot run ssh: {e}");
            127
        }
    }
}

/// Adds the sx options to `args` when this is a plain interactive login to a host.
/// Problems (unreadable config, empty rc) fall back to plain ssh with a message on stderr.
fn prepare_args(args: Vec<String>) -> Vec<String> {
    let Some(host) = injectable_host(&args) else { return args };
    match load_payload(&host) {
        Ok(Some(payload)) => build_argv(args, &payload),
        Ok(None) => args,
        Err(msg) => {
            eprintln!("sx: {msg}; running plain ssh");
            args
        }
    }
}

/// Encoded payload for `host`, or `None` when its profile is off or has nothing in it.
fn load_payload(host: &str) -> Result<Option<String>, String> {
    let dir = config::config_dir().ok_or("cannot find home directory")?;
    let (cfg, created) = config::ensure_initialized(&dir).map_err(|e| e.to_string())?;
    if created {
        eprintln!("sx: created sample snippets in {}", dir.display());
    }
    let Some(profile) = profile_for_target(&cfg, host) else { return Ok(None) };
    let rc = config::build_rc(&dir, &cfg, profile);
    if rc.trim().is_empty() {
        return Ok(None);
    }
    let encoded = payload::encode(&rc).map_err(|e| format!("cannot encode rc: {e}"))?;
    if encoded.len() > payload::WARN_ENCODED_LEN {
        eprintln!(
            "sx: payload is {} bytes encoded (over {}); the ssh command line may be too long",
            encoded.len(),
            payload::WARN_ENCODED_LEN
        );
    }
    Ok(Some(encoded))
}

fn init() -> ExitCode {
    let Some(dir) = config::config_dir() else {
        eprintln!("sx: cannot find home directory");
        return ExitCode::FAILURE;
    };
    match config::ensure_initialized(&dir) {
        Ok((_, true)) => println!("sx: created sample snippets in {}", dir.display()),
        Ok((_, false)) => println!("sx: {} already set up, left unchanged", dir.display()),
        Err(e) => {
            eprintln!("sx: {e}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}

/// The name a host has in `config.toml`: as typed if known there, else without a `user@` prefix.
fn host_key(cfg: &config::Config, host: &str) -> String {
    if cfg.hosts.contains_key(host) {
        return host.to_string();
    }
    host.rsplit_once('@').map_or(host, |(_, h)| h).to_string()
}

/// Looks up the profile by the host exactly as typed, then without a `user@` prefix.
fn profile_for_target<'a>(cfg: &'a config::Config, host: &str) -> Option<&'a str> {
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
fn injectable_host(args: &[String]) -> Option<String> {
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

#[cfg(test)]
#[path = "main/tests.rs"]
mod tests;
