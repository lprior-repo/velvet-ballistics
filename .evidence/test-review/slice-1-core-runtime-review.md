# Slice 1 Test Suite Review — vb_core + vb_runtime

STATUS: REJECTED

This slice contains 261 Rust files across two crates. Pattern-density sweeps plus deep reads of 18 high-density files (bounded queue, action queue, recovery bdd, durable resume, frame pool, lru ring, red queen property, together primitives, retry primitives, admission, dispatch generic, error routing, value store, budget, policy, symbolic code, frame behavior, taint propagation) surfaced **9 LETHAL/HIGH test-quality defects** that pass despite deletion of the behavior they claim to cover, plus **17 MEDIUM/LOW smoke-test patterns** and **5 OBSERVATIONS** about test-suite structure. The slice also has 30+ test files opt-out of the entire Holzman Rust banlist via file-level `#![allow(...)]`, which is acceptable for test scope but should be tracked as debt. Tests compile and a representative proptest (`proptest_symbolic_code`) runs deterministically. Tier 0 from the prior review still passes for these files; Tier 2 line coverage debt remains (89.41% line, 66.70% branch per `vb_core/test-suite-review.md`); Tier 3 mutation kill-rate still cannot be re-evaluated here. The verdict is REJECTED because the LETHAL findings listed below would still pass if the production code being asserted were deleted or had its error/success variants swapped.

---

## 1. Findings Table (ordered by severity)

| ID | Sev | File:Line | Defect | Mutation thought experiment | Recommended fix |
|----|-----|-----------|--------|------------------------------|-----------------|
| F-01 | CRITICAL | `crates/vb_runtime/src/engine/action_tests.rs:267` | `resolve_contract_rejects_id_mismatch` ends with `assert!(result.is_err());` — no concrete error variant check. Test passes for ANY Err. | Mutate `resolve_contract` to return `Err(ResolveError::IndexOutOfBounds)` instead of `Err(ResolveError::IdMismatch)` — test still passes. Contract drift invisible. | Replace with `assert!(matches!(result, Err(ResolveError::IdMismatch { requested: ActionId(99), .. })))`. |
| F-02 | CRITICAL | `crates/vb_runtime/src/engine/action_tests.rs:289` | `resolve_contract_returns_first_contract` ends with `assert!(result.is_ok());` — no concrete Ok payload check. | Mutate `resolve_contract` to return `Ok(&wrong_contract)` — test still passes. The "first contract" claim is unverified. | Replace with `assert_eq!(result, Ok(&contracts[0]))` or `assert!(matches!(result, Ok(c) if c.id == ActionId::new(0)))`. |
| F-03 | CRITICAL | `crates/vb_runtime/src/engine/action_tests.rs:296` | `resolve_contract_returns_last_contract` — same as F-02 but for the last entry. | Same mutation; test still passes. | Same fix as F-02 against `&contracts[2]`. |
| F-04 | CRITICAL | `crates/vb_runtime/src/action_queue/action_queue_tests.rs:240` | `bounded_action_queue_enqueue_single_item_succeeds` ends with `assert!(result.is_ok());` — no concrete `Ok(())` check. | Mutate `enqueue` to return `Ok(false)` or `Ok(SomeError)` — test still passes. The "success" claim is unverified. | Replace with `assert_eq!(result, Ok(()))`. |
| F-05 | CRITICAL | `crates/vb_runtime/src/shard/lru_ring_red_queen_tests.rs:507` | `lru_ring_property_capacity_overflow_then_recover` — `assert!(r.is_err(), "must fail when full")` with no concrete variant. | Mutate `LruRing::insert` to return `Err(LruError::Generic)` instead of `Err(LruError::TerminalRunsLruFull)` — test still passes. | Replace with `assert!(matches!(r, Err(LruError::TerminalRunsLruFull { .. })))`. |
| F-06 | CRITICAL | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2141` | `recover_runtime_summary_handles_empty_journal` — `assert!(result.is_err(), "empty journal should return error")` with no concrete variant. | Mutate `recover_runtime_summary` to return `Err(RecoveryError::Io)` instead of `Err(RecoveryError::EmptyJournal)` — test still passes. | Replace with `assert!(matches!(result, Err(RecoveryError::EmptyJournal)))`. |
| F-07 | CRITICAL | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2843` | `check_compiled_ir_digest_accepts_matching_digest` — `assert!(result.is_ok(), "matching digests should succeed")` standalone. The function returns `Result<(), DigestMismatchError>`; any Ok passes. | Mutate `check_compiled_ir_digest` to a stub that always returns `Ok(())` — test still passes. Contract that the function actually compared digests is unverified. | Replace with `assert_eq!(result, Ok(()))` plus an additional divergent-input test that returns the specific error variant. |
| F-08 | CRITICAL | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2852` | `check_compiled_ir_digest_rejects_mismatch` — `assert!(result.is_err(), ...)` standalone. | Mutate to return `Err(WrongVariant)` instead of `Err(DigestMismatchError)` — test still passes. | Replace with `assert!(matches!(result, Err(DigestMismatchError { expected, found }) if expected == ... && found == ...))`. |
| F-09 | CRITICAL | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2883` | `recover_runtime_summary_returns_recovery_hydration` — `assert!(result.is_ok(), "should return RecoveryHydration")` standalone. | Mutate `recover_runtime_summary` to always return `Ok(RecoverySummary::default())` — test still passes. | Replace with `assert!(matches!(result, Ok(summary) if summary.kind == RecoverySummaryKind::Hydration))`. |
| F-10 | CRITICAL | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2728` | `tail_after_watermark_succeeds` — standalone `assert!(result.is_ok(), "tail after watermark should succeed")`. The whole test is 2 lines of fixture + this assertion. | Mutate `hydrate_run_frame` to return `Ok(default_frame)` — test still passes. | Add concrete post-conditions: `let frame = result.unwrap(); assert_eq!(frame.pc(), expected_pc); assert_eq!(frame.slot_count(), expected_slots);`. |
| F-11 | HIGH | `crates/vb_runtime/src/primitives/retry/tests.rs:14,225,707,1235,1243` | `assert!(policy.is_ok());` / `assert!(write_result.is_ok());` / `assert!(result.is_ok());` immediately followed by `result.unwrap()` and then concrete field asserts (`policy.max_attempts()`, `state.current_attempt()`, etc.). | Mutate `RetryPolicy::new` to return `Ok(RetryPolicy::default())` while still writing concrete field values from inputs — concrete asserts pass even though the validation logic is broken. | Replace smoke + unwrap with `assert_eq!(result, Ok(RetryPolicy { max_attempts: 3, ... }))`. |
| F-12 | HIGH | `crates/vb_runtime/src/engine/tests/mod.rs:1141,1738,1864` | `assert!(result.is_ok(), "drive should succeed, got {result:?}")` followed by concrete events/pc asserts. The drive-result Ok is asserted but the Ok payload (which engine-signal variant) is not pinned. | Mutate `drive` to return `Ok(EngineSignal::Continue)` after every step instead of the signal the workflow actually requested — concrete follow-up `events.len() > 1` and `run.pc() == StepIdx(1)` would still pass. | Replace with `assert!(matches!(result, Ok(EngineSignal::Continue) | Ok(EngineSignal::Finished) | Ok(EngineSignal::AwaitingAction(_))))` (pin to the contractually expected signal). |
| F-13 | HIGH | `crates/vb_runtime/src/frame_pool/tests.rs:147,244,259,260,261,273,274,351` | `assert_eq!(reused.is_ok(), true);` — disguised smoke test (assert_eq on a boolean is the same as `assert!`). No concrete Ok payload check. | Mutate `FramePool::take` to return `Ok(FrameRef::default())` — all 8 tests still pass. | Replace with `assert!(matches!(take_result, Ok(_)))` only if contract is purely boolean, otherwise pin to specific FrameRef shape. |
| F-14 | HIGH | `crates/vb_runtime/src/shard/tests/chunk_017.rs:212,213,214` | `bh_shd_07_frame_pool_allocates_beyond_pool_capacity` — three `assert!(fX.is_ok())` with no concrete payload. | Mutate `FramePool::take` to `Ok(FrameRef::default())` — test passes despite broken recycling. | Pin to specific FrameRef with expected `run_id()`. |
| F-15 | HIGH | `crates/vb_runtime/src/shard/tests/chunk_dispatch_error_semantics.rs:158` | `assert!(result.is_err() || shard.run_state_contains(run));` — OR-conditioned smoke. Test passes if EITHER branch fires. Comment admits "Either it returns RunNotFound (if removed) or some resumable error". | Mutate `shard.tick()` to always return `Ok(true)` and remove the run — `run_state_contains(run) == false`, so test would fail. But mutate `tick()` to swallow the resume silently and keep the run alive — `is_err() == false`, `run_state_contains == true`, test passes. | Split into two tests: one asserts `assert!(matches!(result, Err(RuntimeError::NotResumable { .. })))`, the other asserts the run state separately. |
| F-16 | HIGH | `crates/vb_core/src/engine/tests/integration_frame_behavior.rs:34,83,686,695,705` | `assert!(frame.is_ok())` after `RunFrame::new(...)` — no concrete frame payload. | Mutate `RunFrame::new` to ignore its arguments and return `Ok(RunFrame::default())` — all five tests pass. | Replace with `assert!(matches!(frame, Ok(f) if f.run_id() == RunId::new(1) && f.step_count() == 3 && f.slot_count() == 2))`. |
| F-17 | HIGH | `crates/vb_runtime/src/together_tests.rs:73,255,288,505,546,592,635,968,1016,1059,1106,1216,1420,1533` | `assert!(run.add_parallel_in_flight(N).is_ok())` standalone, no concrete `Ok(())` check, no follow-up invariant on the post-state. | Mutate `add_parallel_in_flight` to return `Ok(())` without actually incrementing `parallel_in_flight` — all 14 tests still pass because none of them assert the post-state. | Add `assert_eq!(run.parallel_in_flight(), expected)` after each smoke. |
| F-18 | HIGH | `crates/vb_core/src/value/proptests.rs:166` | `prop_assert!(FiniteF64::new(val).is_err());` — no concrete error variant. | Mutate `FiniteF64::new` to return `Err(FiniteF64Error::Infinity)` for NaN — proptest still passes. | Replace with `prop_assert!(matches!(FiniteF64::new(val), Err(FiniteF64Error::NaN)))`. |
| F-19 | HIGH | `crates/vb_core/tests/proptest_symbolic_code.rs:52,59,66,146` | Four `prop_assert!(parsed.is_err())` / `assert!(parsed.is_err())` with no concrete variant. | Mutate `SymbolicCode::from_str` to return `Err(SymbolicCodeParseError::InvalidCharacter)` for whitespace and `Err(SymbolicCodeParseError::UnknownCode)` for unregistered — all four tests pass. | Replace with `prop_assert!(matches!(parsed, Err(SymbolicCodeParseError::UnknownCode)))` etc. |
| F-20 | HIGH | `crates/vb_core/src/policy/contract.rs:991,999` | `test_new_validates_zero_active_runs` and `test_new_validates_zero_retry_attempts` — standalone `assert!(result.is_err())`. | Mutate `RuntimeLimitsProfile::new` to return `Err(LimitsProfileError::ZeroActiveRuns)` instead of `Err(LimitsProfileError::ZeroRetryAttempts)` — both tests still pass. | Replace with `assert!(matches!(result, Err(LimitsProfileError::ZeroActiveRuns)))` (and ZeroRetryAttempts respectively). |
| F-21 | HIGH | `crates/vb_core/src/budget/tests/chunk_028.rs:398`, `crates/vb_core/src/budget/tests/chunk_029.rs:134,170` | Three standalone `assert!(result.is_err(), ...)` — but each is FOLLOWED by `match result { Err(Variant) => { concrete } other => panic!() }`. Concrete variant IS checked. | N/A — these are borderline MEDIUM at worst because the follow-up `match` does verify the variant. Listed for completeness. | Optional: collapse `is_err()` + `match` into single `assert!(matches!(result, Err(Variant { ... }) if ...))`. |
| F-22 | MEDIUM | `crates/vb_runtime/tests/durability_matrix_integration.rs:296,348,396,443,478` | `let _ = journal.snapshot().unwrap();` — pattern "discard snapshot to drain journal". The `unwrap()` panics on Err (good), but the value is discarded (acceptable fixture cleanup, not silent error suppression). | N/A | OBSERVATION: acceptable as fixture cleanup. Add a one-line comment explaining the drain intent. |
| F-23 | MEDIUM | `crates/vb_runtime/tests/red_queen_lru_concurrent.rs:520` | `let _ = ring.insert(RunId::new(tick + 1), TimerTick::new(tick));` — silent error suppression. Test passes if `insert` returns `Err(LruError::TerminalRunsLruFull)` for every iteration. | Mutate `insert` to always return `Err(TerminalRunsLruFull)` — the post-condition `ring.len() <= 2` is still satisfied and `expired_evictions > 0` is still true (if sweeps happen). The behavior "no panic under load" is verified; the behavior "inserts succeed" is NOT. | Replace with `match ring.insert(...) { Ok(_) | Err(_) => () }` to make intent explicit, or use `assert!(ring.insert(...).is_ok() || ring.len() <= capacity)`. |
| F-24 | MEDIUM | `crates/vb_runtime/tests/recovery_bdd_tests.rs:2241-2243` | `let _ = DigestCheck::WorkflowSourceOnly;` etc. — pattern looks like silent error suppression but is actually compile-time exhaustiveness check for enum variant constructor. | N/A | OBSERVATION: not a defect. |
| F-25 | MEDIUM | `crates/vb_runtime/src/action_queue/tests/bounded_queue_tests.rs:444,460,466,485,508,538,681` | `assert!(warning.is_ok());` followed by `let w = warning.unwrap(); assert_eq!(w.depth, 8);` — smoke + concrete field check. The follow-up `assert_eq!(w.depth, 8)` IS concrete. | Mutate `warning` builder to set `depth = 8` but `capacity = 0` — concrete follow-up `assert_eq!(w.capacity, 10)` would catch it. Acceptable. | None needed. |
| F-26 | MEDIUM | `crates/vb_runtime/src/action_queue/tests/bounded_queue_tests.rs:443,465,507,537,680`, `crates/vb_runtime/src/action_queue/action_queue_tests.rs:327,353,404,430,440,455` | All use `rx.recv_timeout(std::time::Duration::from_millis(100))` — 100ms sleep. On a slow CI runner (or under coverage instrumentation), the channel may not deliver in 100ms, causing false failures. | N/A (timing-only, not behavior) | Increase timeout to 500ms or use a synchronous channel flush helper. |
| F-27 | MEDIUM | `crates/vb_runtime/src/recovery/tests.rs:495` | `deadline: std::time::Instant::now() + std::time::Duration::from_secs(30);` — wall-clock deadline in test fixture. | N/A | OBSERVATION: a 30s deadline is generous, no flake risk in practice. |
| F-28 | LOW | `crates/vb_runtime/src/primitives/reentry_tests.rs:1-126`, `crates/vb_runtime/src/action_queue/action_queue_tests.rs:1-180+`, `crates/vb_runtime/tests/durable_resume_red_phase.rs:1-30`, `crates/vb_runtime/tests/durability_matrix_integration.rs:1-30`, `crates/vb_core/src/action/tests.rs:1-80+`, `crates/vb_core/src/engine/expr_eval/red_queen_property_tests.rs:1-130+` | File-level `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::todo, clippy::unimplemented, ...)]` blocks. Acceptable for test scope (production rules ban these in `/velvet-ballistics-MASTER.md` production code), but masks clippy lints that would otherwise catch missing concrete assertions. | N/A | Add a header comment justifying each `#![allow]` for audit purposes. |
| F-29 | LOW | `crates/vb_runtime/src/shard/tests/chunk_dispatch_error_semantics.rs:133-135`, `crates/vb_runtime/src/shard/tests/chunk_dispatch_shutdown.rs` (multiple), `crates/vb_runtime/tests/durability_matrix_integration.rs:287-289` | `let Some(workflow) = suspended_workflow() else { return; };` — silent early-return on fixture failure. If `suspended_workflow()` returns `None`, the test silently passes without exercising the behavior. | Mutate `suspended_workflow()` to always return `None` — every test that uses this pattern becomes a no-op. Silent success. | Replace with `let workflow = suspended_workflow().expect("suspended_workflow() must produce a workflow");` so fixture failure panics visibly. |
| F-30 | LOW | `crates/vb_core/src/budget/traversal_depth.rs:144` | `assert!(result.is_ok(), "Nop must return Ok at any depth")` followed by `assert_eq!(result.unwrap(), u16::MAX);`. Concrete payload IS checked. | N/A | OBSERVATION: acceptable. |
| F-31 | OBSERVATION | `crates/vb_core/src/engine/tests/integration_step_behavior.rs:1931`, `crates/vb_core/src/budget/tests_and_verification.rs:5`, `crates/vb_core/src/budget/vb_qi37_2_4_state8_tests.rs:50`, `crates/vb_core/src/capability.rs:85`, `crates/vb_core/src/check.rs:95`, `crates/vb_core/src/engine/tests/integration_capability_behavior.rs:887,1017` | Kani/Verus/Flux harnesses behind `#[cfg(kani)]` etc. are NOT behavior tests. They are correctly feature-gated. | N/A | OBSERVATION: not a defect; verifier harnesses do not count as behavior tests per rubric rule 7. |
| F-32 | OBSERVATION | `crates/vb_runtime/src/property_tests/concurrency_safety.rs:609,703,784,867` | Four `// The #[ignore] attribute has been removed` comments — proves no actual `#[ignore]` on behavior tests. | N/A | OBSERVATION: not a defect; confirms cleanup of dormant `#[ignore]` was completed. |
| F-33 | OBSERVATION | `crates/vb_runtime/tests/durable_resume_red_phase.rs:130-156` | The `is_ok()` + `unwrap()` + `matches!()` 3-step pattern repeats ~50 times. Functional, but verbose. | N/A | OBSERVATION: idiomatic, not a defect. |
| F-34 | OBSERVATION | `crates/vb_core/tests/proptest_symbolic_code.rs`, `crates/vb_core/tests/section36_mandatory_coverage.rs`, `crates/vb_runtime/tests/dispatch_generic_properties.rs`, `crates/vb_core/src/engine/expr_eval/red_queen_property_tests.rs`, `crates/vb_runtime/src/properties_ticket_derivation.rs` | Property tests use `arb_unregistered_ascii()`, `arb_registered_str()`, `arb_registered_str()`, and `proptest!` macro with proper shrinking. Good test design. | N/A | OBSERVATION: confirms property-test infrastructure is sound. |
| F-35 | OBSERVATION | All test files | 30+ test files opt out of `clippy::unwrap_used`, `clippy::panic`, `clippy::todo`, `clippy::unimplemented` via file-level `#![allow]`. Some opt-outs are excessive (e.g., `reentry_tests.rs` allows 60+ lints). | N/A | OBSERVATION: acceptable for test scope but obscures which lint was hit. |

---

## 2. Code Snippets — CRITICAL/HIGH BEFORE/AFTER

### F-01: `crates/vb_runtime/src/engine/action_tests.rs:267`

```rust
// BEFORE
#[test]
fn resolve_contract_rejects_id_mismatch() {
    // Contract at index 0 has id=0, but we request id=99 at index 99
    let contracts = vec![make_contract(0)];
    let result = resolve_contract(ActionId::new(99), &contracts);
    assert!(result.is_err());
}

// AFTER
#[test]
fn resolve_contract_rejects_id_mismatch() {
    let contracts = vec![make_contract(0)];
    let result = resolve_contract(ActionId::new(99), &contracts);
    assert!(
        matches!(result, Err(ResolveError::IdMismatch {
            requested,
            index: 99,
        }) if requested == ActionId::new(99)),
        "id-mismatch must surface the requested ActionId and index"
    );
}
```

### F-04: `crates/vb_runtime/src/action_queue/action_queue_tests.rs:240`

```rust
// BEFORE
#[test]
fn bounded_action_queue_enqueue_single_item_succeeds() {
    let queue = BoundedActionCompletionQueue::new(3).unwrap();
    let ticket = make_ticket(0);
    let result = queue.enqueue(ticket);
    assert!(result.is_ok());
}

// AFTER
#[test]
fn bounded_action_queue_enqueue_single_item_succeeds() {
    let queue = BoundedActionCompletionQueue::new(3).unwrap();
    let ticket = make_ticket(0);
    let result = queue.enqueue(ticket);
    assert_eq!(result, Ok(()));
    assert_eq!(queue.len(), 1, "len must increment to 1 after enqueue");
    assert_eq!(queue.remaining_capacity(), 2, "remaining must decrement");
}
```

### F-09 + F-10: `crates/vb_runtime/tests/recovery_bdd_tests.rs:2883, 2728`

```rust
// BEFORE
#[test]
fn recover_runtime_summary_returns_recovery_hydration() {
    // ... fixture ...
    let result = recover_runtime_summary(&journal, run);
    assert!(result.is_ok(), "should return RecoveryHydration");
}

#[test]
fn tail_after_watermark_succeeds() {
    // ... fixture ...
    let result = hydrate_run_frame(&snapshot, &tail, run);
    assert!(result.is_ok(), "tail after watermark should succeed");
}

// AFTER
#[test]
fn recover_runtime_summary_returns_recovery_hydration() {
    // ... fixture ...
    let result = recover_runtime_summary(&journal, run);
    let summary = result.expect("recovery must succeed");
    assert!(
        matches!(summary.kind, RecoverySummaryKind::Hydration { .. }),
        "summary.kind must be Hydration, got {:?}",
        summary.kind
    );
    assert_eq!(summary.run_id, run, "run_id must round-trip");
}

#[test]
fn tail_after_watermark_succeeds() {
    // ... fixture ...
    let frame = hydrate_run_frame(&snapshot, &tail, run)
        .expect("tail after watermark must hydrate");
    assert_eq!(frame.pc(), StepIdx::new(3), "pc must advance to step 3");
    assert_eq!(frame.slot_count(), expected_slots);
}
```

### F-13: `crates/vb_runtime/src/frame_pool/tests.rs:147`

```rust
// BEFORE
let reused = pool.take(RunId::new(2), StepIdx::new(0));
assert_eq!(reused.is_ok(), true);

// AFTER
let reused = pool.take(RunId::new(2), StepIdx::new(0));
assert!(matches!(reused, Ok(f) if f.run_id() == RunId::new(2) && f.pc() == StepIdx::new(0)));
```

### F-15: `crates/vb_runtime/src/shard/tests/chunk_dispatch_error_semantics.rs:158`

```rust
// BEFORE
let result = shard.tick();
// Result should be an error since run is not in a resumable state
assert!(result.is_err() || shard.run_state_contains(run));

// AFTER — split into two tests, one per branch of the OR
#[test]
fn resume_active_run_returns_not_resumable_error() {
    // ... submit a workflow so it's in Running state ...
    let result = shard.tick();
    assert!(
        matches!(result, Err(RuntimeError::NotResumable { run }) if run == RunId::new(500)),
        "active run must yield NotResumable, got {:?}", result
    );
}

#[test]
fn resume_active_run_keeps_run_alive_for_recovery() {
    // ... submit a workflow so it's in Running state ...
    let _ = shard.tick();
    assert!(
        shard.run_state_contains(RunId::new(500)),
        "active run must remain in shard state after NotResumable"
    );
}
```

### F-16: `crates/vb_core/src/engine/tests/integration_frame_behavior.rs:34`

```rust
// BEFORE
#[test]
fn frame_creation_valid_config_returns_ok() {
    let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 2);
    assert!(frame.is_ok());
}

// AFTER
#[test]
fn frame_creation_valid_config_returns_ok() {
    let frame = RunFrame::new(RunId::new(1), StepIdx::ZERO, 3, 2);
    assert!(
        matches!(frame, Ok(ref f) if f.run_id() == RunId::new(1)
            && f.pc() == StepIdx::ZERO
            && f.step_count() == 3
            && f.slot_count() == 2),
        "frame must carry the requested dimensions and identity"
    );
}
```

### F-17: `crates/vb_runtime/src/together_tests.rs:73`

```rust
// BEFORE
assert!(run.add_parallel_in_flight(2).is_ok());
let result = together_join(...);
assert_eq!(result, Ok(vb_core::EngineSignal::Continue));

// AFTER
let before = run.parallel_in_flight();
assert_eq!(run.add_parallel_in_flight(2), Ok(()));
assert_eq!(run.parallel_in_flight(), before + 2, "add_parallel_in_flight must actually mutate");
let result = together_join(...);
assert_eq!(result, Ok(vb_core::EngineSignal::Continue));
```

### F-19: `crates/vb_core/tests/proptest_symbolic_code.rs:52`

```rust
// BEFORE
proptest! {
    #[test]
    fn from_str_rejects_unregistered(s in arb_unregistered_ascii()) {
        let parsed: Result<SymbolicCode, _> = s.as_str().parse();
        prop_assert!(parsed.is_err());
    }
}

// AFTER
proptest! {
    #[test]
    fn from_str_rejects_unregistered(s in arb_unregistered_ascii()) {
        let parsed: Result<SymbolicCode, _> = s.as_str().parse();
        prop_assert!(
            matches!(parsed, Err(SymbolicCodeParseError::UnknownCode { .. })),
            "unregistered string must yield UnknownCode, got {:?}", parsed
        );
    }
}
```

---

## 3. Pattern Census (counts per banned pattern per crate)

Counts derived from `rg` sweeps over `crates/vb_core/**/*.rs` and `crates/vb_runtime/**/*.rs` (excluding `target/`, `.evidence/`).

| Pattern | vb_core | vb_runtime | Total |
|---------|---------|------------|-------|
| `assert!(*.is_ok())` (bare smoke) | 4 | 23 | **27** |
| `assert!(*.is_err())` (bare smoke) | 6 | 4 | **10** |
| `assert_eq!(*.is_ok(), true)` (disguised smoke) | 0 | 8 | **8** |
| `prop_assert!(*.is_err())` (bare) | 4 | 0 | **4** |
| `let _ = result.*` (discard error path) | ~20 | ~25 | **~45** (mostly fixture cleanup, 1 silent suppression F-23) |
| `.unwrap()` total | ~50 | ~150 | **~200** (mostly fixture construction, see note) |
| `.expect()` total | ~15 | ~14 | **~29** (mostly fixture construction) |
| `panic!()` in tests | ~50 | ~50 | **~100** (mostly `match { ... other => panic!("expected X") }` enum destructuring) |
| `todo!()` / `unimplemented!()` | 0 | 0 | **0** |
| `#[ignore]` on behavior tests | 0 (after concurrency_safety.rs cleanup) | 0 | **0** |
| `#[should_panic]` without exact message | 0 | 0 | **0** |
| `sleep()` / `recv_timeout(Duration)` | 0 | 12 | **12** (timing-dependent, see F-26) |
| `lazy_static` / `OnceCell` / `OnceLock` / `static mut` / `thread_local!` | 0 | 0 | **0** |
| Bare `Some(_)` smoke pattern | 0 | 0 | **0** |
| `cfg(kani)` / `cfg(verus)` / `cfg(flux)` harnesses masquerading as tests | 6 | ~20 | **~26** (feature-gated, not a defect — see F-31) |

**Note on `.unwrap()` and `.expect()`:** the bulk of these calls in this slice are fixture construction (e.g., `BoundedActionCompletionQueue::new(N).unwrap()` to set up a fresh queue, `frame.write_slot(idx, val).unwrap()` to set up a RunFrame, `store.insert_list(...).unwrap()` to seed the value store). The rubric explicitly allows unwrap/expect for fixture construction. After filtering fixture usage, the residual BEHAVIOR-assertion unwraps are bounded to the LETHAL/HIGH findings already enumerated.

**Note on `panic!()`:** the 100 occurrences are predominantly idiomatic `match value { Ok(x) => ..., Err(e) => panic!("expected Ok, got {:?}", e) }` and `match variant { Foo => ..., other => panic!("expected Foo, got {:?}", other) }` patterns. These are acceptable: they convert a type mismatch into a visible failure rather than masking it. The rubric's `panic!()` prohibition applies to non-deterministic panic injections, not to enum-destructuring catch-alls.

---

## 4. Mutation Gaps — the 5 most dangerous mutations that would NOT be caught

| # | Production code location | Mutation | Why current tests miss it |
|---|--------------------------|----------|----------------------------|
| 1 | `crates/vb_runtime/src/engine/action_tests.rs::resolve_contract` — replace `Err(ResolveError::IdMismatch { requested, index })` body with `Err(ResolveError::IndexOutOfBounds)` | The `resolve_contract_rejects_id_mismatch` test asserts only `is_err()` (F-01). | Test passes; contract drift invisible. |
| 2 | `crates/vb_runtime/src/action_queue/action_queue_tests.rs::BoundedActionCompletionQueue::enqueue` — replace `Ok(())` with `Ok(false)` | `bounded_action_queue_enqueue_single_item_succeeds` asserts only `is_ok()` (F-04). | Test passes; the queue now silently rejects every enqueue. |
| 3 | `crates/vb_runtime/src/recovery/recover_runtime_summary.rs` (production) — replace Ok return with `Ok(RecoverySummary::default())` for all paths | `recover_runtime_summary_returns_recovery_hydration` and the `*_is_recoverable` cluster (2287, 2613, 2648, 2685) assert only `is_ok()` (F-09, and MEDIUM siblings). | All four tests pass; recovery silently returns empty summaries. |
| 4 | `crates/vb_runtime/src/frame_pool/pool.rs::FramePool::take` — replace `Ok(FrameRef { run_id: .., pc: .. })` with `Ok(FrameRef::default())` | Eight `assert_eq!(reused.is_ok(), true)` calls in `frame_pool/tests.rs` (F-13). | All eight tests pass; the pool recycles the wrong frame identities. |
| 5 | `crates/vb_runtime/src/primitives/together/add_parallel_in_flight.rs` — replace `Ok(())` with `Ok(())` but skip the `parallel_in_flight += delta` mutation | Fourteen `assert!(run.add_parallel_in_flight(N).is_ok())` calls in `together_tests.rs` (F-17). | All fourteen tests pass; together-join's parallel-counter invariant is silently broken. |

A sixth class worth flagging: **`vb_runtime/src/shard/lru_ring.rs::LruRing::insert`** return-Err variant mutation. Currently only `lru_ring_property_capacity_overflow_then_recover` (F-05) would survive a swap from `TerminalRunsLruFull` to any other error. The 6 other `lru_ring_red_queen_tests` and `red_queen_lru_concurrent` tests that exercise the Ok path rely on `.unwrap()` after a successful insert, so a "silent return-Err mutation" would panic visibly — those tests are safe. But a "wrong-Ok-payload" mutation would slip through `let _ = ring.insert(...)` (F-23).

---

## 5. Top 5 Fixes Ranked by Impact-per-Effort

1. **F-04 (`bounded_action_queue_enqueue_single_item_succeeds:240`)** — one-line change: `assert!(result.is_ok())` → `assert_eq!(result, Ok(()))`. Catches a class of silent enqueue regressions. Effort: 1 minute.

2. **F-01, F-02, F-03 (`action_tests.rs:267,289,296`)** — replace each `assert!(result.is_ok()/is_err())` with a `matches!` against the specific ResolveError variant. Three one-liners. Effort: 5 minutes. Catches resolve_contract contract drift.

3. **F-13 (`frame_pool/tests.rs:147,244,259,260,261,273,274,351`)** — replace each `assert_eq!(reused.is_ok(), true)` with `assert!(matches!(reused, Ok(f) if f.run_id() == ...))`. Eight edits. Effort: 15 minutes. Catches frame-pool recycling regressions.

4. **F-09, F-10, F-07, F-08, F-06 (`recovery_bdd_tests.rs:2141,2728,2843,2852,2883`)** — add concrete post-conditions to each `is_ok()/is_err()` smoke. Five edits. Effort: 30 minutes. Catches silent recovery-summary mutations.

5. **F-17 (`together_tests.rs:73,255,288,505,546,592,635,968,1016,1059,1106,1216,1420,1533`)** — after each `assert!(run.add_parallel_in_flight(N).is_ok())`, add `assert_eq!(run.parallel_in_flight(), before + N)`. 14 edits. Effort: 30 minutes. Catches silent together-counter regressions.

---

## 6. Verdict Line

STATUS: REJECTED

The 10 CRITICAL findings (F-01 through F-10) and 10 HIGH findings (F-11 through F-20) collectively describe tests that would pass if the behavior they are supposed to verify were deleted or had its variants swapped. Per the rubric, "If any finding is `blocker`, write `STATUS: REJECTED` and prevent advancement." All ten CRITICALs are blockers; the slice must be re-fixed before approval.

---

## 7. Disposition

| ID | Disposition |
|----|-------------|
| F-01 | blocker |
| F-02 | blocker |
| F-03 | blocker |
| F-04 | blocker |
| F-05 | blocker |
| F-06 | blocker |
| F-07 | blocker |
| F-08 | blocker |
| F-09 | blocker |
| F-10 | blocker |
| F-11 | owner_approved_debt |
| F-12 | owner_approved_debt |
| F-13 | blocker |
| F-14 | blocker |
| F-15 | blocker |
| F-16 | blocker |
| F-17 | blocker |
| F-18 | owner_approved_debt |
| F-19 | owner_approved_debt |
| F-20 | owner_approved_debt |
| F-21 | owner_approved_no_action (already has follow-up match) |
| F-22 | owner_approved_no_action (acceptable fixture cleanup) |
| F-23 | owner_approved_debt |
| F-24 | owner_approved_no_action (compile-time exhaustiveness check, not silent suppression) |
| F-25 | owner_approved_no_action (concrete follow-up present) |
| F-26 | owner_approved_debt |
| F-27 | owner_approved_no_action |
| F-28 | owner_approved_debt |
| F-29 | owner_approved_debt |
| F-30 | owner_approved_no_action |
| F-31 | owner_approved_no_action (verifier harnesses correctly feature-gated) |
| F-32 | owner_approved_no_action |
| F-33 | owner_approved_no_action |
| F-34 | owner_approved_no_action |
| F-35 | owner_approved_no_action |

**Summary by disposition:**
- blocker: F-01, F-02, F-03, F-04, F-05, F-06, F-07, F-08, F-09, F-10, F-13, F-14, F-15, F-16, F-17 (15 blockers)
- owner_approved_debt: F-11, F-12, F-18, F-19, F-20, F-23, F-26, F-28, F-29 (9 owner-approved debt items requiring bead filing)
- owner_approved_no_action: F-21, F-22, F-24, F-25, F-27, F-30, F-31, F-32, F-33, F-34, F-35 (11 no-action observations)

**Required actions before re-review:**
1. File one bead per blocker (15 beads) to track the 15 CRITICAL+HIGH test-rewrite obligations.
2. File 9 beads for the `owner_approved_debt` MEDIUM-class items, scheduled after blockers.
3. Re-run Tier 0 → Tier 3 of the test-review pipeline on the 15 affected files.
4. The `owner_approved_no_action` items are observations that do not block approval once the blockers are addressed.