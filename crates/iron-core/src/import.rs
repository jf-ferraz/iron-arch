//! Import existing dotfiles into Iron modules.
//!
//! The first supported source is a **home-manager** build output. Running
//! `home-manager build` produces a `result` symlink whose `home-files/` tree is a
//! fully-rendered mirror of `$HOME` (every config the Nix expressions generate,
//! e.g. `home-files/.config/kitty/kitty.conf`). This module walks that tree and
//! scaffolds one Iron module per app: it copies the rendered files into
//! `modules/<id>/config/<id>/` (dereferencing the read-only `/nix/store` symlinks)
//! and writes a `module.toml` wiring the dotfile mapping.
//!
//! Package names are a best-effort guess (the `.config/<app>` directory name, with
//! a small alias map) — the generated `module.toml` is meant to be reviewed.

use crate::module::{DotfileMapping, HookBehavior, Module, ModuleKind};
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// A module that would be created from imported dotfiles.
#[derive(Debug, Clone, Serialize)]
pub struct PlannedModule {
    /// Module id (also the directory name under `modules/`).
    pub id: String,
    /// Guessed module classification.
    pub kind: ModuleKind,
    /// Best-effort guessed packages (review before applying).
    pub packages: Vec<String>,
    /// Dotfile target in `$HOME`, e.g. `~/.config/kitty`.
    pub target: String,
    /// Source path inside the module dir, e.g. `config/kitty`.
    pub rel_source: String,
    /// Number of files that would be copied.
    pub file_count: usize,
    /// Whether `modules/<id>/module.toml` already exists.
    pub already_exists: bool,
    /// Absolute path of the source under `home-files` (not serialized).
    #[serde(skip)]
    pub src: PathBuf,
}

/// The full plan produced from a home-manager tree.
#[derive(Debug, Clone, Serialize)]
pub struct ImportPlan {
    /// Modules to create.
    pub modules: Vec<PlannedModule>,
    /// Top-level directories skipped (too broad to auto-modularize in v1).
    pub skipped_dirs: Vec<String>,
}

/// Outcome of executing an [`ImportPlan`].
#[derive(Debug, Clone, Default, Serialize)]
pub struct ImportReport {
    /// Ids of modules created.
    pub created: Vec<String>,
    /// Ids skipped because they already existed (and `force` was false).
    pub skipped: Vec<String>,
    /// Total files copied across all created modules.
    pub files_copied: usize,
}

/// Imports a home-manager `home-files` tree into Iron modules.
#[derive(Debug)]
pub struct HomeManagerImporter {
    home_files: PathBuf,
    modules_dir: PathBuf,
    only: Option<Vec<String>>,
}

impl HomeManagerImporter {
    /// Create an importer.
    ///
    /// `input` may be the `result` symlink, a home-manager generation directory, or
    /// a `home-files/` directory directly. `modules_dir` is where modules are
    /// written (typically `<iron-root>/modules`). `only`, if set, restricts the
    /// import to those app ids.
    pub fn new(input: &Path, modules_dir: &Path, only: Option<Vec<String>>) -> Result<Self> {
        let home_files = resolve_home_files(input)?;
        Ok(Self {
            home_files,
            modules_dir: modules_dir.to_path_buf(),
            only,
        })
    }

    /// The resolved `home-files` root.
    pub fn home_files(&self) -> &Path {
        &self.home_files
    }

    /// Build the import plan without writing anything.
    pub fn plan(&self) -> Result<ImportPlan> {
        let mut modules = Vec::new();
        let mut skipped_dirs = Vec::new();

        // 1) Each child of `.config/` becomes a module.
        let config_dir = self.home_files.join(".config");
        if config_dir.is_dir() {
            for entry in read_dir_sorted(&config_dir)? {
                let name = entry.file_name().to_string_lossy().into_owned();
                let id = sanitize_id(&name);
                if !self.selected(&id) {
                    continue;
                }
                modules.push(self.plan_one(
                    id,
                    entry.path(),
                    format!("~/.config/{name}"),
                    format!("config/{name}"),
                )?);
            }
        }

        // 2) Top-level regular files (e.g. `.gitconfig`, `.profile`) become modules.
        //    Top-level directories other than `.config` are skipped in v1 — they're
        //    too varied to auto-modularize (`.local`, `.mozilla`, …).
        for entry in read_dir_sorted(&self.home_files)? {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == ".config" {
                continue;
            }
            let meta = std::fs::metadata(entry.path())
                .with_context(|| format!("reading {}", entry.path().display()))?;
            if meta.is_dir() {
                skipped_dirs.push(name);
                continue;
            }
            let id = sanitize_id(&name);
            if !self.selected(&id) {
                continue;
            }
            modules.push(self.plan_one(id, entry.path(), format!("~/{name}"), name)?);
        }

        Ok(ImportPlan {
            modules,
            skipped_dirs,
        })
    }

    /// Execute a plan: copy files and write `module.toml` for each module.
    ///
    /// Modules that already exist are skipped unless `force` is true.
    pub fn execute(&self, plan: &ImportPlan, force: bool) -> Result<ImportReport> {
        let mut report = ImportReport::default();
        for m in &plan.modules {
            if m.already_exists && !force {
                report.skipped.push(m.id.clone());
                continue;
            }
            let module_dir = self.modules_dir.join(&m.id);
            let dst = module_dir.join(&m.rel_source);
            let copied = copy_tree(&m.src, &dst)
                .with_context(|| format!("copying files for module '{}'", m.id))?;
            report.files_copied += copied;

            build_module(m)
                .save(&module_dir)
                .with_context(|| format!("writing module.toml for '{}'", m.id))?;
            report.created.push(m.id.clone());
        }
        Ok(report)
    }

    fn plan_one(
        &self,
        id: String,
        src: PathBuf,
        target: String,
        rel_source: String,
    ) -> Result<PlannedModule> {
        let file_count = count_files(&src)?;
        let already_exists = self.modules_dir.join(&id).join("module.toml").exists();
        Ok(PlannedModule {
            kind: guess_kind(&id),
            packages: guess_packages(&id),
            id,
            target,
            rel_source,
            file_count,
            already_exists,
            src,
        })
    }

    fn selected(&self, id: &str) -> bool {
        match &self.only {
            Some(list) => list.iter().any(|x| x == id),
            None => true,
        }
    }
}

/// Resolve the `home-files` root from a user-supplied path.
fn resolve_home_files(input: &Path) -> Result<PathBuf> {
    let nested = input.join("home-files");
    if nested.is_dir() {
        return Ok(nested);
    }
    if input.join(".config").is_dir() {
        return Ok(input.to_path_buf());
    }
    anyhow::bail!(
        "No home-manager files found at '{}'.\n\
         Pass the `result` symlink from `home-manager build`, its generation \
         directory, or a `home-files/` directory (it must contain a `.config/` folder).",
        input.display()
    )
}

/// Build a `Module` from a planned import.
fn build_module(m: &PlannedModule) -> Module {
    Module {
        id: m.id.clone(),
        name: title_case(&m.id),
        description: Some(format!(
            "Imported from home-manager ({} file(s)). Review packages before applying.",
            m.file_count
        )),
        kind: m.kind.clone(),
        packages: m.packages.clone(),
        aur_packages: Vec::new(),
        dotfiles: vec![DotfileMapping {
            source: m.rel_source.clone(),
            target: m.target.clone(),
            link: true,
        }],
        conflicts: Vec::new(),
        depends: Vec::new(),
        pre_install: None,
        post_install: None,
        pre_uninstall: None,
        status_check: None,
        priority: None,
        requires_root: false,
        security_points: 0,
        hook_behavior: HookBehavior::default(),
        dotfiles_sync: false,
        dotfiles_sync_target: None,
    }
}

/// Guess a module kind from its id.
fn guess_kind(id: &str) -> ModuleKind {
    match id {
        "fish" | "bash" | "zsh" | "nushell" | "nu" | "starship" => ModuleKind::Shell,
        _ => ModuleKind::AppConfig,
    }
}

/// Best-effort guess of the package(s) an app needs. The directory name usually
/// matches the Arch package; a small alias map covers common mismatches.
fn guess_packages(id: &str) -> Vec<String> {
    let pkg = match id {
        "nvim" => "neovim",
        "code" | "vscode" => "code",
        "gitconfig" | "git" => "git",
        _ => id,
    };
    vec![pkg.to_string()]
}

/// `kitty-dev` -> `Kitty Dev`.
fn title_case(id: &str) -> String {
    id.split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Sanitize a file/dir name into a module id.
fn sanitize_id(name: &str) -> String {
    name.trim_start_matches('.')
        .chars()
        .map(|c| match c {
            'A'..='Z' => c.to_ascii_lowercase(),
            'a'..='z' | '0'..='9' | '-' => c,
            _ => '-',
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Read a directory's entries sorted by name (deterministic ordering).
fn read_dir_sorted(dir: &Path) -> Result<Vec<std::fs::DirEntry>> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .with_context(|| format!("reading directory {}", dir.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name());
    Ok(entries)
}

/// Count files under `src` (following symlinks, as home-files entries are
/// `/nix/store` symlinks).
fn count_files(src: &Path) -> Result<usize> {
    let meta = std::fs::metadata(src)?;
    if meta.is_dir() {
        let mut n = 0;
        for entry in std::fs::read_dir(src)? {
            n += count_files(&entry?.path())?;
        }
        Ok(n)
    } else {
        Ok(1)
    }
}

/// Recursively copy `src` to `dst`, dereferencing symlinks and making the copies
/// writable (home-manager files are read-only `0444` symlinks into `/nix/store`).
fn copy_tree(src: &Path, dst: &Path) -> Result<usize> {
    let meta = std::fs::metadata(src)?; // follows symlinks
    if meta.is_dir() {
        std::fs::create_dir_all(dst)?;
        let mut n = 0;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            n += copy_tree(&entry.path(), &dst.join(entry.file_name()))?;
        }
        Ok(n)
    } else {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(src, dst)?;
        make_writable(dst)?;
        Ok(1)
    }
}

/// Ensure the owner can write the file (store copies come out read-only).
fn make_writable(path: &Path) -> Result<()> {
    let mut perms = std::fs::metadata(path)?.permissions();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(perms.mode() | 0o200);
    }
    #[cfg(not(unix))]
    {
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
    }
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Build a fake `home-files` tree: `.config/{kitty,fish}` + a top-level
    /// `.gitconfig` and a skipped `.local` dir.
    fn fake_home_files(root: &Path) {
        let cfg = root.join(".config");
        fs::create_dir_all(cfg.join("kitty")).unwrap();
        fs::write(cfg.join("kitty/kitty.conf"), "font_size 12\n").unwrap();
        fs::create_dir_all(cfg.join("fish")).unwrap();
        fs::write(cfg.join("fish/config.fish"), "set -g X 1\n").unwrap();
        fs::write(root.join(".gitconfig"), "[user]\n  name = Fer\n").unwrap();
        fs::create_dir_all(root.join(".local/share/foo")).unwrap();
        fs::write(root.join(".local/share/foo/data"), "x").unwrap();
    }

    #[test]
    fn plans_modules_from_config_and_toplevel_files() {
        let tmp = TempDir::new().unwrap();
        let hf = tmp.path().join("home-files");
        fs::create_dir_all(&hf).unwrap();
        fake_home_files(&hf);
        let modules = tmp.path().join("modules");

        let importer = HomeManagerImporter::new(tmp.path(), &modules, None).unwrap();
        let plan = importer.plan().unwrap();

        let ids: Vec<&str> = plan.modules.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&"kitty"));
        assert!(ids.contains(&"fish"));
        assert!(ids.contains(&"gitconfig"));
        // top-level dir is skipped, not turned into a module
        assert!(plan.skipped_dirs.contains(&".local".to_string()));
        assert!(!ids.contains(&"local"));
    }

    #[test]
    fn guesses_shell_kind_and_packages() {
        let tmp = TempDir::new().unwrap();
        let hf = tmp.path().join("home-files");
        fs::create_dir_all(&hf).unwrap();
        fake_home_files(&hf);
        let importer =
            HomeManagerImporter::new(tmp.path(), &tmp.path().join("modules"), None).unwrap();
        let plan = importer.plan().unwrap();

        let fish = plan.modules.iter().find(|m| m.id == "fish").unwrap();
        assert!(matches!(fish.kind, ModuleKind::Shell));
        assert_eq!(fish.packages, vec!["fish".to_string()]);

        let kitty = plan.modules.iter().find(|m| m.id == "kitty").unwrap();
        assert!(matches!(kitty.kind, ModuleKind::AppConfig));
        assert_eq!(kitty.target, "~/.config/kitty");
        assert_eq!(kitty.rel_source, "config/kitty");
        assert_eq!(kitty.file_count, 1);
    }

    #[test]
    fn executes_and_writes_valid_modules() {
        let tmp = TempDir::new().unwrap();
        let hf = tmp.path().join("home-files");
        fs::create_dir_all(&hf).unwrap();
        fake_home_files(&hf);
        let modules = tmp.path().join("modules");

        let importer = HomeManagerImporter::new(tmp.path(), &modules, None).unwrap();
        let plan = importer.plan().unwrap();
        let report = importer.execute(&plan, false).unwrap();

        assert!(report.created.contains(&"kitty".to_string()));
        assert!(report.files_copied >= 3);

        // module.toml is valid and round-trips through Module::load
        let kitty = Module::load(&modules.join("kitty")).unwrap();
        assert_eq!(kitty.id, "kitty");
        assert_eq!(kitty.dotfiles.len(), 1);
        assert_eq!(kitty.dotfiles[0].target, "~/.config/kitty");
        assert_eq!(kitty.dotfiles[0].source, "config/kitty");

        // the rendered config file was copied and is writable
        let copied = modules.join("kitty/config/kitty/kitty.conf");
        assert_eq!(fs::read_to_string(&copied).unwrap(), "font_size 12\n");
        assert!(!fs::metadata(&copied).unwrap().permissions().readonly());
    }

    #[test]
    fn skips_existing_unless_forced() {
        let tmp = TempDir::new().unwrap();
        let hf = tmp.path().join("home-files");
        fs::create_dir_all(&hf).unwrap();
        fake_home_files(&hf);
        let modules = tmp.path().join("modules");

        let importer = HomeManagerImporter::new(tmp.path(), &modules, None).unwrap();
        importer.execute(&importer.plan().unwrap(), false).unwrap();

        // second run: kitty now exists, so it's skipped without --force
        let plan2 = importer.plan().unwrap();
        assert!(plan2.modules.iter().find(|m| m.id == "kitty").unwrap().already_exists);
        let report2 = importer.execute(&plan2, false).unwrap();
        assert!(report2.skipped.contains(&"kitty".to_string()));
        assert!(!report2.created.contains(&"kitty".to_string()));

        // with force, it's recreated
        let report3 = importer.execute(&importer.plan().unwrap(), true).unwrap();
        assert!(report3.created.contains(&"kitty".to_string()));
    }

    #[test]
    fn only_filter_restricts_modules() {
        let tmp = TempDir::new().unwrap();
        let hf = tmp.path().join("home-files");
        fs::create_dir_all(&hf).unwrap();
        fake_home_files(&hf);
        let importer = HomeManagerImporter::new(
            tmp.path(),
            &tmp.path().join("modules"),
            Some(vec!["kitty".to_string()]),
        )
        .unwrap();
        let plan = importer.plan().unwrap();
        assert_eq!(plan.modules.len(), 1);
        assert_eq!(plan.modules[0].id, "kitty");
    }

    #[test]
    fn resolves_home_files_dir_directly() {
        // input points straight at a dir containing `.config`
        let tmp = TempDir::new().unwrap();
        fake_home_files(tmp.path());
        let importer =
            HomeManagerImporter::new(tmp.path(), &tmp.path().join("modules"), None).unwrap();
        assert!(importer.home_files().join(".config").is_dir());
    }

    #[test]
    fn errors_when_no_home_files() {
        let tmp = TempDir::new().unwrap();
        let err =
            HomeManagerImporter::new(tmp.path(), &tmp.path().join("modules"), None).unwrap_err();
        assert!(err.to_string().contains("No home-manager files"));
    }

    #[test]
    fn sanitize_and_title_helpers() {
        assert_eq!(sanitize_id(".gitconfig"), "gitconfig");
        assert_eq!(sanitize_id("kitty"), "kitty");
        assert_eq!(sanitize_id(".p10k.zsh"), "p10k-zsh");
        assert_eq!(title_case("kitty-dev"), "Kitty Dev");
    }
}
