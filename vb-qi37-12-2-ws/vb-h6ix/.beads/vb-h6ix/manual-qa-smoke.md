# QA Report — vb-h6ix: "runtime/recovery: Replay latest execution attempt only"

## State 7 — Manual QA Smoke Test

---

## Execution Evidence

### Test 1: cargo test -p vb_storage -- vb_h6ix

```
error: could not compile `vb_storage` (lib test) due to 290 previous errors
```

Full compilation failure — **0 tests ran**.

### Test 2: cargo clippy -p vb_storage --all-targets --all-features -- -D warnings

```
298 errors, 3 warnings
```

All 290 errors are of variant `E0063`: missing field `attempt` in initializer of `JournalEvent`.

---

## Phase Results

### Phase 1 — Discovery
[PASS] Binary/librarary compiles (lib compiles; test target fails)
[FAIL] Tests compile — **CRITICAL**

### Phase 2 — Happy Path
[FAIL] **0 tests executed** — compilation blocked by 290 E0063 errors

### Phase 3 — Hostile Interrogation
[N/A] Compilation failed — no execution possible

---

## Findings

### CRITICAL (block merge)

**Title:** `JournalEvent` struct field `attempt` added but test files not updated

**Evidence:**
```
error[E0063]: missing field `attempt` in initializer of `events::JournalEvent`
  --> crates/vb_storage/src/recovery/tests.rs:460:32
  --> crates/vb_storage/src/recovery/tests.rs:657:27
  ... (290 total occurrences across tests.rs, recovery/tests.rs,
       recovery/vb_h6ix_tests.rs, security_tests.rs, batch.rs,
       codec.rs, trimming.rs, vb_2bok_durability_gate_tests.rs)
```

**Root Cause:** The `vb_h6ix` bead added a required `attempt` field to `JournalEvent`
variants. All 290+ test constructors throughout `vb_storage` were not updated.

**Impact:** `vb_storage` test target is unbuildable. All downstream tests are blocked.

**Affected files (290 E0063 errors across):**
- `crates/vb_storage/src/recovery/tests.rs`
- `crates/vb_storage/src/recovery/vb_h6ix_tests.rs`
- `crates/vb_storage/src/security_tests.rs`
- `crates/vb_storage/src/tests.rs`
- `crates/vb_storage/src/batch.rs`
- `crates/vb_storage/src/codec.rs`
- `crates/vb_storage/src/trimming.rs`
- `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs`

**Recommended Fix:** Update all `JournalEvent` constructors in tests to include the
`attempt` field. The `vb_h6ix` implementation must also provide backward-compatible
constructors or a test harness builder that auto-populates `attempt` for existing tests.

---

## Auto-fixes Applied

None — compilation failure must be resolved before any tests can run.

---

## Beads Filed

- BEAD-TODO: File a bead for the `JournalEvent::attempt` field migration — all 290
  test constructors need the field added. Assign to implementer of vb-h6ix.

---

## VERDICT: FAIL

**Reason:** 290 E0063 compilation errors block all test execution. The `attempt` field
added by vb-h6ix to `JournalEvent` is not propagated to any test constructor. No smoke
tests can be executed.

**Blocking:** YES — this bead cannot be merged until all `JournalEvent` constructors
in the test targets are updated to include the `attempt` field.
