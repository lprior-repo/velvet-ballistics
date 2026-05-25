# Test Suite Review: Wait Digest Coverage — RETRY

**Bead:** vb-xi2f.32
**Date:** 2026-05-25
**Reviewer:** p10-test-reviewer (retry after S1-S3 fixes)
**Schema:** test-suite-review/v1
**Mode:** Suite Review (implementation + tests → review)
**Previous:** REJECTED — S1 contract parity gap, S2 sentinel value unverified, S3 PI-8 property mismatch
**Base:** `contract.md` (DD-4 updated), `test-plan.md`, implementation, and updated test files

## Verdict

**STATUS: APPROVED** — S1, S2, S3 resolved. S4 now mitigated by new exact-sentinel tests.
Four minor documentation inconsistencies noted below (D1-D4); none affect behavioral
coverage or assertion strength.

## Suite Summary

| Layer | Count | Status |
|-------|-------|--------|
| Unit tests (`wait_digest_unit_tests.rs`) | 15 (12 + 3 new) | All pass |
| Integration tests (`v1_primitive_lowering.rs`) | 10 | All pass |
| Pre-existing wait-related proptests | 5 | All pass |
| **Total wait digest tests** | **25 new + 5 existing** | All pass |
| Full `vb_compile` suite | 320 | All pass |

Execution confirmed: `cargo test -p vb_compile` → 320 passed, 0 failed (was 317
before the three new exact-sentinel tests were added).

No ignored tests, no sleeps, no hidden mutable state, no broad mocking, and no
`is_ok()`/`is_err()` assertions without variant matching found.

---

## Findings (Ordered by Severity)

### Resolved: Prior S1 [CRITICAL] → RESOLVED

**Previous:** Contract C2 required `"wait_until"`/`"wait_event"` discriminator strings;
implementation used positional `b"none"` sentinel instead.

**Resolution:**
- `contract.md` C2 (line 21-28): Updated to reflect DD-4 refinement — explicit
  discriminator strings replaced by positional sentinel discriminators. ✓
- New unit test at `wait_digest_unit_tests.rs:297-333`:
  `digest_step_primitive_discriminates_wait_until_from_wait_event_when_event_position_differs`
  verifies WaitUntil vs WaitEvent produce distinct hasher states via the positional
  `b"none"` sentinel in the event position. ✓
- All C2-traceable integration tests (lines 1159-1179) verify the weaker property
  `digest(WaitUntil) != digest(WaitEvent)` — correct under DD-4. ✓
- Kani harness PO-005 (lines 80-120 in `kani_wait_digest.rs`) verifies digest ≠ digest
  without referencing specific discriminator strings — aligned with DD-4. ✓

### Resolved: Prior S2 [HIGH] → RESOLVED

**Previous:** Sentinel tests verified non-collision (`None` ≠ `Some("probe")`)
but not the exact sentinel VALUE `b"none"`.

**Resolution — three new exact sentinel tests added:**

1. `digest_step_primitive_uses_exact_b_none_sentinel_when_event_is_absent` (line 341):
   - Constructs reference hasher with `b"wait" + b"none" + b"30"`
   - Asserts `digest_step_primitive` on WaitUntil (event=None) produces identical hash
   - Any change to sentinel (e.g., `b"nil"`, `b"missing"`, `b""`) breaks `assert_eq!`

2. `digest_step_primitive_uses_exact_b_none_sentinel_when_timeout_is_absent` (line 376):
   - Constructs reference hasher with `b"wait" + b"0" + b"none"`
   - Asserts `digest_step_primitive` on WaitEvent unbounded (timeout=None) produces
     identical hash
   - Same mutation resistance as above

3. The original non-collision tests (`none_event_uses_none_sentinel_when_event_is_absent`
   at line 110 and `none_timeout_uses_none_sentinel_when_timeout_is_absent` at line 142)
   remain as complementary coverage — they verify the sentinel is unambiguous via
   probe comparison; the new tests verify the exact byte value via reference comparison.

### Resolved: Prior S3 [HIGH] → RESOLVED

**Previous:** PI-8 test name claimed "unchanged from pre-fix" but only verified
determinism `compile(case) == compile(case)`.

**Resolution:**
- Test renamed at `v1_primitive_lowering.rs:1337`:
  `proptest_non_wait_workflows_digests_are_deterministic_after_wait_fix`
- Comment at lines 1333-1335 documents: "NOTE: PI-8 does NOT assert 'unchanged
  from pre-fix' — pre-fix baseline comparison is covered by existing regression
  test PI-5 (`proptest_equal_primitive_sources_compile_to_equal_digest_and_ir`)."
- Test correctly verifies determinism on non-Wait workflows (property it actually
  exercises), not "unchanged from pre-fix." ✓

### Resolved: Prior S4 [MODERATE] → NOW MITIGATED

**Previous:** Deleting `hasher.update(b"wait")` from the Wait arm (line 159)
was not caught by `digest_step_wait_includes_wait_label`.

**Current state:** The new exact sentinel tests (S2 fix) now catch this mutation.
- Reference hasher for WaitUntil: `b"wait" + b"none" + b"30"`
- If `b"wait"` is deleted from production code, production hash becomes
  `b"none" + b"30"` — differs from reference → `assert_eq!` fails.
- The test that was originally designed for this (`digest_step_wait_includes_wait_label`
  at line 21) still wouldn't catch it individually, but the exact-sentinel tests
  fill this gap. Coverage is achieved albeit by a different test than originally planned.

### Prior S5 [LOW] → UNCHANGED

Fuzz target `wait_digest_exhaustive_collision.rs` timeout values remain hardcoded
by shape rather than derived from fuzz input bytes. Proptest PI-1 (1024 cases)
covers timeout sensitivity adequately. No action required.

---

## Documentation Inconsistencies (No Behavioral Impact)

### D1. [LOW] domain-model.md DD-4 stale

**File:** `.beads/vb-xi2f.32/domain-model.md:136`
**Issue:** DD-4 still says "hash an explicit 'wait_until' discriminator" and
"hash 'wait_event' + event value" — the pre-DD-4 discriminator-string approach.
**Impact:** contract.md C2 was updated to reflect DD-4 positional sentinel;
domain-model.md was not updated. Documentation divergence only.
**Recommendation:** Update domain-model.md DD-4 to match contract.md C2:
positional `b"none"` sentinel in event position for WaitUntil, actual event text
for WaitEvent.

### D2. [LOW] test-plan.md PI-8 section still has old description

**File:** `.beads/vb-xi2f.32/test-plan.md:250-254`
**Issue:** Section 4 PI-8 still says "Non-Wait workflows produce unchanged digests"
with the old invariant "canonical_digest(workflow) == canonical_digest_pre_fix(workflow)."
The test and contract have been updated to reflect determinism, not pre-fix comparison.
**Impact:** The test-plan.md is a plan artifact; the test itself (line 1337) correctly
describes its property. Traceability is maintained through PI-5. Documentation only.
**Recommendation:** Update test-plan.md PI-8 to match the renamed test: "Non-Wait
workflows produce deterministic digests after the Wait fix."

### D3. [LOW] test-plan.md traceability matrix references old PI-8 semantics

**File:** `.beads/vb-xi2f.32/test-plan.md:589`
**Issue:** C6 row maps PI-8 to "Regression." PI-8 now tests determinism, not
pre-fix regression. C6 coverage should be primarily attributed to PI-5, with
PI-8 listed as supplementary determinism coverage.
**Impact:** Documentation only — PI-5 is correctly listed in the same row.
**Recommendation:** Note in matrix that PI-8 provides determinism coverage
on non-Wait workflows; PI-5 is the primary C6 regression guard.

### D4. [LOW] Stale comment in v1_primitive_lowering.rs

**File:** `crates/vb_compile/tests/v1_primitive_lowering.rs:1157`
**Issue:** Comment says "the explicit discriminator" — refers to pre-DD-4
`"wait_until"`/`"wait_event"` string approach.
**Impact:** Comment only; the assertion is `assert_ne!(digest_until, digest_event)`,
which is correct. The test verifies the correct property.
**Recommendation:** Update comment to "the positional b\"none\" sentinel" for
clarity with DD-4.

---

## Pass Review (What the Suite Does Well)

### Anti-Pattern Compliance

| Rule | Status | Evidence |
|------|--------|----------|
| No `is_ok()` without value assertion | ✅ | All digest assertions use `assert_eq!`/`assert_ne!` on exact hashes |
| No `is_err()` without error variant | ✅ | `wait_invalid_shape_event_none_timeout_none_rejected` matches `CompileError::StepFieldShape { step, field, expected }` with all 3 fields |
| No mock of `blake3::Hasher` | ✅ | `blake3::Hasher::new()` used exclusively |
| No `sleep()` in tests | ✅ | All synchronous |
| Tests are hermetic | ✅ | Each test creates own hasher and primitives |
| Test naming follows pattern | ✅ | All follow `[subject]_[outcome]_when_[condition]` |
| No ignored tests | ✅ | Zero `#[ignore]` attributes |

### Contract Coverage (Post-DD-4)

| Clause | Behaviors Tested | Gap |
|--------|-----------------|-----|
| C1 (Wait field hashing) | B1, B2, B7 | ✅ Unit + integration + proptest all present |
| C2 (WaitUntil/WaitEvent discrimination) | B3 | ✅ Updated to DD-4 positional sentinel; exact sentinel tests verify mechanism |
| C3 (Absent field sentinel) | B4 | ✅ Exact `b"none"` byte sequence verified via reference hasher (S2 fix) |
| C4 (Determinism) | B5 | ✅ Three-time compile test + step-idempotency proptest |
| C5 (Dual consistency) | B6 | ✅ Cross-path proptest (both copies fixed identically) |
| C6 (Regression) | B12 | ✅ PI-5 provides primary regression coverage; PI-8 adds determinism coverage |
| DI-4 (Empty wait invalid) | B11 | ✅ Exact `StepFieldShape { step: 0, field: "wait" }` |
| B9 (Name function) | B9 | ✅ `canonical_primitive_name_returns_wait` + all-variants test |
| B10 (Arm dispatch) | B10 | ✅ Anti-catch-all comparison test |

### Public API Testing

All integration tests use the public API exclusively (`compile_source`,
`compile_workflow`, `YamlCompiler::compile()`). Internal functions
(`digest_step_primitive`, `canonical_primitive_name`) are tested via
in-crate unit tests — correct layering for `pub(crate)` visibility.

### Determinism

- `digest_step_wait_arm_is_deterministic_when_same_input_hashed_twice`: direct
  determinism on `digest_step_primitive`.
- `proptest_wait_digest_step_level_idempotent`: 1024-case proptest same property.
- `wait_digest_is_deterministic_through_compile_source`: three-time compile
  determinism through full pipeline.
- `proptest_non_wait_workflows_digests_are_deterministic_after_wait_fix`: PI-8
  determinism on non-Wait workflows.
- No randomness, no system time, no external state in any test.

### Dual Implementation Consistency

Both copies of `digest_step_primitive` are verified identically fixed:
- `mod_compile_lowering/part_05.rs:158-168` (cold-path)
- `compile/mod.rs:257-267` (warm-path)
- Cross-path proptest PI-6 verifies identical digests at workflow level.

### Error Variant Tests

- `wait_invalid_shape_event_none_timeout_none_rejected_with_step_field_shape`:
  Matches exact `CompileError::StepFieldShape` with `step: 0`, `field: "wait"`,
  and non-empty `expected` message. Gold standard — all three structural fields asserted.

---

## Mutation Resistance Summary (Post-Fix)

| Mutation | Test Plan Says Catches | Actually Caught? | Finding |
|----------|----------------------|-----------------|---------|
| Delete `hasher.update(b"wait")` | `unit_digest_step_wait_includes_label` | **YES** (via exact sentinel tests, S2 fix) | S4 mitigated |
| Delete `hasher.update(e.as_bytes())` | `unit_event_field_affects_hasher_state` | **YES** | ✅ |
| Delete `hasher.update(t.as_bytes())` | `unit_timeout_field_affects_hasher_state` | **YES** | ✅ |
| Change `b"none"` → `b"nil"` | `unit_none_timeout_uses_none_sentinel` | **YES** (via exact sentinel tests, S2 fix) | S2 resolved |
| Remove entire Wait match arm | `integration_until_vs_event_produce_distinct_digests` | **YES** | ✅ |
| Swap event and timeout order | `proptest_wait_field_sensitivity` | **YES** (PI-1 covers ordering) | ✅ |
| Fix only one copy | `cross_path_wait_digest_equivalence` | **YES** | ✅ |

Mutation kill rate estimate: 7/7 = ~100% on tracked mutations.
Above the 90% threshold stated in test-plan.md.

---

## Tooling Status (Not Scored)

The following artifacts are written but execution-blocked (State 7 concern):

| Artifact | Blocked By | Status |
|----------|-----------|--------|
| Kani KH-1, KH-2, KH-3, KH-4 | `Arbitrary` for `String` + musl tooling | Written, pending refactor |
| Fuzz FZ-1 `wait_digest_sensitivity` | musl/sanitizer incompatibility | Written, pending execution |
| Fuzz FZ-2 `wait_sentinel_collision` | musl/sanitizer incompatibility | Written, pending execution |
| Fuzz FZ-3 `wait_digest_exhaustive_collision` | musl/sanitizer incompatibility | Written, pending execution |

The 4 Kani harnesses and 3 fuzz targets are well-structured, use `kani::any()`
(GOD RULE 1), and are aligned with the DD-4 contract (no references to old
discriminator strings). They will strengthen coverage once tooling is resolved.

---

## Required Actions

None required for behavioral coverage. The four documentation inconsistencies
(D1-D4) are recommendations only:

1. **[D1] Recommended:** Update `domain-model.md` DD-4 to reflect positional sentinel
   approach (match contract.md C2).
2. **[D2] Recommended:** Update `test-plan.md` PI-8 section to match the renamed
   test's determinism description.
3. **[D3] Recommended:** Note in `test-plan.md` traceability matrix that PI-8
   provides determinism coverage; PI-5 is primary C6 regression guard.
4. **[D4] Recommended:** Update comment in `v1_primitive_lowering.rs:1157` from
   "explicit discriminator" to "positional b\"none\" sentinel."

---

## Extraneous Non-Findings (Carried Forward from Previous Review)

- **Loop in `digest_step_wait_no_panic_for_three_legal_shapes`:** Acceptable.
- **Loop in `canonical_primitive_name_returns_non_empty_distinct_name_for_every_variant`:** Acceptable.
- **`prop_filter` in strategies:** Used correctly.
- **`expect` in helper functions:** Test-only helpers; acceptable.
- **`unwrap_or(0)` in hash-byte extraction:** Test data generation only.
- **PI-8 filter of `save_`, `do_`, `choose_`:** Correct — these produce `UnsupportedStepPrimitive`.
- **Fuzz target `fuzz_lib_build_workflow` helper:** Deterministic, no mutable state.
- **New exact sentinel tests (lines 341, 376):** Use known reference hashers with
  explicit byte sequences. Deterministic per contract C3. Acceptable.
- **New C2 discriminator test (line 297):** Verifies positional sentinel distinctness
  via `assert_ne!`. Acceptable — exact byte verification is covered by the exact
  sentinel tests (lines 341, 376), keeping the C2 test focused on the discrimination
  property.
