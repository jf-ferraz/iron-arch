//! Config file templates with inline documentation
//!
//! Used by wizards when creating new bundles, profiles, and modules.
//! Each template includes a comment on every field explaining its purpose
//! and valid values, so that non-technical users can understand the config.

/// Generate a `module.toml` template with inline comments.
///
/// # Arguments
/// * `id` – Module identifier (e.g. `"nvim"`)
/// * `description` – Optional human-readable description
/// * `packages` – List of packages to include
pub fn module_toml(id: &str, description: Option<&str>, packages: &[&str]) -> String {
    let desc_line = match description {
        Some(d) if !d.is_empty() => format!("description = \"{}\"\n", d),
        _ => String::new(),
    };

    let packages_list: String = packages.iter().map(|p| format!("  \"{}\",\n", p)).collect();

    format!(
        "# Module: {id}\n\
         # ─────────────────────────────────────────────────────────────────────\n\
         # Iron module configuration. A module represents a single application's\n\
         # configuration (e.g. neovim, kitty, fish). Modules can be independently\n\
         # enabled/disabled without touching the rest of your setup.\n\
         # ─────────────────────────────────────────────────────────────────────\n\
         \n\
         # Unique identifier for this module. Used in profiles and dependencies.\n\
         # Only letters, numbers, hyphens and underscores are allowed.\n\
         id = \"{id}\"\n\
         \n\
         # Human-readable description shown in the TUI and wizard.\n\
         {desc_line}\
         \n\
         # Module kind — controls how Iron categorises this module.\n\
         # Valid values: shell, editor, terminal, wm, theme, utility, service\n\
         kind = \"utility\"\n\
         \n\
         # Packages to install from pacman or AUR when this module is enabled.\n\
         # Example: packages = [\"neovim\", \"tree-sitter\"]\n\
         packages = [\n\
         {packages_list}\
         ]\n\
         \n\
         # AUR packages (requires paru or yay).\n\
         # Example: aur_packages = [\"neovim-git\"]\n\
         aur_packages = []\n\
         \n\
         # Dotfile mappings — source relative to this module's directory,\n\
         # target is the destination path (~ is expanded to $HOME).\n\
         # Example:\n\
         #   [[dotfiles]]\n\
         #   source = \"init.lua\"         # modules/{id}/init.lua\n\
         #   target = \"~/.config/nvim/init.lua\"\n\
         dotfiles = []\n\
         \n\
         # Other modules this module depends on (must be enabled first).\n\
         # Example: depends = [\"base\"]\n\
         depends = []\n\
         \n\
         # Modules that conflict with this one (cannot be active at the same time).\n\
         # Example: conflicts = [\"other-editor\"]\n\
         conflicts = []\n",
        id = id,
        desc_line = desc_line,
        packages_list = packages_list,
    )
}

/// Generate a `profile.toml` template with inline comments.
///
/// # Arguments
/// * `id` – Profile identifier (e.g. `"developer"`)
/// * `description` – Optional human-readable description
/// * `modules` – Module IDs to include in the profile
pub fn profile_toml(id: &str, description: Option<&str>, modules: &[&str]) -> String {
    let desc_line = match description {
        Some(d) if !d.is_empty() => format!("description = \"{}\"\n", d),
        _ => String::new(),
    };

    let modules_list: String = modules.iter().map(|m| format!("  \"{}\",\n", m)).collect();

    format!(
        "# Profile: {id}\n\
         # ─────────────────────────────────────────────────────────────────────\n\
         # Iron profile configuration. A profile is a curated collection of\n\
         # modules. Activate a profile to enable all its modules at once.\n\
         # ─────────────────────────────────────────────────────────────────────\n\
         \n\
         # Unique identifier for this profile.\n\
         id = \"{id}\"\n\
         \n\
         # Human-readable name shown in the TUI.\n\
         name = \"{id}\"\n\
         \n\
         # Optional description shown in the TUI and wizard.\n\
         {desc_line}\
         \n\
         # Modules included in this profile.\n\
         # All listed modules will be enabled when this profile is activated.\n\
         modules = [\n\
         {modules_list}\
         ]\n\
         \n\
         # Optional: inherit modules from another profile.\n\
         # Example: extends = \"base\"\n\
         # extends = \"\"\n\
         \n\
         # Optional: restrict this profile to a specific bundle.\n\
         # Example: for_bundle = \"hyprland\"\n\
         # for_bundle = \"\"\n",
        id = id,
        desc_line = desc_line,
        modules_list = modules_list,
    )
}

/// Generate a `bundle.toml` template with inline comments.
///
/// # Arguments
/// * `id` – Bundle identifier (e.g. `"hyprland"`)
/// * `description` – Optional human-readable description
pub fn bundle_toml(id: &str, description: Option<&str>) -> String {
    let desc_line = match description {
        Some(d) if !d.is_empty() => format!("description = \"{}\"\n", d),
        _ => String::new(),
    };

    format!(
        "# Bundle: {id}\n\
         # ─────────────────────────────────────────────────────────────────────\n\
         # Iron bundle configuration. A bundle represents a complete desktop\n\
         # environment or window manager (e.g. Hyprland, KDE, GNOME). Only one\n\
         # bundle can be active at a time.\n\
         # ─────────────────────────────────────────────────────────────────────\n\
         \n\
         # Unique identifier for this bundle.\n\
         id = \"{id}\"\n\
         \n\
         # Human-readable name shown in the TUI.\n\
         name = \"{id}\"\n\
         \n\
         # Optional description shown in the wizard.\n\
         {desc_line}\
         \n\
         # Bundle type: wm (window manager) or de (desktop environment)\n\
         bundle_type = \"wm\"\n\
         \n\
         # Packages to install when this bundle is activated.\n\
         # Example: packages = [\"hyprland\", \"waybar\", \"dunst\"]\n\
         packages = []\n\
         \n\
         # AUR packages (requires paru or yay).\n\
         aur_packages = []\n\
         \n\
         # Systemd services to enable when this bundle is activated.\n\
         # Example: services = [\"pipewire\", \"pipewire-pulse\"]\n\
         services = []\n\
         \n\
         # Dotfiles directory: place files in bundles/{id}/dotfiles/\n\
         # They will be symlinked to ~/.<relative-path> on activation.\n\
         # (No TOML config needed — files are discovered automatically.)\n",
        id = id,
        desc_line = desc_line,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_toml_basic() {
        let tmpl = module_toml("nvim", Some("Neovim editor"), &["neovim", "tree-sitter"]);
        assert!(tmpl.contains("id = \"nvim\""));
        assert!(tmpl.contains("description = \"Neovim editor\""));
        assert!(tmpl.contains("\"neovim\","));
        assert!(tmpl.contains("\"tree-sitter\","));
    }

    #[test]
    fn test_module_toml_no_description() {
        let tmpl = module_toml("fish", None, &[]);
        assert!(tmpl.contains("id = \"fish\""));
        assert!(!tmpl.contains("description ="));
        assert!(tmpl.contains("packages = [\n]\n"));
    }

    #[test]
    fn test_profile_toml_basic() {
        let tmpl = profile_toml("developer", Some("Dev profile"), &["nvim", "fish"]);
        assert!(tmpl.contains("id = \"developer\""));
        assert!(tmpl.contains("description = \"Dev profile\""));
        assert!(tmpl.contains("\"nvim\","));
    }

    #[test]
    fn test_bundle_toml_basic() {
        let tmpl = bundle_toml("hyprland", Some("Dynamic tiling WM"));
        assert!(tmpl.contains("id = \"hyprland\""));
        assert!(tmpl.contains("description = \"Dynamic tiling WM\""));
        assert!(tmpl.contains("bundle_type = \"wm\""));
    }
}
