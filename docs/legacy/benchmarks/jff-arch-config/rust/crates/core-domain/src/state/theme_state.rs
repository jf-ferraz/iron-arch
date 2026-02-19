use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

use super::{load_state, save_state};

/// Theme descriptor from themes.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeDescriptor {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub shell: String,
    pub dotfiles_module: Option<String>,
}

/// Active theme state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeState {
    pub theme_id: Option<String>,
    pub last_switched: Option<DateTime<Utc>>,
}

impl Default for ThemeState {
    fn default() -> Self {
        ThemeState {
            theme_id: None,
            last_switched: None,
        }
    }
}

impl ThemeState {
    pub fn load(root: &Path) -> io::Result<ThemeState> {
        let path = root.join("app/state/tracking/active_theme.json");
        match load_state(&path) {
            Ok(state) => Ok(state),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(ThemeState::default()),
            Err(e) => Err(e),
        }
    }

    pub fn save(&self, root: &Path) -> io::Result<()> {
        let path = root.join("app/state/tracking/active_theme.json");
        save_state(&path, self)
    }

    pub fn set_theme(&mut self, theme_id: String) {
        self.theme_id = Some(theme_id);
        self.last_switched = Some(Utc::now());
    }
}
