# Unverified Landing Bundles Ledger

**Created by**: vb-zg5sb agent
**Date**: 2026-08-30
**Purpose**: Mark all unverified landing bundles as blockers until proper evidence is approved.
**Authority**: AGENTS.md Formal Verification Mandates + proof-reviewer skill standards

## Blocker Status

**All entries in this ledger are BLOCKED for landing until evidence is re-approved.**
This ledger must be consulted before any landing bundle is pushed to main or a production bookmark.

---

## Category 1: TLA+ Spec Rejections (tla-spec-audit)

All 6 TLA+ specs reviewed in `tla-spec-audit/proof-review.md` are **REJECTED** with LETHAL findings.

| Spec File | Blocker Count | Key LETHAL Issues |
|-----------|--------------|-------------------|
| AskAnswerLifecycle.tla | 3 | Vacuous MonotonicSeqNo, PendingSubset subsumed by TypeOK, temporal properties not verified (fairness missing from cfg) |
| RetryFSM.tla | 2 | Vacuous NoStaleCompletion, liveness not verified, toy model bounds (RunId={1}, StepId={1,2}) |
| RetryJournal.tla | 2 | Vacuous JournalIdempotency (guaranteed by action guard), no THEOREM statements |
| LifecycleJournal.tla | 3 | No THEOREM statements, ReplayBitIdentical incomplete, fairness in spec not in cfg |
| ResumeStateMachine.tla | 2 | No THEOREM statements, NoDoubleRunning/FailedNotResumable structurally guaranteed |
| admission_header_before_ack.tla | 1 | No THEOREM statements, CHECK_DEADLOCK TRUE conflicts with TerminalStutter |

**Global findings (all specs):**
- 35 of 48 non-bead `.tla` files lack TypeOK invariants (73% failure rate)
- 4 of 6 specs have liveness claims with no PROPERTIES in cfg
- All specs use toy bounds (1-2 runs) that cannot catch cross-run interference bugs

**Blocker**: None of these specs may be considered verified evidence for landing. All TLA+ artifacts derived from these specs are BLOCKED.

---

## Category 2: qi37 Landing Bundles

### qi37-all-landing-evidence.md
- **Status**: READY_TO_PUSH
- **Blocker**: proof-review-rounds-1-3.md reports REJECTED with CRITICAL TLA+ TypeOK gap (35 files missing TypeOK)
- **Blocker**: The qi37-combined landing also lacks final approval status
- **Action**: BLOCKED until TypeOK gap is remediated

### qi37-combined-landing-evidence.md
- **Status**: No final approval status recorded
- **Blocker**: No final-evidence-decision.md exists for this combined landing
- **Action**: BLOCKED until evidence approval document is created

### qi37 Sub-beads (all with landing reports but unverified evidence)

| Bead ID | Has Landing Report | Evidence Status | Blocker Reason |
|---------|-------------------|-----------------|----------------|
| vb-qi37.1.4 | Yes | Unverified | Part of qi37 landing chain with rejected proof-review |
| vb-qi37.2.4 | Yes | Unverified | Part of qi37 landing chain with rejected proof-review |
| vb-qi37.2.5 | Yes | Unverified | Part of qi37 landing chain with rejected proof-review |
| vb-qi37.4.2 | Yes | Unverified | Part of qi37 landing chain with rejected proof-review |
| vb-qi37.5.4 | Yes | Unverified | Part of qi37 landing chain with rejected proof-review |
| vb-qi37.6 | Yes | Unverified | Part of qi37 landing chain with rejected proof-review |
| vb-qi37.8 | Yes | Unverified | Part of qi37 landing chain with rejected proof-review |
| vb-qi37.9.2 | Yes | Unverified | Part of qi37 landing chain with rejected proof-review |
| vb-qi37.13 | Yes | Unverified | Part of qi37 landing chain with rejected proof-review |
| vb-qi37.14.1 | Yes | Unverified | Part of qi37 landing chain with rejected proof-review |
| vb-qi37.15.3 | Yes | Unverified | Part of qi37 landing chain with rejected proof-review |
| vb-qi37.22 | Yes | Unverified | Part of qi37 landing chain with rejected proof-review |
| vb-qi37.23 | Yes | Unverified | Part of qi37 landing chain with rejected proof-review |
| vb-qi37.25 | Yes | Unverified | Part of qi37 landing chain with rejected proof-review |
| vb-qi37.1.4 through vb-qi37.17.x | Yes | Unverified | Multiple qi37 sub-beads with landing reports |

**Blocker**: All qi37 landing chain beads are BLOCKED as a group. The qi37 integration evidence was rejected in proof-review-rounds-1-3.

---

## Category 3: Bead Closes Blocked by Infrastructure

| Bead ID | Landing Report Status | Blocker Reason |
|---------|----------------------|----------------|
| vb-8cw4 | LANDED (branch pushed) | Bead close BLOCKED by dolt backend configuration issue (embedded mode vs server mode per AGENTS.md) |
| vb-f7k6 | READY_FOR_LANDING | Has multiple proof rejection cycles; final evidence decision APPROVED but landing not executed |

---

## Category 4: Core Subsystem Beads (Unverified Landing Evidence)

| Bead ID | STATE.md Status | Blocker Reason |
|---------|-----------------|----------------|
| vb-core-lower-control-primitives | complete_ready_to_close | STATE shows complete but verify final-evidence exists |
| vb-core-lower-coverage-matrix | APPROVED | Has APPROVED final-evidence-decision.md but verify landing executed |
| vb-core-lower-values-actions-refs | COMPLETE | STATE shows complete but verify landing executed |
| vb-core-storage-artifact-store | APPROVED | Has APPROVED final-evidence-decision.md but verify landing executed |
| vb-e4mt | unknown (parent status) | Parent status unknown, verify evidence chain |
| vb-f04l | APPROVED | Has APPROVED final-evidence-decision.md but verify landing executed |

---

## Category 5: Other Beads with Landing Reports

| Bead ID | Has Landing Report | Evidence Verified? | Blocker Reason |
|---------|-------------------|-------------------|----------------|
| vb-c3k9 | Yes | Check final-evidence-decision.md | Landing report exists, verify evidence |
| vb-hs9m | Yes | LANDED | Verify evidence completeness |
| vb-hxm0 | Yes | Check final-evidence-decision.md | Landing report exists, verify evidence |
| vb-ib8i | Yes | Check final-evidence-decision.md | Landing report exists, verify evidence |
| vb-jpq7-proof-wave1 | Yes | Check proof evidence | Proof-specific landing, verify proof artifacts |
| vb-kyyf | Yes | Check final-evidence-decision.md | Landing report exists, verify evidence |
| vb-m5gp | Yes | Check final-evidence-decision.md | Landing report exists, verify evidence |
| vb-ogwh | Yes | Check final-evidence-decision.md | Landing report exists, verify evidence |
| vb-te1i | Yes | Check final-evidence-decision.md | Landing report exists, verify evidence |
| vb-v7x6 | Yes | Check final-evidence-decision.md | Landing report exists, verify evidence |
| vb-vcmq | Yes | Check final-evidence-decision.md | Landing report exists, verify evidence |
| vb-vt2f | Yes | Check final-evidence-decision.md | Landing report exists, verify evidence |
| vb-xi2f.10 | Yes | Check final-evidence-decision.md | Landing report exists, verify evidence |
| vb-xi2f.4 | Yes | Check final-evidence-decision.md | Landing report exists, verify evidence |
| vb-ybi5 | Yes | Check final-evidence-decision.md | Landing report exists, verify evidence |
| vb-zioy | Yes | Check final-evidence-decision.md | Landing report exists, verify evidence |
| vb-zrop | Yes | Check final-evidence-decision.md | Landing report exists, verify evidence |

---

## Summary Statistics

| Category | Count | Blocker Severity |
|----------|-------|-----------------|
| TLA+ Spec Rejections (Category 1) | 6 specs, 35 files | LETHAL |
| qi37 Landing Bundles (Category 2) | 17+ beads | LETHAL (chain rejection) |
| Infrastructure Blocked (Category 3) | 2 beads | HIGH |
| Core Subsystem Unverified (Category 4) | 6 beads | MEDIUM-HIGH |
| Other Unverified Landing Reports (Category 5) | 17 beads | MEDIUM |
| **TOTAL UNVERIFIED BUNDLES** | **48+** | **BLOCKED** |

---

## Verification Gates Required

Any landing bundle from this ledger can only be unblocked when ALL of the following are true:

1. **Evidence file exists**: `final-evidence-decision.md` with `STATUS: APPROVED`
2. **Proof review passes**: If TLA+/Verus/Kani involved, `proof-review.md` must not have LETHAL findings
3. **TLA+ TypeOK present**: All `.tla` files in scope must have TypeOK invariants
4. **Machine gates pass**: `moon ci` or equivalent must report PASS
5. **Black-hat review**: `black-hat-review.md` must report `STATUS: APPROVED`
6. **No infrastructure blockers**: Dolt backend must be in server mode, not embedded mode

## Ledger Maintenance

This ledger is automatically maintained by the landing bundle approval workflow.
When a bead is approved and landed, its entry should be removed from the UNVERIFIED section and archived to a VERIFIED archive.

When new landing evidence is produced, it must pass all 6 verification gates before being added to the verified list.

---

**END OF UNVERIFIED LANDING BUNDLES LEDGER**
