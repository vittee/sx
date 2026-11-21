//! Connection history, one line per session in `history.tsv` next to `config.toml`:
//! `unix_time <TAB> host <TAB> seconds <TAB> outcome`.
//! The file is append-only and trimmed to the newest lines when it grows.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const FILE: &str = "history.tsv";
/// Rewrite the file when it passes this many bytes, keeping `KEEP_LINES` lines.
const TRIM_AT_BYTES: u64 = 256 * 1024;
const KEEP_LINES: usize = 1000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub when: u64,
    pub host: String,
    pub secs: u64,
    /// `ok`, `exit N`, `failed` (could not connect) or `aborted`.
    pub outcome: String,
}

fn path(dir: &Path) -> PathBuf {
    dir.join(FILE)
}

/// Text for a finished ssh run: 255 is ssh's own failure code, 0 a clean exit.
pub fn outcome_text(interrupted: bool, code: i32) -> String {
    match (interrupted, code) {
        (true, _) => "aborted".into(),
        (_, 0) => "ok".into(),
        (_, 255) => "failed".into(),
        (_, c) => format!("exit {c}"),
    }
}

fn clean(s: &str) -> String {
    s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect()
}

pub fn append(dir: &Path, e: &Entry) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let mut f = OpenOptions::new().create(true).append(true).open(path(dir))?;
    writeln!(f, "{}\t{}\t{}\t{}", e.when, clean(&e.host), e.secs, clean(&e.outcome))?;
    drop(f);
    if fs::metadata(path(dir)).is_ok_and(|m| m.len() > TRIM_AT_BYTES) {
        trim(dir)?;
    }
    Ok(())
}

fn trim(dir: &Path) -> io::Result<()> {
    let text = fs::read_to_string(path(dir))?;
    let lines: Vec<&str> = text.lines().collect();
    let keep = &lines[lines.len().saturating_sub(KEEP_LINES)..];
    fs::write(path(dir), keep.join("\n") + "\n")
}

fn parse(line: &str) -> Option<Entry> {
    let mut f = line.split('\t');
    Some(Entry {
        when: f.next()?.parse().ok()?,
        host: f.next()?.to_string(),
        secs: f.next()?.parse().ok()?,
        outcome: f.next()?.to_string(),
    })
}

/// The newest `limit` sessions of `host`, newest first. A missing or damaged file gives fewer rows.
pub fn for_host(dir: &Path, host: &str, limit: usize) -> Vec<Entry> {
    let Ok(text) = fs::read_to_string(path(dir)) else { return Vec::new() };
    text.lines().rev().filter_map(parse).filter(|e| e.host == host).take(limit).collect()
}

/// How many sessions were recorded for `host`.
pub fn count_for_host(dir: &Path, host: &str) -> usize {
    let Ok(text) = fs::read_to_string(path(dir)) else { return 0 };
    text.lines().filter_map(parse).filter(|e| e.host == host).count()
}

/// `45s`, `12m 03s`, `2h 05m`.
pub fn format_secs(secs: u64) -> String {
    match secs {
        0..=59 => format!("{secs}s"),
        60..=3599 => format!("{}m {:02}s", secs / 60, secs % 60),
        _ => format!("{}h {:02}m", secs / 3600, secs % 3600 / 60),
    }
}

#[cfg(test)]
mod tests;
