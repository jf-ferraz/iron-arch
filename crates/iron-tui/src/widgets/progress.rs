//! Progress Tracking Component
//!
//! Non-blocking progress display for long-running operations.

use crate::ui::theme;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Gauge, Paragraph};
use std::time::{Duration, Instant};

/// Spinner animation frames
const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Progress tracker for long-running operations
#[derive(Debug, Clone)]
pub struct ProgressTracker {
    /// Operation title
    pub title: String,
    /// Current operation description
    pub description: String,
    /// Progress percentage (0-100), None for indeterminate
    pub percentage: Option<u8>,
    /// Whether the operation is complete
    pub complete: bool,
    /// Whether the operation failed
    pub failed: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Start time of the operation
    pub started_at: Instant,
    /// Current spinner frame index
    spinner_frame: usize,
    /// Last spinner update time
    last_spinner_update: Instant,
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new("Progress", "Loading...")
    }
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new(title: impl Into<String>, description: impl Into<String>) -> Self {
        let now = Instant::now();
        Self {
            title: title.into(),
            description: description.into(),
            percentage: None,
            complete: false,
            failed: false,
            error: None,
            started_at: now,
            spinner_frame: 0,
            last_spinner_update: now,
        }
    }

    /// Create an indeterminate progress tracker (spinner only)
    pub fn indeterminate(title: impl Into<String>, description: impl Into<String>) -> Self {
        Self::new(title, description)
    }

    /// Create a progress tracker with initial percentage
    pub fn with_progress(
        title: impl Into<String>,
        description: impl Into<String>,
        percentage: u8,
    ) -> Self {
        let mut tracker = Self::new(title, description);
        tracker.percentage = Some(percentage.min(100));
        tracker
    }

    /// Update the progress percentage
    pub fn set_progress(&mut self, percentage: u8) {
        self.percentage = Some(percentage.min(100));
    }

    /// Update the description
    pub fn set_description(&mut self, description: impl Into<String>) {
        self.description = description.into();
    }

    /// Mark as complete
    pub fn complete(&mut self) {
        self.complete = true;
        self.percentage = Some(100);
    }

    /// Mark as failed with error message
    pub fn fail(&mut self, error: impl Into<String>) {
        self.failed = true;
        self.error = Some(error.into());
    }

    /// Get elapsed time since start
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Get formatted elapsed time string
    pub fn elapsed_string(&self) -> String {
        let elapsed = self.elapsed();
        let secs = elapsed.as_secs();

        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        }
    }

    /// Update spinner animation (call this on tick)
    pub fn tick(&mut self) {
        const SPINNER_INTERVAL: Duration = Duration::from_millis(80);

        if self.last_spinner_update.elapsed() >= SPINNER_INTERVAL {
            self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
            self.last_spinner_update = Instant::now();
        }
    }

    /// Get current spinner character
    pub fn spinner(&self) -> char {
        SPINNER_FRAMES[self.spinner_frame]
    }

    /// Check if operation is still in progress
    pub fn is_active(&self) -> bool {
        !self.complete && !self.failed
    }
}

/// Progress widget for rendering
pub struct ProgressWidget<'a> {
    tracker: &'a ProgressTracker,
    show_elapsed: bool,
    show_percentage: bool,
}

impl<'a> ProgressWidget<'a> {
    /// Create a new progress widget
    pub fn new(tracker: &'a ProgressTracker) -> Self {
        Self {
            tracker,
            show_elapsed: true,
            show_percentage: true,
        }
    }

    /// Hide elapsed time display
    pub fn hide_elapsed(mut self) -> Self {
        self.show_elapsed = false;
        self
    }

    /// Hide percentage display
    pub fn hide_percentage(mut self) -> Self {
        self.show_percentage = false;
        self
    }

    /// Render the progress widget
    pub fn render(self, frame: &mut Frame, area: Rect) {
        // Determine colors based on state
        let (border_color, text_color) = if self.tracker.failed {
            (theme::RED, theme::RED)
        } else if self.tracker.complete {
            (theme::GREEN, theme::GREEN)
        } else {
            (theme::MAUVE, theme::TEXT)
        };

        // Create block with title
        let title = if self.show_elapsed {
            format!(
                " {} ({}) ",
                self.tracker.title,
                self.tracker.elapsed_string()
            )
        } else {
            format!(" {} ", self.tracker.title)
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        // Calculate inner area
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Layout: description at top, progress bar at bottom
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Description
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Progress/Status
            ])
            .split(inner);

        // Render description with spinner
        let status_icon = if self.tracker.failed {
            "✗"
        } else if self.tracker.complete {
            "✓"
        } else {
            &self.tracker.spinner().to_string()
        };

        let description = if let Some(ref error) = self.tracker.error {
            Line::from(vec![
                Span::styled(format!("{} ", status_icon), Style::default().fg(text_color)),
                Span::styled(error.as_str(), Style::default().fg(theme::RED)),
            ])
        } else {
            Line::from(vec![
                Span::styled(format!("{} ", status_icon), Style::default().fg(text_color)),
                Span::styled(&self.tracker.description, Style::default().fg(text_color)),
            ])
        };

        frame.render_widget(Paragraph::new(description), chunks[0]);

        // Render progress bar or status
        if let Some(percentage) = self.tracker.percentage {
            if self.show_percentage {
                let gauge = Gauge::default()
                    .gauge_style(Style::default().fg(text_color).bg(theme::OVERLAY))
                    .percent(percentage as u16)
                    .label(format!("{}%", percentage));

                frame.render_widget(gauge, chunks[2]);
            }
        } else {
            // Indeterminate - show animated bar
            let progress_text = "━".repeat(inner.width.saturating_sub(2) as usize);
            let offset = (self.tracker.spinner_frame * 2) % progress_text.len().max(1);
            let visible_len = inner.width.saturating_sub(4) as usize;

            let bar: String = progress_text
                .chars()
                .cycle()
                .skip(offset)
                .take(visible_len)
                .collect();

            let progress_line = Line::from(Span::styled(
                format!("[{}]", bar),
                Style::default().fg(theme::MAUVE),
            ));

            frame.render_widget(Paragraph::new(progress_line), chunks[2]);
        }
    }
}

/// Compact inline progress indicator
pub struct InlineProgress<'a> {
    tracker: &'a ProgressTracker,
}

impl<'a> InlineProgress<'a> {
    /// Create a new inline progress indicator
    pub fn new(tracker: &'a ProgressTracker) -> Self {
        Self { tracker }
    }

    /// Render as a Line for inline display
    pub fn render(&self) -> Line<'a> {
        let icon = if self.tracker.failed {
            Span::styled("✗ ", Style::default().fg(theme::RED))
        } else if self.tracker.complete {
            Span::styled("✓ ", Style::default().fg(theme::GREEN))
        } else {
            Span::styled(
                format!("{} ", self.tracker.spinner()),
                Style::default().fg(theme::MAUVE),
            )
        };

        let desc = Span::raw(&self.tracker.description);

        let progress = if let Some(pct) = self.tracker.percentage {
            Span::styled(format!(" ({}%)", pct), Style::default().fg(theme::SUBTEXT))
        } else {
            Span::raw("")
        };

        Line::from(vec![icon, desc, progress])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_progress_tracker_new() {
        let tracker = ProgressTracker::new("Test", "Testing...");
        assert_eq!(tracker.title, "Test");
        assert_eq!(tracker.description, "Testing...");
        assert!(tracker.percentage.is_none());
        assert!(!tracker.complete);
        assert!(!tracker.failed);
    }

    #[test]
    fn test_progress_tracker_indeterminate() {
        let tracker = ProgressTracker::indeterminate("Loading", "Please wait");
        assert!(tracker.percentage.is_none());
        assert!(tracker.is_active());
    }

    #[test]
    fn test_progress_tracker_with_progress() {
        let tracker = ProgressTracker::with_progress("Downloading", "File.zip", 50);
        assert_eq!(tracker.percentage, Some(50));
    }

    #[test]
    fn test_progress_tracker_clamps_percentage() {
        let tracker = ProgressTracker::with_progress("Test", "Test", 150);
        assert_eq!(tracker.percentage, Some(100));
    }

    #[test]
    fn test_progress_tracker_set_progress() {
        let mut tracker = ProgressTracker::new("Test", "Test");
        tracker.set_progress(75);
        assert_eq!(tracker.percentage, Some(75));
    }

    #[test]
    fn test_progress_tracker_set_description() {
        let mut tracker = ProgressTracker::new("Test", "Initial");
        tracker.set_description("Updated");
        assert_eq!(tracker.description, "Updated");
    }

    #[test]
    fn test_progress_tracker_complete() {
        let mut tracker = ProgressTracker::new("Test", "Test");
        tracker.complete();
        assert!(tracker.complete);
        assert_eq!(tracker.percentage, Some(100));
        assert!(!tracker.is_active());
    }

    #[test]
    fn test_progress_tracker_fail() {
        let mut tracker = ProgressTracker::new("Test", "Test");
        tracker.fail("Network error");
        assert!(tracker.failed);
        assert_eq!(tracker.error, Some("Network error".to_string()));
        assert!(!tracker.is_active());
    }

    #[test]
    fn test_progress_tracker_elapsed() {
        let tracker = ProgressTracker::new("Test", "Test");
        sleep(Duration::from_millis(10));
        assert!(tracker.elapsed().as_millis() >= 10);
    }

    #[test]
    fn test_progress_tracker_elapsed_string_seconds() {
        let tracker = ProgressTracker::new("Test", "Test");
        // Just started, should be "0s" or "1s"
        let elapsed = tracker.elapsed_string();
        assert!(elapsed.ends_with('s'));
    }

    #[test]
    fn test_progress_tracker_tick() {
        let mut tracker = ProgressTracker::new("Test", "Test");

        // Force a single tick by setting last_spinner_update in the past
        tracker.last_spinner_update = Instant::now() - Duration::from_millis(100);
        let initial_frame = tracker.spinner_frame;
        tracker.tick();

        // Frame should advance by exactly 1
        assert_eq!(
            tracker.spinner_frame,
            (initial_frame + 1) % SPINNER_FRAMES.len()
        );
    }

    #[test]
    fn test_progress_tracker_spinner() {
        let tracker = ProgressTracker::new("Test", "Test");
        let spinner = tracker.spinner();
        assert!(SPINNER_FRAMES.contains(&spinner));
    }

    #[test]
    fn test_progress_tracker_is_active() {
        let mut tracker = ProgressTracker::new("Test", "Test");
        assert!(tracker.is_active());

        tracker.complete();
        assert!(!tracker.is_active());

        let mut tracker2 = ProgressTracker::new("Test", "Test");
        tracker2.fail("Error");
        assert!(!tracker2.is_active());
    }

    #[test]
    fn test_progress_tracker_default() {
        let tracker = ProgressTracker::default();
        assert_eq!(tracker.title, "Progress");
        assert_eq!(tracker.description, "Loading...");
    }

    #[test]
    fn test_progress_tracker_clone() {
        let tracker = ProgressTracker::with_progress("Test", "Desc", 50);
        let cloned = tracker.clone();
        assert_eq!(tracker.title, cloned.title);
        assert_eq!(tracker.percentage, cloned.percentage);
    }

    #[test]
    fn test_progress_widget_new() {
        let tracker = ProgressTracker::new("Test", "Test");
        let widget = ProgressWidget::new(&tracker);
        assert!(widget.show_elapsed);
        assert!(widget.show_percentage);
    }

    #[test]
    fn test_progress_widget_hide_elapsed() {
        let tracker = ProgressTracker::new("Test", "Test");
        let widget = ProgressWidget::new(&tracker).hide_elapsed();
        assert!(!widget.show_elapsed);
    }

    #[test]
    fn test_progress_widget_hide_percentage() {
        let tracker = ProgressTracker::new("Test", "Test");
        let widget = ProgressWidget::new(&tracker).hide_percentage();
        assert!(!widget.show_percentage);
    }

    #[test]
    fn test_inline_progress_new() {
        let tracker = ProgressTracker::new("Test", "Test");
        let _inline = InlineProgress::new(&tracker);
    }

    #[test]
    fn test_inline_progress_render() {
        let tracker = ProgressTracker::with_progress("Test", "Downloading", 50);
        let inline = InlineProgress::new(&tracker);
        let line = inline.render();
        // Should have multiple spans
        assert!(!line.spans.is_empty());
    }

    #[test]
    fn test_inline_progress_complete() {
        let mut tracker = ProgressTracker::new("Test", "Done");
        tracker.complete();
        let inline = InlineProgress::new(&tracker);
        let line = inline.render();
        // First span should contain checkmark
        let first_content = &line.spans[0].content;
        assert!(first_content.contains('✓'));
    }

    #[test]
    fn test_inline_progress_failed() {
        let mut tracker = ProgressTracker::new("Test", "Error");
        tracker.fail("Failed");
        let inline = InlineProgress::new(&tracker);
        let line = inline.render();
        // First span should contain X
        let first_content = &line.spans[0].content;
        assert!(first_content.contains('✗'));
    }
}
