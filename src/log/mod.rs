//! Colored `sx:` log lines. Color is off for `NO_COLOR`, `TERM=dumb`, and when the stream is not a terminal.

use std::env;
use std::io::{IsTerminal, Write};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Level {
    Info,
    Ok,
    Warn,
    Error,
}

impl Level {
    fn code(self) -> &'static str {
        match self {
            Level::Info => "36",
            Level::Ok => "32",
            Level::Warn => "33",
            Level::Error => "31",
        }
    }
}

/// Builds one log line (no trailing newline).
fn render(level: Level, msg: &str, color: bool) -> String {
    if color {
        let c = level.code();
        format!("\x1b[1;{c}msx:\x1b[0m \x1b[{c}m{msg}\x1b[0m")
    } else {
        format!("sx: {msg}")
    }
}

/// Windows consoles need virtual terminal processing switched on before they understand ANSI.
fn ansi_supported() -> bool {
    static OK: OnceLock<bool> = OnceLock::new();
    *OK.get_or_init(|| {
        #[cfg(windows)]
        {
            ratatui::crossterm::ansi_support::supports_ansi()
        }
        #[cfg(not(windows))]
        {
            true
        }
    })
}

fn color_enabled(is_tty: bool) -> bool {
    is_tty
        && env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
        && env::var("TERM").map_or(true, |t| t != "dumb")
        && ansi_supported()
}

/// Whether to color what goes to stdout (the help text): a terminal that understands it.
pub fn stdout_color() -> bool {
    color_enabled(std::io::stdout().is_terminal())
}

fn emit(level: Level, msg: &str, to_stderr: bool) {
    let (tty, mut out): (bool, Box<dyn Write>) = if to_stderr {
        (std::io::stderr().is_terminal(), Box::new(std::io::stderr()))
    } else {
        (std::io::stdout().is_terminal(), Box::new(std::io::stdout()))
    };
    let _ = writeln!(out, "{}", render(level, msg, color_enabled(tty)));
}

pub fn info(msg: impl AsRef<str>) {
    emit(Level::Info, msg.as_ref(), true);
}

pub fn warn(msg: impl AsRef<str>) {
    emit(Level::Warn, msg.as_ref(), true);
}

pub fn error(msg: impl AsRef<str>) {
    emit(Level::Error, msg.as_ref(), true);
}

/// Escape sequence that deletes the line `up` lines above the cursor: the lines below it move up
/// one row, so the cursor ends on the line it was on (now one row higher), at column `col`.
/// It uses plain relative moves and "delete line", not save/restore cursor, which not every
/// terminal supports. A move by 0 is skipped because terminals treat it as a move by 1.
fn erase_sequence(up: u16, col: u16) -> String {
    let down = if up > 1 { format!("\x1b[{}B", up - 1) } else { String::new() };
    let right = if col > 0 { format!("\x1b[{col}C") } else { String::new() };
    format!("\x1b[{up}A\r\x1b[M{down}\r{right}")
}

/// Deletes the log line `up` lines above the cursor, which sits at column `col`. Does nothing when
/// stderr is not a terminal.
pub fn erase_above(up: u16, col: u16) {
    let mut err = std::io::stderr();
    if err.is_terminal() {
        let _ = err.write_all(erase_sequence(up.max(1), col).as_bytes());
        let _ = err.flush();
    }
}

/// Success message on stdout (command results such as `sx init`).
pub fn ok_out(msg: impl AsRef<str>) {
    emit(Level::Ok, msg.as_ref(), false);
}

#[cfg(test)]
mod tests;
