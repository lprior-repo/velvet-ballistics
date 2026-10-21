**Bead**: vb-cste8 (discovered-from vb-netau)  
**State**: independent CODE review of bounded scheduling delta  
**Reviewer**: black-hat-reviewer  
**Source checkout**: /home/lewis/src/isoloated/velvet-ballistics-netau-abi-code-review-sol  
**Attempt**: 1

**STATUS: APPROVED**

Exact candidate: 0547383f200ea1f08fbaa4f263ddc295b8f61713, compared with 856603b80. Approval covers the newly recorded per-action ABI digest and the added scheduling-only test. It does not close vb-netau, pending-action recovery, retained-input C6, strict artifact/ABI admission rejection, or Ask recovery. No production or test source was changed by this reviewer.

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
| --- | --- | --- |
| Record actual contract digest rather than zero | PASS | lifecycle/chunk_005.rs:122 calls schedule_action_abi_digest with the owned state and actual ticket.action. |
| Fail closed when contract is absent | PASS, source inspection | Lines 7-14 use checked iterator lookup, exact UnsupportedOperation operation action_contract_missing, and propagate hashing errors. No zero fallback. |
| Use existing ABI representation | PASS | recovery/full/admission.rs:33 serializes the actual ActionContract with postcard and hashes it with BLAKE3; this is also used by recovery. |
| Active production wiring | PASS | shard/lifecycle.rs includes chunk_005.rs directly. Older similarly named split files are not the edited production path. |
| Preserve journal boundary and rollback | PASS, source inspection | prepare_awaiting_action owns RunState; attempt mutation is local. prepare_evidence_events builds local vectors. Errors map to BeforeCommit at lines 172-174, before append at 175; drive_run restores its pre-drive clone through restore_run_state_after_drive_failure in chunk_002.rs:226-258. No shard-map lookup is needed for the temporarily removed run. |
| Actual public behavior | PASS | Added test calls public Runtime submission/tick with strict Fjall storage, reads ActionScheduledTicket, and compares actual digest to an independently computed digest plus explicit nonzero assertion. Independent focused execution: 1 passed, 0 failed, 0 ignored, 19 filtered. |

The submission contract registry is checked for duplicate IDs, dense indexing, capability coverage, and workflow Do binding by existing contract_validation.rs and lifecycle/chunk_001.rs:272-283. This delta does not add strict accepted-artifact-versus-submitted-contract equality; that remains within the open parent bug. The empty-evidence await_action path is unchanged and outside this bounded repair.

## PHASE 2: Farley Engineering Rigor

| Function | Physical lines | Limit | Status |
| --- | --- | --- | --- |
| schedule_action_abi_digest | 13 | 25 | PASS; 2 parameters, pure lookup/hash |
| prepare_awaiting_action | 34 | 25 | Pre-existing excess, unchanged line count; delta replaces one event field only |

The new test is 40 lines; test organization is assessed by behavior and determinism, not the production function-size gate. The other source hunk only applies rustfmt wrapping to a pre-existing RunFailed construction. No new abstraction or I/O is hidden in the helper. Inherited function-size debt is recorded rather than presented as newly compliant.

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status |
| --- | --- |
| No new unsafe, unwrap/expect, panic/todo/dbg | PASS |
| No unchecked indexing, casts or arithmetic | PASS; iterator find and existing typed digest |
| Domain types at boundary | PASS; RunState, ActionId, WorkflowDigest, RuntimeResult |
| Fallible operations handled | PASS; missing contract and serialization failures propagate |

## PHASE 4: Ruthless Simplicity & DDD

| Check | Status |
| --- | --- |
| No new Option-based state machine | PASS |
| No speculative generic framework | PASS |
| Owned run authority preserved | PASS; no mutable shard registry lookup or fabricated ABI |
| Pure preparation before persistence | PASS |

## PHASE 5: The Bitter Truth

The candidate repairs the concrete all-zero scheduling record without claiming to repair the wider recovery/admission failures. Author JSON records preserve an exit code and output tails only; they are not full raw logs. This review independently captured complete stdout/stderr and the true exit of the focused public test. The author's 1,914-test library claim was inspected but not independently rerun here; independent TEST review belongs to coordination.

## Findings (Ordered by Severity)

No actionable new defect found in this bounded delta. The inherited 34-line prepare_awaiting_action function is unchanged in size. Wider acceptance gaps remain tracked by the open vb-netau rather than being silently closed by this approval.

## Quality Gates

| Gate | Result | Evidence |
| --- | --- | --- |
| Exact public scheduling test | PASS: 1/0/0 | evidence/current/focused.stdout, focused.stderr, focused.exit |
| Exact changed production file rustfmt | PASS | evidence/current/fmt.stdout, fmt.stderr, fmt.exit |
| Candidate diff check | PASS | evidence/current/diffcheck.* |
| Full moon ci / full recovery suite | Not run; outside bounded independent CODE review | No claim of full integration acceptance |

## Verdict

**STATUS: APPROVED**

The active scheduling path now persists the actual owned contract's ABI digest, with typed failure before append and the existing rollback path intact. Approval is limited to exact 0547383f200ea1f08fbaa4f263ddc295b8f61713 scheduling behavior. The parent bead remains open for its remaining recovery, strict-admission, and Ask obligations.
