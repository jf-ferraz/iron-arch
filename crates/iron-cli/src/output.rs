//! Output Formatting
//!
//! Provides consistent output formatting across all commands.

use crate::cli::OutputFormat;
use iron_core::envelope::IronEnvelope;
use serde::Serialize;
use std::fmt::Display;
use std::time::Instant;

/// Output context for formatting
pub struct Output {
    format: OutputFormat,
    verbose: bool,
    quiet: bool,
    no_color: bool,
    explain: bool,
}

impl Output {
    /// Create new output context
    pub fn new(format: OutputFormat, verbose: bool, quiet: bool, no_color: bool) -> Self {
        Self {
            format,
            verbose,
            quiet,
            no_color,
            explain: false,
        }
    }

    /// Create new output context with explain mode (F0-006)
    pub fn with_explain(mut self, explain: bool) -> Self {
        self.explain = explain;
        self
    }

    /// Print a success message
    pub fn success(&self, msg: &str) {
        if self.quiet {
            return;
        }
        match self.format {
            OutputFormat::Text => {
                if self.no_color {
                    println!("[OK] {}", msg);
                } else {
                    println!("\x1b[32m✓\x1b[0m {}", msg);
                }
            }
            OutputFormat::Json => {
                println!(r#"{{"status":"success","message":"{}"}}"#, msg);
            }
            OutputFormat::Minimal => {}
        }
    }

    /// Print an error message
    pub fn error(&self, msg: &str) {
        match self.format {
            OutputFormat::Text => {
                if self.no_color {
                    eprintln!("[ERROR] {}", msg);
                } else {
                    eprintln!("\x1b[31m✗\x1b[0m {}", msg);
                }
            }
            OutputFormat::Json => {
                eprintln!(r#"{{"status":"error","message":"{}"}}"#, msg);
            }
            OutputFormat::Minimal => {
                eprintln!("error: {}", msg);
            }
        }
    }

    /// Print a warning message
    pub fn warning(&self, msg: &str) {
        if self.quiet {
            return;
        }
        match self.format {
            OutputFormat::Text => {
                if self.no_color {
                    println!("[WARN] {}", msg);
                } else {
                    println!("\x1b[33m⚠\x1b[0m {}", msg);
                }
            }
            OutputFormat::Json => {
                println!(r#"{{"status":"warning","message":"{}"}}"#, msg);
            }
            OutputFormat::Minimal => {}
        }
    }

    /// Print an info message
    pub fn info(&self, msg: &str) {
        if self.quiet {
            return;
        }
        match self.format {
            OutputFormat::Text => {
                if self.no_color {
                    println!("[INFO] {}", msg);
                } else {
                    println!("\x1b[34mℹ\x1b[0m {}", msg);
                }
            }
            OutputFormat::Json => {
                println!(r#"{{"status":"info","message":"{}"}}"#, msg);
            }
            OutputFormat::Minimal => {}
        }
    }

    /// Print verbose details (only in verbose mode)
    pub fn verbose(&self, msg: &str) {
        if !self.verbose || self.quiet {
            return;
        }
        if let OutputFormat::Text = self.format {
            if self.no_color {
                println!("  {}", msg);
            } else {
                println!("  \x1b[90m{}\x1b[0m", msg);
            }
        }
    }

    /// Print a header/title
    pub fn header(&self, title: &str) {
        if self.quiet {
            return;
        }
        if let OutputFormat::Text = self.format {
            if self.no_color {
                println!("\n{}", title);
                println!("{}", "=".repeat(title.len()));
            } else {
                println!("\n\x1b[1;36m{}\x1b[0m", title);
            }
        }
    }

    /// Print a subheader
    pub fn subheader(&self, title: &str) {
        if self.quiet {
            return;
        }
        if let OutputFormat::Text = self.format {
            if self.no_color {
                println!("\n{}", title);
                println!("{}", "-".repeat(title.len()));
            } else {
                println!("\n\x1b[1m{}\x1b[0m", title);
            }
        }
    }

    /// Print a key-value pair
    pub fn kv(&self, key: &str, value: impl Display) {
        if self.quiet {
            return;
        }
        match self.format {
            OutputFormat::Text => {
                if self.no_color {
                    println!("  {}: {}", key, value);
                } else {
                    println!("  \x1b[90m{}:\x1b[0m {}", key, value);
                }
            }
            OutputFormat::Minimal => {
                println!("{}", value);
            }
            _ => {}
        }
    }

    /// Print a list item
    pub fn list_item(&self, item: &str) {
        if self.quiet {
            return;
        }
        match self.format {
            OutputFormat::Text => {
                if self.no_color {
                    println!("  - {}", item);
                } else {
                    println!("  \x1b[90m•\x1b[0m {}", item);
                }
            }
            OutputFormat::Minimal => {
                println!("{}", item);
            }
            _ => {}
        }
    }

    /// Print a list item with status badge
    pub fn list_item_status(&self, item: &str, status: StatusBadge) {
        if self.quiet {
            return;
        }
        match self.format {
            OutputFormat::Text => {
                let badge = status.render(self.no_color);
                println!("  {} {}", badge, item);
            }
            OutputFormat::Minimal => {
                println!("{}", item);
            }
            _ => {}
        }
    }

    /// Print raw text
    pub fn raw(&self, text: &str) {
        println!("{}", text);
    }

    /// Print JSON data
    pub fn json<T: Serialize>(&self, data: &T) {
        if let OutputFormat::Json = self.format
            && let Ok(json) = serde_json::to_string_pretty(data)
        {
            println!("{}", json);
        }
    }

    /// Wrap data in a standard envelope and output as JSON.
    /// Only produces output when format is JSON.
    pub fn json_envelope<T: Serialize>(&self, command: &str, data: T, start_time: Instant) {
        if !self.is_json() {
            return;
        }
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let envelope = IronEnvelope::success(command, data, Some(duration_ms));
        if let Ok(json) = serde_json::to_string_pretty(&envelope) {
            println!("{}", json);
        }
    }

    /// Wrap an error in a standard envelope and output as JSON.
    /// Only produces output when format is JSON.
    #[allow(dead_code)] // Used in future error-path migration
    pub fn json_error_envelope(
        &self,
        command: &str,
        code: &str,
        message: &str,
        start_time: Instant,
    ) {
        if !self.is_json() {
            return;
        }
        let duration_ms = start_time.elapsed().as_millis() as u64;
        let envelope = IronEnvelope::<()>::error(command, code, message, Some(duration_ms));
        if let Ok(json) = serde_json::to_string_pretty(&envelope) {
            eprintln!("{}", json);
        }
    }

    /// Print a table row
    #[allow(dead_code)] // Will be used for CLI table output in Phase 6
    pub fn table_row(&self, cols: &[(&str, usize)]) {
        if self.quiet {
            return;
        }
        match self.format {
            OutputFormat::Text => {
                let row: String = cols
                    .iter()
                    .map(|(val, width)| format!("{:<width$}", val, width = width))
                    .collect::<Vec<_>>()
                    .join(" ");
                println!("{}", row);
            }
            OutputFormat::Minimal => {
                // Print first column only
                if let Some((val, _)) = cols.first() {
                    println!("{}", val);
                }
            }
            _ => {}
        }
    }

    /// Print a separator line
    pub fn separator(&self) {
        if self.quiet {
            return;
        }
        if let OutputFormat::Text = self.format {
            if self.no_color {
                println!("{}", "-".repeat(60));
            } else {
                println!("\x1b[90m{}\x1b[0m", "─".repeat(60));
            }
        }
    }

    /// Check if no-color mode is active
    #[allow(dead_code)]
    pub fn is_no_color(&self) -> bool {
        self.no_color
    }

    /// Apply ANSI color to text, respecting no-color mode
    pub fn colored(&self, text: &str, ansi_code: &str) -> String {
        if self.no_color {
            text.to_string()
        } else {
            format!("{}{}\x1b[0m", ansi_code, text)
        }
    }

    /// Check if JSON output
    pub fn is_json(&self) -> bool {
        matches!(self.format, OutputFormat::Json)
    }

    /// Check if verbose mode
    pub fn is_verbose(&self) -> bool {
        self.verbose
    }

    /// Print an operation summary block (F0-005)
    ///
    /// Renders a final summary line after multi-step operations.
    /// Items: slice of (label, count) pairs. Zero-count items are hidden.
    /// Color: green when no "error"/"fail" items > 0, red otherwise.
    pub fn summary(&self, items: &[(&str, usize)]) {
        if self.quiet {
            return;
        }
        match self.format {
            OutputFormat::Text => {
                let parts: Vec<String> = items
                    .iter()
                    .filter(|(_, count)| *count > 0)
                    .map(|(label, count)| format!("{} {}", count, label))
                    .collect();

                if parts.is_empty() {
                    return;
                }

                let summary_text = parts.join(" · ");
                let has_errors = items.iter().any(|(label, count)| {
                    *count > 0 && (label.contains("error") || label.contains("fail"))
                });

                if self.no_color {
                    let prefix = if has_errors { "[!]" } else { "[=]" };
                    println!("\n  {} Summary: {}", prefix, summary_text);
                } else {
                    let color = if has_errors { "\x1b[31m" } else { "\x1b[32m" };
                    println!("\n  {}▸ Summary:\x1b[0m {}", color, summary_text);
                }
            }
            OutputFormat::Json => {
                let map: serde_json::Value = serde_json::Value::Object(
                    items
                        .iter()
                        .map(|(k, v)| (k.replace(' ', "_"), serde_json::Value::Number((*v).into())))
                        .collect(),
                );
                println!(r#"{{"summary":{}}}"#, map);
            }
            OutputFormat::Minimal => {}
        }
    }

    // ==========================================================================
    // F2-009: Tree-style output renderer
    // ==========================================================================

    /// Print a tree root item
    #[allow(dead_code)]
    pub fn tree_root(&self, label: &str) {
        if self.quiet {
            return;
        }
        if let OutputFormat::Text = self.format {
            println!("{}", label);
        }
    }

    /// Print a tree branch (not last child)
    #[allow(dead_code)]
    pub fn tree_branch(&self, label: &str, depth: usize) {
        if self.quiet {
            return;
        }
        if let OutputFormat::Text = self.format {
            let indent = "│   ".repeat(depth.saturating_sub(1));
            let connector = if depth == 0 { "" } else { "├── " };
            println!("{}{}{}", indent, connector, label);
        }
    }

    /// Print the last child in a tree level
    #[allow(dead_code)]
    pub fn tree_last(&self, label: &str, depth: usize) {
        if self.quiet {
            return;
        }
        if let OutputFormat::Text = self.format {
            let indent = "│   ".repeat(depth.saturating_sub(1));
            let connector = if depth == 0 { "" } else { "└── " };
            println!("{}{}{}", indent, connector, label);
        }
    }

    // ==========================================================================
    // F2-010: Operation summary blocks
    // ==========================================================================

    /// Print a boxed summary block with title and items
    pub fn summary_block(&self, title: &str, items: &[(&str, &str)], duration: Option<f64>) {
        if self.quiet {
            return;
        }
        match self.format {
            OutputFormat::Text => {
                let width = 50;
                if self.no_color {
                    println!("+-{}-+", "-".repeat(width));
                    println!("| {:<width$} |", title);
                    println!("+-{}-+", "-".repeat(width));
                    for (key, value) in items {
                        println!("| {:<20} {:<width$} |", key, value, width = width - 21);
                    }
                    if let Some(dur) = duration {
                        println!("| {:<width$} |", format!("Duration: {:.1}s", dur));
                    }
                    println!("+-{}-+", "-".repeat(width));
                } else {
                    println!("┌─{}─┐", "─".repeat(width));
                    println!("│ \x1b[1m{:<width$}\x1b[0m │", title);
                    println!("├─{}─┤", "─".repeat(width));
                    for (key, value) in items {
                        println!(
                            "│ \x1b[90m{:<20}\x1b[0m {:<width$} │",
                            key,
                            value,
                            width = width - 21
                        );
                    }
                    if let Some(dur) = duration {
                        println!(
                            "│ {:<width$} │",
                            format!("\x1b[90mDuration:\x1b[0m {:.1}s", dur)
                        );
                    }
                    println!("└─{}─┘", "─".repeat(width));
                }
            }
            OutputFormat::Json => {
                let mut map = serde_json::Map::new();
                map.insert("title".into(), serde_json::Value::String(title.into()));
                for (key, value) in items {
                    map.insert(
                        key.to_string(),
                        serde_json::Value::String(value.to_string()),
                    );
                }
                if let Some(dur) = duration {
                    map.insert(
                        "duration_secs".into(),
                        serde_json::Value::Number(
                            serde_json::Number::from_f64(dur).unwrap_or_else(|| 0.into()),
                        ),
                    );
                }
                if let Ok(json) = serde_json::to_string_pretty(&map) {
                    println!("{}", json);
                }
            }
            OutputFormat::Minimal => {}
        }
    }

    // ==========================================================================
    // F2-011: Table output for list commands
    // ==========================================================================

    /// Print a table with headers and rows, auto-sizing columns
    pub fn table(&self, headers: &[&str], rows: &[Vec<String>]) {
        if self.quiet {
            return;
        }
        match self.format {
            OutputFormat::Text => {
                // Calculate column widths
                let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
                for row in rows {
                    for (i, cell) in row.iter().enumerate() {
                        if i < widths.len() {
                            widths[i] = widths[i].max(cell.len());
                        }
                    }
                }

                // Print header
                let header_line: String = headers
                    .iter()
                    .enumerate()
                    .map(|(i, h)| format!("{:<width$}", h, width = widths[i] + 2))
                    .collect();

                if self.no_color {
                    println!("  {}", header_line);
                    let sep: String = widths.iter().map(|w| "-".repeat(w + 2)).collect();
                    println!("  {}", sep);
                } else {
                    println!("  \x1b[1m{}\x1b[0m", header_line);
                    let sep: String = widths.iter().map(|w| "─".repeat(w + 2)).collect();
                    println!("  \x1b[90m{}\x1b[0m", sep);
                }

                // Print rows
                for row in rows {
                    let line: String = row
                        .iter()
                        .enumerate()
                        .map(|(i, cell)| {
                            let w = widths.get(i).copied().unwrap_or(cell.len());
                            format!("{:<width$}", cell, width = w + 2)
                        })
                        .collect();
                    println!("  {}", line);
                }
            }
            OutputFormat::Json => {
                let json_rows: Vec<serde_json::Value> = rows
                    .iter()
                    .map(|row| {
                        let mut map = serde_json::Map::new();
                        for (i, cell) in row.iter().enumerate() {
                            let key = headers.get(i).unwrap_or(&"col");
                            map.insert(key.to_string(), serde_json::Value::String(cell.clone()));
                        }
                        serde_json::Value::Object(map)
                    })
                    .collect();
                if let Ok(json) = serde_json::to_string_pretty(&json_rows) {
                    println!("{}", json);
                }
            }
            OutputFormat::Minimal => {
                for row in rows {
                    if let Some(first) = row.first() {
                        println!("{}", first);
                    }
                }
            }
        }
    }

    // ==========================================================================
    // F2-014: Error messages with suggestions
    // ==========================================================================

    /// Print an error message with a suggestion for recovery
    #[allow(dead_code)]
    pub fn error_with_suggestion(&self, msg: &str, suggestion: &str) {
        self.error(msg);
        if !self.quiet {
            match self.format {
                OutputFormat::Text => {
                    if self.no_color {
                        println!("  Hint: {}", suggestion);
                    } else {
                        println!("  \x1b[90mHint:\x1b[0m {}", suggestion);
                    }
                }
                OutputFormat::Json => {
                    eprintln!(
                        r#"{{"status":"error","message":"{}","suggestion":"{}"}}"#,
                        msg, suggestion
                    );
                }
                OutputFormat::Minimal => {}
            }
        }
    }

    /// Print an explain line showing the command being executed (F0-006)
    ///
    /// Only outputs when `--explain` is active. Shows the underlying system
    /// command so users can learn what Iron does under the hood.
    /// Used by `iron apply --explain` and `iron update --explain` (F2-013).
    #[allow(dead_code)]
    pub fn explain_cmd(&self, cmd: &str) {
        if !self.explain || self.quiet {
            return;
        }
        match self.format {
            OutputFormat::Text => {
                if self.no_color {
                    println!("  -> Running: {}", cmd);
                } else {
                    println!("  \x1b[36m→\x1b[0m \x1b[90m{}\x1b[0m", cmd);
                }
            }
            OutputFormat::Json => {
                println!(r#"{{"command":"{}"}}"#, cmd);
            }
            OutputFormat::Minimal => {}
        }
    }
}

/// Truncate a string to max chars, appending "..." if truncated. UTF-8 safe.
pub fn truncate_str(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(3)).collect();
        format!("{}...", truncated)
    }
}

/// Status badge types
#[derive(Clone, Copy)]
pub enum StatusBadge {
    Ok,
    Warning,
    Error,
    Inactive,
    Partial,
    Active,
    Installed,
    NotInstalled,
    Locked,
    Unlocked,
}

impl StatusBadge {
    /// Render the badge
    pub fn render(self, no_color: bool) -> String {
        if no_color {
            match self {
                StatusBadge::Ok => "[OK]".to_string(),
                StatusBadge::Warning => "[WARN]".to_string(),
                StatusBadge::Error => "[ERR]".to_string(),
                StatusBadge::Inactive => "[OFF]".to_string(),
                StatusBadge::Partial => "[PART]".to_string(),
                StatusBadge::Active => "[ON]".to_string(),
                StatusBadge::Installed => "[INST]".to_string(),
                StatusBadge::NotInstalled => "[--]".to_string(),
                StatusBadge::Locked => "[LOCK]".to_string(),
                StatusBadge::Unlocked => "[OPEN]".to_string(),
            }
        } else {
            match self {
                StatusBadge::Ok => "\x1b[32m●\x1b[0m".to_string(),
                StatusBadge::Warning => "\x1b[33m●\x1b[0m".to_string(),
                StatusBadge::Error => "\x1b[31m●\x1b[0m".to_string(),
                StatusBadge::Inactive => "\x1b[90m○\x1b[0m".to_string(),
                StatusBadge::Partial => "\x1b[33m◐\x1b[0m".to_string(),
                StatusBadge::Active => "\x1b[32m●\x1b[0m".to_string(),
                StatusBadge::Installed => "\x1b[32m✓\x1b[0m".to_string(),
                StatusBadge::NotInstalled => "\x1b[90m-\x1b[0m".to_string(),
                StatusBadge::Locked => "\x1b[33m🔒\x1b[0m".to_string(),
                StatusBadge::Unlocked => "\x1b[32m🔓\x1b[0m".to_string(),
            }
        }
    }
}

/// Risk level rendering
pub fn render_risk(risk: &str, no_color: bool) -> String {
    if no_color {
        format!("[{}]", risk.to_uppercase())
    } else {
        match risk.to_lowercase().as_str() {
            "low" => format!("\x1b[32m{}\x1b[0m", risk),
            "medium" => format!("\x1b[33m{}\x1b[0m", risk),
            "high" => format!("\x1b[31m{}\x1b[0m", risk),
            "critical" => format!("\x1b[1;31m{}\x1b[0m", risk),
            _ => risk.to_string(),
        }
    }
}
