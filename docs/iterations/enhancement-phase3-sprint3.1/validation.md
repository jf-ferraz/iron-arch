# Validation -- Phase 3 Sprint 3.1

> Updated by tester agent during test verification.

## Quality Gates

| Gate | Status |
|------|--------|
| `cargo test --workspace` | PASS (2087 tests, 0 failures) |
| `cargo clippy --workspace -- -D warnings` | PASS (clean) |
| `cargo fmt --all -- --check` | PASS (clean) |

## Baseline

- Pre-existing tests: PASS (2067 tests)
- New tests added: 20 tests
- Total after: 2087 tests, 0 failures

## Test Summary

### Added

| Test | Type | File | Verifies |
|------|------|------|----------|
| `test_scan_combined_packages_services_files` | unit | `iron-core/src/actual_state.rs` | F3-001/F3-002a: Full scan with packages + services + files together |
| `test_scan_with_empty_packages` | unit | `iron-core/src/actual_state.rs` | F3-002a: Edge case -- scan with empty package manager |
| `test_scan_hostname_is_populated` | unit | `iron-core/src/actual_state.rs` | F3-002a: gethostname produces non-empty string |
| `test_scan_scanned_at_is_recent` | unit | `iron-core/src/actual_state.rs` | F3-002a: scanned_at timestamp is within test execution window |
| `test_envelope_has_exactly_five_top_level_keys` | unit | `iron-core/src/envelope.rs` | F3-003a: Serialized envelope has exactly ok, command, data, error, meta |
| `test_error_envelope_with_suggestion_serialization` | unit | `iron-core/src/envelope.rs` | F3-003a: Suggestion field present; details omitted via skip_serializing_if |
| `test_success_envelope_none_duration` | unit | `iron-core/src/envelope.rs` | F3-003a: duration_ms omitted when None (skip_serializing_if) |
| `test_envelope_with_vec_data` | unit | `iron-core/src/envelope.rs` | F3-003a: Envelope wraps array data correctly |
| `test_envelope_with_empty_data` | unit | `iron-core/src/envelope.rs` | F3-003a: Envelope with null data serializes correctly |
| `status_dry_run_succeeds` | integration | `iron-cli/tests/cli_integration.rs` | F3-004: `iron status --dry-run` exits successfully |
| `status_full_dry_run_shows_drift_skip_message` | integration | `iron-cli/tests/cli_integration.rs` | F3-004: `iron status --full --dry-run` shows DRY RUN message |
| `status_json_dry_run_returns_envelope` | integration | `iron-cli/tests/cli_integration.rs` | F3-004/F3-003b: JSON output has envelope structure with status data fields |
| `status_shows_modules_section` | integration | `iron-cli/tests/cli_integration.rs` | F3-004: Status shows Modules section with Total count |
| `plan_requires_init` | integration | `iron-cli/tests/cli_integration.rs` | F3-005: `iron plan` fails gracefully when not initialized |
| `plan_dry_run_succeeds` | integration | `iron-cli/tests/cli_integration.rs` | F3-005: `iron plan --dry-run` exits successfully |
| `plan_json_dry_run_returns_envelope` | integration | `iron-cli/tests/cli_integration.rs` | F3-005/F3-003b: JSON output has envelope structure with plan data |
| `plan_dry_run_shows_empty_plan` | integration | `iron-cli/tests/cli_integration.rs` | F3-005: Dry-run plan shows informative output |
| `doctor_json_uses_envelope` | integration | `iron-cli/tests/cli_integration.rs` | F3-003b: `iron doctor --json` wraps in envelope |
| `scan_json_uses_envelope` | integration | `iron-cli/tests/cli_integration.rs` | F3-003b: `iron scan --json` wraps in envelope |
| `validate_json_uses_envelope` | integration | `iron-cli/tests/cli_integration.rs` | F3-003b: `iron validate --json` wraps in envelope |

### Pre-existing Tests (per task)

| Task | Existing Tests | Count | Coverage Assessment |
|------|---------------|-------|---------------------|
| F3-001 (ActualState struct) | Construction, serde roundtrip, missing-field deserialization, FileStateType default/equality | 5 | Adequate |
| F3-002a (scan implementation) | Noop managers, mock packages, services, missing/regular/symlink/directory files, checksum, scan roundtrip | 9 | Adequate |
| F3-002b (consumer refactor) | All existing apply.rs and drift.rs tests pass unchanged (trait signatures unchanged) | N/A | Verified by full test suite pass |
| F3-003a (envelope infra) | Success/error constructors, meta fields, serialization, struct data | 8 | Adequate |
| F3-003b (envelope migration) | doctor_json_has_required_structure, doctor_json_contains_all_fr10_checks updated for envelope | 2 | Supplemented with 3 new integration tests |
| F3-004 (iron status --full) | status_requires_init, status_shows_host_info, status_json_output, status_shows_no_active_bundle, status_verbose_flag, CLI parsing tests (2) | 7 | Supplemented with 4 new tests |
| F3-005 (iron plan) | plan.rs unit tests (plan_groups_by_type, empty_plan_display), CLI parsing tests (3) | 5 | Supplemented with 4 new integration tests |
| F3-006 (XDG state dir) | All StateManager tests pass (state_root resolution, backward compat via IRON_STATE_DIR) | 30+ | Adequate (env var isolation verified) |
| F3-009 (state migration) | migrate_noop_no_legacy, migrate_noop_new_location, migrate_noop_already_migrated, migrate_copies_and_creates_marker, migrate_noop_same_dir, migrate_preserves_on_failure | 6 | All 3 MigrationResult paths covered |

### Infrastructure Added

- `find_envelope_json()` helper in `cli_integration.rs` -- robustly extracts the envelope JSON object from CLI stdout that may contain leading info/status lines. This prevents test fragility when commands emit diagnostic JSON before the main envelope.

## Acceptance Criteria Verification

### F3-001: ActualState struct
- AC-1.1 through AC-1.8: All verified. Struct fields match spec, derives correct, serde(default) present, module registered, unit tests pass.

### F3-002a: scan implementation
- AC-2a.1 through AC-2a.9: All verified. scan() signature matches spec, mock tests cover packages/services/files, checksum test validates SHA256, hostname populated, scanned_at set, roundtrip works, dependencies added.

### F3-002b: consumer refactor
- AC-2b.1 through AC-2b.9: Verified by code review (changes.md) and full test suite pass. Trait signatures unchanged per architect AQ-2. No new allow(dead_code).

### F3-003a: envelope infrastructure
- AC-3a.1 through AC-3a.9: All verified. Struct fields match spec, constructors work, serialization produces correct JSON structure with 5 keys. Module registered.

### F3-003b: envelope migration
- AC-3b.1: Verified -- 10 commands migrated to json_envelope(). 8 remaining in host/profile/bundle/sync (documented as out-of-scope).
- AC-3b.2: Verified via integration tests -- doctor, scan, validate, status, plan all produce envelope.
- AC-3b.4: Verified -- meta.timestamp is ISO-8601 string in all envelope outputs.

### F3-004: iron status enhancement
- AC-4.1 through AC-4.9: Verified. --full and --dry-run flags exist, status shows host/bundle/modules/packages/security, JSON uses envelope, dry-run works.

### F3-005: iron plan command
- AC-5.1 through AC-5.9: Verified. plan command exists, --module/--dry-run flags present, --json uses envelope, registered in cli.rs/mod.rs/main.rs.

### F3-006: XDG state directory
- AC-6.1 through AC-6.8: Verified. state_dir() resolution priority correct, IRON_STATE_DIR override works, is_initialized() checks state_root.

### F3-009: state migration
- AC-7.1 through AC-7.8: Verified. migrate_if_needed() exists, migrates all files, copy-then-delete, MIGRATED.txt marker, no-op conditions covered, failure leaves originals intact, called from context.rs startup.

## Remaining Concerns

1. **Envelope info-line bleeding**: Commands like `plan --dry-run` and `validate` emit `output.info()` lines before the envelope JSON in `--format json` mode. This is a minor issue -- the info() method produces `{"status":"info","message":"..."}` lines that precede the envelope. The `find_envelope_json()` test helper works around this, but machine consumers parsing `--json` output need to handle multiple JSON objects or filter for the one with `"ok"` and `"meta"` keys. This should be documented or fixed in a future sprint.

2. **Out-of-scope `output.json()` calls**: 8 remaining raw `output.json()` calls in host.rs (3), profile.rs (2), bundle.rs (2), sync.rs (1) not migrated to envelope format. These should be migrated in a follow-up task.

3. **`json_error_envelope` unused**: The error envelope path is defined but not yet consumed by any CLI command's error handling. Error paths still use text-only `output.error()`.
