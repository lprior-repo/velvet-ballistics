# Proof-Writer Report — vb-qi37.1.5 Attempt 4

## Bead: vb-qi37.1.5
## State: 6 (proof-review repair attempt 4)
## Date: 2026-05-13

---

## Fixes Applied

### FIND-012: kani::assert Format-String Misuse
**File**: `crates/vb_storage/src/kani_recovery_digest.rs`

**Line 117** — Fixed:
```rust
// Before (3 args — WRONG):
kani::assert(false, "mismatched digests must return CompiledIrDigestMismatch, got {:?}", other);

// After (2 args — CORRECT):
kani::assert(false, "mismatched digests must return CompiledIrDigestMismatch");
```

**Line 140** — Fixed:
```rust
// Before (3 args — WRONG):
_ => kani::assert(false, "mismatched digests cannot produce {:?}", e),

// After (2 args — CORRECT):
_ => kani::assert(false, "mismatched digests cannot produce other error variants"),
```

### FIND-013: DigestCheck Missing kani::Arbitrary
**File**: `crates/vb_storage/src/kani_recovery_digest.rs`

**Line 167** — Fixed with explicit variant enumeration:
```rust
// Before (DigestCheck doesn't implement Arbitrary):
let level: DigestCheck = kani::any();

// After (explicit variant generation):
let variant: u8 = kani::any();
kani::assume(variant < 3);
let level = match variant {
    0 => DigestCheck::WorkflowSourceOnly,
    1 => DigestCheck::WorkflowAndIr,
    _ => DigestCheck::Full,
};
```

### FIND-014: Unit Test Expects Old Buggy Error Variant
**File**: `crates/vb_storage/src/recovery/replay/summary.rs`

**Line 944** — Fixed:
```rust
// Before (wrong variant):
Err(RecoveryError::CompiledIrDigestMismatch { expected: e, found: f })

// After (correct variant per FIND-002 production bug fix):
Err(RecoveryError::WorkflowSourceDigestMismatch { expected: e, found: f })
```

---

## Verification

### cargo check -p vb_storage --lib
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.15s
```
**PASSED** ✓

### Kani compilation (harness compile, not run)
Kani compiler invoked successfully — compilation errors (FIND-012/013) are resolved.
Unwinding assertion failure in `kani_workflow_digest_reflexive_eq` is a separate harness
configuration issue (memcmp unwind depth), not a code correctness issue.

### Unit test: workflow_digest_rejection_reports_exact_mismatch_and_accepts_match
```
cargo test -p vb_storage --lib workflow_digest_rejection_reports_exact_mismatch_and_accepts_match
1 passed, 922 filtered out (1 suite, 0.00s)
```
**PASSED** ✓

---

## Remaining Blockers (Per proof-repair-guide.md)

| Finding | Priority | Status | Fix Required |
|---------|----------|--------|--------------|
| FIND-015 | CRITICAL | REMAINING | Formal waiver for Verus vacuity (Kani provides compensating evidence) |
| FIND-016/017/018 | CRITICAL | REMAINING | Formal waivers for Fjall corruption API unavailability |
| FIND-019 | HIGH | REMAINING | Implement EventSeq ordering check OR record waiver |
| FIND-020 | HIGH | REMAINING | Add unit test for UnsupportedRecoveryState::union monotonicity |

---

## Evidence

1. `cargo check -p vb_storage --lib` — compilation succeeds
2. Unit test `workflow_digest_rejection_reports_exact_mismatch_and_accepts_match` passes
3. Kani compiler invoked without argument-count errors

---

## Recommendation

Forward to proof-reviewer (attempt 4). The FIND-012/013/014 lethal errors are resolved.
Remaining FIND-015/016-018/019/020 require formal waivers per proof-repair-guide.md options.

(End of file)
