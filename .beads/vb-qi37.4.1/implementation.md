# Implementation: vb-qi37.4.1 — State 6 Repair

## Issue
`AcceptedArtifact` envelope missing `version` field — violates acceptance criteria ("versioned").

## Changes Made

### File: `crates/vb_storage/src/admission.rs`

1. **Added `version: u8` field to `AcceptedArtifact` struct** (line 75):
   ```rust
   pub struct AcceptedArtifact {
       /// Schema version for forward compatibility.
       pub version: u8,
       // ... rest of fields
   }
   ```

2. **Added `ACCEPTED_ARTIFACT_VERSION` constant** (line 92):
   ```rust
   const ACCEPTED_ARTIFACT_VERSION: u8 = 1;
   ```

3. **Added validation to reject `gate_count == 0` under non-Relaxed policies** (lines 138-140):
   ```rust
   if gate_count == 0 && !matches!(policy, vb_core::RuntimePolicy::Relaxed) {
       return Err(JournalError::ArtifactMalformed);
   }
   ```

4. **Updated `AcceptedArtifact` construction** (lines 143-150) to include `version` field.

## Verification

- No forbidden constructs used (`unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`)
- Binary serialization via `postcard` preserved (struct derives `serde::Serialize, serde::Deserialize`)
- `version` field is `u8` as required
- Validation check enforces policy/gate_count consistency

## Pre-existing Build Blocker

The `vb_core` crate has a pre-existing compilation error at `crates/vb_core/src/workflow/mod.rs:745`:
- `budget_error_detail` function does not cover `BudgetError::Overflow` and `BudgetError::Underflow` variants
- This error is unrelated to this repair and prevents test execution

## Evidence

```bash
# Changes verified via git diff:
$ rtk git diff crates/vb_storage/src/admission.rs
+10 -0 lines changed

# The validation ensures:
# - Relaxed policy: gate_count = 0 is allowed
# - Journaled/Strict policies: gate_count = 0 is rejected
```
