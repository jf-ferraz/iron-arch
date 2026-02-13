//! Acceptance Test Suite for Iron CLI
//!
//! This test suite validates the complete user workflows defined in the
//! requirements specification. Each test module covers a specific user story
//! or functional area.
//!
//! Test Structure:
//! - AT-1: First-Time Setup (US-1)
//! - AT-2: Bundle Management (US-5)
//! - AT-3: Profile Management (US-6)
//! - AT-4: Module Operations (FR-4.x)
//! - AT-5: Update Workflow (US-2)
//! - AT-6: Recovery Workflow (US-4)

mod at1_first_time_setup;
mod at2_bundle_management;
mod at3_profile_management;
mod at4_module_operations;
mod at5_update_workflow;
mod at6_recovery_workflow;

// Common test fixture module
mod fixtures;

// Re-export the test fixture for use in test modules
pub use fixtures::TestFixture;
