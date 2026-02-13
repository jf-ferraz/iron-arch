//! Output Formatting
//!
//! Provides consistent output formatting across all commands.

use crate::cli::OutputFormat;
use serde::Serialize;
use std::fmt::Display;

/// Output context for formatting
pub struct Output {
    format: OutputFormat,
    verbose: bool,
    quiet: bool,
    no_color: bool,
}

impl Output {
    /// Create new output context
    pub fn new(format: OutputFormat, verbose: bool, quiet: bool, no_color: bool) -> Self {
        Self {
            format,
            verbose,
            quiet,
            no_color,
        }
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
            && let Ok(json) = serde_json::to_string_pretty(data) {
                println!("{}", json);
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

    /// Check if JSON output
    pub fn is_json(&self) -> bool {
        matches!(self.format, OutputFormat::Json)
    }

    /// Check if verbose mode
    pub fn is_verbose(&self) -> bool {
        self.verbose
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
