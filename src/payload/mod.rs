use std::io::{self, Write};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use flate2::Compression;
use flate2::write::GzEncoder;

/// Keeps the server's own environment: its `~/.bashrc` runs before the user's rc.
const SERVER_RC_PRELUDE: &str = "[ -f ~/.bashrc ] && . ~/.bashrc\n";

/// Encoded payloads above this size risk hitting argument/environment limits.
pub const WARN_ENCODED_LEN: usize = 8192;

/// Turns the user's rc text into the `LC_MONETARY` value: base64(gzip(prelude + rc)).
pub fn encode(user_rc: &str) -> io::Result<String> {
    let rc = format!("{SERVER_RC_PRELUDE}{}", user_rc.replace("\r\n", "\n"));
    let mut gz = GzEncoder::new(Vec::new(), Compression::best());
    gz.write_all(rc.as_bytes())?;
    Ok(STANDARD.encode(gz.finish()?))
}

#[cfg(test)]
mod tests;
