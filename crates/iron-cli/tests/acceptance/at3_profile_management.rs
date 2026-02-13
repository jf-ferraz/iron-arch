//! AT-3: Profile Management acceptance tests
//!
//! Tests profile listing, selection, and inheritance workflows.
//! Covers US-6: As a user, I want to manage configuration profiles
//! so that I can switch between different setups.

use super::fixtures::TestFixture;
use predicates::prelude::*;

/// AT-3.1: User creates a profile with `iron profile create`
#[test]
fn at3_1_profile_create() {
    let fixture = TestFixture::with_initialized_state();
    fixture
        .run_iron(&["profile", "create", "workstation"])
        .success()
        .stdout(predicate::str::contains("reated").or(predicate::str::contains("rofile")));
}

/// AT-3.2: User lists profiles with `iron profile list`
#[test]
fn at3_2_profile_list() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_profile("workstation");
    fixture
        .run_iron(&["profile", "list"])
        .success()
        .stdout(predicate::str::contains("workstation"));
}

/// AT-3.3: User shows profile details
#[test]
fn at3_3_profile_show() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_profile("workstation");
    fixture
        .run_iron(&["profile", "show", "workstation"])
        .success()
        .stdout(predicate::str::contains("workstation"));
}

/// AT-3.4: User selects/activates a profile
#[test]
fn at3_4_profile_select() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_profile("workstation");
    fixture
        .run_iron(&["profile", "select", "workstation"])
        .success();
}
