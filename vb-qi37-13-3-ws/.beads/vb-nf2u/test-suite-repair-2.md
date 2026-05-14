STATUS: RED

# State 10 Second Test-Suite Repair: vb-nf2u

## Files changed
- `tests/vb_nf2u_ui_release_acceptance.rs`
- `crates/vb_ui_snapshot/tests/inventory_bijection.rs`
- `crates/vb_ui_snapshot/tests/layout_checks.rs`
- `crates/vb_ui_snapshot/tests/redaction_checks.rs`
- `crates/vb_ui_snapshot/tests/report_evidence_shape.rs`
- `crates/vb_ui_makepad/tests/shell_reachability.rs`
- `xtask/tests/ui_release_errors.rs`
- `fuzz/fuzz_targets/ui_redaction_artifact.rs`
- `crates/vb_ui_snapshot/Cargo.toml`
- `crates/vb_ui_makepad/Cargo.toml`

## Repair summary
- Replaced `include_str!(...).contains(...)` marker tests with behavioral API/command-boundary tests.
- Replaced `UiSnapshotError` hand-constructed Debug camouflage tests with tests that call public fixture, layout, token, image, report, and snapshot operations. Missing public operations are now compile-time RED requirements (`vb_ui_snapshot::snapshot`, report validators, `redaction`, `layout_kernel`).
- Changed false-pass acceptance tests to require `cargo xtask ai-release --bead vb-nf2u` to fail closed instead of succeeding with decorative evidence text.
- Strengthened negative fixture command-boundary tests so changed control IDs, changed rectangle bounds, fixture nonce, and changed secret values must affect evidence/outcome.
- Added property-style tests for inventory, layout overlap, and redaction. Missing scanner/layout kernel APIs are exposed as compile failures instead of hollow test helpers.
- Strengthened shell reachability to assert the complete `ShellNav -> Screen -> REQUIRED_FIXTURES` bijection.
- Updated the fuzz target to call the required redaction scanner API; current fuzz wiring remains RED because the target is not registered.

## Command results

### Format
Command:
```text
rtk cargo fmt --all
```
Result: PASS, no output.

### Acceptance command
Command:
```text
cargo nextest run -p velvet-ballastics-workspace --test vb_nf2u_ui_release_acceptance
```
Result:
```text
8 tests run: 5 passed, 3 failed, 0 skipped
FAIL overlap_negative_fixture_is_consumed_by_command_boundary
FAIL overlap_false_pass_fixture_is_rejected
FAIL secret_false_pass_fixture_is_rejected
```
Meaning: the stronger command-boundary tests are RED. The implementation still ignores changed overlap control IDs/bounds, and false-pass negative fixtures still return process exit code 0.

### Package command
Command:
```text
cargo nextest run -p vb_ui_snapshot -p vb_ui_makepad -p xtask
```
Result: RED at compile time.
Representative diagnostics:
```text
error[E0433]: cannot find `snapshot` in `vb_ui_snapshot`
error[E0425]: cannot find function `validate_required_screens` in module `vb_ui_snapshot::report`
error[E0425]: cannot find function `validate_report_fields` in module `vb_ui_snapshot::report`
error[E0433]: cannot find `redaction` in `vb_ui_snapshot`
error[E0433]: cannot find `layout_kernel` in `vb_ui_snapshot`
```
Meaning: the required behavioral/public APIs from the contract and test plan are absent; tests now expose that absence instead of passing on marker strings.

### Fuzz smoke
Command:
```text
cargo fuzz run ui_redaction_artifact -- -runs=1
```
Result: RED.
```text
error: no bin target named `ui_redaction_artifact` in default-run packages
```
Meaning: the redaction fuzz target is still not wired as a cargo-fuzz bin target.

## Residual evidence assertion note
- Some acceptance assertions still inspect YAML-like evidence text directly because no structured release-evidence parser is exposed for `.evidence/vb-nf2u/*.yaml`. The repaired assertions now require command exit semantics and exact fixture-derived control IDs, rectangle bounds, nonce, status, and false-pass outcomes, so canned duplicated audit text is no longer sufficient for the strengthened negative paths.

## Self-audit
- Source-presence tests removed/replaced in the bead-scoped files.
- False-pass tests require non-zero release outcome.
- `UiSnapshotError` Debug construction theater removed.
- Property/fuzz-style tests require real scanner/layout/inventory kernels or fail compiling.
- Production implementation code was not modified.
