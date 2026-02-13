//! AT-5: Update Workflow acceptance tests
//!
//! Tests system update, dry-run, risk assessment, and resume workflows.
//! Covers US-2: As a user, I want to safely update my system
//! with risk assessment and rollback capabilities.
//!
//! NOTE: These tests require actual pacman access and are ignored by default.
//! Run with `cargo test -- --ignored` to execute them on a real system.

use super::fixtures::TestFixture;

/// AT-5.1: User runs `iron update --dry-run` to preview changes
/// Ignored: Requires system pacman access
#[test]
#[ignore]
fn at5_1_update_dry_run() {
    let fixture = TestFixture::with_initialized_state();
    fixture.run_iron(&["update", "--dry-run"]).success();
}

/// AT-5.2: User runs `iron update` to apply updates
/// Ignored: Requires system pacman access
#[test]
#[ignore]
fn at5_2_update_applies_changes() {
    let fixture = TestFixture::with_initialized_state();
    fixture.run_iron(&["update", "--yes"]).success();
}

/// AT-5.3: User runs `iron update` with no changes needed
/// Ignored: Requires system pacman access
#[test]
#[ignore]
fn at5_3_update_no_changes() {
    let fixture = TestFixture::with_initialized_state();
    fixture.run_iron(&["update"]).success();
}

/// AT-5.4: User runs `iron update --status` to check progress
#[test]
fn at5_4_update_status() {
    let fixture = TestFixture::with_initialized_state();
    fixture.run_iron(&["update", "--status"]).success();
}
