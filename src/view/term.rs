//! Borrowing the terminal, and giving it back.
//!
//! A program that panics in raw mode leaves somebody with no cursor, no echo
//! and a scrambled screen. Doing that once loses the trust of a person whose
//! entire reason for running this is to be told the truth about their machine,
//! so every exit path restores: normal quit, panic, and the signals a closing
//! terminal sends.
//!
//! Restoration is idempotent and never reports failure. If the screen is
//! already back, saying so again costs nothing; if it cannot be given back,
//! there is nowhere left to complain to.

use std::io::{self, Stdout};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{ExecutableCommand as _, cursor};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

/// The terminal, borrowed for as long as this lives.
pub struct Screen {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    /// Set when a signal asked us to stop.
    interrupted: Arc<AtomicBool>,
}

impl Screen {
    /// Take the terminal: raw mode, the alternate screen, mouse reporting, and
    /// no cursor.
    ///
    /// Installs a panic hook and signal handlers as a side effect, because a
    /// `Screen` that restores only on `Drop` does not survive the two ways this
    /// actually ends.
    ///
    /// # Errors
    ///
    /// Returns the underlying error if the terminal will not enter raw mode or
    /// the alternate screen — usually because there is no terminal.
    pub fn open() -> io::Result<Self> {
        install_panic_hook();
        let interrupted = install_signal_handlers()?;

        enable_raw_mode()?;
        let mut out = io::stdout();
        out.execute(EnterAlternateScreen)?;
        out.execute(EnableMouseCapture)?;
        out.execute(cursor::Hide)?;

        Ok(Self {
            terminal: Terminal::new(CrosstermBackend::new(out))?,
            interrupted,
        })
    }

    /// Whether a signal has asked this to stop.
    ///
    /// Polled rather than acted on inside the handler: the only thing safe to
    /// do in a signal handler is set a flag, and the loop is where the terminal
    /// can be given back in an orderly way.
    #[must_use]
    pub fn interrupted(&self) -> bool {
        self.interrupted.load(Ordering::Relaxed)
    }

    /// Draw one frame.
    ///
    /// # Errors
    ///
    /// Returns the underlying error if the frame could not be written.
    pub fn draw<F>(&mut self, render: F) -> io::Result<()>
    where
        F: FnOnce(&mut ratatui::Frame),
    {
        self.terminal.draw(render)?;
        Ok(())
    }

    /// The drawable area, in cells.
    ///
    /// # Errors
    ///
    /// Returns the underlying error if the size cannot be read.
    pub fn area(&self) -> io::Result<ratatui::layout::Rect> {
        self.terminal.size().map(|size| ratatui::layout::Rect {
            x: 0,
            y: 0,
            width: size.width,
            height: size.height,
        })
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        restore();
    }
}

/// Give the terminal back.
///
/// Every step is attempted even if an earlier one failed: leaving the screen in
/// raw mode because the cursor would not come back is the worse outcome.
pub fn restore() {
    // Nothing to give back if nothing was taken, and writing escapes into a
    // pipe or a test harness is worse than doing nothing.
    if !io::IsTerminal::is_terminal(&io::stdout()) {
        return;
    }
    let mut out = io::stdout();
    let _ = out.execute(DisableMouseCapture);
    let _ = out.execute(LeaveAlternateScreen);
    let _ = out.execute(cursor::Show);
    let _ = disable_raw_mode();
}

/// Restore before reporting a panic, so the message is readable.
///
/// Runs once however many times it is called: the hook chains to whatever was
/// installed before it, and installing twice would print the panic twice.
pub fn install_panic_hook() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore();
            previous(info);
        }));
    });
}

/// Notice the signals a closing terminal sends.
///
/// `SIGINT` is included so that ctrl-c leaves the screen in one piece rather
/// than killing the process where it stands.
///
/// # Errors
///
/// Returns the underlying error if a handler cannot be registered.
pub fn install_signal_handlers() -> io::Result<Arc<AtomicBool>> {
    let flag = Arc::new(AtomicBool::new(false));
    for signal in [
        signal_hook::consts::SIGINT,
        signal_hook::consts::SIGTERM,
        signal_hook::consts::SIGHUP,
    ] {
        signal_hook::flag::register(signal, Arc::clone(&flag))?;
    }
    Ok(flag)
}

#[cfg(test)]
mod tests {
    use super::{install_panic_hook, install_signal_handlers, restore};
    use std::sync::atomic::Ordering;

    #[test]
    fn restoring_a_terminal_that_was_never_taken_is_harmless() {
        // There is no terminal in a test run, and that must not be fatal.
        restore();
        restore();
    }

    #[test]
    fn the_panic_hook_installs_once_however_often_it_is_asked() {
        install_panic_hook();
        install_panic_hook();
        install_panic_hook();
    }

    #[test]
    fn a_signal_sets_the_flag_rather_than_killing_the_process() {
        let flag = install_signal_handlers().expect("register handlers");
        assert!(!flag.load(Ordering::Relaxed));

        signal_hook::low_level::raise(signal_hook::consts::SIGINT).expect("raise");

        // The handler runs on this thread before `raise` returns.
        assert!(
            flag.load(Ordering::Relaxed),
            "ctrl-c must leave the screen in one piece rather than killing us where we stand"
        );
    }
}
