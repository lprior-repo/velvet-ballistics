# Regression Diff — vb-qi37.2.5 State 11 (fresh execution)

STATUS: NO_REGRESSION

## Baseline Context
- `baseline-report.md` identifies the known global `crates/vb_runtime/src/runtime.rs` / `runtime/chunk_001.rs` issue as outside this bead's local boundedness scope.
- `delivery-scope.jsonl` explicitly marks that issue as `deferred-global`.

## Fresh Execution Results vs. Prior State 11 Run
- Prior State 11 run (2026-05-16T00:00:00Z): REJECTED due to `FUZZ-RESOURCE-001` exact cargo-fuzz command failing (musl+ASAN incompatibility).
- This fresh State 11 run (2026-05-16T12:30:00Z): APPROVED.
- Key change: `proof-obligations.jsonl` was repaired by State 3/4/5 cycle to use stdin replay + proptest as the approved command; old cargo-fuzz command moved to `waived_command` field.

## Change Analysis
- `FUZZ-RESOURCE-001` obligation is now satisfied by the repaired stdin replay + proptest command from proof-obligations.jsonl.
- The old `cargo fuzz run resource_budget -- -runs=1000` is explicitly waived per `proof-obligations.jsonl` `waived_command` field and contract-verification-review.md.
- No behavioral regression introduced by repair; no source code behavioral changes in this state.

## New Findings
- None. All 11 obligations passed, were waived, or were deferred-global (pre-existing).

## Passed Local Gates
- Verus (step + budget): 16 verified lemmas, 0 errors.
- TLC (slice + admission): 342 total states model-checked, 0 errors.
- Proptest (budget + value): 8 tests passed across 5+3 invocations.
- Miri: 3 scoped tests passed.
- Lint: production source clean.
- Focused integration: 22 tests passed; 3 proptests passed.
- FUZZ-RESOURCE-001 repaired command: 1000 stdin cases + 3 proptests passed.

## Non-Regression / Deferred Global
- `DEFERRED-GLOBAL-001`: no State 11 focused command hit the vb_runtime missing chunk failure; keep as `DEFERRED_GLOBAL` follow-up.
- `KANI-LOOP-001`: waived per approved contract; no Kani behavioral regression.

## Decision
- NO_REGRESSION: no new failures introduced; all obligations satisfied by repaired evidence lanes.
- APPROVED to advance.
