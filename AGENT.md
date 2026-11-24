# sx

Rust CLI and TUI that wraps `ssh` so the remote bash uses the user's own prompt, aliases, vim, tmux and inputrc.
Nothing is written on the server. The README explains the mechanism, the config and the keys.

## Layout

- `src/main.rs`: argument handling, builds the ssh command line, `--options`, `init`.
- `src/bootstrap`, `src/payload`: the `RemoteCommand` and the encoded rc sent through `SetEnv`.
- `src/config`: `config.toml`, snippets, profiles, per-host `HostPrefs` and named `ConnOptions`.
- `src/ssh_config`: hosts from `~/.ssh/config` and `known_hosts`. It never writes there.
- `src/tui`: ratatui screens. `hostedit.rs` is the options form, `hosts.rs` the host list, `tests.rs` the UI tests.
- `src/completions`: `sx completions` scripts for five shells and `--install`. `sx __complete` is hidden and used by the scripts.
- `src/history`, `src/log`, `src/prompt`, `src/editor`, `src/highlight`: smaller pieces.
- `src/main/testutil.rs`: helpers for tests that start bash, see Conventions.

## Checks

Run both before saying a change is done:

```sh
cargo clippy --all-targets -q -- -D warnings
cargo test -q
```

Clippy runs with `-D warnings` on Ubuntu, macOS and Windows in CI.
Code that only needs `mut` or an import under `#[cfg(windows)]` fails on the other two,
so keep the `mut` inside the `cfg` branch.

On Windows `cargo build` fails with "Access is denied" while an `sx.exe` is running.
Use `cargo check`, clippy and test, or build with `--target-dir` somewhere else.

## Conventions

- ssh keeps the first value of an option. Options sx adds for a host go after what the user typed, so the user's win.
- Saved connection options live in sx's `config.toml`, never in `~/.ssh/config`.
- UI tests render a 110x30 `TestBackend`. Reuse the helpers at the top of `src/tui/tests.rs` (`app`, `render`, `key`, `ctrl`, `typed`, `click`, `squash`).
- A test that starts bash uses `crate::tests::testutil::bash()` and holds `crate::tests::testutil::bash_lock()` while it runs.
  Git Bash flips the console mode on Windows, and parallel bash processes garble the terminal.
- Usage text follows `sx --help`: `[]` is optional, `<>` is required.
- `README.md` has CRLF line endings. Keep them, for example by editing it with Python and `newline=''`.
- Update the README in the same commit as the change it describes.
- The README has a `## Contents` list of its `##` headings. Update it when you add, rename or move one.
  Check that links such as `#sx-theme-sxt` match the heading, with punctuation dropped and spaces turned into `-`.
- Wrap prose in the README at clause ends, one clause per line where it helps.

## Commits and releases

- Commit only when asked. Never push. Never add `Co-Authored-By` or other attribution lines.
- One topic per commit, a one-line conventional message (`feat:`, `fix:`, `docs:`, `chore:`).
- `demo/sx.gif` is recorded with `sh demo/run.sh` (VHS in Docker).
- `demo/screens/*.png` are the README screenshots. `sh demo/run.sh screens.tape` records them.

### Releasing

The changelog is built from the conventional commit messages with [git-cliff](https://git-cliff.org).
The release workflow (cargo-dist) runs when a `vX.Y.Z` tag is pushed and uses the matching `## vX.Y.Z`
section of `CHANGELOG.md` as the GitHub release notes.

```sh
# set the new version in Cargo.toml, then
cargo check                                               # updates Cargo.lock
git cliff --unreleased --tag vX.Y.Z --prepend CHANGELOG.md
git commit -am "chore: release vX.Y.Z"
git tag vX.Y.Z
git push origin main
git push origin vX.Y.Z                                    # push the tag on its own
```

Releasing is the user's call. Prepare it only when asked, and never push the branch or the tag.
