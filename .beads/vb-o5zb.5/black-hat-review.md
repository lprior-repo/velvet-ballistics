# Black Hat Review: vb-o5zb.5 (closure reconciliation packet)

**Bead**: vb-o5zb.5
**State**: 13 (black-hat-review of audit)
**Reviewer**: black-hat-reviewer
**Source checkout**: `/home/lewis/src/velvet-ballistics`
**Branch**: `process/vb-63st6.2-worktree-loom-route`
**Commit under review**: `c7cc2850`
**Attempt**: 1

## Header

**Verdict candidate**: REPAIR-ROUTED

The audit packet is mostly accurate, evidence-backed, and process-compliant. ACCEPTED on vb-o5zb.3 is sound. ROUTE-TO-REPAIR on vb-o5zb.1 (stale SpecTaint) and vb-o5zb.4 (collect timeout test gap) are well-supported. However, the ROUTE-TO-REPAIR verdict on vb-o5zb.2 (terminal StepState) suffers from a **critical cross-bead conflict with vb-kr3l7** that the packet fails to acknowledge, and the ACCEPTED verdict on vb-o5zb.3 contains a **technical inaccuracy** in the cited Verus type annotation. Both issues are tractable but neither is optional.

---

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
|-------------|--------|----------|
| Packet maps vb-o5zb.1-.4 to source/test/proof with file:line refs | PASS | All four child sections present with explicit citations |
| Each child explicitly ACCEPTED or ROUTE-TO-REPAIR | PASS | All 4 verdicts explicit, no equivocation |
| Parent decision documented | PASS | "Recommend PARENT REMAINS BLOCKED" |
| Repair beads filed for ROUTE-TO-REPAIR children | PASS | vb-yurs3 (P0), vb-53k3r (P0), vb-izu26 (P1) all open and blocks-linked to vb-o5zb |
| Cross-bead conflicts acknowledged | FAIL | **CRITICAL**: vb-53k3r contradicts vb-kr3l7 (closed same day) - packet does not reconcile |
| Source/test/proof evidence verified to exist | PARTIAL | Most refs verified; one inaccuracy at `encoding_injectivity.rs:164` (claimed `[&str; 18]`, actual `[[[&str; 17]str; 18]str; 18]`) |

---

## PHASE 2: ACCEPTED Audit (vb-o5zb.3 ResourceContract)

**Verdict soundness**: ACCEPTED - with one technical inaccuracy that does not flip the verdict.

### Evidence verified

| Claim | File:Line | Verified | Notes |
|-------|-----------|----------|-------|
| 18 fields in `ResourceContract` | `crates/vb_core/src/workflow/types.rs:167-206` | PASS | 18 fields counted (max_steps ... allows_secret_results) |
| DEFAULT tightened to Phase 45 | `crates/vb_core/src/workflow/types.rs:208-230` | PASS | max_steps=1_000, max_constants=8_192, max_retry_attempts=3, max_fanout=64, max_collect_items=1_024 |
| Verus array count fixed to 18 | `verification/verus/vb_compile/encoding_injectivity.rs:164` | PARTIAL | Array literal HAS 18 elements (correct count), but the type annotation is `[[[&str; 17]str; 18]str; 18]` - **NOT `[&str; 18]` as claimed** |
| Lemma bounds `i < 18 && j < 18` | `encoding_injectivity.rs:189-191` | PASS | Confirmed |
| Kani harness asserts Err for allow_true | `kani_resource_contract_validation_18_fields.rs:120-134` | PASS | Confirmed at lines 122-128 |
| Stale "17 fields" comments remain | `contract_encoding.rs:17,28`, `digest_contract_binding.rs:131`, `resource_contract_type_integrity.rs:5,11,14,15,23,193,237` | PASS | Confirmed (P2 doc drift, non-behavior-affecting) |

### Finding F-A1: Verus type annotation claim is technically wrong

**Location**: `verification/verus/vb_compile/encoding_injectivity.rs:164`
**Packet claim**: `pub const CONTRACT_FIELD_TAGS: [&str; 18] = [` with 18 elements
**Actual**: `pub const CONTRACT_FIELD_TAGS: [[[&str; 17]str; 18]str; 18] = [` (18 elements, garbled type)
**Severity**: LOW (does not flip verdict - the count is 18 which is the property the original P0-F3 wanted; the Verus spec is not rustc-checked so the garbled type may be a known Verus quirk, but the audit's specific claim is wrong)
**Required fix**: Either correct the type annotation to `[&str; 18]` and verify Verus still accepts, OR amend the audit's evidence citation to clarify that the **array count** is 18 (the property black-hat-review P0-F3 actually flagged).

### Finding F-A2: "All P0 findings have been subsequently resolved" is partly true

The audit's claim that P0-F3 was resolved is true for the lemma bounds and the array literal count. The Kani harness (P0-F7) was fixed. P0-F1 (false claim about compiled_workflow.rs DEFAULT) was resolved by the type-count fix. P0-F2 (P2 doc fixes claimed but not applied) is acknowledged in the packet as residual P2 drift. ACCEPTED verdict is sound.

---

## PHASE 3: ROUTE-TO-REPAIR Audit

### 3.1 - vb-o5zb.1 (Stale SpecTaint) -> vb-yurs3

**Verdict soundness**: ROUTE-TO-REPAIR is correct. The evidence is verified.

| Claim | Verified |
|-------|----------|
| Production `Taint` enum is 3 variants | PASS `value.rs:14-21` |
| Zero production `Taint::Random` / `Taint::TimeDependent` | PASS grep returns 0 matches in `crates/vb_core/src/` |
| Verus spec has 5-variant `SpecTaint` | PASS `verification/verus/run_frame_invariant.rs:581-587` defines `Clean, DerivedFromSecret, Secret, Random, TimeDependent` |
| 28 SpecTaint references in Verus spec | PASS Counted via grep |
| Stale comment lines at `kani_taint_propagation.rs:199,213,220` | PASS Verified |
| Review chain incomplete (no proof-review, no black-hat-review) | PASS Confirmed via ls of isolated worktree |

**Verdict**: ROUTE-TO-REPAIR correctly classified. vb-yurs3 (P0) is filed and blocks vb-o5zb. The GOD RULE 2 disconnect (Verus model not equal to production enum) is real and documented. No conflicts.

### 3.2 - vb-o5zb.4 (Collect timeout) -> vb-izu26

**Verdict soundness**: ROUTE-TO-REPAIR is **partially sound** - the F1/F2/F3/F4 mapping is correct, but the audit is overly pessimistic about F4 by failing to acknowledge the partial test that DOES exist.

| Claim | Verified |
|-------|----------|
| `from_journal: bool` field added to `CollectPaginationState` | PASS `state.rs:45` |
| `upsert_started_collect` reuses `existing.start_millis` when `existing.from_journal == true` | PASS `mod.rs:172-181` |
| F1 PARTIAL: `millis_since_epoch` still uses `SystemTime::now()` | PASS `mod.rs:280-289` |
| F2 NOT MET: `std::time::SystemTime` imported at `mod.rs:12` | PASS |
| F4 NOT MET: "tests at tests.rs:1386,1408,1427 only have from_journal: false" | PASS confirmed for those 3 lines |
| "NO test that creates a state with from_journal: true and asserts start_millis is preserved" | PARTIAL: A test at `tests.rs:1494` (`collect_states_from_journal_flag_preserved_through_upsert`) DOES create state with `from_journal: true`, `start_millis: 1234567890`, calls `states.upsert()`, and asserts `start_millis == original_start_millis`. However, this test uses `states.upsert()` (the lower-level data-structure method) rather than `upsert_started_collect` (the runtime handler that the proof-review F4 specifically demanded). The F4 gap is REAL but PARTIAL. |

**Finding F-C1**: F4 phrasing is misleading

**Location**: `closure-reconciliation-packet.md:98` and `:138`
**Packet claim**: "NO test that creates a state with `from_journal: true` and asserts `start_millis` is preserved through `upsert_started_collect`"
**Actual**: A test at line 1494 covers the data-structure level preservation but NOT the `upsert_started_collect` runtime handler.
**Severity**: LOW (does not flip ROUTE-TO-REPAIR verdict; the audit is right that the runtime handler path is untested)
**Required fix**: Tighten the F4 wording to "no test exercises the `upsert_started_collect` runtime handler's `from_journal` short-circuit" and acknowledge the data-structure-level test at line 1494 as partial coverage.

**Verdict**: ROUTE-TO-REPAIR correctly classified. vb-izu26 (P1) is filed. Note: the test gap is narrower than the audit claims.

### 3.3 - vb-o5zb.2 (Terminal StepState) -> vb-53k3r - **CRITICAL CONFLICT**

**Verdict soundness**: ROUTE-TO-REPAIR is technically accurate but **incomplete** - the audit fails to acknowledge that the previous fix (vb-kr3l7, closed 2026-06-06T18:10:07Z, ~2.5 hours before this audit) deliberately established the production code as the authoritative contract.

| Claim | Verified |
|-------|----------|
| `frame.rs:54` has `(StepState::Succeeded, StepState::Running)` | PASS `frame.rs:54` |
| `vb_proof_kernels/src/step_state.rs:48` has same | PASS `step_state.rs:48` |
| `vb_proof_kernels/src/step_state.rs:105-115` has Succeeded special case in `terminal_cannot_transition_to_non_terminal` | PASS `step_state.rs:102-120` (matches enum match at line 109-111) |
| `integration_step_behavior.rs:1324-1327` asserts Succeeded->Running is valid | PASS test renamed by vb-kr3l7 to `succeeded_to_running_is_valid_transition_for_loop_reentry` |
| 4 Kani harnesses still encode exception | PASS file refs are reasonable; spot-checked `frame.rs:1101` which has the comment "Succeeded may transition to Running for loop reentry" |
| proof-review STATUS: REJECTED | PASS Confirmed at `/home/lewis/src/vb-isolated/vb-o5zb.2/.beads/vb-o5zb.2/proof-review.md` |
| black-hat-review STATUS: REJECTED | PASS Confirmed at same path |

**But**: vb-kr3l7 (closed 2026-06-06T18:10:07Z, before this audit at 2026-06-06T13:12:46Z in `c7cc2850`) explicitly rationalized the production code as the contract:

> "The committed production contract at crates/vb_core/src/frame.rs:36-62 admits Succeeded->Running as a valid transition for loop body re-entry. The test succeeded_to_running_is_invalid_transition asserted the opposite direction, and the matching Kani harness ... and the TLA+ spec ... still modelled Succeeded as fully absorbing. Repair all three to honour the production contract."

**This is a direct contract reversal**: vb-kr3l7's commit message says "the production contract ADMITS Succeeded->Running ... honour the production contract." The audit packet (vb-53k3r) says "the production bug is STILL present ... remove (Succeeded, Running) from VALID_TRANSITIONS." These two are in direct opposition.

**The audit's narrow technical observation is correct** (the production code does have Succeeded->Running; the test/Kani/TLA+ now match it), **but the audit's INTERPRETATION as a "bug to be removed" reverses the most recent deliberate decision**. The audit never mentions vb-kr3l7 in any of:
- The child 2 evidence verdict
- The Topic Coverage -> Terminal StepState section
- The repair bead vb-53k3r's description (which actually IS aware of vb-kr3l7 but treats the situation as "vb-kr3l7 fixed tests/specs, production bug still present" rather than "vb-kr3l7 reversed the contract interpretation")

**Finding F-B1 - CRITICAL**: Cross-bead conflict with vb-kr3l7 is unacknowledged

**Location**: `closure-reconciliation-packet.md:42-58, 111-116`
**Severity**: HIGH
**Problem**: The audit packet's diagnosis of vb-o5zb.2 as ROUTE-TO-REPAIR directly contradicts vb-kr3l7 (closed 2.5 hours before this audit). vb-kr3l7's commit message is explicit: "the production contract ... admits Succeeded->Running ... honour the production contract." The audit packet does not engage with this, treats the production code as a "bug," and files vb-53k3r to undo what vb-kr3l7 just ratified. There are two valid interpretations:

1. **Production is wrong, master is right** (audit packet's position): Succeeded->Running must be removed. But then vb-kr3l7 was a regression that should be re-reverted. The audit does not say this.
2. **Production is right, master must be updated** (vb-kr3l7's position): Succeeded->Running is the production contract. The master doc (`velvet-ballistics-MASTER.md:528`) and vb-o5zb.2 acceptance_criteria ("absorbing except explicitly allowed idempotent self-mark behavior") need to be amended to admit "loop body reentry" as a documented exception. vb-53k3r's prescription (remove the production transition) is then wrong.

**Required fix**: The audit packet must explicitly state which interpretation it endorses and reconcile the conflict. Either:
- (a) Add a finding that vb-kr3l7's contract reversal is itself the regression, and the ROUTE-TO-REPAIR verdict stands; or
- (b) Acknowledge that the contract has been reversed and the appropriate repair is a master-doc update (a different repair bead, not vb-53k3r as filed); or
- (c) Explicitly waive the Succeeded->Running exception as the production contract and re-classify vb-o5zb.2 as ACCEPTED-with-master-doc-update-pending.

**Without this reconciliation, the audit is internally inconsistent and the filed repair bead (vb-53k3r) is at best mis-scoped and at worst actively harmful** (it will require reverting the just-closed vb-kr3l7 work).

---

## PHASE 4: Process Compliance

| Check | Status | Notes |
|-------|--------|-------|
| Structure: per-child Source/Tests/Proof/Review/Evidence verdict | PASS | All 4 children have all 5 sections |
| Each topic explicitly ACCEPTED or ROUTE-TO-REPAIR | PASS | 1 ACCEPTED, 3 ROUTE-TO-REPAIR, no equivocation |
| Parent decision documented | PASS | "PARENT REMAINS BLOCKED" with rationale |
| Repair beads filed for each ROUTE-TO-REPAIR topic | PASS | vb-yurs3 (P0), vb-53k3r (P0), vb-izu26 (P1) - all open, blocks-linked to vb-o5zb |
| Smoke test commands documented | PASS | Table at lines 156-165 |
| Cross-bead conflicts surfaced | FAIL | vb-kr3l7 not mentioned anywhere in the packet |
| Verus/Kani/Flux evidence verified against current source | PARTIAL | Most verified, one technical inaccuracy at `encoding_injectivity.rs:164` |

**Parent decision soundness**: Keeping `vb-o5zb` BLOCKED is correct given 3 active repair dependencies. Could the parent be split into `vb-o5zb.A` (closed: vb-o5zb.3 only) and `vb-o5zb.B` (still open: the 3 repair children)? Possibly, but the audit correctly chooses the more conservative path of "remain blocked." Not a defect.

---

## PHASE 5: Adversarial Findings (Ordered by Severity)

### [FINDING-001] CRITICAL - Cross-bead conflict vb-53k3r vs vb-kr3l7 is unacknowledged

**Location**: `closure-reconciliation-packet.md:42-58, 111-116, 148` (and `vb-53k3r` description)
**Problem**: vb-kr3l7 (closed 2026-06-06T18:10:07Z, ~2.5 hours before this audit) explicitly established the production Succeeded->Running transition as the authoritative contract via its commit message. The audit packet (vb-53k3r) reverses this without acknowledgment. Two valid interpretations exist (production-wrong-vs-master-right vs production-right-vs-master-wrong); the audit picks one silently and files a repair bead (vb-53k3r) that will require reverting the just-closed vb-kr3l7 work.
**Evidence**: `git show 376a18a46` shows vb-kr3l7's commit message contains the verbatim phrase "the production contract ... admits Succeeded->Running as a valid transition for loop body re-entry."
**Required fix**: Audit must either (a) argue vb-kr3l7 was itself a regression, (b) reframe the repair as a master-doc update, or (c) reclassify vb-o5zb.2 as ACCEPTED-with-master-doc-update-pending.

### [FINDING-002] HIGH - Verus type annotation claim is technically wrong

**Location**: `closure-reconciliation-packet.md:75`
**Problem**: Packet claims `pub const CONTRACT_FIELD_TAGS: [&str; 18]` at `encoding_injectivity.rs:164`. Actual: `pub const CONTRACT_FIELD_TAGS: [[[&str; 17]str; 18]str; 18]`. The array literal has 18 elements (which was the P0-F3 fix), but the type annotation is garbled. The packet's evidence citation is wrong.
**Required fix**: Either correct the type annotation in source and re-verify Verus still accepts, or amend the audit's wording to clarify that the **array count** is 18 (the property that mattered) rather than the garbled type string.

### [FINDING-003] MEDIUM - vb-o5zb.4 F4 phrasing overstates the test gap

**Location**: `closure-reconciliation-packet.md:98, 138`
**Problem**: Packet claims "NO test that creates a state with from_journal: true and asserts start_millis is preserved through upsert_started_collect." The reality: `tests.rs:1494` (`collect_states_from_journal_flag_preserved_through_upsert`) DOES create such a state and DOES assert start_millis preservation - but through `states.upsert()` (the data-structure method), not `upsert_started_collect` (the runtime handler). The F4 gap is real but partial.
**Required fix**: Tighten the F4 wording to specify that the **runtime handler** path (`upsert_started_collect`) is untested; acknowledge the data-structure-level coverage as partial.

### [FINDING-004] LOW - `kani_resource_contract_validation_18_fields.rs:120-134` is correctly cited

The Kani harness for `prove_allows_secret_results_valid_bool_accepted` does assert `result.is_err()` when `allow_true == true` (lines 122-128). The audit's ACCEPTED verdict on this P0 finding is verified and sound.

### [FINDING-005] LOW - `vb-o5zb.1` formal-verification-report.md "163 matches" claim is unverified but not material

The audit notes that the formal-verification-report.md said "163 matches across 13 files" but the current source has 0 production matches. The audit correctly notes this is a stale or pre-fix claim. The actual Verus spec still has SpecTaint::Random/TimeDependent (28 references, all in `verification/verus/run_frame_invariant.rs`). The packet's diagnosis is correct, the exact "163" number is unverifiable but not material to the ROUTE-TO-REPAIR verdict.

### [FINDING-006] LOW - Parent could be split into `vb-o5zb.A` (closed) and `vb-o5zb.B` (open)

The audit chose to keep `vb-o5zb` BLOCKED with 3 new repair dependencies. An alternative would be to split: close a `vb-o5zb.A` parent covering just the ACCEPTED vb-o5zb.3, and create `vb-o5zb.B` covering the 3 ROUTE-TO-REPAIR children. The audit's choice is conservative and acceptable. Not a defect.

---

## Quality Gates

| Gate | Status | Evidence |
|------|--------|----------|
| All 4 children mapped to source/test/proof | PASS | Lines 15-107 of packet |
| Each child has ACCEPTED or ROUTE-TO-REPAIR verdict | PASS | Lines 35, 59, 84, 107 |
| Parent decision documented | PASS | Lines 141-150 |
| Repair beads filed | PASS | vb-yurs3, vb-53k3r, vb-izu26 all open, blocks vb-o5zb |
| Cross-bead conflicts surfaced | FAIL | FINDING-001 |
| Evidence spot-checked against current source | PASS | 6 file:line refs verified directly |
| No `unwrap`/`expect`/`panic`/`todo`/`unimplemented` in audit doc | PASS | Audit is markdown, no Rust |
| No hallucinated evidence | PARTIAL | One technical inaccuracy (FINDING-002); no outright hallucinations |

---

## Verdict

**STATUS: REPAIR-ROUTED**

### Summary

The audit packet is a mostly thorough, evidence-backed reconciliation that correctly classifies 1 of 4 children as ACCEPTED and 3 as ROUTE-TO-REPAIR. The ACCEPTED verdict on vb-o5zb.3 is sound despite a minor technical inaccuracy in the Verus type citation. The ROUTE-TO-REPAIR verdicts on vb-o5zb.1 and vb-o5zb.4 are well-supported. The ROUTE-TO-REPAIR verdict on vb-o5zb.2, however, contains a critical cross-bead conflict with vb-kr3l7 (closed ~2.5 hours before this audit) that the audit never acknowledges - vb-kr3l7 explicitly established the production Succeeded->Running transition as the authoritative contract, and the audit silently reverses that decision without engaging with the conflict. The audit must reconcile the vb-53k3r-vs-vb-kr3l7 tension before its repair routing can be trusted. Two smaller findings (F-A1, F-C1) tighten citations but do not flip verdicts.

### Required Repair Actions

1. **[CRITICAL - FINDING-001]**: Reconcile the vb-53k3r vs vb-kr3l7 cross-bead conflict. Either (a) add a finding that vb-kr3l7 was itself a regression and ROUTE-TO-REPAIR stands, (b) reframe the repair as a master-doc update (a different repair bead), or (c) reclassify vb-o5zb.2 as ACCEPTED-with-master-doc-update-pending.
2. **[HIGH - FINDING-002]**: Either correct the Verus type annotation at `verification/verus/vb_compile/encoding_injectivity.rs:164` to `[&str; 18]` and re-verify, or amend the audit's evidence citation to clarify that the array count (not the type string) is what was fixed.
3. **[MEDIUM - FINDING-003]**: Tighten the vb-o5zb.4 F4 wording to acknowledge the partial data-structure-level test at `tests.rs:1494` and specify that the **runtime handler** path (`upsert_started_collect`) is the actual untested surface.
4. **[LOW - FINDING-006]**: Consider splitting `vb-o5zb` into `vb-o5zb.A` (closed: vb-o5zb.3 only) and `vb-o5zb.B` (open: 3 repair children) for cleaner dependency tracking. Optional, not blocking.

### Repair Routing

- **Audit state**: `REPAIR-ROUTED` (back to author for the 3 mandatory repairs above)
- **Do not** ship this audit as-is. FINDING-001 will create a follow-up conflict when vb-53k3r work begins (it will require reverting vb-kr3l7's just-closed work, which the author may not realize).
- **Once FINDING-001 is resolved**, the audit can proceed to APPROVED.

---

## Evidence

- **Source checkout**: `/home/lewis/src/velvet-ballistics`
- **Branch**: `process/vb-63st6.2-worktree-loom-route`
- **Commit under review**: `c7cc2850f` ("audit(vb-o5zb.5): produce child-closure reconciliation packet")
- **Files directly read**:
  - `.beads/vb-o5zb.5/closure-reconciliation-packet.md` (172 lines, full read)
  - `crates/vb_core/src/frame.rs:1-80, 1318-1337, 1095-1104` (transition table, integration test, Kani harness)
  - `crates/vb_proof_kernels/src/step_state.rs:1-120` (transition table + terminal function)
  - `verification/verus/run_frame_invariant.rs:580-707` (SpecTaint 5-variant def + lemma refs)
  - `crates/vb_core/src/value.rs:14-21` (production Taint 3-variant def)
  - `crates/vb_core/src/workflow/types.rs:167-230` (ResourceContract 18 fields + DEFAULT)
  - `verification/verus/vb_compile/encoding_injectivity.rs:1-220` (CONTRACT_FIELD_TAGS garbled type)
  - `crates/vb_core/src/kani_resource_contract_validation_18_fields.rs:100-149` (allows_secret_results harness)
  - `crates/vb_runtime/src/primitives/collect/state.rs:1-50` (CollectPaginationState with from_journal)
  - `crates/vb_runtime/src/primitives/collect/mod.rs:1-25, 160-279` (upsert_started_collect + from_journal short-circuit)
  - `crates/vb_runtime/src/primitives/collect/tests.rs:1380-1604` (existing tests, F4 gap analysis)
  - `crates/vb_core/src/kani_taint_propagation.rs:195-225` (stale TimeDependent comments)
  - `crates/vb_core/src/engine/tests/integration_step_behavior.rs:1318-1337` (Succeeded->Running test, renamed by vb-kr3l7)
  - `.beads/vb-o5zb.4/proof-review.md` (F1/F2/F3/F4 findings - full read)
- **Beads queried via `bd show --json`**: vb-kr3l7, vb-o5zb, vb-o5zb.1, vb-o5zb.2, vb-o5zb.3, vb-o5zb.4, vb-yurs3, vb-53k3r, vb-izu26
- **Commit inspected**: `376a18a46` (vb-kr3l7 - "fix step-state test to allow Succeeded->Running for loop re-entry")
- **Commit inspected**: `e88d71ab4` (vb-o5zb.4 closing - added 24 lines to tests.rs including the line 1494 partial test)
