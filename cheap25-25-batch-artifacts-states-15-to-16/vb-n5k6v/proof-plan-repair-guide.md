# Proof Plan Repair Guide — vb-n5k6v

**bead_id:** vb-n5k6v
**reviewer_skill:** proof-plan-reviewer
**reviewer_invocation_id:** cheap25-vb-n5k6v-p4b-proof-plan-reviewer
**review_state:** 4
**disposition:** STATUS: REJECTED
**finding_count:** 1 blocker
**smallest_state_to_rerun:** 4 (proof-planner)

---

## Summary

The proof plan is structurally sound and ready for execution **except** for
a stale pre-wire baseline tally reference that hard-codes a wrong absolute
number in `PO-WIRE-DELTA-005.expected_evidence`. The +26 delta invariant
is correct, but the specific baseline (924) and post-wire (950) numbers
must be updated to the current actual baseline (1530) and post-wire
expected (1556). The fix is mechanical: replace 924 → 1530 and 950 →
1556 across 5 plan artifacts, then resubmit for review.

---

## Finding F-001 — Stale pre-wire baseline tally (blocker)

**Finding code:** `E_LANE_OBLIGATION_MISMATCH`
**Severity:** blocker
**Artifact:** `proof-obligations.planned.jsonl#PO-WIRE-DELTA-005`
**Disposition:** blocker
**Owner:** proof-planner (cheap25-vb-n5k6v-p4-proof-planner)

### What is wrong

`PO-WIRE-DELTA-005.expected_evidence` requires the post-wire cargo test
summary to match exactly:

```
test result: ok. 950 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in <duration>s
```

The 950 figure is derived from a stale pre-wire baseline of 924 sourced
from two artifacts (`.beads/vb-2bok/qa-report.md:5` and
`.beads/vb-core-atomic-admission/STATE.md:1349`). Both sources correctly
captured the tally at their respective historical times:

- `.beads/vb-2bok/qa-report.md:5` line 5 is the Test Command header
  (`**Test Command:** \`cargo test -p vb_storage --lib\``), not a tally
  line. Line 26 of the same file reports 909 passed; 13 failed (for a
  different profile). The plan's citation to "line 5" for the value 924
  is incorrect.
- `.beads/vb-core-atomic-admission/STATE.md:1349` correctly reports
  `cargo test -p vb_storage --lib - **924 passed; 0 failed**` for the
  May 2026 verification.

Direct execution at 2026-07-01 from the isolated workdir produces:

```
PROPTEST_CASES=1 cargo test -p vb_storage --lib 2>&1 | tail -3
test result: ok. 1530 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s
```

vb_storage has accumulated additional `#[test]` fns between the May
2026 baseline captures and the current bead's planning (2026-07-01). The
**delta of +26 is correct and invariant** (the wire adds exactly the 26
dormant tests in `crates/vb_storage/src/edge_case_tests.rs`), but the
**absolute numbers are stale**.

### How to repair

1. **Re-measure the pre-wire baseline** from the isolated workdir:

   ```bash
   cd /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-n5k6v
   PROPTEST_CASES=1 cargo test -p vb_storage --lib 2>&1 | tail -3
   ```

   Capture the output to `.beads/vb-n5k6v/baseline-report.md` (or a new
   `.beads/vb-n5k6v/evidence/pre-wire-tally.txt`). The expected output
   is `test result: ok. 1530 passed; 0 failed; 0 ignored; 0 measured; 0
   filtered out`.

2. **Update `proof-obligations.planned.jsonl`** row `PO-WIRE-DELTA-005`:

   - In `expected_evidence`: replace the string `950 passed` with
     `1556 passed`, replace `924 (pre-wire baseline` with
     `1530 (pre-wire baseline`, and replace
     `.beads/vb-2bok/qa-report.md:5 + .beads/vb-core-atomic-admission/STATE.md:1349`
     with the new evidence artifact path
     `.beads/vb-n5k6v/evidence/pre-wire-tally.txt`
     (and/or `.beads/vb-core-atomic-admission/STATE.md:1349` flagged as
     the historic May 2026 baseline, not the current pre-wire value).
   - The +26 delta invariant and the anti-invariant language remain
     unchanged: `regression = below 1530`, `partial wire = delta below
     +26`, `test-budget leak = delta above +26`.

3. **Update `contract.md`** clause `CC-WIRE-005`:

   - In the Invariant section: replace `Pre-wire baseline
     (verified ...): 924 tests.` with
     `Pre-wire baseline (verified
     .beads/vb-n5k6v/evidence/pre-wire-tally.txt
     at 2026-07-01): 1530 tests.`
   - Replace `Post-wire expected: 924 + 26 = 950 tests.` with
     `Post-wire expected: 1530 + 26 = 1556 tests.`
   - In the Test pinning line: replace `950 passed` with `1556 passed`.

4. **Update `proof-strategy.md`** §6 Strategy summary:

   - Replace `cargo test -p vb_storage --lib 2>&1 | tail -5 reports 950
     passed (924 pre-wire + 26 delta)` with
     `cargo test -p vb_storage --lib 2>&1 | tail -5 reports 1556 passed
     (1530 pre-wire + 26 delta)`.

5. **Update `proof-coverage-matrix.md`** §1 (line 25) and §8 (line
   161-162):

   - Line 25: replace `test count delta = +26 (924 → 950)` with
     `test count delta = +26 (1530 → 1556)`.
   - Lines 161-162: replace `924 tests` with `1530 tests` and add a new
     row referencing the current evidence artifact path.

6. **Update `trusted-base-plan.md`** §6 (line 41-43), §6 table (lines
   214-215), §12 (lines 368-369), §13 (line 394), §15 (line 436):

   - Replace `N=924` with `N=1530` and `N=950` with `N=1556`.
   - Update the evidence reference at line 41 from
     `.beads/vb-2bok/qa-report.md:5` to the current artifact path.
   - Replace `Pre-wire tally is 924` with `Pre-wire tally is 1530`.
   - Add a note flagging the historic May 2026 baseline (924) in the
     trusted-base-plan.md §13 assumptions table as
     `historic_2026_05_baseline` (not the current pre-wire value).

### Validation after repair

After applying the fixes, run the following from the isolated workdir
(`/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-n5k6v`):

```bash
# Pre-wire (should be 1530)
PROPTEST_CASES=1 cargo test -p vb_storage --lib 2>&1 | tail -3

# Validate the JSONL edit
jq '.expected_evidence' .beads/vb-n5k6v/proof-obligations.planned.jsonl | grep -c '1556'

# Verify no remaining stale '924' or '950' literals in plan artifacts
rtk rg -n '\b924\b|\b950\b' \
  .beads/vb-n5k6v/contract.md \
  .beads/vb-n5k6v/proof-strategy.md \
  .beads/vb-n5k6v/proof-coverage-matrix.md \
  .beads/vb-n5k6v/trusted-base-plan.md \
  .beads/vb-n5k6v/proof-obligations.planned.jsonl
```

The `rtk rg` should return **zero hits** (or only hits within historic
notes explicitly flagged as `historic_2026_05_baseline`).

---

## What does NOT need to change

- **The +26 delta invariant.** This is correct and invariant.
- **The 3-line wire at `lib.rs:182`.** The byte-pattern is correct and
  matches the 16 sibling declarations at `lib.rs:118-181`.
- **The 26 dormant `#[test]` fns in `edge_case_tests.rs`.** Verified
  via `rtk rg "#\[test\]"` returning 26 hits.
- **The 637-line file size.** Verified via `rtk wc -l`.
- **The Cargo.toml unchanged invariant.** Verified via
  `git diff crates/vb_storage/Cargo.toml` (empty, since we have not yet
  touched the file).
- **All 105 verifier-lane-decision rows.** All `required` lanes are
  accepted with concrete obligation refs; all `not_applicable` lanes
  cite concrete evidence refs (production source paths, Kani harness
  precedents, etc.). No weak "out of scope" hand-waving was found.
- **The trusted-base reduction claims** (Fjall `&self` append paths,
  `Mutex<InnerState>` serialization, `tempfile::tempdir()` isolation).
  These are evidence-anchored and verifiable.
- **The no-waiver stance.** The empty `waiver-candidates.jsonl` is
  correct: the bead has zero behavior-affecting obligations requiring a
  waiver.
- **The Verus production-binding gate.** Vacuously satisfied (zero
  Verus obligations in the plan).

---

## Rerun instructions

1. **Apply the fix in the smallest rerun scope:** rerun **State 4
   (proof-planner)** only. States 1-3 (intake, explore, rust-contract)
   are unaffected because the baseline tally is the only delta. Do not
   re-run states 1-3 unless additional drift is found.

2. **The proof-planner must update the 5 affected artifacts** listed
   above, then re-emit `proof-obligations.planned.jsonl` with the new
   `expected_evidence` for `PO-WIRE-DELTA-005`. The other 2 obligations
   (`PO-WIRE-DECL-001`, `PO-WIRE-RUN-004`) are unaffected.

3. **The proof-planner should also add a `trusted-base-plan.md` §13
   note** documenting the historic May 2026 924 baseline as a
   non-current artifact, so that future replays don't confuse the
   historic value with the current pre-wire tally.

4. **Re-submit** by appending a new state4 entry to
   `agent-invocation-ledger.jsonl` (the planner invocation ID
   `cheap25-vb-n5k6v-p4-proof-planner` may remain; the entry_hash must
   be a fresh SHA-256 over the new ledger row).

5. **proof-plan-reviewer (this agent) will re-review** and emit
   `STATUS: APPROVED` if the fix is correctly applied. The
   `verifier-lane-review.jsonl` rows are unchanged from this review
   (all 105 lanes are accepted); only the obligation
   `expected_evidence` text and the supporting plan artifacts change.

---

END OF PROOF PLAN REPAIR GUIDE.