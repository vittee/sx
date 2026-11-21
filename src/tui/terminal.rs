//! Terminal plumbing: entering and leaving the TUI, the event loop and running ssh.

use super::*;

pub(super) fn enter(mouse: bool) -> io::Result<DefaultTerminal> {
    let term = ratatui::init();
    if mouse {
        execute!(io::stdout(), EnableMouseCapture)?;
    }
    Ok(term)
}

/// Mouse capture stops the terminal from selecting text, so F2 turns it off and on.
pub(super) fn apply_mouse(on: bool) -> io::Result<()> {
    if on {
        execute!(io::stdout(), EnableMouseCapture)
    } else {
        execute!(io::stdout(), DisableMouseCapture)
    }
}

/// Gives the terminal back: ratatui hides the cursor while drawing and `restore` does not undo it,
/// so without `Show` the ssh session would run with no visible cursor.
pub(super) fn leave() {
    let _ = execute!(io::stdout(), DisableMouseCapture);
    ratatui::restore();
    let _ = execute!(io::stdout(), Show);
}

pub fn run() -> ExitCode {
    let Some(dir) = config::config_dir() else {
        log::error("cannot find home directory");
        return ExitCode::FAILURE;
    };
    let cfg = match config::ensure_initialized(&dir) {
        Ok((cfg, _)) => cfg,
        Err(e) => {
            log::error(e.to_string());
            return ExitCode::FAILURE;
        }
    };
    let hosts = config::home_dir().map(|h| ssh_config::load_hosts(&h)).unwrap_or_default();
    let mut app = App::new(dir, cfg, hosts);

    let mut term = match enter(app.mouse) {
        Ok(t) => t,
        Err(e) => {
            log::error(format!("cannot start the terminal UI: {e}"));
            return ExitCode::FAILURE;
        }
    };
    let result = event_loop(&mut term, &mut app);
    leave();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error(e.to_string());
            ExitCode::FAILURE
        }
    }
}

pub(super) fn event_loop(term: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    let mut mouse_applied = app.mouse;
    while !app.quit {
        term.draw(|f| app.draw(f))?;
        if event::poll(Duration::from_millis(250))? {
            app.on_event(event::read()?);
        }
        if app.mouse != mouse_applied {
            apply_mouse(app.mouse)?;
            mouse_applied = app.mouse;
        }
        if let Some(args) = app.pending_connect.take() {
            connect(term, app, args)?;
        }
    }
    Ok(())
}

/// Column and row of the cursor, or `None` when the terminal does not answer.
fn cursor_pos() -> Option<(u16, u16)> {
    ratatui::crossterm::cursor::position().ok()
}

/// Leaves the TUI, runs ssh in the normal terminal, then comes back.
pub(super) fn connect(term: &mut DefaultTerminal, app: &mut App, args: Vec<String>) -> io::Result<()> {
    let host = crate::host_arg(&args);
    app.note_connected(&host);

    keep_sx_alive_on_ctrl_c();
    leave();
    // The remote side rewrites this line to "connected" once the login succeeds. If you abort
    // instead, the line is deleted again below.
    log::info(format!("connecting to {host} ... (Ctrl-C to abort)"));
    let below = cursor_pos(); // the row under that line
    let outcome = crate::run_and_record(&crate::prepare_args(args), &host);
    app.history_cache = None; // read the new session
    if outcome == SshOutcome::Interrupted {
        // Lines ssh printed in between (a password prompt, the ^C echo) push the cursor down.
        let (up, col) = match (below, cursor_pos()) {
            (Some((_, b)), Some((col, now))) => (now.saturating_sub(b) + 1, col),
            _ => (1, 0),
        };
        log::erase_above(up, col);
    }
    if outcome == SshOutcome::Exited(255) {
        log::error("ssh failed (exit 255). Press Enter to return.");
        let _ = io::stdin().read_line(&mut String::new());
    }
    *term = enter(app.mouse)?;
    term.clear()?;
    match outcome {
        SshOutcome::Interrupted => app.say(format!("connection to {host} aborted")),
        SshOutcome::Exited(0 | 255) => app.status = None,
        SshOutcome::Exited(_) => {}
    }
    Ok(())
}

/// Ctrl-C reaches every process in the terminal's foreground group, so it would kill `sx` along
/// with `ssh`. A no-op handler keeps `sx` running. It must be a handler, not "ignore": ignored
/// signals are inherited by child processes, while handled ones reset to default on exec, so
/// `ssh` still dies on Ctrl-C. The handler also records the Ctrl-C, see `crate::CTRL_C`. Inside the TUI (raw mode) Ctrl-C is a key event, not a signal.
pub(super) fn keep_sx_alive_on_ctrl_c() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let _ = ctrlc::set_handler(|| crate::CTRL_C.store(true, std::sync::atomic::Ordering::Relaxed));
    });
}
