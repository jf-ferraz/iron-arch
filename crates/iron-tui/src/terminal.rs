//! Terminal management for Iron TUI
//!
//! Handles terminal initialization, cleanup, and panic recovery.

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::{self, stdout, Stdout};
use std::panic;

/// Terminal wrapper that handles setup and cleanup
pub struct Terminal {
    /// The ratatui terminal instance
    terminal: ratatui::Terminal<CrosstermBackend<Stdout>>,
}

impl Terminal {
    /// Create and initialize a new terminal
    pub fn new() -> anyhow::Result<Self> {
        // Set up panic hook to restore terminal on panic
        let original_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            Self::restore_terminal().expect("Failed to restore terminal");
            original_hook(panic_info);
        }));

        // Initialize terminal
        terminal::enable_raw_mode()?;
        let mut stdout = stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = ratatui::Terminal::new(backend)?;

        Ok(Self { terminal })
    }

    /// Restore terminal to original state
    fn restore_terminal() -> anyhow::Result<()> {
        terminal::disable_raw_mode()?;
        execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
        Ok(())
    }

    /// Draw a frame using the provided render function
    pub fn draw<F>(&mut self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&mut Frame),
    {
        self.terminal.draw(f)?;
        Ok(())
    }

    /// Get the terminal size
    pub fn size(&self) -> io::Result<ratatui::layout::Size> {
        self.terminal.size()
    }

    /// Clear the terminal
    pub fn clear(&mut self) -> anyhow::Result<()> {
        self.terminal.clear()?;
        Ok(())
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        Self::restore_terminal().expect("Failed to restore terminal on drop");
    }
}
