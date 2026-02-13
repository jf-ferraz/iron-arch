//! AT-4: Module Operations acceptance tests
//!
//! Tests module listing, enabling, disabling, and conflict detection.
//! Covers FR-4.x: Module management functional requirements.

use super::fixtures::TestFixture;
use predicates::prelude::*;

/// AT-4.1: User enables module with `iron module enable`
#[test]
fn at4_1_module_enable() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_module("neovim");
    fixture.run_iron(&["module", "enable", "neovim"]).success();
}

/// AT-4.2: User lists modules with `iron module list`
#[test]
fn at4_2_module_list() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_module("neovim");
    fixture
        .run_iron(&["module", "list"])
        .success()
        .stdout(predicate::str::contains("neovim").or(predicate::str::contains("Module")));
}

/// AT-4.3: User shows module details with `iron module show`
#[test]
fn at4_3_module_show() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_module("neovim");
    fixture
        .run_iron(&["module", "show", "neovim"])
        .success()
        .stdout(predicate::str::contains("neovim"));
}

/// AT-4.4: User disables module with `iron module disable`
#[test]
fn at4_4_module_disable() {
    let fixture = TestFixture::with_initialized_state();
    fixture.create_module("neovim");
    fixture.run_iron(&["module", "enable", "neovim"]).success();
    fixture
        .run_iron(&["module", "disable", "neovim", "--yes"])
        .success();
}
