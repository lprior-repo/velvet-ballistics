# Manual QA Smoke Report — bead `vb-yd5x`

## Files Reviewed
- `.beads/vb-yd5x/contract.md` — **NOT FOUND**
- `.beads/vb-yd5x/test-plan.md` — **NOT FOUND**
- `.beads/vb-yd5x/red-phase.md` — **NOT FOUND**

## Command Executed

```bash
rtk cargo test -p vb_compile vb_yd5x
```

## Raw Output

```
warning: /home/lewis/src/Velvet-ballistics/crates/velvet_ballastics/Cargo.toml: file `/home/lewis/src/Velvet-ballistics/crates/velvet_ballastics/src/main.rs` found to be present in multiple build targets:
  * `bin` target `vb`
  * `bin` target `velvet-ballistics`
warning: skipping duplicate package `bitflags v2.10.0 (https://github.com/makepad/makepad?branch=dev#20b6c53b)`:

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.05s
     Running unittests src/lib.rs (target/debug/deps/vb_compile-38c5b9c4cd2d5577)
---- schema::tests::vb_yd5x_malformed_loop_body_fails_consistently stdout ----

thread 'schema::tests::vb_yd5x_malformed_loop_body_fails_consistently' (2189118) panicked at crates/vb_compile/src/schema.rs:683:9:
assertion `left == right` failed: expected LOOP_BODY_STEP_OUT_OF_RANGE
  left: Some("INVALID_THEN_TARGET")
 right: Some("LOOP_BODY_STEP_OUT_OF_RANGE")

---- schema::tests::vb_yd5x_malformed_slot_ref_fails_consistently stdout ----

thread 'schema::tests::vb_yd5x_malformed_slot_ref_fails_consistently' (2189119) panicked at crates/vb_compile/src/schema.rs:664:9:
compile should fail for bad slot ref

---- schema::tests::vb_yd5x_diagnostic_codes_remain_stable stdout ----

thread 'schema::tests::vb_yd5x_diagnostic_codes_remain_stable' (2189116) panicked at crates/vb_compile/src/schema.rs:727:13:
assertion `left == right` failed: expected SLOT_REFERENCE_OUT_OF_RANGE
  left: None
 right: Some("SLOT_REFERENCE_OUT_OF_RANGE")

error: test failed, to rerun pass `-p vb_compile --lib`
test result: FAILED. 3 passed; 3 failed; 0 ignored; 0 measured; 233 filtered out; finished in 0.00s
```

## Test Summary

| Test | Result |
|------|--------|
| `vb_yd5x_valid_workflow_passes_both_paths` | PASS |
| `vb_yd5x_malformed_duplicate_id_fails_consistently` | PASS |
| `vb_yd5x_malformed_unknown_ref_fails_consistently` | PASS |
| `vb_yd5x_diagnostic_codes_remain_stable` | FAIL |
| `vb_yd5x_malformed_loop_body_fails_consistently` | FAIL |
| `vb_yd5x_malformed_slot_ref_fails_consistently` | FAIL |

## Findings

**3 tests FAIL** — diagnostic code assertions do not match expected values.

### FAIL 1: `vb_yd5x_malformed_slot_ref_fails_consistently`
- **File**: `crates/vb_compile/src/schema.rs:664`
- **Expected**: slot ref error code `SLOT_REFERENCE_OUT_OF_RANGE`
- **Actual**: test assertion triggered with message "compile should fail for bad slot ref"
- **Actual code returned**: `None`

### FAIL 2: `vb_yd5x_malformed_loop_body_fails_consistently`
- **File**: `crates/vb_compile/src/schema.rs:683`
- **Expected**: error code `LOOP_BODY_STEP_OUT_OF_RANGE`
- **Actual**: `INVALID_THEN_TARGET`

### FAIL 3: `vb_yd5x_diagnostic_codes_remain_stable`
- **File**: `crates/vb_compile/src/schema.rs:727`
- **Expected**: `SLOT_REFERENCE_OUT_OF_RANGE`
- **Actual**: `None`

## Artifact

`.beads/vb-yd5x/manual-qa-smoke.md`

## STATUS: FAIL
