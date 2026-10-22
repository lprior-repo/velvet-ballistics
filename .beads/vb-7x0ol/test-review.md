# Test Review — vb-7x0ol (Independent Integration Review)

**Bead**: vb-7x0ol
**Reviewer**: local-5090 independent reviewer (DeepSeek v4 Pro via OMP)
**Scope**: suite review (implementation + tests), not proof artifacts.
**Source**: frozen merge `08af7113e984fa9ddaa0103d23faa57afcf33ac8`
(integration `90ab770633c2130501ae2db889374c205a960d69` + digest `c789b1b35cb5d19a60cf2f786cbda5947109dcb1`).
Final candidate `1e7aeee2495f2cfca0e6c1e7719727dc7515ea0a` audited; blockers persist.

## Gate Result

**STATUS: REJECTED**

---

## Findings (Ordered by Severity)

### T-1 [BLOCKER] C6 acceptance test is not executable — YAML parsed as JSON

**Location**: `crates/velvet_ballistics/tests/vb_md3w_retained_input_recovery.rs:41,49`

**Problem**: `submit()` runs `--emit yaml` then `serde_json::from_str(stdout)`. `--emit yaml`
produces YAML via `serde_saphyr::to_string` (`output.rs::json_out`), so the parse fails before
any recovery assertion is reached. The test cannot demonstrate "public CLI retained input
recovery" — it fails at the submission harness, not at the behavior under test.

**Evidence** (real subprocess, `CARGO_BUILD_JOBS=2`, timeout 240s, full log preserved):
```
cargo test -p velvet-ballistics --test vb_md3w_retained_input_recovery -- --nocapture
exit code 101
test retained_inputs_recovered_from_cold_storage_with_runtime_api ... FAILED
Error: Error("expected value", line: 1, column: 1)
```
Log: `.beads/vb-7x0ol/evidence/c6_retained_input_failure.log`.

**Required Fix**: parse YAML (or emit/parse a format the CLI truly outputs) and re-run; then the
test's later assertions (`retained.inputs() == [SlotValue::I64(value)]`,
`RuntimeRecoveryProduct::Resumable`, `run_id` equality) must actually pass.

---

### T-2 [HIGH] Digest "collision oracles" are mostly not regression oracles for the new framing

**Location**: `crates/vb_compile/tests/digest_framing_collision.rs`

**Problem**: Of the 7 tests, only `nested_step_id_output_framing_collision_regression` is a true
old-scheme oracle (step id `"a"`+output `"setb"` vs `"aset"`+`"b"` collide under unframed
concatenation). `finish_string_vs_integer_type_tag` does **not** discriminate old vs new:
under the old scheme `Finish(String "0")` hashed `b"finish" + b"0"` while `Finish(Integer 0)`
hashed `b"finish" + [0u8; 8]` — they already differed before type tags were added. The
coordinator's concern ("scalar String0/Integer0 already differ without type new tags") is
confirmed: these tests would pass even if the type-tag framing were removed, so they do not
guard the claimed injectivity improvement.

**Required Fix**: add true collision-oracle pairs that collide under the *old* encoding and
diverge under the *new* one (e.g., ambiguous string/sequence/mapping boundary cases), and a
future-unknown-variant fail-closed oracle once F-4 is fixed.

---

### T-3 [MEDIUM] `foreach_digest_tests` assertion flip is un-oracled

**Location**: `crates/vb_compile/src/tests/foreach_digest_tests.rs:129-160`

**Problem**: `foreach_at_once_none_some1_produces_identical_*` tests were changed from
`assert_eq!` to `assert_ne!` (None vs Some(1) now must differ). This flips a previously
asserted semantic-equivalence contract without an independent oracle establishing that the two
source forms are genuinely distinct identities rather than equivalent spellings. If the
compile layer lowers both to the same IR, this creates digest instability. The test now
asserts the implementation's new behavior tautologically.

**Required Fix**: document/verify the lowering semantics; assert against the compiled (IR)
digest, not just the source digest, to prove whether None/Some(1) are semantically distinct.

---

### T-4 [HIGH] No public-API test exercises the retry durable path or registered execution

**Location**: (absence) `crates/velvet_ballistics/tests/`, `crates/workspace_tests/tests/`

**Problem**: `cmd_retry` (`run_ops.rs:59`) returns "durable retry execution is unavailable" for
every valid retry; no test asserts the opposite (a successful durable retry with
`RunCheckpointV1` persistence + cold reopen + exact result). Likewise
`execute_registered_action`/`reconcile_registered_action` have no in-repo caller, so no test
exercises "recovery reconciles receipts and never re-runs a live effect" via public API. The
executor-conformance test (`workspace_tests/tests/vb_7r7jv_executor_conformance.rs`) grows in
the new candidate but does not wire the runtime dispatch surface.

**Required Fix**: add public-API integration tests that (a) retry a genuinely failed step
through the CLI and observe a resumed/retried result, and (b) recover a run with a pending
registered action and assert only receipt reconciliation occurs (no second target write).

---

### T-5 [LOW] Digest unit tests re-pin byte-level encodings that will churn with any framing change

**Location**: `crates/vb_compile/src/tests/digest_unit_tests.rs`,
`crates/vb_compile/src/tests/wait_digest_unit_tests.rs`

**Observation**: the updated tests hard-code the new byte layout (type tags, length prefixes,
`TAG_NONE`/`TAG_SOME`) as exact expected byte arrays. These are brittle to any future framing
revision and assert implementation detail rather than observable identity; they also encode the
saturating `unwrap_or(u32::MAX)` length inline, coupling the test to the F-4 defect. Not a
blocker on its own, but they should assert identity semantics (equal/different) plus a few
targeted golden vectors rather than re-deriving the whole wire format.

---

## Suite Review Gates

| Gate | Result | Evidence |
|------|--------|----------|
| Tests compile & run deterministically | ❌ C6 test fails at runtime | T-1 |
| Integration tests use public API | ⚠ C6 uses CLI + public Runtime; but harness broken | T-1 |
| Assertions concrete (not `is_ok`) | ✅ mostly; recovery uses exact-value asserts | source read |
| Mutation resistance | ❌ no oracle for None/Some(1) flip, no true type-tag oracle | T-2/T-3 |
| No ignored/silent suppression | ✅ none observed in reviewed files | — |
| Durable retry / registered exec covered by public API | ❌ absent | T-4 |

---

## Verdict

**STATUS: REJECTED**

The C6 acceptance test cannot run (T-1), the digest collision suite does not actually guard the
new framing's injectivity (T-2), and the retry durable path plus registered-execution recovery
have no public-API coverage at all (T-4). The final candidate `1e7aeee2` does not change the C6
helper (T-1 persists) and does not wire `execute/reconcile_registered_action` (T-4 persists).

## Required Repair Actions

1. [BLOCKER] Fix the C6 test harness YAML/JSON mismatch and run to green.
2. [HIGH] Add true old-scheme collision oracles and a fail-closed-unknown-variant oracle.
3. [HIGH] Add public-API tests for durable retry and registered-action reconciliation.
4. [MEDIUM] Resolve/verify the None-vs-Some(1) digest semantic question.
5. [LOW] Prefer identity-semantic assertions over full wire-format golden bytes.
