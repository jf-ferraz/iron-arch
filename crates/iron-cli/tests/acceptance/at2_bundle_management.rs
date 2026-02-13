//! AT-2: Bundle Management acceptance tests
//!
//! Tests bundle installation, switching, and removal workflows.
//! Covers US-5: As a user, I want to manage desktop environment bundles
//! so that I can switch between different configurations.

use super::fixtures::TestFixture;
use predicates::prelude::*;

/// AT-2.1: User lists bundles with `iron bundle list`
#[test]
fn at2_1_bundle_list() {
    let fixture = TestFixture::with_initialized_state();
    fixture.run_iron(&["bundle", "list"]).success();
}

/// AT-2.2: User shows bundle details with `iron bundle status`
#[test]
fn at2_2_bundle_status() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_bundle("dev-tools");
    fixture
        .run_iron(&["bundle", "status", "dev-tools"])
        .success()
        .stdout(predicate::str::contains("dev-tools"));
}

/// AT-2.3: User installs a bundle with `iron bundle install`
#[test]
fn at2_3_bundle_install() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_bundle("test-bundle");
    fixture
        .run_iron(&["bundle", "install", "test-bundle"])
        .success();
}

/// AT-2.4: User switches bundles with `iron bundle switch`
#[test]
fn at2_4_bundle_switch() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_bundle("first-bundle");
    fixture.create_bundle("second-bundle");
    fixture.run_iron(&["bundle", "install", "first-bundle"]).success();
    fixture
        .run_iron(&["bundle", "switch", "second-bundle"])
        .success();
}

/// AT-2.5: User removes bundle with `iron bundle remove`
#[test]
fn at2_5_bundle_remove() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_bundle("temp-bundle");
    fixture
        .run_iron(&["bundle", "remove", "temp-bundle"])
        .success();
}
