# Proof Repair Guide — vb-0sps attempt6 TLC evidence

## Status: REJECTED — Formal evidence missing for attempt6 spec changes

## Critical repairs required

1. **Rerun formal verification on attempt6 spec**
   - The formal-run-attempt5-logs/ were created 32 minutes BEFORE the attempt6 spec modifications
   - Spec (modified 22:47:36) has GenSourceAcceptOrEmit in PairedNext and SameJournalPrefix short-circuit removed
   - Logs (created 22:04-22:16) were on OLD spec without these repairs
   - **Action**: Rerun all 5 TLC configs (success, suspension_resume, typed_error, unsupported_reject, divergence_sanity) on the attempt6 spec
   - **Action**: Save raw logs under `.beads/vb-0sps/formal-run-attempt6-logs/` with naming convention: `success.attempt6.log`, `suspension_resume.attempt6.log`, etc.
   - **Command template**: `timeout 120 tlc -config verification/tla/generated_ir_parity/GeneratedIrParity_<config>.cfg -workers 1 verification/tla/generated_ir_parity/GeneratedIrParity.tla 2>&1 | tee .beads/vb-0sps/formal-run-attempt6-logs/<config>.attempt6.log`

2. **Achieve and document 1.6M+/1.9M states at depth 10**
   - User claims these numbers but actual logs show 638K-896K at depth 9
   - **Action**: Increase MaxStep, MaxSlot, MaxEvent bounds to achieve deeper/larger state space
   - **Action**: Verify depth 10 is reached in all positive configs
   - **Action**: Confirm unsupported_reject achieves 1.9M+ states (may need larger feature selector bounds)

3. **Verify sourceEmitted=TRUE reachability**
   - GenSourceAcceptOrEmit is in PairedNext but not yet verified reachable
   - **Action**: Add TLC property or witness proving sourceEmitted can become TRUE on supported path
   - **Action**: Or run a new supported-source config that exercises GenSourceAcceptOrEmit and verify sourceEmitted=TRUE state is reachable

4. **Repair ObservationRefinesOracle (HIGH priority)**
   - Still OR-based per attempt4 finding, not addressed in attempt6 spec
   - **Action**: Rewrite as conjunction or CASE statement over mutually exclusive states:
     - Terminal case: both ir_terminal # None /\ gen_terminal # None
     - Blocked case: both ir_blocked # None /\ gen_blocked # None
     - Running case: both None
     - Mismatch case: one None, other not None
   - **Action**: Add negative TLC config where ObservationRefinesOracle itself fails (not just SameJournalPrefix)

5. **Confirm divergence_sanity on attempt6 spec**
   - Negative sanity passed at depth 2 on OLD spec (IR=clean vs Gen=tainted_a)
   - **Action**: Rerun on attempt6 spec to confirm SameJournalPrefix violation still occurs
   - **Action**: Verify PairedNext with GenSourceAcceptOrEmit does not change divergence behavior

## Repair checklist

- [ ] Create `.beads/vb-0sps/formal-run-attempt6-logs/` directory
- [ ] Rerun success.attempt6.log and verify >= 1.6M states, depth 10
- [ ] Rerun suspension_resume.attempt6.log and verify >= 1.6M states, depth 10
- [ ] Rerun typed_error.attempt6.log and verify >= 1.6M states, depth 10
- [ ] Rerun unsupported_reject.attempt6.log and verify >= 1.9M states, depth 10
- [ ] Rerun divergence_sanity.attempt6.log and verify EXIT_CODE != 0 at depth 2
- [ ] Add sourceEmitted=TRUE reachability witness/property
- [ ] Rewrite ObservationRefinesOracle as non-OR invariant
- [ ] Update proof-review attempt counter to 7-of-7

## Waiver confirmations

- WAIVER-TLA-PAIRED-REDUCTION-001: confirmed in proof-obligations.jsonl line 21

## Exit criteria

STATUS APPROVED when:
- All 5 formal-run-attempt6-logs/ exist and match claimed state counts (1.6M+/1.9M, depth 10)
- sourceEmitted=TRUE reachability proven
- ObservationRefinesOracle rewritten and verified
- divergence_sanity fails as expected on attempt6 spec
- All 3 attempt4 findings resolved

STATUS: REJECTED
