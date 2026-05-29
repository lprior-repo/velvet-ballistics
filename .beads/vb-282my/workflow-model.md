# Workflow Model — vb-282my

**Bead:** vb-282my (P1)
**Title:** TLA bridge refinement harness workflow
**Date:** 2026-05-29

## Legal States

```
┌──────────────┐   ┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│ PLANNED      │──▶│ MATERIALIZED │──▶│ VERIFIED     │──▶│ CLOSED       │
│ (row exists, │   │ (code exists,│   │ (reviewer    │   │ (all evidence│
│  no harness) │   │  not reviewed)   │  approved)   │   │  is final)   │
└──────────────┘   └──────────────┘   └──────────────┘   └──────────────┘
       │                   │                   │
       │                   │                   │
       ▼                   ▼                   ▼
┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│ PARTIAL      │   │ REJECTED     │   │ WAIVED       │
│ (some evidence │   │ (reviewer    │   │ (approved    │
│  exists, gaps │   │  rejected)   │   │  waiver      │
│  remain)     │   │              │   │  exists)     │
└──────────────┘   └──────────────┘   └──────────────┘
```

### State Definitions

| State | MappingStatus | Description |
|-------|--------------|-------------|
| **PLANNED** | `planned` | RRO row is defined. No refinement harness or waiver exists. Behavior tests may exist. TLC evidence may exist. |
| **MATERIALIZED** | `materialized` | A refinement harness or waiver has been created. Code compiles. Not yet reviewer-approved. Binding may be partial. |
| **VERIFIED** | `verified` | Independent reviewer has confirmed the harness/waiver binding. Harness passes verification. Waiver meets policy requirements. |
| **CLOSED** | — | All evidence is final and documented. The RRO row is no longer blocking the bridge verdict. |
| **PARTIAL** | `partial` | Some evidence exists (TLC, behavior tests) but a complete harness/waiver is missing. Current state of all 7 rows. |
| **REJECTED** | — | Reviewer rejected the harness or waiver. RRO returns to PLANNED or MATERIALIZED for fix. |
| **WAIVED** | — | An approved formal waiver exists. The row is closed via waiver path (only for non-behavior-affecting claims). |

## State Transitions

### PLANNED → MATERIALIZED

**Guard:** A refinement harness has been written and compiles, OR a proportional waiver has been drafted.
**Command:** `proof-writer` agent produces harness/waiver artifact.
**Event:** `HarnessCreated` or `WaiverDrafted`
**Outcome:** `refinement_harness_refs` is non-empty (harness path) OR `waiver_candidate` exists. `mapping_status` → `materialized`.

### PLANNED → PARTIAL

**Guard:** Some evidence exists (TLC, behavior tests) but a complete harness is absent. This is the current state of all 7 rows.
**Command:** Initial bridge setup by `proof-to-implementation`.
**Event:** `EvidenceRecorded`
**Outcome:** TLC and behavior tests are recorded; `refinement_harness_refs` is empty.

### PARTIAL → MATERIALIZED

**Guard:** A refinement harness or waiver has been created for the row.
**Command:** `proof-writer` agent writes harness.
**Event:** `HarnessCreated`
**Outcome:** Same as PLANNED → MATERIALIZED but transitions from PARTIAL.

### MATERIALIZED → VERIFIED

**Guard:** Independent reviewer confirms the harness binding covers the full claim. Harness passes verification. OR waiver is reviewer-approved.
**Command:** `proof-reviewer` agent reviews and accepts.
**Event:** `HarnessApproved` or `WaiverApproved`
**Outcome:** `mapping_status` → `verified`. `reviewer_disposition` → `accepted`.

### MATERIALIZED → REJECTED

**Guard:** Reviewer finds the harness binding incomplete, incorrect, or insufficient. Or waiver fails policy check.
**Command:** `proof-reviewer` agent rejects.
**Event:** `HarnessRejected` or `WaiverRejected`
**Outcome:** `reviewer_disposition` → `rejected`. Row returns to MATERIALIZED for fix or PLANNED for re-design.

### VERIFIED → CLOSED

**Guard:** All evidence is recorded: TLC pass, behavior test pass, harness pass, reviewer approval. Raw command evidence exists for each layer.
**Command:** `formal-verifier` agent records closure evidence.
**Event:** `RroClosed`
**Outcome:** Row is removed from blocking findings. Bridge verdict can advance.

### Any → REJECTED (Bridge-level)

**Guard:** `proof-reviewer` issues `TLA-BRIDGE-REFINEMENT-HARNESS-GAP` or similar finding.
**Command:** Bridge-level review.
**Event:** `BridgeRejected`
**Outcome:** Overall verdict: REJECTED. Individual rows remain in their current state.

## Harness Creation Workflow (Within MATERIALIZED)

```
┌────────────────┐
│ Select Verifier │──▶ Kani | Flux | Verus | Proptest
└───────┬────────┘
        ▼
┌────────────────┐
│ Write Harness   │──▶ #[kani::proof] fn ...
│ (proof-writer)  │    #[sig] fn ...
└───────┬────────┘
        ▼
┌────────────────┐
│ Verify Harness  │──▶ PASS or FAIL (counterexample)
│ (cargo kani,    │
│  cargo flux,    │
│  verus, etc.)   │
└───────┬────────┘
        ▼
┌────────────────┐
│ Confirm Binding │──▶ Claim-coverage analysis
│ (proof-reviewer)│    Source-ref alignment
└───────┬────────┘
        ▼
   ┌─────────┐
   │ ACCEPT  │──▶ VERIFIED → CLOSED
   └────┬────┘
        │
   ┌────┴────┐
   │ REJECT  │──▶ REJECTED → Fix or re-design
   └─────────┘
```

## Waiver Creation Workflow (Alternative Path)

```
┌──────────────────────┐
│ Draft Waiver         │──▶ waiver-candidate/v1
│ (proof-writer)       │
└───────┬──────────────┘
        ▼
┌──────────────────────┐
│ Behavior Scope Check │──▶ behavior_affecting == true? → REJECT (INV-3)
│ (proof-plan-reviewer)│
└───────┬──────────────┘
        ▼
┌──────────────────────┐
│ Compensating Evidence│──▶ TLC + behavior tests + boundary argument
│ Review               │
└───────┬──────────────┘
        ▼
┌──────────────────────┐
│ Formal Waiver        │──▶ formal-waiver/v1
│ Approval             │
└───────┬──────────────┘
        ▼
   ┌─────────┐
   │ ACCEPT  │──▶ WAIVED → CLOSED
   └────┬────┘
        │
   ┌────┴────┐
   │ REJECT  │──▶ REJECTED → Write harness instead
   └─────────┘
```

## Commands and Events

| Command | Actor | Pre-State | Post-State | Event |
|---------|-------|-----------|------------|-------|
| `CreateHarness` | proof-writer | PLANNED \| PARTIAL | MATERIALIZED | HarnessCreated |
| `DraftWaiver` | proof-writer | PLANNED \| PARTIAL | MATERIALIZED | WaiverDrafted |
| `ApproveHarness` | proof-reviewer | MATERIALIZED | VERIFIED | HarnessApproved |
| `RejectHarness` | proof-reviewer | MATERIALIZED | REJECTED | HarnessRejected |
| `ApproveWaiver` | proof-reviewer | MATERIALIZED | VERIFIED (WAIVED) | WaiverApproved |
| `RejectWaiver` | proof-reviewer | MATERIALIZED | REJECTED | WaiverRejected |
| `RecordEvidence` | formal-verifier | VERIFIED | CLOSED | RroClosed |
| `ReviewBridge` | proof-reviewer | * | REJECTED (bridge) | BridgeRejected |
| `ApproveBridge` | proof-reviewer | all VERIFIED/CLOSED | PASS | BridgeApproved |

## Guards (Transition Pre-Conditions)

| Transition | Guards |
|-----------|--------|
| PLANNED → MATERIALIZED | Harness file exists AND compiles; OR waiver document exists |
| MATERIALIZED → VERIFIED | Harness passes verification (exit 0); binding_status is Confirmed; reviewer NOT self; OR waiver meets INV-3 |
| VERIFIED → CLOSED | All evidence commands recorded with raw output; invocation provenance is independent |
| Any → WAIVED (via waiver) | `behavior_affecting` is FALSE (INV-3); compensating evidence cited; reviewer accepted |
| Any → with `behavior_affecting: true` and only waiver | FORBIDDEN (INV-3) |
| Any → VERIFIED with empty `refinement_harness_refs` and no waiver | FORBIDDEN (INV-4) |
| PLANNED at State 12 | FORBIDDEN (lifecycle gate) |

## Retry / Error Paths

| Scenario | Recovery Path |
|----------|--------------|
| Kani harness finds counterexample | Fix production code (GOD RULE 4); re-run harness; do not weaken the contract |
| Harness binding is partial (RetryFSM case) | Extend harness to cover full claim OR document explicit sub-claim waiver for uncovered portion |
| Reviewer rejects harness | Fix identified gaps; re-submit for review |
| Waiver rejected (behavior-affecting) | Write a refinement harness instead; waivers cannot cover behavior-affecting claims |
| TLC evidence invalidated by model change | Re-run TLC; update bounded config if needed; do not close row until TLC passes |
| Behavior tests fail after production change | Fix tests or fix production; do not close row until tests pass |
| Cross-crate harness compilation fails | Verify crate dependencies; ensure harness is in correct crate or workspace_tests |

## Terminal Outcomes

| Outcome | Condition | Row Status |
|---------|-----------|-----------|
| **RRO Closed (Harness)** | Kani/Flux/Verus/proptest harness passes + reviewer approved | `mapping_status: verified` + `reviewer_disposition: accepted` |
| **RRO Closed (Waiver)** | Formal waiver with `behavior_affecting: false` + reviewer approved | `mapping_status: verified` (via waiver path) |
| **RRO Rejected** | Harness or waiver rejected by reviewer; requires rework | Return to MATERIALIZED or PLANNED |
| **Bridge Pass** | All 7 RRO rows closed (harness or waiver); no blocking findings | Bridge verdict: PASS |
| **Bridge Rejected** | One or more rows remain PARTIAL or REJECTED | Bridge verdict: REJECTED; blocking finding persists |

## Current State (All 7 Rows)

All 7 rows are in **PARTIAL** state:

| RRO Row | Harness Status | Waiver Status | Path to CLOSED |
|---------|---------------|---------------|----------------|
| RRO-TLA-CHOOSE-LOWERING-001 | Empty (none) | None | MATERIALIZED → VERIFIED → CLOSED (harness) |
| RRO-TLA-CHOOSE-REPLAY-001 | Empty (none) | None | MATERIALIZED → VERIFIED → CLOSED (harness) |
| RRO-TLA-ASK-ANSWER-001 | Empty (none) | None | MATERIALIZED → VERIFIED → CLOSED (harness) |
| RRO-TLA-RETRY-FSM-001 | PARTIAL (Kani exists, unapproved) | None | MATERIALIZED (extend harness) → VERIFIED → CLOSED |
| RRO-TLA-RETRY-JOURNAL-001 | Empty (none) | None | MATERIALIZED → VERIFIED → CLOSED (harness) |
| RRO-TLA-RESUME-001 | Empty (none) | None | MATERIALIZED → VERIFIED → CLOSED (harness) |
| RRO-TLA-ADMISSION-001 | Empty (none) | None | MATERIALIZED → VERIFIED → CLOSED (harness) |

**Blocking Finding:** `TLA-BRIDGE-REFINEMENT-HARNESS-GAP` (severity: high)
**Bridge Verdict:** REJECTED

## Idempotence Requirements

- **Harness approval** is idempotent: submitting the same harness for review with the same reviewer produces the same result.
- **RRO closure** is idempotent: recording evidence for an already-closed row is a no-op.
- **Bridge review** is idempotent: running `proof-reviewer` on the same artifacts produces the same verdict.
- **Waiver approval** is NOT idempotent if the codebase has changed since the waiver was issued (compensating evidence may be stale).

## Cancellation Paths

- A harness in MATERIALIZED state can be **abandoned** if a different verifier is chosen (e.g., switch from Kani to Flux). Requires new harness and re-review.
- A waiver in MATERIALIZED state can be **withdrawn** in favor of a harness. The waiver is archived, not deleted.
- Individual RRO rows cannot be cancelled independently; the bridge requires all 7 rows to close.
