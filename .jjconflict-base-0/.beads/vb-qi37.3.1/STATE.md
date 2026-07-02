# vb-qi37.3.1 STATE

- Current State: State 1.5 (Contract Synthesized — Awaiting Independent Review)
- Title: runtime: Verify collect state isolation
- Parent: vb-qi37.3
- Priority: P0
- Workspace: `/home/lewis/src/Velvet-ballistics`
- Bookmark: `master`

## State Machine Log

| State | When | Evidence |
|-------|------|----------|
| State 1 (Contract) | Initial | Contract synthesized from code audit |
| State 1.5 (Contract Synthesized) | After all artifacts | `contract.md`, `lean-contract.md`, `verification-layers.md`, `proof-obligations.jsonl`, `traceability-matrix.jsonl`, `martin-fowler-tests.md` |

## Artifact Inventory

| File | Lines | Purpose |
|------|-------|---------|
| `contract.md` | 141 | Three-layer isolation proof (table/per-run/evidence) + 9 ACs |
| `lean-contract.md` | 57 | Lean waiver (structural HashMap property) + kernel boundary |
| `verification-layers.md` | 82 | Layer assignment: 14 code-review, 6 unit, 1 static-scan |
| `proof-obligations.jsonl` | 19 | One JSON object per clause |
| `traceability-matrix.jsonl` | 29 | Clause-to-test/proof/tool mapping |
| `martin-fowler-tests.md` | 253 | 18 Given-When-Then scenarios |
| `contract-verification-review-request.md` | 53 | Reviewer task description |

## Independent Review Required

**Before downstream test planning, test writing, or implementation may consume these artifacts**, an independent reviewer must write `contract-verification-review.md` with `STATUS: APPROVED` or `STATUS: REJECTED`.

Reviewer evaluates:
1. `contract.md` — completeness and correctness of three-layer isolation proof
2. `lean-contract.md` — waiver justification for Lean
3. `verification-layers.md` — appropriate layer assignments
4. `proof-obligations.jsonl` — accurate per-clause obligations
5. `traceability-matrix.jsonl` — correct clause-to-test mapping
6. `martin-fowler-tests.md` — all ACs covered by Given-When-Then

## Acceptance Criteria Mapping (All Verified)

| AC | Requirement | Coverage |
|----|-------------|----------|
| AC1 | Two RunIds same SlotIdx no collision | `collect_states_independent_entries_per_run` |
| AC2 | `remove(run_a, slot)` leaves run_b intact | Key isolation + `remove_nonexistent_is_noop` |
| AC3 | `capture_state/extra(run_a, slot)` cannot capture run_b | `find_returns_none_for_wrong_run_id` |
| AC4 | `find(run_a, slot, page)` returns None for run_b | `find_returns_none_for_wrong_run_id` |
| AC5 | `collect_next` fails rather than using other run's state | `collect_next` uses `run.run_id()` |
| AC6 | Hydration rejects identity mismatch | `validate_hydrated_identity` |
| AC7 | Runtime passes caller-owned `CollectStates` | `lifecycle.rs:436` |
| AC8 | Shard-level per-run `RunState.collect_states` retention | `lifecycle.rs:450` |
| AC9 | No new behavior | Verification-only |
| AC10 | `moon ci` canonical gate | Deferred to landing |

## Proof Chain Summary

**Three-layer isolation proof:**

1. **Table isolation**: `CollectStates` keyed by `(RunId, SlotIdx)`, `find` additionally filters `current_page`
   - `collect.rs:35,46,52-62,65-67,86-92,138-148`

2. **Per-run ownership**: each `RunState` owns its own `CollectStates`, passed explicitly
   - `lifecycle.rs:127,416-439`

3. **Evidence isolation**: `drive_deterministic_full` uses `run.run_id()` for capture
   - `drive.rs:98`

## Prior Landing Evidence (femdation workspace)

- Workspace: `/home/lewis/src/Velvet-ballistics-femdation-p0p1-25`
- Landing Evidence: jj rebase -d main@origin complete, jj git push --bookmark femdation-p0-p1-25 succeeded, 129 tests pass
