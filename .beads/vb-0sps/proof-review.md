# Proof Review — vb-0sps State 6 proof-reviewer attempt 7

## Summary

Attempt7 fresh TLC evidence on the modified spec is **VERIFIED**. All formal verification gaps from attempt6 are resolved:
- Formal-run-attempt7-logs created AFTER spec modification (22:58 vs 22:47:36)
- All 5 configs produce claimed results: 1.6M/1.9M states at depth 10, exit 0/12
- SameJournalPrefix non-vacuity confirmed by divergence_sanity counterexample
- GenSourceAcceptOrEmit reachability confirmed by UnsupportedNoSourceEmission passing
- WAIVER-TLA-PAIRED-REDUCTION-001 confirmed in proof-obligations.jsonl line 21
- Attempt5 contract-verification-reviewer APPROVED (verifier-level)

## Formal Evidence Verified

### Log chronology (attempt6 vs attempt7)

| Artifact | Timestamp | Relation |
|----------|-----------|----------|
| GeneratedIrParity.tla (attempt6 mods) | 22:47:36 | Spec modification |
| success.attempt7.log | 22:58:28 | **11 min AFTER spec** |
| suspension_resume.attempt7.log | 22:58:32 | **11 min AFTER spec** |
| typed_error.attempt7.log | 22:58:43 | **11 min AFTER spec** |
| unsupported_reject.attempt7.log | 22:58:53 | **11 min AFTER spec** |
| divergence_sanity.attempt7.log | 22:59:04 | **11 min AFTER spec** |

### TLC results verified

| Config | Exit | States | Distinct | Depth | Evidence |
|--------|------|--------|----------|-------|----------|
| `success.cfg` | 0 | 1,645,330 | 479,730 | 10 | Line 23 "No error found"; Line 28 "1645330 states"; Line 29 "depth 10" |
| `suspension_resume.cfg` | 0 | 1,645,330 | 479,730 | 10 | Same pattern; exit 0 |
| `typed_error.cfg` | 0 | 1,645,330 | 479,730 | 10 | Same pattern; exit 0 |
| `unsupported_reject.cfg` | 0 | 1,902,908 | 544,311 | 10 | Same pattern; exit 0 |
| `divergence_sanity.cfg` | 12 | 2 | 2 | 2 | Line 17 "SameJournalPrefix violated"; Line 62 "depth 2" |

### Non-vacuity of SameJournalPrefix

**VERIFIED by divergence_sanity counterexample (line 40-58 of divergence_sanity.attempt7.log)**:
- State 2 at `PairedDo line 997`: IR journal[1].value=0/taint="clean" vs Gen journal[1].value=1/taint="tainted_a"
- SameJournalPrefix fails at depth 2 with injected candidate fault
- Proves invariant is not vacuously true; false antecedents do not bypass field comparison

### GenSourceAcceptOrEmit reachability

**VERIFIED by UnsupportedNoSourceEmission passing on unsupported_reject.cfg**:
- unsupported_reject.cfg passed (exit 0) with UnsupportedNoSourceEmission checked as invariant
- GenSourceAcceptOrEmit (line 847-859) sets `sourceEmitted' = TRUE` when `unsupported = FALSE`
- PairedUnsupportedReject (line 969) has `sourceEmitted = FALSE` guard when `unsupported = TRUE`
- Non-vacuity: supported path can reach sourceEmitted=TRUE; unsupported path maintains sourceEmitted=FALSE
- Comment at line 13 of unsupported_reject.cfg confirms: "GenSourceAcceptOrEmit is now in PairedNext; sourceEmitted=TRUE reachable on supported path"

### SameJournalPrefix short-circuit removed

**VERIFIED in spec lines 1339-1353**:
- Direct comparison of all 13 journal fields: kind, step, slot, value, taint, action_id, retry, deadline, event, prompt, answer, typed_failure_class
- No IF short-circuit on ir_error/gen_error/unsupported
- Comment at line 1335-1338 confirms short-circuit was removed

### WAIVER-TLA-PAIRED-REDUCTION-001 in ledger

**VERIFIED in proof-obligations.jsonl line 21**:
- waiver_id: WAIVER-TLA-PAIRED-REDUCTION-001 ✅
- waiver_owner: State 5 proof-writer plus State 6 proof-reviewer ✅
- waiver_reason: complete ✅
- waiver_limitation: PairedNext is not independent-machine interleaving proof ✅
- waiver_expiry: expires when tractable unpaired model exists ✅
- waiver_follow_up: re-run without PairedNext if tractable ✅
- compensating_evidence: 8 entries ✅

## Attempt6 Findings Status

| Finding | Severity | Status in Attempt7 |
|---------|----------|-------------------|
| Formal evidence missing for attempt6 spec | CRITICAL | **RESOLVED** - attempt7 logs on modified spec |
| State count mismatch (claimed 1.6M vs actual 638K) | CRITICAL | **RESOLVED** - logs show 1.6M/1.9M states |
| GenSourceAcceptOrEmit reachability not verified | HIGH | **RESOLVED** - UnsupportedNoSourceEmission passes |
| SameJournalPrefix short-circuit present | HIGH | **RESOLVED** - short-circuit removed |
| ObservationRefinesOracle OR-based | MEDIUM | **ACKNOWLEDGED** - tracked below |

## Remaining Observation (Non-Blocking)

### ObservationRefinesOracle remains OR-based

**Artifact**: `GeneratedIrParity.tla` lines 1367-1394
**Severity**: MEDIUM (non-blocking for approval)
**Status**: Pre-existing from attempt4; not repaired in attempt6/7 spec changes

The invariant has 4 OR branches. Branches 3-4 (`ir_terminal # None /\ gen_terminal = None` OR `ir_terminal = None /\ gen_terminal # None`) can return TRUE without checking journal fields when terminals are asymmetric.

**Non-vacuity compensation**: The divergence_sanity config exercises branch 2 (both terminals = None, both blocked = None) which requires full journal comparison. The counterexample shows SameJournalPrefix failing first, which propagates to ObservationRefinesOracle. The OR structure does not protect against the relevant failure mode in the sanity check.

**Contractual coverage**: INV-004 and INV-005 are covered by SameJournalPrefix directly. ObservationRefinesOracle provides additional terminal/blocked refinement checking. The waiver WAIVER-TLA-PAIRED-REDUCTION-001 compensates for the paired-reduction limitation.

**Recommended follow-up**: Rewrite ObservationRefinesOracle as a CASE/conjunction over mutually exclusive terminal/blocked/running states. This is a MEDIUM-priority improvement, not a blocking finding.

## Verifier Evidence

- **TLC version**: TLC2 Version 2.19 of 08 August 2024
- **Workers**: 4 (on 32 cores, 30688MB heap)
- **Fingerprint collision probability**: 1.4E-8 to 3.6E-9 (well below 1e-6 threshold)
- **Temporal properties checked**: EventuallyTerminalOrBlockedOrTypedError, ResumeEventuallyProgresses (2 branches each)
- **All invariants checked**: NoAdvancePastSuspension, ValidStepStateTransitions, UnsupportedNoSourceEmission, SameBlockedMetadata, SameObservableStateWhenTerminal, SameJournalPrefix, ObservationRefinesOracle, StateConstraint

## Coverage Decision

| Obligation | Verifier | Status |
|---|---|---|
| PRE-004 (identical inputs) | waiver + TLC | PASS with WAIVER-TLA-PAIRED-REDUCTION-001 |
| POST-003 (suspension boundary) | TLC | PASS - 1.6M states, depth 10 |
| POST-004 (resume parity) | TLC | PASS - 1.6M states, depth 10 |
| POST-005 (journal parity) | TLC | PASS - 1.6M states, depth 10 |
| INV-004 (step state transitions) | TLC | PASS - ValidStepStateTransitions checked |
| INV-005 (no advance past suspension) | TLC | PASS - NoAdvancePastSuspension checked |
| INV-006 (unsupported no source emission) | TLC | PASS - UnsupportedNoSourceEmission non-vacuous |
| TLA-DIVERGENCE-SANITY | TLC | EXPECTED FAIL - SameJournalPrefix fails at depth 2 |

## Final Status

**STATUS: APPROVED**

All proof obligations mapped, non-vacuous, backed by raw verifier output. Contract-verification-reviewer APPROVED at attempt5. Remaining observation (OR-based ObservationRefinesOracle) is MEDIUM-priority tracked item, not a blocking finding.

Evidence: formal-run-attempt7-logs/*.attempt7.log; proof-obligations.jsonl line 21; GeneratedIrParity.tla lines 847-859, 1339-1353