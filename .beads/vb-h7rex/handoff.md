# vb-h7rex Root Compile Fix — Handoff

## Completed

**Edit applied** to `crates/workspace_tests/tests/vb_qi37_4_2_strict_runtime_admission.rs`:

Replaced the problematic match branch at line 1335:
```rust
// Before (E0069: return; in non-() return type)
let admission = match result {
    Ok(a) => a,
    Err(e) => {
        prop_assert!(false, "expected admission, got error: {e:?}");
        return;
    }
};

// After
let admission = result.map_err(|error| TestCaseError::fail(format!("expected admission, got error: {error:?}")))?;
```

The fix converts the match/error/prop_assert/return pattern to proper proptest `?` operator usage with `TestCaseError::fail`, which is the idiomatic way to fail a proptest case from a Result.

## Compile Status

The original compile error (E0069) should now be resolved. Run the full compile verification:

```bash
cd /home/lewis/src/isoloated/vb-policy-fixture-v21
cargo test -p velvet-ballistics-workspace-tests --test vb_qi37_4_2_strict_runtime_admission -- --nocapture
```

Or the full root compile test suite. The original error message from `.beads/vb-h7rex/evidence/ROOT_V21_FIXTURE.stderr` is preserved for reference.

## No Other Changes

- All 7 field asserts in the proptest branch are preserved
- All other mutation branches (1, 2, 3, 4, 5, 6+) are unchanged
- No generator changes
- No VCS operations
- No test execution — compile verification is the next step