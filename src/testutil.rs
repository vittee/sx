//! Helpers for the tests that run a real `bash`.

use std::io::Write;

/// The version of the `bash` on the PATH, or `None` when there is no working one. A launcher that is
/// not a shell (the WSL `bash.exe` with no distribution) prints a message instead of the version.
fn bash_version() -> Option<(u32, u32)> {
    let mut child = std::process::Command::new("bash")
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
