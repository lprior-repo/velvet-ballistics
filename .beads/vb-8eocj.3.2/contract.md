# vb-8eocj.3.2 Repeat Counter Slot Reset Probe — Pre-Fix Validation Outcome

**Type:** MALFORMED COMPILER IR negative probe (not canonical source acceptance)

**Objective:** Reproduce the pre-fix validation outcome for a Repeat body whose
SetConst output is manually aliased to the RepeatAttempt counter state slot.
The canonical decoder already rejects `max_attempts=0`; this probe targets
a distinct vulnerability where the body clobbers the counter during execution.

**Workflow under test:**
- Header: `version: velvet-ballistics/v1`, `name: counter_reset_probe`, `when: manual`
- Steps: Set answer=42, Repeat max_attempts=3 with body Set body_result=12884901888 (valid packed max=3/current=0), Finish result=answer
- Canonical source emits a private attempt slot distinct from body_result; unmutated source control must finish with 42 within 64 steps (body count 3).

**Mutation:** After compilation, mutate ONLY `body.output = Some(attempt_slot)` via `iter_mut`/`find` on the node list. Do not replace canonical execution IR or change production.

**Validation path:** Call `CompiledWorkflow::try_from_parts(parts)` on the mutated parts.
- If `Err`: Print exact typed `WorkflowError`; report which guard rejects. No fix needed yet.
- If `Ok`: Run the explicit malformed-IR probe bounded to 64 steps. Record actual `RuntimeSignal`, body step count, read attempt slot value, counter slot values.

**Expected hypothesis (pre-fix):** Body repeatedly resets valid counter; counter only 0/1; body count > 3; `StepBudgetExhausted`.

**Test outcome:** Return `Err` with actual accepted/reset observations so this negative contract test is RED. Do NOT return green while vulnerable. Do NOT assert safety based on unrelated error or blind `is_ok`.

**Scope:** Two tests only:
1. Baseline unmutated source control — should PASS (finish 42, body count 3)
2. Red malformed-case — should FAIL (RED) with actual observations

**Version History:**

- v1: Initial draft
- v2: Attempted RTX 3090 execution; timeout exit code 124; unauthorized .cargo/config.toml changes introduced (jobs=1); missing CollectStates import caused broad nextest compile failure (missing `vb_runtime::primitives::collect::CollectStates`); no pre-fix constructor/runtime outcome established; pushed as `6b998b3b33c2bed6ae9fd755ba8723cee41e8850`
- v3 (current): Config restored to clean baseline `913cf25b`; test rewritten with correct imports (`vb_runtime::primitives::collect::CollectStates`), no expect/panic/unchecked indexing, proper body-node lookup via `iter_mut().find()` matching RepeatStart body StepIdx, actual RuntimeSignal/slot value collection from real journal events, typed constructor error reporting, explicit malformed-IR negative coverage. Draft only — tests written but NOT compiled or executed per root instructions; no production fix applied.
