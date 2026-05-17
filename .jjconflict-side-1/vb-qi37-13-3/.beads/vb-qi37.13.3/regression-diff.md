# Regression Diff: vb-qi37.13.3

**Bead:** vb-qi37.13.3 — cli: Implement text yaml and postcard emitters
**Baseline:** 336dbd58bfb5d17ccacb75dfb2713e17ac002e46
**Date:** 2026-05-14

---

## Status: N/A

The `emitter.rs` file (and the `vb_ui_model` crate) did not exist at baseline commit `336dbd58`. The emitter module was introduced in a subsequent commit that is part of the vb-qi37.13.3 work.

**Baseline file count:** 2359 files
**Baseline did not include:** `crates/vb_ui_model/src/emitter.rs` — file did not exist

---

## Change Summary

Since the emitter module was introduced after the baseline (not modified from an existing file):

- **No prior version exists** for diff comparison
- The regression-diff would show the entire `emitter.rs` as an addition (+771 lines)
- The bug fix (u64 overflow at line 199) was applied to the newly-introduced file

**Relevant context from STATE.md:**
- Baseline parent: `336dbd58bfb5d17ccacb75dfb2713e17ac002e46`
- The emitter code was introduced as part of vb-qi37.13.3 implementation
- The bug fix at `emitter.rs:199` replaced `unwrap_or(i64::MAX)` with `map_err(|_| EmitterError::YamlEncodeFailed)?`

---

## Bug Fix Delta (emitter.rs:198-201)

**Before (buggy version at time of introduction):**
```rust
} else if let Some(u) = n.as_u64() {
    let val = i64::try_from(u).unwrap_or(i64::MAX);
    Ok(Yaml::Value(Scalar::Integer(val)))
```

**After (fixed version in current workspace):**
```rust
} else if let Some(u) = n.as_u64() {
    Ok(i64::try_from(u)
        .map(|v| Yaml::Value(Scalar::Integer(v)))
        .map_err(|_| EmitterError::YamlEncodeFailed)?)
```

**Impact:** Values of type u64 exceeding i64::MAX (9,223,372,036,854,775,807) now return `Err(EmitterError::YamlEncodeFailed)` instead of silently truncating to i64::MAX.

---

## Files Introduced by vb-qi37.13.3

The emitter module was introduced as part of this bead's implementation. The complete diff is the full emitter.rs file (771 lines) — available at `crates/vb_ui_model/src/emitter.rs` in the current workspace.

**Note:** The emitter.rs file is in an untracked state in the current workspace git context (not committed to the local git history). The code is present in the working directory and passes all machine gates.
