//! AT-6: Recovery Workflow acceptance tests
//!
//! Tests export, script generation, and import workflows.
//! Covers US-4: As a user, I want to recover my system configuration
//! in case of failure or when setting up a new machine.

use super::fixtures::TestFixture;
use predicates::prelude::*;

/// AT-6.1: User runs `iron recover` -> shows available recovery options
#[test]
fn at6_1_recover_shows_options() {
    let fixture = TestFixture::with_initialized_state();
    fixture
        .run_iron(&["recover"])
        .success()
        .stdout(predicate::str::contains("ecovery").or(predicate::str::contains("xport")));
}

/// AT-6.2: User runs `iron recover --export` -> initiates state export
#[test]
fn at6_2_recover_export() {
    let fixture = TestFixture::with_initialized_state();
    fixture.run_iron(&["recover", "--export"]).success();
}

/// AT-6.3: User runs `iron doctor` after init -> shows health status
#[test]
fn at6_3_doctor_shows_health() {
    let fixture = TestFixture::with_initialized_state();
    fixture
        .run_iron(&["doctor"])
        .success()
        .stdout(predicate::str::contains("Health").or(predicate::str::contains("Check")));
}
