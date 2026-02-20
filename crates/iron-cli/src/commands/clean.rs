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
    all: bool,
) -> Result<()> {
    require_init(ctx)?;

    let output = &ctx.output;
    let service = DefaultCleanupService::new();

    // Build category list from flags
    let categories: Vec<CleanupCategory> = if all || (!orphans && !cache && !symlinks) {
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
            // Symlinks don't have a dedicated CleanupCategory — handle via
            // category list (UserCache is closest). For backwards compat we
            // include the safe set minus aggressive.
            cats.push(CleanupCategory::UserCache);
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

    Ok(())
}
