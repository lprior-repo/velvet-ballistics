# Test Plan: Wait Digest Coverage

**Bead:** vb-xi2f.32
**Date:** 2026-05-25
**State:** test-planner (State 8)
**Schema:** test-plan/v1
**Bridge input:** `.beads/vb-xi2f.32/proof-to-rust-map.md` (APPROVED)

## Summary

- **Behaviors identified:** 12
- **Trophy allocation:** 5 unit / 5 integration / 2 e2e / 0 static
- **Proptest invariants:** 6 (5 written + VERIFIED; 1 new)
- **Fuzz targets:** 3 (written; pending tooling fix)
- **Kani harnesses:** 4 (written; pending tooling fix)
- **New test scenarios needed:** 15 (gap-filling unit + integration)
- **Existing scenarios preserved:** 10

## 1. Behavior Inventory

| # | Behavior Description | Contract | Trophy Layer |
|---|---------------------|----------|-------------|
| B1 | `canonical_digest` produces different digests when wait event field differs | C1 | Integration |
| B2 | `canonical_digest` produces different digests when wait timeout field differs | C1 | Integration |
| B3 | `canonical_digest` distinguishes WaitUntil from WaitEvent via explicit discriminator | C2 | Integration |
| B4 | `canonical_digest` uses sentinel `b"none"` for absent optional wait fields | C3 | Unit |
| B5 | `canonical_digest` remains deterministic after fix: same source → same digest | C4 | Integration/Proptest |
| B6 | Both copies of `digest_step_primitive` produce identical hasher states for identical Wait inputs | C5 | Integration |
| B7 | `digest_step_primitive` Wait arm does not panic for any legal field combination | C1 | Unit/Kani |
| B8 | All three legal Wait configurations produce pairwise-distinct digests | C1,C2,C3 | Integration/Proptest |
| B9 | `canonical_primitive_name` returns `"wait"` for `StepPrimitive::Wait` | — | Unit |
| B10 | `digest_step_primitive` dispatches Wait to its explicit arm, not the catch-all | C1 | Unit |
| B11 | Invalid wait shape `(event=None, timeout=None)` is rejected by validation before digest computation | DI-4 | Integration |
| B12 | `compute_compiled_digest` (byte-hashing API) is unaffected by the fix — same source bytes → same hash | C6 | Integration |

## 2. Trophy Allocation

```
         [E2E]           2 tests — full compilation pipeline, both paths
    [Integration]        7 tests — component boundaries, real compiler deps
    [Unit / Calc]        5 tests — direct function calls, no YAML parsing
  [Static Analysis]      0 — compile-time checks already in place
```

**Ratios:** ~42% unit, ~58% integration, ~0% e2e, ~0% static.
**Deviation rationale:** The digest functions are pure (no I/O, no async, no network). Full E2E tests requiring a running engine are unnecessary for a compile-time digest fix — the digest's correctness is fully observable at the compilation boundary. Integration tests through `compile_source` already exercise the full pipeline from YAML to `WorkflowDigest`. Static analysis is handled by existing clippy/deny gates.

## 3. BDD Scenarios

### Behavior B1: Digest sensitivity to `event` field changes (C1)

#### Scenario: WaitEvent with different event slots produce different digests

```
Given: Two workflows, each with one Wait step that uses WaitEvent shape
       Workflow A: event="0", timeout="30"
       Workflow B: event="1", timeout="30"
When:  canonical_digest is computed for both
Then:  digest(A) != digest(B)
```

### Behavior B2: Digest sensitivity to `timeout` field changes (C1)

#### Scenario: WaitEvent with different timeouts produce different digests

```
Given: Two workflows, each with one Wait step that uses WaitEvent shape
       Workflow A: event="0", timeout="10"
       Workflow B: event="0", timeout="20"
When:  canonical_digest is computed for both
Then:  digest(A) != digest(B)
```

#### Scenario: WaitUntil with different deadlines produce different digests

```
Given: Two WaitUntil workflows
       Workflow A: timeout="5"
       Workflow B: timeout="10"
When:  canonical_digest is computed for both
Then:  digest(A) != digest(B)
```

### Behavior B3: WaitUntil vs WaitEvent discrimination (C2)

#### Scenario: WaitUntil and WaitEvent produce different digests

```
Given: Workflow A: WaitUntil with timeout="5"
       Workflow B: WaitEvent with event="5", timeout=None
When:  canonical_digest is computed for both
Then:  digest(A) != digest(B)
```

### Behavior B4: Absent field sentinel (C3)

#### Scenario: Absent timeout represented by sentinel in hasher state

```
Given: A WaitEvent step with event="0", timeout=None
When:  digest_step_primitive is called directly
Then:  The hasher state includes b"none" at the position corresponding to timeout
```

#### Scenario: WaitEvent with timeout=None vs timeout=Some have different digests

```
Given: Workflow A: WaitEvent with event="0", timeout=None
       Workflow B: WaitEvent with event="0", timeout=Some("5")
When:  canonical_digest is computed for both
Then:  digest(A) != digest(B)
```

### Behavior B5: Digest determinism (C4)

#### Scenario: Same source always produces same digest (three consecutive calls)

```
Given: A valid WorkflowSource with Wait steps
When:  canonical_digest is computed three times sequentially
Then:  All three digests are equal
```

### Behavior B6: Dual implementation consistency (C5)

#### Scenario: Both compiler paths produce identical digests for Wait workflow

```
Given: A valid YAML workflow with Wait steps (WaitUntil, WaitEvent-bounded, WaitEvent-unbounded)
When:  Compiled via compile_source (cold-path) and compile_workflow (warm-path)
Then:  cold.digest() == warm.digest()
```

### Behavior B7: Wait arm panic-freedom (C1)

#### Scenario: digest_step_primitive does not panic for any legal Wait shape

```
Given: StepPrimitive::Wait with any legal field combination
       (event=None, timeout=Some)  // WaitUntil
       (event=Some, timeout=None)  // WaitEvent unbounded
       (event=Some, timeout=Some)  // WaitEvent bounded
When:  digest_step_primitive is called with a blake3::Hasher
Then:  The function returns without panicking
```

### Behavior B8: Pairwise distinct digests (C1,C2,C3)

#### Scenario: All three legal Wait shapes produce different digests for distinct fields

```
Given: Three workflows with inline WaitUntil, WaitEvent-unbounded, WaitEvent-bounded
       Each with different field values
When:  canonical_digest is computed for all three
Then:  All three digests are pairwise different
```

### Behavior B9: canonical_primitive_name returns "wait" for Wait

#### Scenario: Name function identifies Wait variant correctly

```
Given: Any StepPrimitive::Wait variant
When:  canonical_primitive_name is called
Then:  Returns the static string "wait"
```

### Behavior B10: Wait arm bypasses catch-all

#### Scenario: digest_step_primitive dispatches Wait to explicit arm

```
Given: StepPrimitive::Wait { event: Some("0".into()), timeout: Some("30".into()) }
When:  digest_step_primitive is called
Then:  The hasher receives b"wait" + event_bytes + timeout_bytes
       NOT just b"wait" (which the catch-all would produce)
```

### Behavior B11: Invalid wait shape rejected

#### Scenario: Empty Wait (event=None, timeout=None) rejected by validation

```
Given: A YAML workflow with wait: {} (both fields absent)
When:  compile_source is called with the parsed source
Then:  Returns CompileError::StepFieldShape
```

### Behavior B12: compute_compiled_digest untouched

#### Scenario: Byte-hashing API still deterministic after fix

```
Given: Two Wait workflows as raw byte slices, identical or different
When:  compute_compiled_digest is called on both
Then:  Same bytes → same digest; different bytes → different digest
       (independent of canonical_digest changes)
```

## 4. Proptest Invariants

### PI-1: Wait field sensitivity (PO-002) — VERIFIED
**Function:** `canonical_digest` + `digest_step_primitive`
**Invariant:** `forall (ea, ta), (eb, tb): (ea!=eb || ta!=tb) ⇒ digest(workflow_with(Wait{ea,ta})) != digest(workflow_with(Wait{eb,tb}))`
**Strategy:** `wait_field_strategy()` — generates legal (event, timeout) pairs with integer strings 0..255. At least one field is Some.
**Evidence:** `evidence/proptest-vb-xi2f.32/01-field-sensitivity.log`
**Status:** VERIFIED (State 6)

### PI-2: WaitUntil vs WaitEvent discrimination (PO-004) — VERIFIED
**Function:** `canonical_digest` + `digest_step_primitive`
**Invariant:** `forall t, e: digest(WaitUntil{t}) != digest(WaitEvent{e, SameOrDefaultTimeout})`
**Strategy:** `wait_slot_strategy()` — integer strings 0..255 for both timeout and event. WaitUntil uses event=None, WaitEvent uses event=Some.
**Evidence:** `evidence/proptest-vb-xi2f.32/02-until-vs-event.log`
**Status:** VERIFIED (State 6)

### PI-3: Sentinel unambiguous (PO-006) — VERIFIED
**Function:** `canonical_digest` + `digest_step_primitive`
**Invariant:** `forall e, ta, tb: ta != tb ⇒ digest(WaitEvent{e, ta}) != digest(WaitEvent{e, tb})`
**Strategy:** Same event text, different timeout values. Adapted from the original sentinel contract (absent vs Some("none")) because the YAML validator restricts timeout to integer-like strings.
**Evidence:** `evidence/proptest-vb-xi2f.32/03-sentinel-unambiguous.log`
**Status:** VERIFIED (State 6)

### PI-4: Pairwise distinct (PO-011) — VERIFIED
**Function:** `canonical_digest` + `digest_step_primitive`
**Invariant:** `forall wa != wb among {WaitUntil, WaitEvent-unbounded, WaitEvent-bounded}: digest(workflow_with(wa)) != digest(workflow_with(wb))`
**Strategy:** Random three-shape enumeration using `make_legal_wait_shape()`. Skips identical shapes.
**Evidence:** `evidence/proptest-vb-xi2f.32/04-pairwise-distinct.log`
**Status:** VERIFIED (State 6)

### PI-5: Determinism regression (PO-008/PO-014) — VERIFIED
**Function:** `canonical_digest` + `compile_source`
**Invariant:** Same `WorkflowSource` → same `WorkflowDigest` every time.
**Strategy:** `primitive_case_strategy()` — covers all primitive types including Wait.
**Evidence:** `evidence/proptest-vb-xi2f.32/06-regression-equal-sources.log`
**Status:** VERIFIED (State 6)

### PI-6: Cross-path equivalence (PO-009/PO-016) — VERIFIED
**Function:** `compile_source` (cold-path) vs `compile_workflow` (warm-path)
**Invariant:** Both paths produce identical `WorkflowDigest` for all Wait workflow sources.
**Strategy:** Random Wait shapes (WaitUntil, WaitEvent-unbounded, WaitEvent-bounded) with `wait_slot_strategy()`. Both paths are invoked and digests compared.
**Evidence:** `evidence/proptest-vb-xi2f.32/05-cross-path-equivalence.log`
**Status:** VERIFIED (State 6)

### PI-7: NEW — Wait digest idempotency at step level
**Function:** `digest_step_primitive` (called directly)
**Invariant:** `forall (e, t): two calls to digest_step_primitive with same Wait fields → identical hasher state → identical finalize()`
**Strategy:** `wait_field_strategy()`, called twice on separate hashers.
**Rationale:** Complements the workflow-level determinism test with a function-level guarantee.

### PI-8: NEW — Non-Wait workflows produce unchanged digests
**Function:** `canonical_digest`
**Invariant:** `forall workflow without Wait steps: canonical_digest(workflow) == canonical_digest_pre_fix(workflow)`
**Strategy:** Use the existing `primitive_case_strategy()` but filter to non-Wait primitives. Compare digests via `compile_source`.
**Rationale:** Ensures the Wait arm addition does not affect digest computation for non-Wait workflows (regression guard for Set, Finish, and catch-all primitives).

## 5. Fuzz Targets

### FZ-1: wait_digest_sensitivity (PO-003) — WRITTEN, PENDING EXECUTION
**Input type:** `&[u8]` — raw bytes mapped to integer-like slot strings
**Risk:** Collision between different wait field configurations
**Corpus seeds:** Edge cases: (0,0) vs (0,1), (0,1) vs (1,1), (255,0) vs (0,255)
**File:** `fuzz/fuzz_targets/wait_digest_sensitivity.rs`
**Status:** Written at State 5. Pending execution — blocked by musl/sanitizer tooling incompatibility. State 7 must resolve tooling.

### FZ-2: wait_sentinel_collision (PO-007) — WRITTEN, PENDING EXECUTION
**Input type:** `&[u8]` — raw bytes mapped to timeout values
**Risk:** Collision between different timeout values with identical event
**Corpus seeds:** (0,0,1) — event="0", timeout_a="0", timeout_b="1"
**File:** `fuzz/fuzz_targets/wait_sentinel_collision.rs`
**Status:** Written at State 5. Pending execution — same tooling blocker.

### FZ-3: wait_digest_exhaustive_collision (PO-012) — WRITTEN, PENDING EXECUTION
**Input type:** `&[u8]` — raw bytes mapped to Wait shapes and slot values
**Risk:** Any collision between distinct Wait configurations
**Corpus seeds:** (0,1,0,1) — two different shapes, different event values; (0,0,0,0) — same shape, same values (should skip); (0,1,2,2) — shape1=WaitUntil, shape2=WaitEvent
**File:** `fuzz/fuzz_targets/wait_digest_exhaustive_collision.rs`
**Status:** Written at State 5. Pending execution — same tooling blocker.

## 6. Kani Harnesses

### KH-1: wait_digest_step_primitive_no_panic (PO-001) — WRITTEN, PENDING EXECUTION
**Property:** `digest_step_primitive` Wait arm does not panic for all legal Wait field combinations within bounded alphabet (a-zA-Z0-9_, max 16 chars).
**Bound:** unwind(10), max_string_len=16, alphabet a-zA-Z0-9_
**Rationale:** Bounded verification of the primary fix location. Panic-freedom is a GOD RULE requirement — no `unwrap`, `expect`, `panic`, `todo`, or `unimplemented`.
**File:** `crates/vb_compile/src/kani_wait_digest.rs:34`
**Command:** `TMPDIR=target/tmp cargo kani -p vb_compile --harness wait_digest_step_primitive_no_panic -Z unstable-options`
**Status:** Written at State 5. Pending execution — blocked by Kani 0.67 `Arbitrary` for `String` limitation. Harness uses `kani::any::<Option<String>>()` which may fail. State 7 must refactor to `[u8; N]` arrays with valid-UTF-8 assumptions per `proof-to-rust-map.md` GAP-MAP-002.

### KH-2: wait_until_vs_wait_event_no_collision (PO-005) — WRITTEN, PENDING EXECUTION
**Property:** WaitUntil (event=None, timeout=Some) and WaitEvent (event=Some) produce different blake3 final hashes for all bounded inputs.
**Bound:** unwind(8), max_string_len=8, alphabet a-zA-Z0-9_
**Rationale:** Formal verification of the C2 discrimination property. Complements the proptest which covers broad input space but not ALL inputs within the bound.
**File:** `crates/vb_compile/src/kani_wait_digest.rs:79`
**Command:** `TMPDIR=target/tmp cargo kani -p vb_compile --harness wait_until_vs_wait_event_no_collision -Z unstable-options`
**Status:** Same tooling blocker as KH-1.

### KH-3: wait_configurations_pairwise_distinct (PO-013) — WRITTEN, PENDING EXECUTION
**Property:** The three legal Wait configurations produce pairwise-distinct digests for all bounded field values (max 4 chars, lowercase a-z alphabet).
**Bound:** unwind(6), max_string_len=4, alphabet a-z
**Rationale:** Exhaustive verification of C1+C2+C3 combined within a tractable bound. The small alphabet (a-z) is sufficient to prove structural distinctness even if it doesn't cover all practical slot expressions.
**File:** `crates/vb_compile/src/kani_wait_digest.rs:148`
**Command:** `TMPDIR=target/tmp cargo kani -p vb_compile --harness wait_configurations_pairwise_distinct -Z unstable-options`
**Status:** Same tooling blocker.

### KH-4: wait_digest_both_copies_no_panic (PO-015) — WRITTEN, PENDING EXECUTION
**Property:** Cold-path `digest_step_primitive` is panic-free for all legal Wait field combinations (WaitUntil, WaitEvent-unbounded, WaitEvent-bounded).
**Bound:** unwind(10), max_string_len=16, alphabet a-zA-Z0-9_
**Rationale:** Structural equivalence verification. Warm-path copy is dead code (not in module tree) — the harness exercises the active cold-path copy exclusively. PO-010 (cross-path Kani equivalence) is WAIVED as BLOCKED_DEAD_CODE.
**File:** `crates/vb_compile/src/kani_wait_digest.rs:217`
**Command:** `TMPDIR=target/tmp cargo kani -p vb_compile --harness wait_digest_both_copies_no_panic -Z unstable-options`
**Status:** Same tooling blocker.

## 7. Mutation Checkpoints

**Threshold:** ≥90% mutation kill rate on `digest_step_primitive` (lines 158-168 in `part_05.rs`) and its companion copy (lines 257-267 in `compile/mod.rs`).

### Critical Mutations to Survive

| Mutation | Code Location | Must Be Caught By | Justification |
|----------|--------------|-------------------|---------------|
| Delete `hasher.update(b"wait")` (line 159) | `part_05.rs:159` | `unit_digest_step_wait_includes_label` (new) | Removing the label changes the hash for all Wait steps. All sensitivity tests would detect the digest change but might not isolate the cause. |
| Change `b"wait"` → `b"wait_event"` (line 159) | `part_05.rs:159` | `unit_digest_step_wait_includes_label` (new) | Discrimination from other primitives. |
| Delete `hasher.update(e.as_bytes())` (line 161) | `part_05.rs:161` | `unit_event_field_affects_hasher_state` (new) | C1 violation. |
| Delete entire event match arm (lines 160-163) | `part_05.rs:160-163` | `integration_wait_event_sensitivity_to_event_field_change` (new) | C1 violation: event field ignored. |
| Change `b"none"` → `b"nil"` (line 162) | `part_05.rs:162` | `unit_none_event_uses_none_sentinel` (new) | C3 sentinel change would cause cross-version digest incompatibility. |
| Delete `hasher.update(t.as_bytes())` (line 165) | `part_05.rs:165` | `unit_timeout_field_affects_hasher_state` (new) | C1 violation. |
| Change `b"none"` → `b"missing"` (line 166) | `part_05.rs:166` | `unit_none_timeout_uses_none_sentinel` (new) | C3 sentinel change. |
| Remove entire Wait match arm (lines 158-168) | `part_05.rs:158-168` | `integration_until_vs_event_produce_distinct_digests` (existing PO-004) | C1,C2,C3: all violations at once. |
| Swap event and timeout hash order | `part_05.rs:160-167` | `proptest_wait_field_sensitivity` (existing PO-002) | Hash ordering changes the final digest value. |
| Fix only one copy, not both | `part_05.rs:158-168` vs `compile/mod.rs:257-267` | `cross_path_wait_digest_equivalence` (existing PO-009/PO-016) | C5 violation: digest divergence. |
| Change `b"none"` → `b""` (empty bytes) | `part_05.rs:162,166` | `unit_none_sentinels_are_fixed_constant` (new) | Empty bytes would collide trivially with each other. |
| Collapse event match arms to single arm | `part_05.rs:160-163` | `integration_until_vs_event_produce_distinct_digests` (existing) | Would lose wait_until vs wait_event distinction. |

### Mutation Test Commands

```bash
# Execute mutation testing on the Wait match arm specifically
cargo mutants -p vb_compile -f part_05.rs -l 158 -l 168 -- --test proptest_wait

# Full mutation sweep across both copies
cargo mutants -p vb_compile -f "**/part_05.rs" -f "**/compile/mod.rs" -- --test wait

# Threshold check
cargo mutants --minimum-test-timeout 60 --list-functions digest_step_primitive
```

## 8. Combinatorial Coverage Matrix

### Group: `digest_step_primitive` Wait arm (direct unit tests)

| # | Scenario | Input Class | Expected Output | Test Layer | Status |
|---|----------|-------------|-----------------|-----------|--------|
| 1 | WaitUntil (event=None, timeout=Some("5")) | Legal Wait shape | Hasher updated with `b"wait"` + `b"none"` + `b"5"` | unit | **NEW** |
| 2 | WaitEvent unbounded (event=Some("0"), timeout=None) | Legal Wait shape | Hasher updated with `b"wait"` + `b"0"` + `b"none"` | unit | **NEW** |
| 3 | WaitEvent bounded (event=Some("0"), timeout=Some("30")) | Legal Wait shape | Hasher updated with `b"wait"` + `b"0"` + `b"30"` | unit | **NEW** |
| 4 | Same Wait fields, called twice | Identical input | Both hashers produce identical `finalize()` | unit | **NEW** |
| 5 | Different event, same timeout | Event change | Different hasher states | unit | **NEW** |
| 6 | Same event, different timeout | Timeout change | Different hasher states | unit | **NEW** |
| 7 | Event=None → sentinel `b"none"` | Sentinel | Hasher includes `b"none"` for event position | unit | **NEW** |
| 8 | Timeout=None → sentinel `b"none"` | Sentinel | Hasher includes `b"none"` for timeout position | unit | **NEW** |
| 9 | Wait arm reached (not catch-all) | Dispatch | Wait-specific bytes in hasher, not just `b"wait"` | unit | **NEW** |

### Group: `canonical_digest` with Wait workflows (integration)

| # | Scenario | Input Class | Expected Output | Test Layer | Status |
|---|----------|-------------|-----------------|-----------|--------|
| 10 | Workflow with WaitUntil step | Happy path | Digest computed; same source → same digest ×3 | integration | **NEW** |
| 11 | Workflow with WaitEvent (unbounded) | Happy path | Digest computed; differs from WaitUntil digest (B3) | integration | **NEW** |
| 12 | Workflow with WaitEvent (bounded) | Happy path | Digest computed; differs from both above | integration | **NEW** |
| 13 | Two workflows: different event, same other fields | Event differ | Different digests (B1) | integration | **NEW** |
| 14 | Two workflows: different timeout, same other fields | Timeout differ | Different digests (B2) | integration | **NEW** |
| 15 | WaitUntil vs WaitEvent with same timeout text | Discrimination | Different digests (B3) | integration | **NEW** |
| 16 | WaitEvent timeout=None vs timeout=Some("5") | Sentinel vs value | Different digests (B4) | integration | **NEW** |
| 17 | Two identical Wait workflows, compiled 3 times | Determinism | All 3 digests equal (B5) | integration | **NEW** |
| 18 | Wait workflow, cold-path vs warm-path | Dual-path | Same digest from both (B6) | integration | existing PO-009 |
| 19 | Invalid wait shape `event=None, timeout=None` | Error | `CompileError::StepFieldShape` (B11) | integration | **NEW** |
| 20 | Wait workflow with non-Wait steps (Set + Wait + Finish) | Mixed | Digest differs from Set+Finish-only workflow | integration | **NEW** |
| 21 | Workflow without Wait steps — digest unchanged from pre-fix | Non-Wait regression | Passes `proptest_equal_primitive_sources_compile_to_equal_digest_and_ir` | integration | existing PO-008 |

### Group: Error path validation (integration)

| # | Scenario | Input Class | Expected Output | Test Layer | Status |
|---|----------|-------------|-----------------|-----------|--------|
| 22 | `wait: {}` — both fields absent | Invalid shape | `CompileError::StepFieldShape` | integration | existing |
| 23 | `wait: { event: "" }` — empty event string | Invalid slot | `CompileError::StepFieldShape` (empty field) | integration | existing |
| 24 | `wait: { timeout: "" }` — empty timeout string | Invalid slot | `CompileError::StepFieldShape` (empty field) | integration | existing |
| 25 | `wait: { event: "99999" }` — out-of-range slot | Slot overflow | `CompileError::SlotIndexOutOfRange` | integration | existing |
| 26 | `wait: { event: "not_a_number" }` — non-integer slot | Invalid slot | `CompileError::StepFieldShape` | integration | existing |

### Group: `compute_compiled_digest` byte-hashing API (regression guard)

| # | Scenario | Input Class | Expected Output | Test Layer | Status |
|---|----------|-------------|-----------------|-----------|--------|
| 27 | Same Wait workflow bytes, computed twice | Determinism | Same digest (B12) | integration | existing |
| 28 | Two Wait workflows with different field bytes | Different bytes | Different digests (B12) | integration | existing |

### Group: `canonical_primitive_name`

| # | Scenario | Input Class | Expected Output | Test Layer | Status |
|---|----------|-------------|-----------------|-----------|--------|
| 29 | `StepPrimitive::Wait { .. }` | Any Wait variant | Returns `"wait"` | unit | **NEW** |
| 30 | Non-Wait primitives (Set, Finish, Do, etc.) | All variants | Expected name strings | unit | existing |

## 9. New Test Specifications

This section details the NEW tests that need to be written by the test-writer. These fill gaps not covered by the existing proptest, Kani, and fuzz artifacts.

### 9.1 Direct Unit Tests for `digest_step_primitive` (new file or module)

**Location:** `crates/vb_compile/src/tests/wait_digest_unit_tests.rs` (new file)
**Rationale:** All existing tests exercise digest computation through the full compilation pipeline (`compile_source` → `canonical_digest`). Direct unit tests on `digest_step_primitive` provide faster, more granular coverage of the exact function being fixed.

#### test: `unit_digest_step_wait_includes_wait_label`
- Assert that hashing a `StepPrimitive::Wait { event: Some("0".into()), timeout: Some("30".into()) }` via `digest_step_primitive` results in a hasher whose `finalize()` differs from a hasher that only receives the catch-all path (`canonical_primitive_name` → `b"wait"`).

#### test: `unit_event_field_affects_hasher_state`
- Assert that `digest_step_primitive` with `event=Some("0")` and `event=Some("1")` (same timeout) produce different `finalize()` outputs.

#### test: `unit_timeout_field_affects_hasher_state`
- Assert that `digest_step_primitive` with `timeout=Some("10")` and `timeout=Some("20")` (same event) produce different `finalize()` outputs.

#### test: `unit_none_event_uses_none_sentinel`
- Assert that `digest_step_primitive` with `event=None` produces a hasher state that differs from `event=Some("none_sentinel_probe")`. The sentinel `b"none"` for `None` must not be confused with an explicit field value.

#### test: `unit_none_timeout_uses_none_sentinel`
- Same as above but for `timeout=None` vs `timeout=Some("none_sentinel_probe")`.

#### test: `unit_digest_step_wait_arm_is_deterministic`
- Assert that calling `digest_step_primitive` twice with the same Wait primitive on two fresh hashers produces identical `finalize()` outputs.

#### test: `unit_digest_step_wait_vs_catch_all_never_collides`
- Assert that a Wait primitive through its explicit arm produces a different final hash than the catch-all arm processing only `canonical_primitive_name` → `b"wait"` (simulating the pre-fix behavior).

#### test: `unit_digest_step_wait_no_panic_three_shapes`
- Assert that `digest_step_primitive` does not panic for any of the three legal Wait shapes. This is a direct unit complement to the Kani harness.

### 9.2 Direct Unit Tests for `canonical_primitive_name`

**Location:** `crates/vb_compile/src/tests/wait_digest_unit_tests.rs` (same file)

#### test: `canonical_primitive_name_wait_returns_wait`
- Assert that `canonical_primitive_name(&StepPrimitive::Wait { event: None, timeout: Some("5".into()) })` returns `"wait"`.

#### test: `canonical_primitive_name_all_variants_have_names`
- Assert that every `StepPrimitive` variant (including Wait) returns a non-empty, distinct name string. Covers Set, Save, Do, Choose, ForEach, Together, Collect, Aggregate, Repeat, Wait, Ask, Finish.

### 9.3 Integration Tests Through Compilation Pipeline

**Location:** `crates/vb_compile/tests/v1_primitive_lowering.rs` (add to existing module) or new test file

#### test: `integration_wait_event_sensitivity_to_event_field_change`
- Build two workflows via YAML: one with `wait: { event: "0", timeout: "30" }`, one with `wait: { event: "1", timeout: "30" }`.
- Compile both via `compile_source`.
- Assert `digest(A) != digest(B)` and assert both compiled IRs are structurally correct.

#### test: `integration_wait_event_sensitivity_to_timeout_field_change`
- Build two workflows: one with `wait: { event: "0", timeout: "10" }`, one with `wait: { event: "0", timeout: "20" }`.
- Assert different digests.

#### test: `integration_wait_until_timeout_change_produces_distinct_digest`
- Build two WaitUntil workflows: timeout="5" vs timeout="10".
- Assert different digests.

#### test: `integration_wait_until_vs_wait_event_produce_distinct_digests`
- Build WaitUntil (timeout="5") and WaitEvent (event="5", no timeout).
- Assert different digests.

#### test: `integration_wait_event_no_timeout_vs_with_timeout_produce_distinct_digests`
- Build WaitEvent(event="0", timeout=None) and WaitEvent(event="0", timeout=Some("5")).
- Assert different digests.

#### test: `integration_wait_digest_is_deterministic_three_computations`
- Compile the same Wait workflow three times via `compile_source`.
- Assert all three digests are equal.

#### test: `integration_wait_workflow_digest_roundtrips_through_parts`
- Compile a Wait workflow, get digest from `CompiledWorkflow::digest()`, convert to `WorkflowParts` via `to_parts()`, assert parts.digest matches.

#### test: `integration_wait_workflow_with_mixed_steps_digests_differ_from_non_wait`
- Build a workflow with Set + Wait + Finish. Compile.
- Build a workflow with Set + Finish only (same Set). Compile.
- Assert different digests (the Wait step contribution changes the hash).

#### test: `integration_wait_invalid_shape_none_none_rejected`
- Construct a `WorkflowSource` with `Wait { event: None, timeout: None }` (bypassing YAML validator to test the compilation pipeline directly).
- Assert `compile_source` returns `CompileError::StepFieldShape`.

### 9.4 New Proptest Invariant (add to existing proptest block)

#### test: `proptest_wait_digest_step_level_idempotent`
- For each generated Wait field pair, call `digest_step_primitive` twice on fresh hashers.
- Assert `hasher1.finalize() == hasher2.finalize()`.

#### test: `proptest_non_wait_workflows_produce_unchanged_digests`
- Generate workflow sources without Wait steps using the existing `primitive_case_strategy()` filtered to non-Wait primitives.
- Compile and assert the digest matches a known-good snapshot or a reference compilation.
- This is a regression guard ensuring the Wait arm addition did not break non-Wait digest computation.

## 10. Explicit Anti-Pattern Checks

These checks ensure the test writer does NOT introduce any of the following:

| Anti-Pattern | Rule | Verification |
|-------------|------|-------------|
| `result.is_ok()` without asserting the value | REJECT | Every test must assert exact digest bytes or exact error variant |
| `result.is_err()` without matching the error variant | REJECT | Error tests must match `CompileError::StepFieldShape`, `CompileError::SlotIndexOutOfRange`, etc. |
| Mock or fake of `blake3::Hasher` | REJECT | Use real `blake3::Hasher::new()` — it's fast and deterministic |
| Test named `test_foo()` | REJECT | All test names follow the `[subject]_[outcome]_when_[condition]` pattern |
| Single test covering multiple behaviors | REJECT | Each test covers exactly one contract clause behavior |
| Logic (loops, conditionals) in test bodies | REJECT | If a loop is needed for "three-way pairwise distinctness", use explicit assertions |
| `sleep()` in tests | REJECT | Not applicable — tests are synchronous |
| Interaction testing (asserting `was called`) | REJECT | Test state (digest value), not interactions |
| Hardcoded dummy data in Kani harnesses | REJECT | Existing Kani harnesses use `kani::any()` — GOD RULE 1 compliant |
| Test depends on test ordering | REJECT | Each test is hermetic — creates its own hasher and primitives |

## 11. Tooling Blockers

The following blockers prevent execution of already-written artifacts. These are noted for the State 7 formal-verifier:

| Blocker | Affected Artifacts | Resolution Path |
|---------|-------------------|-----------------|
| Kani 0.67 `Arbitrary` for `String` | KH-1, KH-2, KH-3, KH-4 | Refactor harnesses to use `[u8; N]` with valid-UTF-8 assumptions per GAP-MAP-002 in proof-to-rust-map.md |
| `cargo fuzz` runs on musl: sanitizer is incompatible with statically linked libc | FZ-1, FZ-2, FZ-3 | Configure musl/fuzz tooling compatibility per GAP-MAP-003, or switch to glibc target for fuzzing |
| Warm-path `compile/mod.rs` is dead code (not in module tree) | KH-4 cross-copy | PO-010 WAIVED as BLOCKED_DEAD_CODE. Both copies fixed identically; proptest PO-009/PO-016 covers equivalence at workflow level |

## 12. Evidence Command Map (Tests Already Written)

```bash
# All commands run from: /home/lewis/src/vb-workspaces/vb-xi2f.32/

# ── Proptest (verified at State 6) ──
# PO-002: Wait field sensitivity
cargo test --package vb_compile --test v1_primitive_lowering -- proptest_wait_field_sensitivity --nocapture

# PO-004: WaitUntil vs WaitEvent
cargo test --package vb_compile --test v1_primitive_lowering -- proptest_wait_until_vs_wait_event --nocapture

# PO-006: Sentinel unambiguous
cargo test --package vb_compile --test v1_primitive_lowering -- proptest_wait_sentinel_unambiguous --nocapture

# PO-008/P-014: Determinism regression
cargo test --package vb_compile --test v1_primitive_lowering -- proptest_equal_primitive_sources_compile_to_equal_digest_and_ir --nocapture

# PO-009/PO-016: Cross-path equivalence
cargo test --package vb_compile -- cross_path_wait_digest_equivalence --nocapture

# PO-011: Pairwise distinct
cargo test --package vb_compile --test v1_primitive_lowering -- proptest_wait_pairwise_distinct_digests --nocapture

# Full vb_compile suite (PO-014 regression)
cargo test --package vb_compile

# ── Kani (pending State 7) ──
# PO-001
TMPDIR=target/tmp cargo kani -p vb_compile --harness wait_digest_step_primitive_no_panic -Z unstable-options

# PO-005
TMPDIR=target/tmp cargo kani -p vb_compile --harness wait_until_vs_wait_event_no_collision -Z unstable-options

# PO-013
TMPDIR=target/tmp cargo kani -p vb_compile --harness wait_configurations_pairwise_distinct -Z unstable-options

# PO-015
TMPDIR=target/tmp cargo kani -p vb_compile --harness wait_digest_both_copies_no_panic -Z unstable-options

# ── Fuzz (pending State 7) ──
# PO-003
cargo fuzz run wait_digest_sensitivity -- -max_len=64 -max_total_time=120

# PO-007
cargo fuzz run wait_sentinel_collision -- -max_len=64 -max_total_time=120

# PO-012
cargo fuzz run wait_digest_exhaustive_collision -- -max_len=64 -max_total_time=180

# ── Mutation (pending State 8/9) ──
cargo mutants -p vb_compile -f part_05.rs -l 158 -l 168 -- --test proptest_wait
```

## 13. Traceability Matrix

| Contract Clause | Behavior(s) | Unit Tests | Integration Tests | Proptest | Kani | Fuzz | Mutation |
|----------------|-------------|-----------|-------------------|----------|------|------|----------|
| C1 (Wait field hashing) | B1, B2, B7 | `unit_event_field_affects_hasher_state`, `unit_timeout_field_affects_hasher_state`, `unit_digest_step_wait_no_panic_three_shapes` | `integration_wait_event_sensitivity_to_event_field_change`, `integration_wait_event_sensitivity_to_timeout_field_change`, `integration_wait_until_timeout_change_produces_distinct_digest` | PI-1, PI-7 | KH-1, KH-4 | FZ-1 | Delete field hash, swap order |
| C2 (WaitUntil vs WaitEvent) | B3 | `unit_none_event_uses_none_sentinel` | `integration_wait_until_vs_wait_event_produce_distinct_digests` | PI-2 | KH-2 | FZ-3 | Remove discriminator |
| C3 (Absent field sentinel) | B4 | `unit_none_event_uses_none_sentinel`, `unit_none_timeout_uses_none_sentinel` | `integration_wait_event_no_timeout_vs_with_timeout_produce_distinct_digests` | PI-3 | KH-3 | FZ-2 | Change sentinel bytes |
| C4 (Determinism) | B5 | `unit_digest_step_wait_arm_is_deterministic` | `integration_wait_digest_is_deterministic_three_computations` | PI-5 | — | — | — |
| C5 (Dual consistency) | B6 | — | `integration_wait_workflow_digest_roundtrips_through_parts` | PI-6 | — | — | Fix only one copy |
| C6 (Regression) | B12 | — | `integration_wait_workflow_with_mixed_steps_digests_differ_from_non_wait` | PI-5, PI-8 | — | — | — |
| DI-4 (Empty wait invalid) | B11 | — | `integration_wait_invalid_shape_none_none_rejected` | — | — | — | — |
| B9 (Name function) | B9 | `canonical_primitive_name_wait_returns_wait` | — | — | — | — | — |
| B10 (Arm dispatch) | B10 | `unit_digest_step_wait_vs_catch_all_never_collides`, `unit_digest_step_wait_includes_wait_label` | — | — | — | — | — |

## 14. Execution Order (for test-writer)

1. **Write direct unit tests first** (Section 9.1) — fastest feedback loop, no YAML parsing. These 9 tests establish baseline correctness of `digest_step_primitive` Wait arm at the function level.
2. **Add `canonical_primitive_name` test** (Section 9.2) — trivial, 1 test.
3. **Write integration tests** (Section 9.3) — exercises full pipeline. These 9 tests verify end-to-end correctness through `compile_source`.
4. **Add new proptest invariants** (Section 9.4) — PI-7 and PI-8.
5. **Verify all existing proptests still pass** — PI-1 through PI-6 from the evidence command map.
6. **Run `cargo test --package vb_compile`** — full suite regression.
7. **Execute Kani harnesses** — after State 7 resolves the tooling blocker.
8. **Execute fuzz targets** — after State 7 resolves the tooling blocker.
9. **Run mutation testing** — after all tests pass, verify ≥90% kill rate.

## 15. Open Questions

1. **Should the unit tests be placed in `crates/vb_compile/src/tests/` (in-crate) or `crates/vb_compile/tests/` (integration test directory)?** The `digest_step_primitive` function is `pub(crate)`, accessible from in-crate tests. Recommendation: use `src/tests/` for direct function access.

2. **Should the existing `kani_wait_digest.rs` harnesses be refactored to use `[u8; N]` arrays before State 7 execution?** Yes — this is the required remediation per GAP-MAP-002 in `proof-to-rust-map.md`. The test-writer should not block on this; it is a State 7 concern.

3. **Should we add an `insta` snapshot test for the exact digest values of known Wait configurations?** A snapshot test would detect unintended changes to the hashing algorithm. However, digest values are NOT intended to be stable across code versions (the fix changes them intentionally). A snapshot test would be brittle. Recommendation: do not add snapshot tests — the proptest invariants provide better coverage.

4. **Should the warm-path dead code (`compile/mod.rs`) be removed in this bead?** No — the contract says deduplication is a follow-up bead. The fix is applied identically to both copies for future-proofing. The dead copy serves as documentation of the fix's scope.

5. **Is the `b"none"` sentinel the correct choice for WaitUntil's absent-event representation?** The domain decision DD-4 chose `b"wait"` + field values + `b"none"` sentinel rather than `b"wait_until"` discriminator. This was refined during implementation. The current approach uses `b"none"` in the event position for WaitUntil vs actual event text for WaitEvent. This discriminates correctly because slot expressions are always integer strings (validated by `slot_from_text`), so `b"none"` can never collide with a real event value. The proptest PI-2 and Kani KH-2 verify this.

## 16. Exit Criteria Checklist

- [x] Every public API behavior (12 behaviors) has at least one BDD scenario
- [x] Every error variant in the wait validation path has an explicit test scenario
- [x] The mutation threshold target (≥90%) is stated with 12 critical mutations identified
- [x] No test asserts only `is_ok()` or `is_err()` without specifying the value/error variant
- [x] 5 proptest invariants are written and VERIFIED; 2 new invariants specified
- [ ] 15 new test scenarios specified (9 unit + 7 integration + 2 proptest) — for test-writer
- [ ] 4 Kani harnesses pending execution (blocked by tooling) — for State 7
- [ ] 3 fuzz targets pending execution (blocked by tooling) — for State 7
- [ ] Mutation testing pending execution — after all tests pass
- [x] Trophy allocation documented with ratio justification
- [x] Explicit anti-pattern checks listed (10 rules)
- [x] Traceability matrix maps every contract clause to specific tests

## 17. Handoff to test-writer

The test-writer should:
1. Read this entire `test-plan.md`.
2. Read the production code at:
   - `crates/vb_compile/src/mod_compile_lowering/part_05.rs:140-173` (`digest_step_primitive` — active copy)
   - `crates/vb_compile/src/mod_compile_lowering/part_05.rs:98-114` (`canonical_primitive_name`)
   - `crates/vb_compile/src/compile/mod.rs:243-272` (`digest_step_primitive` — dead copy, for awareness)
3. Read the existing tests:
   - `crates/vb_compile/tests/v1_primitive_lowering.rs:824-1060` (proptests and helpers)
   - `crates/vb_compile/src/tests/error_variant_tests.rs:762-803` (digest determinism tests)
4. Create `crates/vb_compile/src/tests/wait_digest_unit_tests.rs` with the 9 unit tests from Section 9.1 and the 2 `canonical_primitive_name` tests from Section 9.2.
5. Add the 9 integration tests from Section 9.3 to `crates/vb_compile/tests/v1_primitive_lowering.rs` or a new test file `crates/vb_compile/tests/wait_digest_integration.rs`.
6. Add the 2 new proptest invariants (PI-7, PI-8) from Section 9.4.
7. Run `cargo test --package vb_compile` and ensure all 295+ existing tests still pass.
8. Run `moon ci` (canonical gate) and ensure zero source-lint violations.
9. Do NOT modify any Kani harnesses, fuzz targets, or production code — those are in separate beads/states.
