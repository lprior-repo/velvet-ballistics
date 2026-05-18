# Test Review — Round 2 (C1-C25) Suite Inquisition

**Files Reviewed:**
1. `crates/vb_runtime/src/engine/property_tests.rs` (240 lines)
2. `fuzz/fuzz_targets.rs` + `fuzz/src/lib.rs` (fuzz targets: `generated_compare`, `compiled_ir`, `ipc_frame`, `expression`)
3. `crates/vb_runtime/src/shard/directive.rs` (460 lines)

**Tests Run:**
- `cargo test -p vb_runtime -- property_tests` → **19 passed**
- `cargo test -p vb_runtime -- shard` → **471 passed**

---

## VERDICT: REJECTED

---

### Tier 0 — Static Analysis

**[PASS]** Banned pattern scan — No `assert!(result.is_ok())` / `assert!(result.is_err())` found in the reviewed files. The `assert!(result.is_ok())` at `fuzz/src/lib.rs:54` is inside a three-way conditional branch (`if empty → Err`, `else if invalid → Err`, `else → Ok`) and is not a standalone weak assertion.

**[FAIL]** Silent error suppression in `fuzz_ipc_frame`:
- `fuzz/src/lib.rs:208` — `let _ = decoded_header.payload_len;` — discards header field, no assertion
- `fuzz/src/lib.rs:220` — `let _ = decode_frame_payload(&header, payload);` — discards payload decode result, no assertion

These are not "weak assertions" — they are **no assertions at all**. The comment says "Just verify it returns without panicking" — panicking is already prevented by returning `Result`; the `let _` adds zero verification value.

**[PASS]** Determinism/evidence scan — No `static mut`, `lazy_static!`, `once_cell.*Mutex` found.

**[PASS]** Mock interrogation — No `mockall` patterns found in the reviewed files.

**[PASS]** Integration test purity — N/A for unit-test files reviewed.

**[PASS]** Error variant completeness — `ShardDirective` has 6 variants fully covered. `RuntimeEngineError` tested for its 3 referenced variants.

**[PASS]** Density audit — 19 property tests covering 4 main types (`EvidenceCollector`, `RetryPolicy`, `RuntimeEngineError`, `RuntimeSignal`, `EvidenceEvent`). Ratio is adequate for unit-level invariant testing.

---

### Tier 1 — Execution

**[PASS]** Test compile: all targets compile cleanly.

**[PASS]** Tests pass: 19 property_tests + 471 shard tests all passing.

**[SKIP]** Ordering probe — not run due to Tier 0 FAIL.

**[SKIP]** Insta — not applicable in reviewed files.

---

### Tier 2 — Coverage

**[SKIP]** Not run due to Tier 0 FAIL.

---

### Tier 3 — Mutation

**[SKIP]** Not run due to Tier 0 FAIL.

---

## LETHAL FINDINGS

### `fuzz/src/lib.rs:208,220` — `fuzz_ipc_frame` asserts nothing

```rust
// Line 208
let _ = decoded_header.payload_len;

// Line 220
let _ = decode_frame_payload(&header, payload);
```

**Problem:** `decode_frame_payload` returns `Result<...>` but the result is immediately discarded. This means:
- A corrupt payload that returns `Err(FrameError::MalformedPayload)` is silently ignored.
- A payload that returns `Ok(Frame { ... })` with garbage inside is silently ignored.
- The fuzz target exercises zero code paths that verify correctness — it only confirms the decoders don't panic.

**What the comment says:** "Just verify it returns without panicking." This is a fundamental misunderstanding of fuzzing. The `?` operator / `Result` return type already guarantees no panic on the happy path. Discarding the result adds no coverage; it adds a warm feeling that a test is running.

**REQUIRED FIX:** Replace `let _ = decode_frame_payload(...)` with an actual assertion on the decoded result. For example:
- Assert `header.payload_len` matches the actual decoded payload length
- Assert that when header says `version = 2`, the payload decode also succeeds (or fails with the correct error)
- Assert something about the decoded frame's structure

The current form is **silent error suppression** per SKILL.md Tier 0 rules: `grep -rn "let _ = \|\.ok()\s*;" src/ tests/` is a Tier 0 banned pattern check.

---

## MAJOR FINDINGS (0)

None.

---

## MINOR FINDINGS (2)

1. **`fuzz/src/lib.rs:192–223` — `fuzz_ipc_frame` comment admits non-testing**: The comment "Just verify it returns without panicking" explicitly states the function does not test correctness. While `fuzz_ipc_frame_decode` (separate, line 227) does have real assertions, this target is purely a smoke test.

2. **`fuzz/fuzz_targets.rs` is a shim with no assertions**: The file at the path the user listed (`fuzz/fuzz_targets/generated_compare.rs`) does not exist. The actual fuzz target bodies are in `fuzz/src/lib.rs`. The shim at `fuzz/fuzz_targets.rs` just delegates to `fuzz_lib::*` — the real test code is in `fuzz/src/lib.rs`.

---

## MANDATE

1. **`fuzz_ipc_frame`** (`fuzz/src/lib.rs:192–223`): Replace `let _ = decode_frame_payload(...)` with an actual `assert!` that checks decoded frame fields. At minimum, verify `payload_len` is consistent with the actual payload slice length.

2. After fix: re-run **Tier 0 full re-run** including banned pattern scan and the `let _ =` suppression check on `fuzz/src/lib.rs`.

3. Resubmit for full re-review — all tiers restart from Tier 0.

---

## FILE-BY-FILE NOTES

### `crates/vb_runtime/src/engine/property_tests.rs` ✅ CLEAN
- 240 lines, 19 tests covering `EvidenceCollector`, `RetryPolicy`, `RuntimeEngineError`, `RuntimeSignal`, `EvidenceEvent`
- Sharp assertions: `assert_eq!` on exact field values, `matches!` on exact enum variants
- `#[forbid(unsafe_code)]` enforced
- No weak `is_ok()` / `is_err()` assertions
- No `let _ = .ok()` suppression
- **APPROVED** on its own

### `fuzz/src/lib.rs` (`fuzz_generated_compare`, `fuzz_compiled_ir`, `fuzz_expression`) ✅ MOSTLY CLEAN
- `fuzz_generated_compare:401` — `assert!(validated.is_ok() == workflow.is_ok())` — This is a strong equivalence check between two independent code paths. **APPROVED**.
- `fuzz_compiled_ir:358–368` — Asserts `node_count() >= 1` and `slot_count() >= 1` on successfully decoded workflow. **APPROVED**.
- `fuzz_expression:343` — Asserts `!type_name.is_empty()` on evaluated expression result. Weak but not lethal. **ACCEPTABLE**.
- `fuzz_ipc_frame:208,220` — **REJECTED** — `let _ =` discards all decode results.

### `crates/vb_runtime/src/shard/directive.rs` ✅ CLEAN
- `Migrate { target: u32 }` variant present and tested (lines 50–53, 144–148, 365–395)
- `Shutdown` variant present and tested (lines 58–59, 402–430)
- All 6 variants covered with equality, copy, debug, admission, completion, and is_alive tests
- 460 lines of tests in `#[cfg(test)]` module
- **APPROVED**
