# Codebase Map: vb-qi37.4.2

## Workspace Structure

```
/home/lewis/src/vb-femdation/vb-qi37-4-2/
├── Cargo.toml                    # Workspace root
├── velvet-ballistics-MASTER.md   # Master spec (5961 lines)
├── Cargo.lock
├── crates/
│   ├── vb_core/                  # Hot in-memory execution core (forbid unsafe)
│   ├── vb_runtime/               # Hot-path runtime engine (shard, journal, recovery)
│   ├── vb_storage/               # Fjall persistence, record decode
│   ├── vb_ipc/                   # Binary IPC frame decode
│   ├── vb_expr/                  # Expression evaluator (AST + bytecode)
│   ├── vb_validate/              # YAML validation
│   ├── vb_compile/               # Compilation (YAML → IR)
│   ├── vb_codegen/               # Rust code generation
│   ├── vb_yaml/                  # YAML authoring surface
│   ├── vb_ui_model/              # UI domain model
│   ├── vb_ui_makepad/            # Makepad UI renderer
│   ├── vb_ui_snapshot/           # Snapshot/replay UI
│   ├── vb_ui/                    # UI integration (excluded from workspace)
│   ├── velvet_ballastics/        # Main binary
│   ├── vb_proof_kernels/         # Verification kernels
│   ├── workspace_tests/          # Integration tests
│   └── vb_benchmark/             # Benchmarks
├── contracts/
│   ├── invariants.yaml           # Mechanical invariants (scan commands)
│   ├── proof_obligations.yaml    # 865 lines, 50+ proof obligations
│   ├── tla/                      # TLA+ specs (Lifecycle, Retry, Journal, etc.)
│   └── verus/                   # Verus specs (taint, step, budget, signals)
├── verification/
│   ├── tla/                     # TLA+ model checking specs
│   ├── verus/                   # Verus deductive specs
│   └── vb-qi37.7.3/            # Gate 07-12 verification
├── kani/                        # Kani harnesses (idempotency gates)
├── tests/
│   ├── bdd_validation_tests.rs  # BDD scenario tests
│   └── proptest_validation.rs   # Property-based tests
├── reference/                   # Reference engine model
├── fuzz/                        # Fuzzing harnesses
├── benches/                     # Criterion benchmarks
├── specs/                       # Spec artifacts
└── scripts/                    # Build/test scripts
```

## Crate Dependency Graph

```
vb_yaml ──► vb_validate ──► vb_compile ──► vb_codegen ──► vb_core (IR)
                                                      └─► vb_expr (bytecode)

vb_core ──► vb_expr
       └─► vb_runtime ──► vb_storage (Fjall)
                     └─► vb_ipc

vb_ui_model ──► vb_core
vb_ui_snapshot ──► vb_ui_model
vb_ui_makepad ──► vb_ui_model
```

## Key Source Files

### vb_core (crates/vb_core/src/)
| File | Lines | Purpose |
|------|-------|---------|
| `lib.rs` | 79 | Module exports, #![forbid(unsafe_code)] |
| `action.rs` | 75K | Action contracts, tickets, outcomes |
| `budget.rs` | 55K | WholeWorkflowBudget, AggregateResourceBudget |
| `errors.rs` | 62K | Error taxonomy (CoreError, EngineError) |
| `frame.rs` | 2104 | RunFrame, StepState enum (9 states) |
| `value.rs` | 1115 | SlotValue, Taint (3-level lattice), FiniteF64 |
| `value_store.rs` | 86K | Object field storage, handle validity |
| `engine.rs` | 50 | new_run_frame, drive_deterministic, step_once |
| `ids/mod.rs` | 1083 | Numeric IDs: WorkflowId, StepIdx, SlotIdx, ExprIdx, ActionId, etc. |
| `kani_taint.rs` | 2.9K | Kani taint lattice harnesses |
| `kani_expr_bound.rs` | 6.5K | Kani expression bounds |
| `kani_idempotency_gates.rs` | 10.5K | Kani idempotency gate harnesses |
| `kani_capability_harnesses.rs` | 3.0K | Kani capability |
| `replay/` | — | Replay/recovery support |
| `validation/` | — | Validation layer |
| `workflow/` | — | CompiledWorkflow, CompiledNode, ResourceContract |
| `budget/tests.rs` | 5980 | Budget module tests (modified per baseline) |

### vb_runtime (crates/vb_runtime/src/)
| File | Purpose |
|------|---------|
| `lib.rs` | Module exports, #![forbid(unsafe_code)] |
| `engine/` | Runtime engine implementation |
| `shard/` | Shard scheduling, frame pools, timer wheel |
| `journal/` | Journal writer, event sequencing |
| `idempotency.rs` | Idempotency key computation |
| `recovery.rs` | Snapshot + replay |
| `action.rs` | Action dispatch, completion, retry |
| `frame_pool.rs` | Bounded frame pools |
| `engine/tests.rs` | Engine tests (2537 lines, modified per baseline) |

### vb_storage (crates/vb_storage/src/)
| File | Purpose |
|------|---------|
| `record.rs` | Record decode: magic, schema, kind, payload_len, CRC, BLAKE3 |
| `fjall.rs` | Fjall keyspace management |

### vb_ipc (crates/vb_ipc/src/)
| File | Purpose |
|------|---------|
| `frame.rs` | IPC decoder: header_len=60, reject before allocation |

### vb_expr (crates/vb_expr/src/)
| File | Purpose |
|------|---------|
| `eval.rs` | AST interpreter |
| `bytecode.rs` | Bytecode evaluator |
| `stack.rs` | Expression stack (MAX_EXPR_STACK=64) |
| `typecheck.rs` | Type checking |

### vb_ui_model (crates/vb_ui_model/src/)
| File | Purpose |
|------|---------|
| `envelope/output/tests.rs` | OutputEnvelope tests (modified per baseline) |
| `envelope/` | Envelope types, error handling |
| `workflow.rs`, `system.rs`, `run.rs` | Domain model |
| `verify.rs` | Verification certificates |

## Key Type Taxonomy

### Numeric IDs (vb_core::ids)
- `WorkflowId(u32)`, `StepIdx(u16)`, `SlotIdx(u16)`, `ExprIdx(u16)`
- `ActionId(u16)`, `AccessorIdx(u16)`, `ConstIdx(u16)`
- `SymbolId(u32)`, `ListId(u32)`, `ObjectId(u32)`, `BlobId(u64)`
- `RunId(u64)`, `EventSeq(u64)`, `SeqNo(u64)`
- `BranchIdx(u16)`, `FanoutLimit(u32)`, `MaxAttempts(u16)`, `RetryCount(u16)`

### StepState (frame.rs)
```rust
pub enum StepState {
    Pending, Running, Succeeded, Failed,
    Skipped, Waiting, Asking, Cancelled,
}
```
Valid transitions:
- Pending → Running, Succeeded, Failed, Cancelled, Skipped
- Running → Succeeded, Failed, Waiting, Asking, Cancelled, Skipped
- Waiting → Running, Asking → Running
- Terminal: Succeeded/Failed/Cancelled/Skipped → themselves only

### Taint (value.rs)
```rust
pub enum Taint { Clean, DerivedFromSecret, Secret }
// join_taint lattice: Clean < DerivedFromSecret < Secret
```

### SlotValue (value.rs)
```rust
pub enum SlotValue {
    Null, Bool(bool), I64(i64), F64(FiniteF64),
    Symbol(SymbolId), List(ListId), Object(ObjectId), Blob(BlobId),
}
```

### EngineSignal (signals.rs)
```rust
pub enum EngineSignal {
    Running,
    Waiting { on: WaitToken },
    Asking { ticket: AskTicket },
    Finished(SlotValue, Taint),  // canonical form per spec
    StepBudgetExhausted,
    ...
}
```

## Verification Artifacts

### TLA+ (verification/tla/, contracts/tla/)
- `LifecycleJournal.tla` / `LifecycleJournal.cfg`
- `RetryJournal.tla` / `RetryJournal.cfg`
- `RetryFSM.tla` / `RetryFSM.cfg`
- `AskAnswerLifecycle.tla` / `AskAnswerLifecycle.cfg`
- `ResumeStateMachine.tla`
- `CapabilityLifecycle.tla`

### Verus (verification/verus/, contracts/verus/)
- `taint_lattice.rs` — Lattice laws (associative, commutative, idempotent, identity)
- `step_state_machine.rs` — State transition invariants
- `step_budget.rs` — Budget monotonicity
- `resource_budget.rs` — Sequential/branch/loop composition
- `signals_invariant.rs` — EngineSignal invariants
- `budget_bounded.rs`, `budget_monotonic.rs`
- `run_loop_termination.rs`
- `value_store_invariant.rs`
- `recovery_verification.rs`

### Kani (kani/, vb_core/src/kani_*.rs)
- `kani_taint_lattice.rs` — Taint join properties
- `kani_step_state.rs` — State transitions
- `kani_step_budget.rs` — Budget try_take
- `kani_index_access.rs` — Bounds checking
- `kani_ipc_header.rs`, `kani_ipc_header_rejects_oversize.rs`
- `kani_record_*.rs` — Storage decode properties
- `kani_resource_budget.rs`
- `kani_expr_stack.rs` — Stack depth bounds
- `kani_terminal_state.rs` — Terminal state immutability

### Proof Obligations (contracts/proof_obligations.yaml)
- **VB-CORE-TAINT-001 to VB-CORE-TAINT-006**: Taint lattice (L4)
- **VB-CORE-STATE-001 to VB-CORE-STATE-003**: Step state machine (L3/L4)
- **VB-CORE-BUDGET-001 to VB-CORE-BUDGET-003**: Step budget (L3/L4)
- **VB-CORE-IDX-001, VB-CORE-IDX-002**: Index safety (L3/L0)
- **VB-IPC-DECODE-001 to VB-IPC-DECODE-003**: IPC decoder (L3)
- **VB-STORAGE-DECODE-001 to VB-STORAGE-DECODE-006**: Storage decode (L3)
- **VB-REPLAY-001 to VB-REPLAY-007**: Journal/replay safety (L2/L3)
- **VB-EXPR-001 to VB-EXPR-003**: Expression evaluator (L2/L3)
- **VB-CONC-001 to VB-CONC-005**: Concurrency (Loom, L3)

## Invariants (contracts/invariants.yaml)
- `no_runtime_yaml/json/http` — Runtime core exclusion
- `no_hot_path_formatting/allocation` — Hot path purity
- `no_unchecked_indexing/casts/arithmetic` — Safety
- `no_ignored_result` — Error handling
- `step_state_valid_transitions` — State machine
- `terminal_state_immutable` — Terminal state
- `taint_lattice_join_*` — Lattice laws
- `budget_try_take_never_underflows` — Budget safety
- `handle_validity_checked` — Handle access
- `ipc_reject_before_alloc` — DoS prevention
- `record_magic_before_alloc` — Storage safety
- `journal_before_dispatch` — Ordering
- `no_unsafe_in_first_party` — Global
- `no_panic_in_first_party` — Global
- `idempotency_key_well_formed` — Replay safety
- `single_shard_owner` — Shard ownership

## Modified Files (Baseline Report)
1. `crates/vb_core/src/budget/tests.rs` — Budget module integration tests
2. `crates/vb_runtime/src/engine/tests.rs` — Runtime engine tests (2537 lines)
3. `crates/vb_ui_model/src/envelope/output/tests.rs` — OutputEnvelope tests (216 lines)
4. `tests/bdd_validation_tests.rs` — BDD scenario tests
5. `tests/proptest_validation.rs` — Property-based validation tests

## Required Verifier Modes
| Crate | L0 | L1 | L2 | L3 | L4 | L5 | L6 |
|-------|----|----|----|----|----|----|-----|
| vb_core::ids | — | — | — | ✓ | ✓ | — | — |
| vb_core::value | — | — | — | — | ✓ | — | — |
| vb_core::frame | — | — | — | — | ✓ | — | — |
| vb_core::engine | — | — | ✓ | — | ✓ | — | — |
| vb_expr | — | — | ✓ | ✓ | — | — | — |
| vb_yaml | — | ✓ | ✓ | — | — | — | — |
| vb_validate | — | ✓ | — | — | ✓ | — | — |
| vb_compile | — | — | ✓ | — | ✓ | — | — |
| vb_storage | — | — | ✓ | ✓ | — | — | — |
| vb_runtime | — | — | ✓ | ✓ | — | — | — |
| vb_ipc | — | — | ✓ | ✓ | — | — | — |
| vb_codegen | — | ✓ | ✓ | — | — | — | — |
| vb_ui_model | — | ✓ | — | — | — | — | — |
| supply_chain | ✓ | — | — | — | — | — | ✓ |
