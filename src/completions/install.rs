//! `sx completions --install [shell]`: puts the completion where the shell loads it, after
//! showing what it will change and asking.

use super::Shell;
use crate::config;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// The places the install writes under. Everything comes from here so a test can point it at a
/// scratch directory.
pub(super) struct Dirs {
    pub home: PathBuf,
    /// `$ZDOTDIR`, where zsh looks for `.zshrc`.
    pub zdotdir: Option<PathBuf>,
    /// `$XDG_CONFIG_HOME`.
    pub xdg_config: Option<PathBuf>,
    /// `%APPDATA%`, where nushell keeps its config on Windows.
    pub appdata: Option<PathBuf>,
    /// The PowerShell profile file, asked from PowerShell itself.
    pub ps_profile: Option<PathBuf>,
}

impl Dirs {
    fn config_home(&self) -> PathBuf {
        self.xdg_config.clone().unwrap_or_else(|| self.home.join(".config"))
    }

    pub(super) fn nushell(&self) -> PathBuf {
        if cfg!(windows) {
            self.appdata.clone().unwrap_or_else(|| self.home.join("AppData").join("Roaming")).join("nushell")
        } else if cfg!(target_os = "macos") {
            self.home.join("Library").join("Application Support").join("nushell")
        } else {
            self.config_home().join("nushell")
        }
    }
}

/// One change the install makes.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Action {
    /// Adds `text` to the end of `file`, unless `marker` is in it already.
    Append { file: PathBuf, text: String, marker: &'static str },
    /// Makes `file` hold exactly `content`.
    Write { file: PathBuf, content: String },
}

impl Action {
    pub(super) fn file(&self) -> &Path {
        match self {
            Action::Append { file, .. } | Action::Write { file, .. } => file,
        }
    }

    /// True when there is nothing left to do.
    pub(super) fn is_done(&self) -> bool {
        let now = fs::read_to_string(self.file()).unwrap_or_default();
        match self {
            Action::Append { marker, .. } => now.contains(marker),
            Action::Write { content, .. } => now == *content,
        }
    }

    pub(super) fn describe(&self) -> String {
        match self {
            Action::Append { file, text, .. } => format!("add to {}:\n      {text}", file.display()),
            Action::Write { file, .. } => format!("write {}", file.display()),
        }
    }

    pub(super) fn apply(&self) -> io::Result<()> {
        if let Some(dir) = self.file().parent() {
            fs::create_dir_all(dir)?;
        }
        match self {
            Action::Append { file, text, .. } => {
                let mut now = fs::read_to_string(file).unwrap_or_default();
                if !now.is_empty() && !now.ends_with('\n') {
                    now.push('\n');
                }
                now.push_str(&format!("\n# sx shell completion\n{text}\n"));
                fs::write(file, now)
            }
            Action::Write { file, content } => fs::write(file, content),
        }
    }
}

/// What installing the completion of `shell` changes.
pub(super) fn plan(shell: Shell, dirs: &Dirs) -> Result<Vec<Action>, String> {
    let append = |file: PathBuf, text: &str, marker: &'static str| Action::Append { file, text: text.to_string(), marker };
    Ok(match shell {
        Shell::Bash => vec![append(dirs.home.join(".bashrc"), "eval \"$(sx completions bash)\"", "sx completions bash")],
        Shell::Zsh => {
            let rc = dirs.zdotdir.clone().unwrap_or_else(|| dirs.home.clone()).join(".zshrc");
            vec![append(rc, "source <(sx completions zsh)", "sx completions zsh")]
        }
        // fish loads every file in this directory by itself.
        Shell::Fish => vec![Action::Write {
            file: dirs.config_home().join("fish").join("completions").join("sx.fish"),
            content: shell.script().to_string(),
        }],
        Shell::Powershell => {
            let profile = dirs.ps_profile.clone().ok_or("cannot ask PowerShell for its profile path (is it installed?)")?;
            vec![append(profile, "sx completions powershell | Out-String | Invoke-Expression", "sx completions powershell")]
        }
        Shell::Nushell => {
            let dir = dirs.nushell();
            vec![
                Action::Write { file: dir.join("sx-completions.nu"), content: shell.script().to_string() },
                append(dir.join("config.nu"), "use sx-completions.nu *", "sx-completions.nu"),
            ]
        }
    })
}

/// The shell to install for when none is named, from `$SHELL` and nushell's `$NU_VERSION`.
/// Both are inherited by child shells, so when they disagree it is a guess, and `None` asks the user to name the shell.
pub(super) fn detect(login_shell: Option<&str>, nu_version: bool) -> Option<Shell> {
    let name = login_shell.and_then(|s| Path::new(s).file_stem()).and_then(|s| s.to_str());
    match (name.and_then(Shell::parse), nu_version) {
        (Some(_), true) => None,
        (Some(shell), false) => Some(shell),
        (None, true) => Some(Shell::Nushell),
        (None, false) => cfg!(windows).then_some(Shell::Powershell),
    }
}

/// The profile file of PowerShell, asked from `exe` in turn.
fn powershell_profile(order: &[&str]) -> Option<PathBuf> {
    order.iter().find_map(|exe| {
        let out = Command::new(exe).args(["-NoProfile", "-NonInteractive", "-Command", "$PROFILE"]).output().ok()?;
        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (out.status.success() && !path.is_empty()).then(|| PathBuf::from(path))
    })
}

/// `sx completions --install [shell] [--yes]`.
pub(super) fn run(rest: &[String]) -> ExitCode {
    let mut yes = false;
    let mut name: Option<&str> = None;
    for a in rest {
        match a.as_str() {
            "--install" => {}
            "--yes" | "-y" => yes = true,
            other if name.is_none() && !other.starts_with('-') => name = Some(other),
            _ => return usage(),
        }
    }
    let shell = match name {
        Some(n) => match Shell::parse(n) {
            Some(s) => s,
            None => return usage(),
        },
        None => match detect(std::env::var("SHELL").ok().as_deref(), std::env::var_os("NU_VERSION").is_some()) {
            Some(s) => s,
            None => {
                eprintln!("sx: cannot tell which shell this is: name it, as in `sx completions --install bash`");
                return ExitCode::from(2);
            }
        },
    };
    let Some(home) = config::home_dir() else {
        eprintln!("sx: cannot find the home directory");
        return ExitCode::FAILURE;
    };
    let ps_profile = (shell == Shell::Powershell)
        .then(|| powershell_profile(if name == Some("pwsh") { &["pwsh", "powershell"] } else { &["powershell", "pwsh"] }))
        .flatten();
    let dirs = Dirs {
        home,
        zdotdir: std::env::var_os("ZDOTDIR").map(PathBuf::from),
        xdg_config: std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
        appdata: std::env::var_os("APPDATA").map(PathBuf::from),
        ps_profile,
    };
    let actions = match plan(shell, &dirs) {
        Ok(a) => a,
        Err(msg) => {
            eprintln!("sx: {msg}");
            return ExitCode::FAILURE;
        }
    };
    let todo: Vec<&Action> = actions.iter().filter(|a| !a.is_done()).collect();
    if todo.is_empty() {
        println!("completion for {} is already installed", shell.name());
        return ExitCode::SUCCESS;
    }
    println!("To complete sx in {}, sx will:", shell.name());
    for a in &todo {
        println!("  - {}", a.describe());
    }
    if !yes && !confirm() {
        println!("nothing changed");
        return ExitCode::SUCCESS;
    }
    for a in todo {
        if let Err(e) = a.apply() {
            eprintln!("sx: cannot write {}: {e}", a.file().display());
            return ExitCode::FAILURE;
        }
    }
    println!("done: open a new {} to use it", shell.name());
    ExitCode::SUCCESS
}

fn usage() -> ExitCode {
    eprintln!("{}", super::USAGE);
    ExitCode::from(2)
}

fn confirm() -> bool {
    print!("Go ahead? [y/N] ");
    let _ = io::stdout().flush();
    let mut line = String::new();
    io::stdin().read_line(&mut line).is_ok() && matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
}
