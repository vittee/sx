/// Command run on the server by sshd's `RemoteCommand`, for a login to `host` (as typed).
/// `payload` is the encoded rc (base64 of gzip), carried inside the command itself, so the server
/// needs no `AcceptEnv` setting.
///
/// Rules this string must follow:
/// - No `%`: ssh expands `%h`, `%p` and friends in `RemoteCommand` and fails on unknown tokens.
///   Base64 has no `%`, and `encode` is the only source of `payload`.
/// - Process substitution (`<(...)`) sits inside single quotes passed to `bash -c`, so a
///   POSIX `sh` login shell can still parse the line. Base64 has no single quote either.
/// - Before starting bash it checks that `bash` exists and that `base64 -d` and `gunzip` work.
///   If a check fails it says why on one line and starts the plain login shell instead, so the
///   server's own rc still loads. Without the checks bash would start with an empty rcfile: no
///   prompt, no aliases and not even the server's `~/.bashrc`.
/// - It first rewrites the local `sx: connecting to ...` line (the line above the cursor) to
///   `sx: connected to <host>`: this only runs once the login succeeded, so it needs no ssh
///   feature. The printf has no `%` conversion, so it is also safe from ssh's token expansion.
pub fn remote_command(host: &str, payload: &str) -> String {
    debug_assert!(payload.chars().all(is_base64_char), "payload must be base64");
    format!(
        concat!(
            "printf '\\033[1A\\r\\033[2K\\033[1;32msx:\\033[0m \\033[32mconnected to {host}\\033[0m\\n'; ",
            "W=; ",
            "if ! command -v bash >/dev/null 2>&1; then W=\"bash is not installed\"; ",
            "elif ! echo QQ== | base64 -d >/dev/null 2>&1 || ! command -v gunzip >/dev/null 2>&1; ",
            "then W=\"base64 -d or gunzip is missing\"; fi; ",
            "if [ -z \"$W\" ]; then exec bash -c '",
            "P={payload}; export SX_RC=$P; ",
            "exec bash --rcfile <(echo \"$P\" | base64 -d | gunzip) -i",
            "'; fi; ",
            "printf '\\033[1;33msx:\\033[0m '; echo \"$W, using the plain shell\"; ",
            "exec \"$SHELL\" -l"
        ),
        host = printable(host),
        payload = payload
    )
}

fn is_base64_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=')
}

/// The host as it is safe to paste inside the single-quoted printf text: anything that could end
/// the quote, start a printf escape or conversion, or be expanded by ssh is dropped.
fn printable(host: &str) -> String {
    host.chars().filter(|c| c.is_ascii_alphanumeric() || "._-@:".contains(*c)).collect()
}

#[cfg(test)]
mod tests;
