# Test Review: vb-8eocj.1 — F2 Source Slot Preservation Suite

**Reviewer:** local5090 agent (independent test review)
**Workspace:** /home/lewis/src/isoloated/velvet-ballistics-vb-8eocj1-local5090-test-review-v1
**Date:** 2026-10-22
**Verdict:** PASS (test suite quality); production failures are genuine, not vacuity

## Frozen Source Hash Verified

```
49bcff41f94eaed46dab343ea2b771e7b5aa70cb2151b2f91fc99792915d708e
  crates/workspace_tests/tests/vb_f2_source_slot_preservation.rs
```

Matches declared frozen SHA256. No modifications performed.

## Root Execution Receipts (Read-Only Inspection)

Root executed the unchanged suite twice (V7 and serial V8):
- **Compiled:** 19 tests
- **Passed:** 5
- **Failed:** 14
- **Ignored/filtered:** 0
- **Exit code:** 101

Raw receipts inspected:
- `/home/lewis/.cache/velvet-integration-preserved-20261022/ROOT_F2_SOURCE_PRESERVATION_V8_SERIAL.json`
- `/home/lewis/.cache/velvet-integration-preserved-20261022/ROOT_F2_SOURCE_PRESERVATION_V8_SERIAL.stdout`

## Test Suite Architecture Assessment

### Public API Usage — CORRECT

- Compiles via `compile_workflow(source.as_bytes())` — the canonical public compiler entry
- Executes via `drive_deterministic_full()` — the canonical public runtime entry
- Frame creation via `new_run_frame(RunId, &workflow)` — correct public constructor
- ValueStore, RunFrame, SlotValue, Taint — all used through documented public APIs
- No private module imports or field hacks

### Test Oracle Strength — STRONG

The suite uses a multi-faceted oracle that resists vacuity:

1. **Sentinel preservation:** Two constant values (42, 99) written before composite steps; verified unchanged after execution. This detects slot clobbering.

2. **Taint preservation:** Explicit taint (Clean, DerivedFromSecret, Secret) checked before/after for each sentinel and declared input. Detects taint propagation errors.

3. **IR immutability check:** `assert_eq!(run.workflow.to_parts(), original)` verifies the compiled workflow is not mutated during execution. This catches a class of "eager optimization" bugs where the runtime modifies its input.

4. **Body execution count verification:** `assert_body_counts()` inspects EvidenceCollector events to confirm composite body steps executed the correct number of times (e.g., repeat=3, for_each=list length). This catches body-skip bugs.

5. **Role isolation:** `assert_role_isolation()` verifies generated composite control-flow slots do not alias source outputs. This is a structural check against slot overwriting.

6. **Payload content verification:** For declared inputs, `assert_payload_content()` verifies actual list/object values are preserved, not just handles. This catches handle-corruption bugs.

### Coverage Matrix — COMPREHENSIVE FOR BOUNDED SCOPE

Test coverage by composite type and configuration:

| Composite | Entry | Finish | Body Count | Role Isolation | Taint |
|-----------|-------|--------|------------|----------------|-------|
| Together  | Yes   | Yes    | Yes        | Yes            | Yes   |
| Repeat    | No    | Yes    | Yes        | Yes            | Yes   |
| ForEach   | Yes   | Yes    | Yes        | Via role check | Yes   |
| Reduce    | Yes   | Yes    | Yes        | Yes (4 roles)  | Yes   |

Coverage by input configuration:
- Numeric list sizes: 0, 1, 2, 3 items (covers empty/single/normal)
- Taint levels: Clean, DerivedFromSecret, Secret (full range)
- Declared input types: scalar (I64), list, object with nested list
- Declared input taints: all three taints per type (9 combinations)

Coverage by composition:
- Sequential together + repeat
- Sequential repeat + together

## Distinction: Test Quality vs Production Failures

**The 14 failures are genuine production bugs detected by the tests, not test vacuity or false positives.** The tests pass the quality bar for F2 source slot preservation coverage. A suite with 14 real failures cannot be release acceptance, but that is a production implementation problem, not a test problem.

Failure modes observed (from root's V8 serial run):
- `invalid_state_transition` (9 tests) — runtime engine state machine errors
- `EvidenceGapsRecorded` (4 tests) — slot read failures during execution
- `ImproperLoopNesting` (2 tests) — compile-time validation of nested loops

These are orthogonal failure modes that the tests happen to trigger. The tests are strong enough to expose all three.

## Specific Assertions Validated Against Production APIs

I verified each assertion type against the actual production source:

### Slot and Taint Reads (lines 150-157 of test file)
- `frame.read_slot(slot)` — exists in `vb_core/src/frame.rs`, returns `Result<SlotValue>`
- `frame.read_taint(slot)` — exists, returns `Result<Taint>`
- Correct usage of the slot/taint read API

### Evidence Event Filtering (lines 161-173 of test file)
- `evidence.drain()` — exists in `vb_runtime/src/engine/types.rs`
- `EvidenceEvent::StepSucceeded { step, .. }` — correct variant name
- Filtering logic matches actual node kind enumeration

### Workflow Parts Inspection (lines 92-102 of test file)
- `workflow.to_parts()` — exists, returns `WorkflowParts`
- `node.kind` inspection matches `CompiledNodeKind` enum
- `workflow.constant(ConstIdx)` — correct public API

### Node Kind Matching (throughout)
- `CompiledNodeKind::SetConst` — verified in `crates/vb_core/src/workflow/mod.rs`
- `CompiledNodeKind::ForEachStart/Next` — verified
- `CompiledNodeKind::ReduceStart/Next` — verified
- `CompiledNodeKind::TogetherBranch/Join` — verified
- `CompiledNodeKind::RepeatCheck/Start` — verified

All production API usage is correct and current.

## Missing Cases (Noted for Future, Not Blocking)

For the current vb-8eocj.1 contract, coverage is adequate. Future iterations should add:

1. **Collect node coverage** — `CollectStart/Page/Next/Finish` not tested
2. **Deep nesting** — 3+ composite types nested (currently only 2)
3. **Larger lists** — only 0/1/2/3 items; larger sizes (10, 100) not tested
4. **Action suspension/resume** — no test for Do node with async action
5. **Retry policies** — `RetryPolicy::NEVER` only tested
6. **Step budget exhaustion** — only one entry test checks budget exhaustion

These are improvements, not defects, for the current bounded scope.

## Verdict

**PASS** — The frozen 19-case test suite is a strong, non-vacuous oracle for F2 source slot preservation. It:

- Uses correct public APIs throughout
- Has strong multi-faceted assertions (value + taint + count + structure)
- Covers all declared input types and taint combinations
- Detects genuine production bugs (not false positives)
- Is frozen and reproducible

The 14 failures are production implementation bugs that the tests correctly detect. Test suite quality is not the blocker; production repair is.

## Explicit Limitations

- This review read production source to verify API correctness but did not run compilation, execution, or moon/cargo commands
- No modifications were made to any source, test, or configuration files
- No live external workspace inspection; this workspace only
- Cannot claim every taint scenario executes to completion (a loop over taints stops at first panic)
- Bounded to vb-8eocj.1 scope; not a full F1 or product release acceptance review
