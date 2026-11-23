//! Helpers for the tests that run a real `bash`.

use std::io::Write;

/// A `bash` command for the tests. On Windows it gets a hidden console of its own: Git Bash changes
/// the modes of the console it shares with `cargo test`, and the output printed meanwhile loses its
/// carriage returns.
pub fn bash() -> std::process::Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut c = std::process::Command::new("bash");
        c.creation_flags(CREATE_NO_WINDOW);
        c
    }
    #[cfg(not(windows))]
    std::process::Command::new("bash")
}

/// What [`bash_lock`] hands out: it holds the lock on Windows and is empty elsewhere.
pub struct Lock(#[cfg(windows)] #[allow(dead_code)] std::sync::MutexGuard<'static, ()>);

/// Held while a test runs a `bash`. Hidden consoles started and closed at the same time still confuse
/// each other, and the console of `cargo test` can end up without carriage returns. One at a time is safe.
pub fn bash_lock() -> Lock {
    #[cfg(windows)]
    {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        Lock(LOCK.lock().unwrap_or_else(|e| e.into_inner()))
    }
    #[cfg(not(windows))]
    Lock()
}

/// The version of the `bash` on the PATH, or `None` when there is no working one. A launcher that is
/// not a shell (the WSL `bash.exe` with no distribution) prints a message instead of the version.
fn bash_version() -> Option<(u32, u32)> {
    let _bash = bash_lock();
    let mut child = bash()
        .arg("-s")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(b"echo ${BASH_VERSINFO[0]} ${BASH_VERSINFO[1]}\n").ok()?;
    let out = child.wait_with_output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let mut v = text.split_whitespace().map(|n| n.parse::<u32>());
    let (Some(Ok(major)), Some(Ok(minor))) = (v.next(), v.next()) else { return None };
    Some((major, minor))
}

/// Whether the `bash` the tests run is at least this version, for a test to skip itself when it is
/// not (macOS ships 3.2, which has no `${PS1@P}`, and Windows may have no bash at all). A skip is
/// said on stderr. On Linux CI there is always a bash, so a missing or old one fails the test
/// instead of letting it pass without running.
pub fn bash_at_least(major: u32, minor: u32) -> bool {
    let found = bash_version();
    if found.is_some_and(|v| v >= (major, minor)) {
        return true;
    }
    let ci_on_linux = cfg!(target_os = "linux") && std::env::var_os("CI").is_some();
    assert!(!ci_on_linux, "CI on Linux needs bash {major}.{minor} or newer, found {found:?}");
    eprintln!("skipped: needs bash {major}.{minor} or newer, found {found:?}");
    false
}
