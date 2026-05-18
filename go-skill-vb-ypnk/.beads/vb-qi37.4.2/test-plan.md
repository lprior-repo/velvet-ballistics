<<<<<<< HEAD
# Test Plan: vb-qi37.4.2 — Phase 3 Hot-Path Execution Core (test-planner state 7)

## Summary

- Behaviors identified: 38
- Trophy allocation: 12 unit / 18 integration / 4 e2e / 4 static-analysis
- Proptest invariants: 9
- Fuzz targets: 2 (1 missing, 1 existing)
- Kani harnesses: 14 missing (formal waivers filed); compensating coverage via integration tests

## Phase 1: Evidence Packets

### Already-PASS nextest Tests (no further action required)

| Filter | Obligation | Evidence |
|--------|------------|----------|
| `step_state_invalid` | VB-CORE-STATE-003 | nextest: 1 passed (StepState invalid transitions rejected) |
| `resource_policy` | VB-CORE-RESOURCE-004-PROP | nextest: 1 passed (WholeWorkflowBudget within BoundednessPolicy::DEFAULT) |
| `ast_bytecode_equiv` | VB-EXPR-001 | nextest: 1 passed (AST/bytecode equivalence differential) |
| `serde_json_` | VB-UI-MODEL-envelope-002 | nextest: 1 passed (envelope JSON roundtrip) |
| `envelope_` | VB-UI-MODEL-envelope-001 | nextest: 18 passed (roundtrip, redaction, schema) |

### PASS Verification Lanes (compensating evidence for DEFERRED_GLOBAL)

| Lane | Obligations | Status |
|------|-------------|--------|
| Verus L4 | 19 (taint_lattice, signals, step_state_machine, step_budget, run_frame_invariant, resource_budget) | All PASS |
| TLA+ L3 | 13 (LifecycleJournal, RetryFSM, CapabilityLifecycle, ConcurrencyControl) | All PASS |
| Proptest/Differential L1 | 5 (resource_policy, ast_bytecode_equiv, idempotency_key_well_formed, envelope_, serde_json_) | All PASS |
| Fuzz L2 | 2 (expr_eval 500k, decode_record 1M) | PASS |
| Loom L3 | 1 (concurrency bounded_queue) | PASS |
| Static-scan L0 | 2 (clippy: no unsafe, no panic) | PASS |

### DEFERRED_GLOBAL Obligations (19 total)

All have formal waivers filed in `.beads/vb-qi37.4.2/formal-waivers.jsonl`.

**14 missing Kani harnesses** (scope: missing-artifact): compensating evidence = corresponding Verus proofs (19 PASS) + fuzz/proptest layers.

**1 missing fuzz target** (VB-IPC-DECODE-FUZZ, scope: missing-artifact): `ipc_decode` target absent; compensating = decode_record fuzz (1M runs).

**1 missing xtask** (VB-CORE-IDX-002, scope: missing-tool): forbidden-scan unavailable; compensating = clippy clean.

**2 downstream gauntlet** (GATE-001/002, scope: downstream-blocked): will self-resolve when upstream clears.

---

## 2. Gaps: BDD Scenarios Not Yet Written

The following contract clauses have Verus/proptest/fuzz compensating evidence but lack explicit BDD scenario tests in the integration/unit layers. These are the gaps the test-writer must close.

### Gap A: FiniteF64 Precondition Coverage (PRE-003)

**Contract**: `FiniteF64::new(value)` requires `value.is_finite()` — rejects NaN, ±infinity.

**Already covered**: proptest `finite_f64_property` (PASS). **Gap**: no explicit BDD scenario naming the exact error variant.

```rust
// GIVEN: a non-finite f64 value (NaN or infinity)
// WHEN:  FiniteF64::new(value) is called
// THEN:  CoreError::NonFiniteNumber is returned exactly
fn finite_f64_rejects_nan_when_constructed() -> CoreResult<()>
fn finite_f64_rejects_positive_infinity_when_constructed() -> CoreResult<()>
fn finite_f64_rejects_negative_infinity_when_constructed() -> CoreResult<()>
fn finite_f64_accepts_valid_finite_value() -> CoreResult<()>
```

### Gap B: RunFrame Construction Preconditions (PRE-001, POST-001)

**Contract**: `RunFrame::new` requires `step_count > 0` and `first_step.as_usize() < step_count`.

```rust
// GIVEN: step_count = 0 and valid run_id, first_step, slot_count
// WHEN:  RunFrame::new is called
// THEN:  CoreError::InvalidCompiledWorkflow { reason: "step_count_zero" }
fn run_frame_new_rejects_zero_step_count()

// GIVEN: first_step >= step_count with step_count > 0
// WHEN:  RunFrame::new is called
// THEN:  CoreError::InvalidProgramCounter { step } (first_step PC invalid)
fn run_frame_new_rejects_first_step_out_of_bounds()

// GIVEN: valid dimensions (step_count > 0, first_step < step_count)
// WHEN:  RunFrame::new succeeds
// THEN:  frame.states.len() == step_count, frame.slots.len() == slot_count,
//         frame.taint.len() == slot_count, all states = Pending, all taint = Clean
fn run_frame_new_returns_correct_dimensions()
fn run_frame_new_initializes_all_states_to_pending()
fn run_frame_new_initializes_all_taint_to_clean()
```

### Gap C: RunFrame Reinitialize Preconditions (PRE-001 variant)

**Contract**: `reinitialize` checks same preconditions as `new` and additionally rejects dimension changes.

```rust
// GIVEN: a valid RunFrame
// WHEN:  reinitialize is called with step_count = 0
// THEN:  CoreError::InvalidCompiledWorkflow { reason: "step_count_zero" }
fn run_frame_reinitialize_rejects_zero_step_count()

// GIVEN: a valid RunFrame
// WHEN:  reinitialize is called with different step_count than construction
// THEN:  CoreError::InvalidCompiledWorkflow { reason: "frame_dimension_mismatch" }
fn run_frame_reinitialize_rejects_dimension_mismatch_step_count()
fn run_frame_reinitialize_rejects_dimension_mismatch_slot_count()

// GIVEN: a valid RunFrame
// WHEN:  reinitialize is called with valid dimensions
// THEN:  states reset to Pending, taint reset to Clean, dimensions unchanged
fn run_frame_reinitialize_preserves_dimensions()
fn run_frame_reinitialize_resets_states_to_pending()
```

### Gap D: WholeWorkflowBudget::compute Precondition (PRE-002)

**Contract**: `entry.as_usize() < nodes.len()`.

```rust
// GIVEN: entry >= nodes.len() with valid nodes slice
// WHEN:  WholeWorkflowBudget::compute is called
// THEN:  WorkflowError::EntryOutOfBounds { entry }
fn budget_compute_rejects_entry_out_of_bounds()

// GIVEN: valid entry within bounds
// WHEN:  WholeWorkflowBudget::compute succeeds
// THEN:  every field <= BoundednessPolicy::DEFAULT corresponding limit
fn whole_workflow_budget_within_policy_limits()
```

### Gap E: AggregateResourceBudget::try_take Precondition (PRE-006)

**Contract**: `amount <= budget.remaining`.

```rust
// GIVEN: budget with known remaining, amount > remaining
// WHEN:  try_take(amount) is called
// THEN:  Err(AggregateResourceBudgetExhausted) or Err(StepBudgetExhausted)
fn budget_try_take_rejects_amount_exceeding_remaining()
fn budget_try_take_accepts_exact_remaining()
fn budget_try_take_accepts_zero()
```

### Gap F: IPC Decoder Rejects-Before-Allocation (INV-011, POST-007)

**Contract**: IPC decoder returns `Err` before allocating any buffer when `header_len < 60` or `payload_len > MAX_PAYLOAD`.

```rust
// GIVEN: a byte slice with header_len < 60
// WHEN:  IpcFrameDecoder::decode_header is called
// THEN:  IpcError::HeaderTooShort is returned, no buffer allocated
fn ipc_decoder_rejects_header_too_short_before_allocation()

// GIVEN: a byte slice with payload_len > MAX_PAYLOAD
// WHEN:  IpcFrameDecoder::decode_frame is called
// THEN:  IpcError::PayloadTooLarge is returned, no buffer allocated
fn ipc_decoder_rejects_payload_too_large_before_allocation()

// GIVEN: valid header_len >= 60 and valid magic
// WHEN:  decode_frame succeeds
// THEN:  Frame is returned with correct slot values
fn ipc_decoder_accepts_valid_frame()
```

### Gap G: Record Decoder Rejects-Before-Allocation (INV-012, POST-008)

**Contract**: All five validations (magic, schema, kind, payload_len, CRC) occur before any heap allocation.

```rust
// GIVEN: record bytes with invalid magic
// WHEN:  RecordDecoder::decode is called
// THEN:  StorageError::RecordMagicInvalid, no heap allocation
fn record_decoder_rejects_magic_before_allocation()

// GIVEN: record bytes with invalid schema
// WHEN:  RecordDecoder::decode is called
// THEN:  StorageError::RecordSchemaInvalid, no heap allocation
fn record_decoder_rejects_schema_before_allocation()

// GIVEN: record bytes with invalid kind
// WHEN:  RecordDecoder::decode is called
// THEN:  StorageError::RecordKindInvalid, no heap allocation
fn record_decoder_rejects_kind_before_allocation()

// GIVEN: record bytes with invalid payload_len
// WHEN:  RecordDecoder::decode is called
// THEN:  StorageError::RecordPayloadLenInvalid, no heap allocation
fn record_decoder_rejects_payload_len_before_allocation()

// GIVEN: record bytes with invalid CRC
// WHEN:  RecordDecoder::decode is called
// THEN:  StorageError::RecordCrcInvalid, no heap allocation
fn record_decoder_rejects_crc_before_allocation()

// GIVEN: valid record bytes (all validations pass)
// WHEN:  RecordDecoder::decode succeeds
// THEN:  Record is returned
fn record_decoder_accepts_valid_record()
```

### Gap H: Journal Sequence Monotonicity (INV-009, POST-009)

**Contract**: Journal entry sequence numbers are strictly monotonically increasing per shard.

```rust
// GIVEN: two journal entries for the same shard with seqN >= seqN+1
// WHEN:  replay is attempted
// THEN:  replay detects violation and returns error
fn journal_sequence_rejects_non_monotonic_entries()

// GIVEN: valid journal entries for a shard
// WHEN:  replay succeeds
// THEN:  entries are applied in strictly increasing seq order
fn journal_replay_applies_entries_in_seq_order()
```

### Gap I: Idempotency Key Well-Formedness (INV-014)

**Contract**: Idempotency keys must satisfy `idempotency_key_well_formed`.

```rust
// GIVEN: a normalized, non-empty, valid-alphabet key within size bounds
// WHEN:  idempotency_key_well_formed is checked
// THEN:  true is returned
fn idempotency_key_accepts_valid_normalized_key()

// GIVEN: an empty key
// WHEN:  idempotency_key_well_formed is checked
// THEN:  false is returned
fn idempotency_key_rejects_empty_key()

// GIVEN: a key exceeding maximum size
// WHEN:  idempotency_key_well_formed is checked
// THEN:  false is returned
fn idempotency_key_rejects_oversized_key()

// GIVEN: a key containing invalid alphabet characters
// WHEN:  idempotency_key_well_formed is checked
// THEN:  false is returned
fn idempotency_key_rejects_invalid_alphabet()
```

### Gap J: Concurrency Invariants (INV-015)

**Contract**: Each shard has a single owner; no cross-shard mutable aliasing.

```rust
// GIVEN: two frames on the same shard
// WHEN:  concurrent access is attempted
// THEN:  only one owner at a time; other waits or gets error
fn shard_enforces_single_owner()

// GIVEN: frames on different shards
// WHEN:  concurrent access is attempted
// THEN:  no aliasing; both can proceed in parallel
fn different_shards_allow_parallel_access()

// GIVEN: frame pool with bounded capacity
// WHEN:  pool is at capacity and new frame is requested
// THEN:  pool does not grow unbounded; old frames recycled or error returned
fn frame_pool_respects_bounded_capacity()
```

### Gap K: StepBudget try_take Postconditions (POST-003)

**Contract**: `try_take` returns `Ok(remaining)` where `remaining == old - amount`, or `Err(StepBudgetExhausted)`.

```rust
// GIVEN: StepBudget with remaining >= amount
// WHEN:  try_take(amount) succeeds
// THEN:  Ok(remaining) where remaining == old_remaining - amount
fn step_budget_try_take_returns_correct_remaining()

// GIVEN: StepBudget with remaining < amount
// WHEN:  try_take(amount) is called
// THEN:  Err(StepBudgetExhausted), remaining unchanged
fn step_budget_try_take_returns_exhausted_error()

// GIVEN: StepBudget at zero remaining
// WHEN:  try_take(0) is called
// THEN:  Ok(0)
fn step_budget_try_take_accepts_zero_from_exhausted()

// GIVEN: StepBudget remaining is monotonically non-increasing
// WHEN:  multiple try_take calls are made
// THEN:  remaining never increases
fn step_budget_remaining_is_monotonic()
```

### Gap L: EngineSignal Finished Canonical Form (INV-010, POST-004)

**Contract**: `EngineSignal::Finished` always carries `(SlotValue, Taint)`, never legacy `Finished(SlotValue)`.

```rust
// GIVEN: a step reaches finished state
// WHEN:  EngineSignal is emitted
// THEN:  signal is Finished(SlotValue, Taint) — Taint field always present
fn finish_signal_always_carries_taint()

// GIVEN: a slot value with taint = Secret
// WHEN:  Finished is emitted
// THEN:  Taint field is Secret
fn finish_signal_propagates_secret_taint()

// GIVEN: a slot value with taint = Clean
// WHEN:  Finished is emitted
// THEN:  Taint field is Clean
fn finish_signal_propagates_clean_taint()
```

### Gap M: Resource Budget Saturating Arithmetic (POST-010)

**Contract**: Sequential composition = saturating add at policy max; branch = max; loop = saturating multiply.

```rust
// GIVEN: two resource budgets, sequential composition
// WHEN:  budgets are combined sequentially
// THEN:  result = min(sum, BoundednessPolicy::DEFAULT), no overflow/panic
fn resource_budget_sequential_sum_is_bounded()

// GIVEN: two resource budgets, branch composition
// WHEN:  budgets are combined via branch
// THEN:  result = max(a, b)
fn resource_budget_branch_takes_max()

// GIVEN: resource budget, loop composition (iteration count * body budget)
// WHEN:  loop budget is computed
// THEN:  result = min(product, BoundednessPolicy::DEFAULT), no overflow/panic
fn resource_budget_loop_multiply_is_bounded()
fn resource_budget_loop_at_policy_max_saturates()
```

---

## 3. Integration Tests (Real Dependencies)

These tests compose multiple components with real implementations — no mocks.

### IT-1: RunFrame Lifecycle with Real Engine

```rust
// GIVEN: a valid compiled workflow with N steps and M slots
// WHEN:  RunFrame::new succeeds, then reinitialize is called
// THEN:  dimensions preserved, states reset, taint reset
fn run_frame_lifecycle_with_engine()
```

### IT-2: IPC Frame → Record Decode Pipeline

```rust
// GIVEN: a valid IPC frame bytes sequence
// WHEN:  frame is decoded then record is encoded from the frame data
// THEN:  roundtrip produces original data
fn ipc_frame_to_record_roundtrip()

// GIVEN: invalid IPC frame bytes (header too short)
// WHEN:  decode is attempted
// THEN:  IpcError::HeaderTooShort returned before any allocation
fn ipc_frame_decode_rejects_before_allocation_integration()
```

### IT-3: Journal Replay with Real Storage

```rust
// GIVEN: a journal with entries for a single shard in increasing seq order
// WHEN:  replay is executed
// THEN:  actions are dispatched in journal order; final state matches expected
fn journal_replay_preserves_order()

// GIVEN: a journal with out-of-order sequence numbers
// WHEN:  replay is attempted
// THEN:  error is returned before any action dispatch
fn journal_replay_rejects_out_of_order()
```

### IT-4: Taint Propagation Through Expression Evaluation

```rust
// GIVEN: an expression tree with mixed taint operands
// WHEN:  expression is evaluated
// THEN:  result taint = join of all operand taints per taint lattice rules
fn taint_propagates_through_expression_evaluation()
fn taint_join_is_associative_in_evaluation()
fn taint_join_is_commutative_in_evaluation()
```

### IT-5: Budget Exhaustion Through Workflow

```rust
// GIVEN: a workflow where step budget is exhausted mid-execution
// WHEN:  engine processes steps until budget depleted
// THEN:  EngineSignal::StepBudgetExhausted is emitted, no underflow
fn engine_emits_budget_exhausted_signal()
fn step_budget_exhaustion_is_deterministic()
```

---

## 4. Proptest Invariants

### PI-1: Taint Lattice Laws

```rust
// Property: join is associative for all Taint combinations
proptest! {
    fn taint_join_associative(a: Taint, b: Taint, c: Taint) {
        assert_eq!(join(join(a, b), c), join(a, join(b, c)));
    }
}

// Property: join is commutative
proptest! {
    fn taint_join_commutative(a: Taint, b: Taint) {
        assert_eq!(join(a, b), join(b, a));
    }
}

// Property: join is idempotent
proptest! {
    fn taint_join_idempotent(a: Taint) {
        assert_eq!(join(a, a), a);
    }
}

// Property: Clean is identity
proptest! {
    fn taint_join_identity(a: Taint) {
        assert_eq!(join(Clean, a), a);
        assert_eq!(join(a, Clean), a);
    }
}

// Property: Secret is absorbing
proptest! {
    fn taint_join_secret_absorbing(a: Taint) {
        assert_eq!(join(Secret, a), Secret);
    }
}
```

### PI-2: StepBudget Monotonicity

```rust
proptest! {
    fn step_budget_never_increases(remaining: u32, amount: u32) {
        let mut budget = StepBudget::new(remaining);
        let before = budget.remaining();
        let _ = budget.try_take(amount);
        let after = budget.remaining();
        prop_assert!(after <= before);
    }
}
```

### PI-3: FiniteF64 Validity

```rust
proptest! {
    fn finite_f64_roundtrip(v: FiniteF64) {
        let raw = v.into_raw();
        let recovered = FiniteF64::new(raw);
        prop_assert!(recovered.is_ok());
    }
}

proptest! {
    fn nan_rejected(v in nan_strategy()) {
        prop_assert!(FiniteF64::new(v).is_err());
    }
}
```

### PI-4: RunFrame Dimension Stability

```rust
proptest! {
    fn frame_dimensions_immutable_after_reinit(
        run_id: RunId,
        first_step: StepIdx,
        step_count: u16,
        slot_count: u16,
    ) {
        let mut frame = RunFrame::new(run_id, first_step, step_count, slot_count)?;
        let original_step_count = frame.step_count;
        let original_slot_count = frame.slot_count;
        frame.reinitialize(run_id, first_step, step_count, slot_count)?;
        prop_assert_eq!(frame.step_count, original_step_count);
        prop_assert_eq!(frame.slot_count, original_slot_count);
    }
}
```

### PI-5: Idempotency Key Well-Formed Bounds

```rust
proptest! {
    fn idempotency_key_valid_keys_accepted(key: IdempotencyKey) {
        if key.as_bytes().len() > 0 && key.as_bytes().len() <= 64 {
            prop_assert!(idempotency_key_well_formed(&key));
        }
    }
}

proptest! {
    fn idempotency_key_empty_rejected() {
        prop_assert!(!idempotency_key_well_formed(&IdempotencyKey::default()));
    }
}
```

### PI-6: Resource Budget Boundedness

```rust
proptest! {
    fn workflow_budget_always_within_policy(nodes: Vec<CompiledNode>, entry: StepIdx) {
        let budget = WholeWorkflowBudget::compute(&nodes, entry, &Default::default())?;
        let policy = BoundednessPolicy::DEFAULT;
        // all fields bounded by policy
        prop_assert!(budget.step_count <= policy.max_step_count);
        prop_assert!(budget.slot_count <= policy.max_slot_count);
    }
}
```

### PI-7: StepState Transition Matrix Exhaustive

```rust
proptest! {
    fn all_valid_transitions_accepted(from: StepState, to: StepState) {
        if is_valid_transition(from, to) {
            let mut frame = RunFrame::new(...);
            frame.transition_state(to)?;
            prop_assert_eq!(frame.state(), to);
        }
    }
}

proptest! {
    fn all_invalid_transitions_rejected(from: StepState, to: StepState) {
        if !is_valid_transition(from, to) {
            let mut frame = RunFrame::new(...);
            prop_assert!(frame.transition_state(to).is_err());
        }
    }
}
```

### PI-8: Record Decoder Rejects-Before-Alloc

```rust
proptest! {
    fn record_decode_always_validates_before_allocation(record: ArbitraryRecord) {
        let bytes = record.encode();
        let before = get_heap_allocated_bytes();
        let result = RecordDecoder::decode(&bytes);
        let after = get_heap_allocated_bytes();
        // If decode failed, heap should not have grown
        if result.is_err() {
            prop_assert_eq!(after, before);
        }
    }
}
```

### PI-9: IPC Decoder Rejects-Before-Alloc

```rust
proptest! {
    fn ipc_decode_always_validates_before_allocation(frame: ArbitraryIpcFrame) {
        let bytes = frame.encode();
        let before = get_heap_allocated_bytes();
        let result = IpcFrameDecoder::decode_frame(&bytes);
        let after = get_heap_allocated_bytes();
        if result.is_err() {
            prop_assert_eq!(after, before);
        }
    }
}
```

---

## 5. Fuzz Targets

### FT-1: ipc_decode (MISSING — formal waiver filed)

**Risk**: panics, OOM, logic errors in IPC header parsing.

**Existing compensating evidence**: decode_record fuzz (1M runs), expr_eval fuzz (500k runs), TLA+ ConcurrencyControl.

**Recommendation**: Write `fuzz/fuzz_targets/ipc_decode.rs` targeting `vb_ipc::decode_frame_header` with:
- Corpus seeds: minimal valid header, header at each boundary (59, 60, 61 bytes), max payload frames
- Mutation: flip bytes in header fields, corrupt magic, corrupt length fields

### FT-2: envelope_redaction (existing via VB-UI-MODEL-envelope-001)

Already covered by 18 nextest tests plus `cargo fuzz run ui_redaction_artifact`.

---

## 6. Kani Harnesses (Missing — Formal Waivers Filed)

All 14 missing Kani harnesses have compensating evidence and formal waivers. Integration test coverage for each:

| Missing Harness | Compensating Coverage | Integration Gap Fill |
|-----------------|----------------------|---------------------|
| `kani_taint_propagation` | Verus taint_lattice (13 verified) + `test_taint_eval_expr_join` | Gap F (IT-4 above) |
| `kani_step_budget_zero/one` | Verus step_budget (6 verified) + Gap K | Gap K + IT-5 |
| `kani_step_budget` | Verus + proptest | Gap K |
| `kani_index_access` | Verus + clippy clean | INV-009 gap: `test_checked_index_validates_bounds` |
| `kani_resource_budget_bounded` | Verus resource_budget + `resource_policy` | Gap M + IT-5 |
| `kani_ipc_header` (x2) | TLA+ + decode_record fuzz | Gap F + IT-2 |
| `kani_ipc_header_rejects_oversize` | TLA+ | Gap F |
| `kani_record_magic/schema/kind/payload_len/crc` | decode_record fuzz (1M) | Gap G + IT-2 |
| `kani_expr_stack` | expr_eval fuzz (500k) | Gap A + IT-4 |

---

## 7. Mutation Checkpoints

Threshold: ≥90% mutation kill rate.

### Critical Mutations

| Function | Mutation | Must Be Caught By |
|----------|----------|-------------------|
| `join_taint` | Remove Secretabsorbing case | `test_taint_no_downgrade_Secret` |
| `join_taint` | Remove DerivedFromSecret absorbing | `test_taint_no_downgrade_DerivedFromSecret` |
| `StepBudget::try_take` | Allow underflow | `test_step_budget_try_take_no_underflow` |
| `RunFrame::reinitialize` | Allow dimension change | `test_frame_dimension_immutable_after_reinitialize` |
| `FiniteF64::new` | Accept NaN | `test_finite_f64_rejects_nan` |
| `EngineSignal::Finished` | Omit Taint field | `test_finish_signal_carries_taint` |
| `RecordDecoder::decode` | Skip CRC check | `test_record_crc_validation` |
| `RecordDecoder::decode` | Skip magic check | `test_record_magic_validation` |
| `IpcFrameDecoder::decode_header` | Skip header_len check | `test_ipc_header_validation_rejects_short` |
| `IpcFrameDecoder::decode_frame` | Skip payload_len check | `test_ipc_header_validation_rejects_oversize` |

---

## 8. E2E Smoke Tests

### E2E-1: Full Workflow Execution

```bash
# GIVEN: a minimal compiled workflow with 3 steps
# WHEN:  velvet-ballistics executes the workflow
# THEN:  exit code 0, correct slot values in output
cargo run -- --workflow test-workflows/minimal.yaml
```

### E2E-2: Budget Exhaustion Signal

```bash
# GIVEN: a workflow with step_budget = 1
# WHEN:  workflow requires > 1 step
# THEN:  engine signals StepBudgetExhausted, workflow fails gracefully
```

### E2E-3: IPC Frame Rejection

```bash
# GIVEN: an oversized IPC frame (> MAX_PAYLOAD)
# WHEN:  frame is submitted to engine
# THEN:  IpcError::PayloadTooLarge returned, no crash
```

### E2E-4: Envelope Redaction in CLI Output

```bash
# GIVEN: an envelope with sensitive data marked as Secret taint
# WHEN:  velvet-ballistics outputs the envelope as JSON
# THEN:  redacted fields are null or [REDACTED], not actual values
```

---

## 9. Open Questions

| # | Question | Owner | Resolution |
|---|----------|-------|------------|
| OQ-1 | Does `idempotency_key_well_formed` accept Unicode or ASCII-only? | spec | Ascii-only per contract; confirm or update |
| OQ-2 | What is `MAX_PAYLOAD` constant value for IPC frames? | vb_ipc | Needed for Gap F boundary test |
| OQ-3 | Is there a maximum retry count for journal replay? | vb_storage | Needed for Gap H |
| OQ-4 | What is the `BoundednessPolicy::DEFAULT` values for each field? | vb_core::budget | Needed for Gap M assertion values |
| OQ-5 | Does `ui_redaction_artifact` fuzz target exist? | fuzz/fuzz_targets | Need to confirm for FT-2 |
=======
# Test Plan: vb-qi37.4.2 - Strict Runtime Admission Before Run Creation

## Summary

- Bead: `vb-qi37.4.2`
- Scope: runtime enforcement of persisted accepted-artifact envelopes before strict/journaled run creation.
- Planning state: go-skill State 7 test-planner.
- Inputs approved: `proof-review.md` (`STATUS: APPROVED`) and `contract-verification-review.md` (`STATUS: APPROVED`).
- Behaviors identified: 16
- Trophy allocation: 8 unit / 7 integration / 1 E2E, plus static gates for all scoped files. Deviation from the nominal 60% integration ratio is intentional: this bead has a high-risk pure admission predicate/error taxonomy surface that needs exhaustive calc coverage before broader integration.
- Proptest invariants: 6
- Fuzz targets: 3
- Kani harnesses: 2 planned/deferred-policy harnesses
- Mutation threshold: `>= 90%` kill rate, with zero surviving mutants for diagnostic category, digest preservation, no-allocation denial, and gate/capability comparison branches.
- No production code or test code is written by this plan.

## Startup Skill Citations

- Read `/home/lewis/.claude/skills/test-planner/SKILL.md`: requires behavior inventory, Testing Trophy allocation, Given/When/Then scenarios, proptest/fuzz/Kani/mutation checkpoints, exact value/error assertions, and `test-plan.md` output only.
- Read `/home/lewis/.agents/skills/test-planner/SKILL.md`: same content; per instruction the `.agents` copy wins if conflict exists.
- Read `/home/lewis/.agents/skills/test-planner/references/testing-philosophy.md`: test behaviors via public APIs, prefer real implementations/fakes over mocks, use DAMP scenario names, and reject `is_ok()`/`is_err()`-only assertions.

## 0. Input and Isolation Evidence

- Workspace verified by command: `pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2" && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; esac && jj workspace root && jq -c . .beads/vb-qi37.4.2/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-qi37.4.2/proof-obligations.planned.jsonl >/dev/null && jq -c . .beads/vb-qi37.4.2/traceability-matrix.jsonl >/dev/null`.
- Command exit: 0.
- Output roots: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2` for both `pwd -P` and `jj workspace root`.
- Source checkout `/home/lewis/src/velvet-ballistics` was not written.
- JSONL inputs parsed: `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, `traceability-matrix.jsonl`.

## 1. Behavior Inventory

| ID | Behavior | Contract clauses | Trace tests / obligations |
|---|---|---|---|
| B01 | Strict runtime rejects missing artifacts before allocation when requested digest is absent. | PRE-001, POST-002, POST-003, ERR-001 | `given_missing_artifact_when_strict_run_created_then_artifact_not_found_before_allocation`; TEST-STRICT-009, TLA-ADMIT-001 |
| B02 | Strict runtime rejects raw `WorkflowParts`, YAML, JSON, truncated postcard, and malformed bytes before allocation when bytes are not accepted-artifact v1. | PRE-001, INV-003, POST-002, ERR-002 | `given_raw_or_malformed_bytes_when_strict_run_created_then_decode_failed_with_rejected_digest`; FUZZ-ENV-008, TEST-STRICT-009 |
| B03 | Strict runtime rejects decoded envelopes missing required acceptance fields, unsupported schema, non-durable proof, or unsupported proof status when envelope is semantically invalid. | PRE-002, PRE-004, INV-003, ERR-003 | `given_decoded_envelope_missing_required_acceptance_fields_then_invalid_envelope_denies`; VERUS-ENV-006, TEST-STRICT-009 |
| B04 | Strict runtime rejects gate-count or gate-status mismatch when canonical accepted-artifact gate contract is not satisfied. | PRE-002, INV-001, ERR-004 | `given_gate_count_zero_two_or_failed_status_when_strict_run_created_then_gate_mismatch_denies`; TLA-GATE-002, VERUS-ENV-006 |
| B05 | Strict runtime rejects digest mismatch when requested digest, persisted record digest, or envelope digest differ. | PRE-003, ERR-005 | `given_digest_mismatch_when_strict_run_created_then_digest_mismatch_denies`; KANI-DIGEST-007, TEST-STRICT-009 |
| B06 | Strict runtime rejects stale certificate/evidence before allocation when admission metadata is stale for the required profile. | PRE-004, ERR-006 | `given_stale_artifact_when_strict_run_created_then_stale_certificate_denies`; VERUS-ENV-006, TEST-STRICT-009 |
| B07 | Strict runtime rejects missing, excess, duplicate, prefix-only, or action-mismatched capabilities when granted profile is not exact. | PRE-005, INV-006, ERR-007 | `given_missing_excess_prefix_or_action_mismatched_capability_then_capability_denied`; VERUS-CAP-005, TLA-CAP-003 |
| B08 | Runtime/API/CLI/IPC error mapping preserves admission category, rejected digest when available, and semantic cause when diagnostics cross boundaries. | POST-004, INV-007, ERR-008 | `given_cli_ipc_runtime_error_mapping_when_serialized_then_error_category_digest_and_cause_are_preserved`; MUT-DIAG-011, TEST-STRICT-009 |
| B09 | Successful strict/journaled admission creates a run only after loading accepted-artifact v1 and validating digest, gates, durability, non-staleness, and exact capabilities. | PRE-001..006, POST-001, POST-005 | `given_valid_accepted_artifact_when_admitted_then_admission_record_contains_digest_certificate_profile`; TEST-STRICT-009 |
| B10 | Successful admission records artifact digest, admission certificate/profile, and metadata needed by downstream header persistence. | POST-005 | `given_valid_accepted_artifact_when_admitted_then_admission_record_contains_digest_certificate_profile`; TLA-ADMIT-001 |
| B11 | Any admission denial leaves no frame, run map entry, runnable state, `drive_run`, `RunAccepted`, or success acknowledgement. | POST-003, INV-005 | `given_any_admission_error_when_runtime_returns_then_no_frame_run_or_drive_state_allocated`; TLA-ADMIT-001 |
| B12 | Strict/journaled production constructors require a storage-backed accepted-artifact loader and cannot use `AlwaysPresentArtifactStore`/existence-only checks. | PRE-006, INV-002 | `given_strict_journaled_runtime_when_constructed_then_storage_backed_artifact_store_is_required`; TLA-BYPASS-004, STATIC-BYPASS-010 |
| B13 | Runtime admission never parses YAML or JSON after accepted-artifact handoff when admitting strict/journaled persisted artifacts. | POST-001, INV-004 | `given_valid_accepted_artifact_when_runtime_admits_then_yaml_json_decoder_is_not_called`; STATIC-BYPASS-010, TEST-STRICT-009 |
| B14 | Relaxed/test-only admission may use dummy existence behavior only outside protected strict/journaled production paths. | PRE-006, INV-002 | `given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied`; TLA-BYPASS-004 |
| B15 | Storage and runtime share exactly one canonical accepted-artifact gate-count contract until upstream contract changes it. | PRE-002, INV-001 | `given_storage_and_runtime_gate_constants_when_compared_then_single_canonical_gate_contract_holds`; VERUS-ENV-006 |
| B16 | Resource/budget admission errors remain distinct from artifact-envelope/capability admission errors when budgeted admission is used. | POST-002, INV-007 | delivery-scope `admit_run_with_budget`; local error taxonomy guard |

## 2. Trophy Allocation

| Behavior IDs | Layer | Planned tests | Rationale |
|---|---|---:|---|
| B03, B04, B05, B06, B07, B08, B15, B16 | Unit / Calc | 8 | Pure predicates, constructors, error mapping, exact capability/gate/digest distinctions, and budget error separation must be exhaustive and fast. |
| B01, B02, B05, B07, B09, B10, B11 | Integration | 7 | Admission is valuable only at runtime/storage/journal boundaries with real or in-memory Fjall-backed stores; verify state, not interactions. |
| B08 | E2E / acceptance | 1 | CLI/IPC/API diagnostics must be black-box checked once to prove caller-observable category/digest/cause preservation. |
| B12, B13, B14, B15 plus all scoped source | Static gates | all | Bypass prevention and no runtime YAML/JSON parsing require source audit plus lint; grep alone is not proof without review. |

## 3. BDD Scenarios

### Behavior B01: missing artifact denies before allocation

Test name: `given_missing_artifact_when_strict_run_created_then_artifact_not_found_before_allocation`

Given: a strict or journaled runtime backed by an accepted-artifact store with no record for requested digest `D`.

When: a run is created for digest `D`.

Then: the result is exactly `AdmissionError::ArtifactNotFound { digest: D }` or `RuntimeError::AdmissionArtifactNotFound { digest: D }`.

And: journal/state inspection shows no frame taken, no run entry inserted, no runnable state, no `drive_run`, and no `RunAccepted` event.

Mapped clauses: PRE-001, POST-002, POST-003, ERR-001. Obligations: TEST-STRICT-009, TLA-ADMIT-001.

### Behavior B02: raw or malformed bytes deny as decode failure

Test name: `given_raw_or_malformed_bytes_when_strict_run_created_then_decode_failed_with_rejected_digest`

Given: storage contains a record at requested digest `D` whose bytes are one of raw `WorkflowParts`, YAML bytes, JSON bytes, empty bytes, truncated postcard bytes, or random malformed bytes.

When: strict or journaled runtime creates a run for `D`.

Then: the result is exactly `AdmissionError::ArtifactEnvelopeDecodeFailed` or the runtime mapped invalid-envelope variant that preserves `D` and decode/malformed cause.

And: no allocation or `RunAccepted` event exists.

Mapped clauses: PRE-001, INV-003, ERR-002. Obligations: FUZZ-ENV-008, TEST-STRICT-009.

### Behavior B03: semantically invalid decoded envelope fails closed

Test name: `given_decoded_envelope_missing_required_acceptance_fields_then_invalid_envelope_denies`

Given: storage contains a decoded accepted-artifact-like envelope at digest `D` with unsupported schema/version, missing acceptance field, `durable == false`, unsupported proof status, or absent required proof marker.

When: strict or journaled runtime admits `D`.

Then: the result is exactly `AdmissionError::ArtifactEnvelopeInvalid` or the current precise runtime variant (`ArtifactInvalidProofFlag { flag }` / invalid envelope mapping) with the semantic invalid cause preserved.

And: no allocation occurs.

Mapped clauses: PRE-002, PRE-004, INV-003, ERR-003. Obligations: VERUS-ENV-006, TEST-STRICT-009.

### Behavior B04: gate mismatch denies with observed and required gates

Test name: `given_gate_count_zero_two_or_failed_status_when_strict_run_created_then_gate_mismatch_denies`

Given: storage contains an accepted-artifact envelope at digest `D` with `gate_count` in `{0, 2, 14, 16}` or failed gate status while `REQUIRED_GATE_COUNT == 15`.

When: strict or journaled runtime creates a run for `D`.

Then: the result is exactly `AdmissionError::ArtifactGateMismatch` or current `AdmissionError::ArtifactInvalidGateCount { found, required: 15 }` with observed `found` preserved.

And: no allocation occurs.

Mapped clauses: PRE-002, INV-001, ERR-004. Obligations: TLA-GATE-002, VERUS-ENV-006, TEST-STRICT-009.

### Behavior B05: digest mismatch denies without diagnostic collapse

Test name: `given_digest_mismatch_when_strict_run_created_then_digest_mismatch_denies`

Given: requested digest `D_req`, persisted record digest `D_record`, and envelope digest `D_env` are not all identical.

When: strict or journaled runtime admits `D_req`.

Then: the result is exactly `AdmissionError::ArtifactDigestMismatch` or equivalent typed runtime invalid diagnostic that preserves all available requested/observed digest identities.

And: the error is not collapsed into generic decode failure or artifact not found.

And: no allocation occurs.

Mapped clauses: PRE-003, ERR-005. Obligations: KANI-DIGEST-007, TEST-STRICT-009.

### Behavior B06: stale artifact denies before allocation

Test name: `given_stale_artifact_when_strict_run_created_then_stale_certificate_denies`

Given: an accepted-artifact envelope has otherwise valid fields but stale admission certificate/evidence for the required runtime profile.

When: strict or journaled runtime admits the artifact.

Then: the result is exactly `AdmissionError::ArtifactStale` or the current explicit invalid-envelope mapping with staleness cause and rejected digest preserved.

And: no allocation occurs.

Mapped clauses: PRE-004, ERR-006. Obligations: VERUS-ENV-006, TEST-STRICT-009.

### Behavior B07: capability profile must be exact

Test name: `given_missing_excess_prefix_or_action_mismatched_capability_then_capability_denied`

Given: an accepted artifact requires capability profile `R`.

When: strict admission is attempted with grants that are missing one required capability, include an excess capability, duplicate a grant, use a lexical prefix only, or use the right name with the wrong action.

Then: the result is exactly `AdmissionError::CapabilityDenied { action, required, granted }` with the mismatch class reconstructable from fields.

And: no allocation occurs.

Mapped clauses: PRE-005, INV-006, ERR-007. Obligations: TLA-CAP-003, VERUS-CAP-005, TEST-STRICT-009.

### Behavior B08: public diagnostics preserve category, digest, and cause

Test name: `given_cli_ipc_runtime_error_mapping_when_serialized_then_error_category_digest_and_cause_are_preserved`

Given: each admission error category ERR-001 through ERR-007 is produced by runtime admission.

When: the error is mapped through `RuntimeError`, API, CLI, or IPC serialization.

Then: the caller observes the exact category (`not_found`, `decode_failed`, `invalid_envelope`, `gate_mismatch`, `digest_mismatch`, `stale`, `capability_denied`) and the rejected digest when present.

And: semantic cause fields such as gate count, required gate, required/granted capability, or malformed cause are not erased.

Mapped clauses: POST-004, INV-007, ERR-008. Obligations: MUT-DIAG-011, TEST-STRICT-009.

### Behavior B09: valid accepted artifact admits only after full validation

Test name: `given_valid_accepted_artifact_when_run_created_then_runtime_does_not_parse_yaml_or_json`

Given: storage contains accepted-artifact v1 with digest match, gate_count 15, all proof flags/statuses accepted, durable and non-stale metadata, and exact capability profile.

When: strict or journaled runtime creates a run.

Then: admission succeeds with a `RunAdmission`/`AdmissionRecord` containing the exact requested digest, run id, granted capabilities, and strict/journaled policy.

And: any runtime YAML/JSON parse hooks remain unused/not linked on the admission path.

Mapped clauses: PRE-001..006, POST-001, POST-005, INV-004. Obligations: TEST-STRICT-009, STATIC-BYPASS-010.

### Behavior B10: successful admission records downstream metadata

Test name: `given_valid_accepted_artifact_when_admitted_then_admission_record_contains_digest_certificate_profile`

Given: a valid accepted artifact with admission certificate/profile metadata.

When: admission succeeds.

Then: the admission record/journal-adjacent metadata contains the exact artifact digest, certificate/profile identity, gate evidence summary, and required capability profile needed by downstream header-persistence work.

Mapped clauses: POST-005. Obligations: TEST-STRICT-009.

### Behavior B11: every denial is pre-allocation

Test name: `given_any_admission_error_when_runtime_returns_then_no_frame_run_or_drive_state_allocated`

Given: a table of denial fixtures covering ERR-001 through ERR-007.

When: each fixture attempts strict/journaled run creation.

Then: each result is the exact expected error variant/value.

And: for every row no frame is removed from the pool, no run id exists in runtime state, no runnable state exists, no `drive_run` occurs, and no `RunAccepted` journal event is emitted.

Mapped clauses: POST-003, INV-005. Obligations: TLA-ADMIT-001, TEST-STRICT-009.

### Behavior B12: strict/journaled constructors require storage-backed accepted-artifact loading

Test name: `given_strict_journaled_runtime_when_constructed_then_storage_backed_artifact_store_is_required`

Given: production strict/journaled runtime construction through shard, runtime, CLI, and IPC entry points.

When: constructors are called without a storage-backed accepted-artifact loader or verified equivalent.

Then: construction fails with a typed unsupported/missing-store error or uses `StorageArtifactStore`.

And: `AlwaysPresentArtifactStore` is reachable only from relaxed/test-only constructors.

Mapped clauses: PRE-006, INV-002. Obligations: TLA-BYPASS-004, STATIC-BYPASS-010.

### Behavior B13: strict admission never parses YAML/JSON

Test name: `given_valid_accepted_artifact_when_runtime_admits_then_yaml_json_decoder_is_not_called`

Given: an accepted-artifact envelope already persisted for strict/journaled admission.

When: runtime admits the artifact.

Then: no `serde_yaml`, `serde_json`, or raw `WorkflowParts` parser path is executed or statically reachable from strict admission.

Mapped clauses: POST-001, INV-004. Obligations: STATIC-BYPASS-010, TEST-STRICT-009.

### Behavior B14: existence-only store cannot satisfy protected strict submission

Test name: `given_existence_only_artifact_check_when_strict_admission_then_bypass_is_denied`

Given: a store or path that can only answer `compiled_ir_exists == true` without loading accepted-artifact evidence.

When: protected strict/journaled submission attempts to use that path.

Then: admission is denied or construction is rejected before run creation.

And: no success acknowledgement is emitted.

Mapped clauses: PRE-006, INV-002. Obligations: TLA-BYPASS-004, STATIC-BYPASS-010.

### Behavior B15: runtime and storage use a single canonical gate count

Test name: `given_storage_and_runtime_gate_constants_when_compared_then_single_canonical_gate_contract_holds`

Given: runtime `REQUIRED_GATE_COUNT` and storage accepted-artifact gate-count metadata.

When: accepted artifact fixtures are generated and admitted.

Then: only the canonical value `15` is accepted until an explicit contract update changes both sides.

And: storage artifacts with gate_count `0` or `2` deny under strict runtime.

Mapped clauses: PRE-002, INV-001. Obligations: TLA-GATE-002, VERUS-ENV-006.

### Behavior B16: budget errors remain distinct from admission envelope errors

Test name: `given_budget_over_capacity_when_admission_with_budget_runs_then_resource_capacity_error_is_preserved`

Given: budgeted admission receives an available capacity lower than requested budget and an otherwise present/valid artifact context.

When: `admit_run_with_budget` or its replacement public API is called.

Then: the result is exactly `AdmissionError::ResourceCapacityExceeded { resource, requested, available }`.

And: this error is not mapped to artifact not found, invalid envelope, or capability denied.

Mapped clauses: POST-002, INV-007.

## 4. Unit Test Plan

Unit tests must be small, deterministic, and assert exact values/errors. No test may stop at `is_ok()` or `is_err()`.

| Unit group | Public surface | Cases | Expected exact assertions | Trace |
|---|---|---|---|---|
| Accepted envelope validation | `AcceptedArtifactStore::load_accepted_artifact`, `admit_artifact_run` through fake in-memory store | valid, missing, decode fail, unsupported schema, gate 0/2/14/16, failed each proof flag, durable false, stale | Exact `ArtifactEnvelopeError` and mapped `AdmissionError` variants, preserving found/required gate and digest/cause | PRE-001..004, ERR-001..006 |
| Digest consistency | `admit_artifact_run` / storage-backed loader | `D_req == D_record == D_env`; each pairwise mismatch; all distinct | Exact digest mismatch diagnostic with requested and observed digests; success only for all equal | PRE-003, ERR-005, KANI-DIGEST-007 |
| Capability exactness | `check_capability`, `admit_artifact_run` | exact match, missing, excess, duplicate, prefix-only, action mismatch, empty required with extra grant | Exact `CapabilityDenied { action, required, granted }`; success only for equal cardinality and exact grants | PRE-005, INV-006, ERR-007 |
| Diagnostic mapping | conversion to `RuntimeError` and CLI/IPC/API serializable forms | every ERR-001..ERR-008 category | Exact public category, digest, semantic cause; no category collapse | POST-004, INV-007, ERR-008 |
| Admission record | `RunAdmission` / `AdmissionRecord` accessors | strict, journaled, relaxed, budgeted | Exact digest, run id, capabilities, policy, budget/certificate/profile fields | POST-005 |
| Budget separation | `admit_run_with_budget` or successor | capacity exceeded, overflow, valid budget | Exact `ResourceCapacityExceeded` for budget failures; not artifact/capability variants | INV-007 |
| Canonical gate constant | runtime/storage constants and fixture constructors | 15, 0, 2, boundary u8 values | Only 15 accepted; found/required preserved for all others | INV-001, ERR-004 |
| Relaxed/test-only boundary | relaxed admission constructors | relaxed missing artifact, strict missing artifact, journaled missing artifact | Relaxed admits with exact record; strict/journaled return exact not-found/invalid-store errors | PRE-006, INV-002 |

## 5. Integration Test Plan

Use real storage/journal components or in-memory/local-temp Fjall-backed stores where available. Prefer fakes only for deterministic accepted-artifact fixture loading; never mock read-only queries by interaction count.

| Integration scenario | Components | Expected observable state | Trace |
|---|---|---|---|
| missing artifact pre-allocation | Runtime + Shard + Journal + empty `StorageArtifactStore` | Exact not-found error; no frame/run/runnable/`RunAccepted`; journal unchanged for accepted event | ERR-001, POST-003 |
| raw/malformed bytes pre-allocation | Storage writes raw `WorkflowParts`, YAML, JSON, empty, truncated, malformed bytes; runtime strict create | Exact decode failed/invalid envelope with digest and cause; no allocation | ERR-002, INV-003 |
| gate/proof/stale matrix | Storage persists accepted-artifact variants with gate 0/2/14/16, failed flags, durable false, stale | Exact gate mismatch/invalid/stale diagnostics; no allocation | ERR-003, ERR-004, ERR-006 |
| digest mismatch | Storage record/envelope/request mismatch table | Exact digest mismatch; no generic invalid-envelope collapse; no allocation | PRE-003, ERR-005 |
| capability mismatch | Accepted artifact with required profile + granted profile matrix | Exact `CapabilityDenied`; no allocation | PRE-005, ERR-007 |
| valid strict/journaled admit | Storage-backed accepted artifact with gate 15, non-stale, durable, exact capabilities | Run accepted after admission; record contains digest/certificate/profile; no YAML/JSON parsing | POST-001, POST-005 |
| constructor bypass prevention | Shard/Runtime/CLI/IPC constructors | Protected strict/journaled paths use `StorageArtifactStore` or reject construction; no `AlwaysPresentArtifactStore` success path | PRE-006, INV-002 |
| public diagnostics | Runtime error mapping through API/CLI/IPC serializers | Category, digest, and semantic cause retained for ERR-001..ERR-008 | ERR-008 |

## 6. E2E / Acceptance Test Plan

### Acceptance: strict CLI/IPC rejects unaccepted artifacts before success acknowledgement


Test name: `given_cli_or_ipc_strict_run_with_unaccepted_artifact_then_diagnostic_is_typed_and_no_run_is_accepted`

Given: a temporary workspace/storage containing either missing, raw, malformed, gate-mismatched, digest-mismatched, stale, or capability-mismatched artifact fixture.

When: the user invokes the strict/journaled CLI or IPC run path with the artifact digest.

Then: process/API result exposes the exact admission category, digest when available, and semantic cause.

And: a follow-up journal/state query shows no accepted/runnable run and no `RunAccepted` event.

Trace: POST-002, POST-003, POST-004, ERR-001..ERR-008.

## 7. Proptest Invariants

### Proptest P01: exact capability profiles admit if and only if sets are identical

Invariant: for any generated required capability list and granted capability set, strict admission succeeds iff granted profile has the same cardinality and every required `(name, action)` exactly matches one granted capability.

Strategy: generate small non-empty and empty vectors of `Capability` with unique `(name, action)` pairs; generate mutations adding missing, excess, duplicate, prefix, suffix, and wrong-action grants.

Anti-invariant: any missing, excess, duplicate, prefix-only, or action-mismatched grant must return exact `CapabilityDenied`.

Trace: PRE-005, INV-006, ERR-007, PO-009, VERUS-CAP-005.

### Proptest P02: gate count acceptance is singleton canonical 15

Invariant: for any `u8 gate_count`, accepted-envelope validation succeeds on gate count only when `gate_count == 15` and all other validity inputs are true.

Strategy: generate `u8` gate counts with weighted seeds `[0, 1, 2, 14, 15, 16, 255]` plus random values.

Anti-invariant: any gate count other than 15 must produce exact gate mismatch/invalid gate diagnostic with found and required fields.

Trace: PRE-002, INV-001, ERR-004, VERUS-ENV-006.

### Proptest P03: fail-closed envelope predicate

Invariant: an accepted-envelope-like value admits only when schema v1, canonical gate count, durable/non-stale evidence, accepted statuses, digest equality, and all required proof flags are true.

Strategy: generate boolean/product-space accepted-envelope records; shrink to single invalid field.

Anti-invariant: any unknown schema, missing field, stale evidence, non-durable marker, unsupported proof status, or failed required flag denies with a typed cause.

Trace: PRE-002, PRE-004, INV-003, ERR-003, ERR-006, PO-009.

### Proptest P04: digest equality is required across requested, persisted, and envelope identities

Invariant: admission may succeed only when requested digest, persisted record digest, and envelope digest are equal.

Strategy: generate triples of 32-byte digests with cases all equal, one differing, two differing, all distinct.

Anti-invariant: any inequality returns exact digest mismatch with all available identities preserved.

Trace: PRE-003, ERR-005, KANI-DIGEST-007.

### Proptest P05: diagnostic mapping is injective over admission error categories

Invariant: mapping from `AdmissionError` categories to public runtime/API/CLI/IPC diagnostics preserves enough data to distinguish all ERR-001..ERR-008 categories.

Strategy: generate representative admission errors with arbitrary digests, gates, flags, capabilities, stale causes, and malformed causes.

Anti-invariant: two different categories must not serialize to the same category/cause tuple unless the contract explicitly marks them equivalent.

Trace: POST-004, INV-007, ERR-008, MUT-DIAG-011.

### Proptest P06: denial is state-invariant

Invariant: for every generated invalid artifact/capability/digest/gate fixture, runtime state snapshot before and after denial is equal for frame allocation, run map membership, runnable state, drive state, and `RunAccepted` journal events.

Strategy: generate denial fixture enum covering ERR-001..ERR-007 with deterministic temporary journal/runtime setup.

Anti-invariant: any invalid fixture that changes run allocation state fails the property.

Trace: POST-003, INV-005, TLA-ADMIT-001, TEST-STRICT-009.

## 8. Fuzz Targets

### Fuzz Target F01: accepted-artifact envelope hostile bytes

Target: `fuzz/fuzz_targets/accepted_artifact_envelope.rs` or equivalent accepted-envelope decode boundary.

Input type: bytes.

Risk: panic, OOM, malformed postcard acceptance, raw YAML/JSON/WorkflowParts acceptance, digest/cause loss.

Corpus seeds: empty bytes, one byte, truncated postcard header, valid accepted-artifact v1, raw `WorkflowParts`, YAML text, JSON text, all-zero bytes, all-0xff bytes, large length-prefix bytes, gate_count 0/2/15 records, wrong schema/version.

Expected property: decode either returns a fully valid accepted artifact satisfying the contract or exact typed decode/invalid-envelope failure; never panics and never admits malformed/raw bytes.

Trace: PRE-001, INV-003, ERR-002, ERR-003, FUZZ-ENV-008.

### Fuzz Target F02: CLI/IPC admission payload parser

Target: CLI/IPC input boundary that resolves artifact digest and run request.

Input type: bytes / string.

Risk: malformed digest accepted, panic on Unicode/control bytes, category collapse to generic error.

Corpus seeds: valid digest, short digest, long digest, non-hex, embedded NUL, UTF-8 invalid bytes, JSON-like payload, YAML-like payload, empty input.

Expected property: invalid payloads return typed parse/admission diagnostics without run creation; valid payloads proceed only to accepted-artifact admission.

Trace: ERR-008, POST-004, POST-003.

### Fuzz Target F03: diagnostic serialization roundtrip

Target: API/CLI/IPC diagnostic serializer/deserializer if public roundtrip exists.

Input type: arbitrary diagnostic structs or serialized bytes.

Risk: losing rejected digest, gate found/required, capability mismatch, or stale/decode cause.

Corpus seeds: each ERR-001..ERR-008 canonical diagnostic, max/min digest bytes, long capability names, empty capability set, high gate_count 255.

Expected property: serialization preserves category, digest when present, and semantic cause fields; invalid serialized diagnostics do not panic.

Trace: POST-004, INV-007, ERR-008, MUT-DIAG-011.

## 9. Kani Harnesses

### Kani Harness K01: digest mismatch denial

Property: for bounded 32-byte requested, persisted, and envelope digest arrays, admission success implies all three arrays are equal; any inequality returns digest mismatch and cannot allocate state.

Bound: 3 digest arrays of 32 bytes; branch over equality partitions rather than unconstrained full storage I/O.

Rationale: digest mismatch is a security boundary; proptest samples cannot prove all equality partitions.

Trace: PRE-003, ERR-005, KANI-DIGEST-007 / PO-007. Current approved reviews say no Kani pass is claimed until downstream State 8/formal-verifier creates/runs or records WAIVED/DEFERRED evidence.

### Kani Harness K02: capability exactness finite boundary

Property: for bounded required/granted capability arrays of length 0..3 over bounded name/action IDs, success iff exact cardinality and exact name/action match hold; otherwise `CapabilityDenied`.

Bound: length <= 3, action IDs <= 3, symbolic name IDs <= 4 or small interned enum used by harness.

Rationale: exact-cardinality failures are security-sensitive and easy to weaken by mutation; Verus covers model predicates, Kani can bind executable Rust behavior if harness is later created.

Trace: PRE-005, INV-006, ERR-007, VERUS-CAP-005.

## 10. Mutation Checkpoints

Threshold: `>= 90%` overall mutation kill rate for scoped admission/error files; `100%` kill for critical checkpoint mutants below.

Critical mutants that must be killed:

- Change `REQUIRED_GATE_COUNT` from `15` to `0`, `2`, `14`, or `16`; killed by B04/B15 unit and integration scenarios.
- Replace `!= REQUIRED_GATE_COUNT` with `== REQUIRED_GATE_COUNT`; killed by gate mismatch and valid acceptance scenarios.
- Remove one required proof-flag check; killed by B03 and P03.
- Treat `durable == false` as accepted; killed by B03/B06.
- Drop stale evidence check or invert stale predicate; killed by B06/P03.
- Skip digest comparison or compare only requested vs persisted while ignoring envelope digest; killed by B05/P04/K01.
- Replace exact capability match with prefix or name-only match; killed by B07/P01/K02.
- Remove cardinality comparison before capability loop; killed by missing/excess capability scenarios.
- Collapse `ArtifactEnvelopeDecodeFailed`, invalid envelope, gate mismatch, stale, and digest mismatch into one generic runtime error; killed by B08/P05/F03.
- Zero or omit rejected digest during error mapping; killed by B01/B02/B05/B08.
- Move frame/run allocation before admission; killed by B11/P06 and integration journal/state checks.
- Route strict/journaled constructors through `AlwaysPresentArtifactStore`; killed by B12/B14 static and integration checks.
- Add `serde_yaml`/`serde_json` parser to strict admission path; killed by B13 static gate.
- Map budget capacity errors to artifact/capability errors; killed by B16.

Suggested bounded command after tests exist: `cargo mutants --package vb_runtime --file crates/vb_runtime/src/admission.rs --file crates/vb_runtime/src/error/mod.rs --timeout 120 -- --all-features` plus any crate/package selector required by the final workspace layout. Exact command must be recorded by State 8/formal-verifier before claiming evidence.

## 11. Static Gates

| Gate | Command / check | Required assertion | Trace |
|---|---|---|---|
| Canonical CI | `moon ci` before landing or downstream WAIVED/DEFERRED with owner/reason/expiry/limitation | No unrelated failure hidden; no pass claimed by State 7 | GATE-STATE3-012 / PO-012 |
| Source lint | `moon run :lint-src` | zero lint violations in production source | STATIC-BYPASS-010 |
| Bypass scan | `rtk grep -n "AlwaysPresentArtifactStore|compiled_ir_exists\(|admit_run\(|admit_run_with_budget\(" crates/vb_runtime/src crates/velvet_ballastics/src` plus reviewer audit | No protected strict/journaled production path uses dummy/existence-only admission | PRE-006, INV-002 |
| Parser scan | `rtk grep -n "serde_yaml|serde_json|WorkflowParts" crates/vb_runtime/src crates/velvet_ballastics/src` plus reviewer audit | Strict accepted-artifact admission path does not parse YAML/JSON/raw workflow parts | POST-001, INV-004 |
| Panic/unsafe governance | existing repo gates for `unsafe`, `unwrap`, `expect`, `panic`, `todo`, unchecked indexing/casts | No new prohibited constructs in production/test plan scope; test writer must not add weak `is_ok`/`is_err` assertions | AGENTS.md, test-planner rules |
| JSONL traceability | `jq -c .` for proof and trace JSONL | All trace rows remain parseable and mapped to tests/obligations | all clauses |

## 12. Combinatorial Coverage Matrix

### Admission envelope and digest matrix

| Scenario | Input class | Expected output | Test layer | Trace |
|---|---|---|---|---|
| happy path | accepted v1, digest match, gate 15, durable, non-stale, all statuses accepted, exact caps | `Ok(AdmissionRecord/RunAdmission)` with exact digest/profile | unit + integration | POST-001, POST-005 |
| missing artifact | no storage record for digest | `Err(ArtifactNotFound { digest })`; no allocation | integration | ERR-001 |
| raw workflow | raw `WorkflowParts` bytes | `Err(ArtifactEnvelopeDecodeFailed)` with digest/cause | integration + fuzz | ERR-002 |
| YAML/JSON | textual bytes | `Err(ArtifactEnvelopeDecodeFailed)` with digest/cause | integration + fuzz | ERR-002 |
| malformed postcard | truncated/random bytes | `Err(ArtifactEnvelopeDecodeFailed)`; no panic | fuzz + integration | ERR-002 |
| unknown schema | decoded unsupported version | `Err(ArtifactEnvelopeInvalid)` with schema cause | unit + proptest | ERR-003 |
| gate low | gate 0/2/14 | `Err(ArtifactGateMismatch/ArtifactInvalidGateCount { found, required: 15 })` | unit + integration | ERR-004 |
| gate high | gate 16/255 | same exact gate mismatch with found | unit + proptest | ERR-004 |
| failed status/flag | each required proof flag/status false | exact invalid proof/status cause | unit + proptest | ERR-003/004 |
| non-durable | durable false | exact invalid/stale/durable cause | unit + integration | ERR-003/006 |
| stale | stale certificate/evidence | exact `ArtifactStale`/stale cause | unit + integration | ERR-006 |
| digest mismatch | any inequality among request/record/envelope | exact digest mismatch preserving identities | unit + proptest + Kani | ERR-005 |

### Capability matrix

| Scenario | Input class | Expected output | Test layer | Trace |
|---|---|---|---|---|
| exact empty | required empty, granted empty | `Ok(...)` if envelope otherwise valid | unit + proptest | PRE-005 |
| exact non-empty | same `(name, action)` set | `Ok(...)` with exact grants | unit + proptest | INV-006 |
| missing | omit one required | `Err(CapabilityDenied { action, required, granted })` | unit + integration | ERR-007 |
| excess | add extra grant | exact `CapabilityDenied`, mismatch visible | unit + integration | ERR-007 |
| duplicate | duplicate grant changes cardinality | exact `CapabilityDenied` | unit + proptest | ERR-007 |
| prefix-only | `network` grants cannot satisfy `network.rpc` | exact `CapabilityDenied` | unit + proptest | INV-006 |
| partial lexical prefix | `net` cannot satisfy `network.rpc` | exact `CapabilityDenied` | unit | INV-006 |
| action mismatch | right name, wrong action | exact `CapabilityDenied` | unit + proptest | ERR-007 |

### Runtime lifecycle matrix

| Scenario | Input class | Expected output | Test layer | Trace |
|---|---|---|---|---|
| any ERR-001..ERR-007 denial | table-driven invalid fixture | exact error and unchanged frame/run/runnable/journal state | integration + proptest | POST-003, INV-005 |
| success | valid accepted artifact | allocation happens after admission and `RunAccepted` is emitted once | integration | POST-005 |
| constructor without accepted store | strict/journaled production path missing store | typed constructor failure or storage-backed path; no dummy success | integration + static | PRE-006 |
| relaxed test-only | relaxed policy with dummy/missing artifact | exact relaxed success where permitted | unit | PRE-006 non-goal boundary |

### Diagnostic matrix

| Scenario | Input class | Expected output | Test layer | Trace |
|---|---|---|---|---|
| ERR-001 | not found | public not-found category + digest | unit + E2E | ERR-001/008 |
| ERR-002 | decode/malformed | decode category + digest + malformed cause | unit + fuzz + E2E | ERR-002/008 |
| ERR-003 | invalid envelope | invalid-envelope category + specific cause | unit + E2E | ERR-003/008 |
| ERR-004 | gate mismatch | gate category + found/required | unit + E2E | ERR-004/008 |
| ERR-005 | digest mismatch | digest category + requested/observed identities | unit + E2E | ERR-005/008 |
| ERR-006 | stale | stale category + digest/staleness cause | unit + E2E | ERR-006/008 |
| ERR-007 | capability denied | capability category + required/granted/action | unit + E2E | ERR-007/008 |
| ERR-008 mapping | every category through runtime/API/CLI/IPC | injective category/cause preservation | mutation + E2E | ERR-008 |

## 13. Traceability Crosswalk

| Clause | Required test evidence | Required non-test evidence |
|---|---|---|
| PRE-001 | B01, B02 integration; F01 fuzz | TEST-STRICT-009, FUZZ-ENV-008 |
| PRE-002 | B03, B04, B15 unit/integration; P02/P03 | TLA-GATE-002, VERUS-ENV-006 |
| PRE-003 | B05; P04; K01 | KANI-DIGEST-007 downstream policy |
| PRE-004 | B03, B06; P03 | VERUS-ENV-006 |
| PRE-005 | B07; P01; K02 | TLA-CAP-003, VERUS-CAP-005 |
| PRE-006 | B12, B14; static bypass scan | TLA-BYPASS-004, STATIC-BYPASS-010 |
| POST-001 | B09, B13; parser scan | TEST-STRICT-009, STATIC-BYPASS-010 |
| POST-002 | B01..B08 exact error assertions | TEST-STRICT-009, MUT-DIAG-011 |
| POST-003 | B11; P06; lifecycle integration | TLA-ADMIT-001 |
| POST-004 | B08; P05; F03; mutation | MUT-DIAG-011 |
| POST-005 | B09, B10 integration | TLA-ADMIT-001 |
| INV-001 | B04, B15; P02 | TLA-GATE-002, VERUS-ENV-006 |
| INV-002 | B12, B14; static bypass scan | TLA-BYPASS-004, STATIC-BYPASS-010 |
| INV-003 | B02, B03; P03; F01 | FUZZ-ENV-008, VERUS-ENV-006 |
| INV-004 | B09, B13; parser scan | STATIC-BYPASS-010 |
| INV-005 | B11; P06 | TLA-ADMIT-001 |
| INV-006 | B07; P01; K02 | VERUS-CAP-005, TLA-CAP-003 |
| INV-007 | B08, B16; P05; F03 | MUT-DIAG-011 |
| ERR-001..ERR-008 | Per-variant BDD scenarios and diagnostic matrix | TEST-STRICT-009, MUT-DIAG-011 |

## 14. Test Writer Acceptance Rules

- Every scenario must assert exact success value or exact error variant and fields.
- `assert!(result.is_ok())`, `assert!(result.is_err())`, or `matches!` without binding/checking required fields is insufficient.
- Denial tests must assert both the diagnostic and the negative state side effect: no frame, no run entry, no runnable state, no `drive_run`, no `RunAccepted`.
- Public behavior must be tested through public APIs: admission functions, runtime/shard constructors, CLI/IPC/API boundaries, and storage-backed loaders.
- Use real temp storage/journal integration where feasible; use fakes only for deterministic accepted-artifact fixtures.
- Do not claim Kani/fuzz/proptest/mutation/CI evidence until the exact command runs or downstream formal evidence records WAIVED/DEFERRED with owner, reason, expiry, limitation, and compensating evidence.

## Open Questions

- Contract leaves open whether runtime diagnostics introduce finer public variants or preserve existing variants with structured detail. Test writer must bind to the final public API but preserve ERR-001..ERR-008 semantic distinctions.
- Contract leaves open whether inner IR digest and envelope digest are enforced at the same boundary. Tests must cover all requested/persisted/envelope digest inequality classes wherever the final design exposes them.
- Package selectors in `cargo nextest`, `cargo fuzz`, `cargo kani`, and `cargo mutants` may need adjustment to final workspace naming; any adjustment must be recorded exactly in State 8 evidence.

## Completion Evidence

- State 7 test plan written at `2026-05-16T04:46:43Z`.
- Written artifact: `.beads/vb-qi37.4.2/test-plan.md`.
- No production code, test code, proof code, dependency files, or CI configuration edited.
- Source checkout `/home/lewis/src/velvet-ballistics` not written.
>>>>>>> origin/go-skill-p0-vb-qi37-4-2
