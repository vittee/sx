use std::io::{self, Write};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use flate2::Compression;
use flate2::write::GzEncoder;

/// Keeps the server's own environment: its `~/.bashrc` runs before the user's rc.
const SERVER_RC_PRELUDE: &str = "[ -f ~/.bashrc ] && . ~/.bashrc\n";

/// The payload travels in the `ssh` command line. Windows allows about 32 KB for a whole command
/// line (`CreateProcess`), the lowest limit on a client, so a payload above this size may be refused
/// there. Linux and macOS allow far more.
pub const WARN_ENCODED_LEN: usize = 24_000;

/// Turns the user's rc text into the payload `sx` sends: base64(gzip(prelude + rc)).
pub fn encode(user_rc: &str) -> io::Result<String> {
    let rc = format!("{SERVER_RC_PRELUDE}{}", user_rc.replace("\r\n", "\n"));
    let mut gz = GzEncoder::new(Vec::new(), Compression::best());
    gz.write_all(rc.as_bytes())?;
    Ok(STANDARD.encode(gz.finish()?))
}

#[cfg(test)]
mod tests;
