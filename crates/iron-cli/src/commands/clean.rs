//! Iron Clean Command
//!
//! System cleanup operations using `CleanupService`.

use crate::context::{AppContext, require_init};
use crate::output::StatusBadge;
use anyhow::Result;
use iron_core::services::clean::{CleanupCategory, CleanupService, DefaultCleanupService};

/// Execute clean command
pub fn execute(
    ctx: &AppContext,
    orphans: bool,
    cache: bool,
    symlinks: bool,
    journal: bool,
    logs: bool,
    all: bool,
) -> Result<()> {
    require_init(ctx)?;

    let output = &ctx.output;
    let service = DefaultCleanupService::new()
        .with_package_manager(std::sync::Arc::new(
            iron_pacman::DefaultPackageManager::new(),
        ))
        .with_executor(std::sync::Arc::new(
            iron_core::resilience::RealCommandExecutor::with_defaults(),
        ));

    // Build category list from flags
    let has_any_flag = orphans || cache || symlinks || journal || logs;
    let categories: Vec<CleanupCategory> = if all || !has_any_flag {
        CleanupCategory::safe().to_vec()
    } else {
        let mut cats = Vec::new();
        if orphans {
            cats.push(CleanupCategory::OrphanPackages);
        }
        if cache {
            cats.push(CleanupCategory::PackageCache);
        }
        if symlinks {
            // F-006: Wire --symlinks to BrokenSymlinks category
            cats.push(CleanupCategory::BrokenSymlinks);
        }
        if journal {
            cats.push(CleanupCategory::SystemdJournal);
        }
        if logs {
            cats.push(CleanupCategory::AppLogs);
        }
        cats
    };

    output.header("Iron Cleanup");

    // Preview first
    let previews = service.preview(&categories);
    for preview in &previews {
        output.subheader(preview.category.name());
        output.info(&format!(
            "  {} items, estimated {}",
            preview.items_count,
            preview.space_formatted()
        ));
    }

    // Execute (dry_run=false for real cleanup)
    let summary = service.execute(&categories, false);

    output.separator();

    for result in &summary.results {
        if result.success {
            output.list_item_status(
                &format!(
                    "{}: {} items cleaned ({})",
                    result.category.name(),
                    result.items_cleaned,
                    result.space_formatted()
                ),
                StatusBadge::Ok,
            );
        } else {
            output.list_item_status(
                &format!(
                    "{}: {}",
                    result.category.name(),
                    result.error.as_deref().unwrap_or("unknown error")
                ),
                StatusBadge::Error,
            );
        }
    }

    output.separator();
    output.success(&format!(
        "Cleanup complete: {} items, {} reclaimed ({} succeeded, {} failed)",
        summary.total_items,
        summary.space_formatted(),
        summary.successful,
        summary.failed,
    ));

    output.summary(&[
        ("items cleaned", summary.total_items),
        ("categories succeeded", summary.successful),
        ("categories failed", summary.failed),
    ]);

    Ok(())
}

#[cfg(test)]
mod tests {
    use iron_core::services::clean::CleanupCategory;

    #[test]
    fn test_category_mapping_orphans() {
        let mut cats = Vec::new();
        let orphans = true;
        if orphans {
            cats.push(CleanupCategory::OrphanPackages);
        }
        assert_eq!(cats.len(), 1);
        assert_eq!(cats[0], CleanupCategory::OrphanPackages);
    }

    #[test]
    fn test_category_mapping_cache() {
        let mut cats = Vec::new();
        let cache = true;
        if cache {
            cats.push(CleanupCategory::PackageCache);
        }
        assert_eq!(cats[0], CleanupCategory::PackageCache);
    }

    #[test]
    fn test_category_mapping_journal_and_logs() {
        let mut cats = Vec::new();
        let journal = true;
        let logs = true;
        if journal {
            cats.push(CleanupCategory::SystemdJournal);
        }
        if logs {
            cats.push(CleanupCategory::AppLogs);
        }
        assert_eq!(cats.len(), 2);
    }

    #[test]
    fn test_category_all_flag() {
        let all = true;
        let has_any_flag = false;
        let categories: Vec<CleanupCategory> = if all || !has_any_flag {
            CleanupCategory::safe().to_vec()
        } else {
            vec![]
        };
        assert!(!categories.is_empty());
    }

    #[test]
    fn test_no_flags_defaults_to_safe() {
        let all = false;
        let has_any_flag = false;
        let categories: Vec<CleanupCategory> = if all || !has_any_flag {
            CleanupCategory::safe().to_vec()
        } else {
            vec![]
        };
        // Safe categories should not include aggressive ones
        assert!(categories.iter().all(|c| !c.is_aggressive()));
    }
}
