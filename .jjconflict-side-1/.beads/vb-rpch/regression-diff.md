# Regression Diff — vb-rpch State 13

**Bead**: vb-rpch
**Date**: 2026-05-19
**State**: 13 (LETHAL fix attempt)

---

## Summary

No production code changes. Only test file modifications and documentation artifact creation.

---

## Files Changed

### crates/vb_storage/tests/recovery_bdd_tests.rs

**Changes**:
1. Lines 301-315: Added frame validation assertions to `snapshot_plus_tail_applies_tail_after_watermark`
2. Lines 1928-end: Added 35 new tests

**Diff (LETHAL-1 fix)**:
```diff
-    assert!(
-        result.is_ok(),
-        "hydrate_run_frame should succeed when tail events are after snapshot seq: {result:?}"
-    );
+    let frame = result.expect("hydrate_run_frame should succeed when tail events are after snapshot seq");
+    assert_eq!(
+        frame.pc(),
+        StepIdx::new(1),
+        "PC must advance to step 1 after tail StepStarted"
+    );
+    assert_eq!(
+        frame.step_count(),
+        1,
+        "step_count must reflect tail events"
+    );
```

**Diff (LETHAL-2 fix)**: +35 tests added at end of file (see full file for details)

### .beads/vb-rpch/formal-waivers.jsonl

**Status**: CREATED (new file)

### .beads/vb-rpch/formal-verification-report.md

**Status**: CREATED (new file)

### .beads/vb-rpch/verification-ledger.jsonl

**Status**: CREATED (new file)

### .beads/vb-rpch/black-hat-review.md

**Status**: CREATED (new file)

### .beads/vb-rpch/machine-gate-report.md

**Status**: CREATED (new file)

### .beads/vb-rpch/regression-diff.md

**Status**: CREATED (this file)

---

## Production Code Impact

**NONE** — No changes to:
- `crates/vb_storage/src/recovery/`
- `crates/vb_core/`
- `crates/vb_runtime/`
- `crates/vb_compile/`
- Any runtime-critical paths

---

## Test Code Impact

**ADDITIVE ONLY** — No existing tests modified (except LETHAL-1 fix which strengthens assertions).

---

## Risk Assessment

- **Regressions**: LOW — No production code changes
- **Test Coverage**: INCREASED — 35 new tests added
- **Breaking Changes**: NONE

---

## Recommendation

**APPROVED FOR LANDING** — No risk of regression to production code.

---

*Regression Diff: CLEAN*
*Generated: 2026-05-19*
