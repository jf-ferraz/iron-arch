//! Progress Reporting
//!
//! F2-012: Progress spinner/bar for long-running operations.
//! Wraps `indicatif` with Iron-specific defaults.

use indicatif::{ProgressBar, ProgressStyle};

/// Progress reporter for CLI operations.
pub struct ProgressReporter {
    bar: ProgressBar,
}

impl ProgressReporter {
    /// Create an indeterminate spinner with a message.
    pub fn spinner(msg: &str) -> Self {
        let bar = ProgressBar::new_spinner();
        bar.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner())
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );
        bar.set_message(msg.to_string());
        bar.enable_steady_tick(std::time::Duration::from_millis(80));
        Self { bar }
    }

    /// Create a progress bar with a known total.
    pub fn bar(total: u64, msg: &str) -> Self {
        let bar = ProgressBar::new(total);
        bar.set_style(
            ProgressStyle::with_template("{msg} [{bar:30.cyan/gray}] {pos}/{len} ({eta})")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("━╸─"),
        );
        bar.set_message(msg.to_string());
        Self { bar }
    }

    /// Update the spinner/bar message.
    pub fn tick(&self, msg: &str) {
        self.bar.set_message(msg.to_string());
    }

    /// Increment the progress bar by one.
    pub fn inc(&self) {
        self.bar.inc(1);
    }

    /// Mark the operation as finished with a final message.
    pub fn finish(&self, msg: &str) {
        self.bar.finish_with_message(msg.to_string());
    }

    /// Abandon the progress display (e.g., on error).
    pub fn abandon(&self, msg: &str) {
        self.bar.abandon_with_message(msg.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_creation() {
        let p = ProgressReporter::spinner("Loading...");
        p.tick("Still loading...");
        p.finish("Done");
    }

    #[test]
    fn test_bar_creation() {
        let p = ProgressReporter::bar(10, "Installing");
        for _ in 0..10 {
            p.inc();
        }
        p.finish("Installed 10 packages");
    }

    #[test]
    fn test_abandon() {
        let p = ProgressReporter::spinner("Working...");
        p.abandon("Failed!");
    }
}
