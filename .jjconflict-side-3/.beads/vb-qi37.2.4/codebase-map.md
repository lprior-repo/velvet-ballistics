# vb-qi37.2.4 Codebase Map

## Bead Context
- **bead_id**: vb-qi37.2.4
- **phase**: 1 (explore)
- **isolated_workspace**: /home/lewis/src/vb-femdation/vb-qi37-2-4
- **source_checkout**: /home/lewis/src/velvet-ballistics (control-plane only, no writes)

## Workspace Overview

This is a **femdation isolated workspace** for the velvet-ballistics project — a Rust-nightly, no-unsafe, no-panic, single-server ultra-low-latency durable execution engine for workflow orchestration.

### Protected Source Changes (in source_checkout, not in this workspace)
These changes are preserved in the control checkout and NOT present in this isolated workspace:
- `crates/vb_core/src/budget/tests.rs`
- `crates/vb_runtime/src/engine/tests.rs`
- `crates/vb_ui_model/src/envelope/output/tests.rs`
- `tests/bdd_validation_tests.rs`
- `tests/proptest_validation.rs`

---

## Crate Architecture

### Core Runtime Crates

| Crate | Purpose | Key Dependencies |
|-------|---------|------------------|
| `vb_core` | Hot in-memory execution core; owns compiled workflow IR, numeric identifiers, slot model, synchronous state-machine loop | bytes, chrono, indexmap, serde, thiserror |
| `vb_runtime` | Runtime orchestration with storage integration | vb_core, vb_storage, crossbeam-queue, rtrb, postcard |
| `vb_storage` | Fjall append-only journal with full recovery support | vb_core, fjall, blake3, crc32c, postcard, rustix |
| `vb_validate` | Workflow validation gates (taint, capability, cycles, slots, accessors) | vb_core |
| `vb_compile` | YAML → compiled IR lowering | vb_core, vb_codegen, vb_validate, vb_yaml, blake3, saphyr |
| `vb_yaml` | Strict YAML parsing via saphyr | saphyr, saphyr-parser |
| `vb_expr` | Expression parsing, lexing, eval, bytecode generation | vb_core, logos, arrayvec |
| `vb_codegen` | Generated Rust workflow mode | vb_core |
| `vb_ipc` | Binary IPC ingress | vb_core, vb_runtime, vb_validate, mio, crossbeam-channel |
| `velvet_ballistics` | Main binary crate (top-level integration) | all above crates |

### UI Crates
- `vb_ui_model` - UI domain models
- `vb_ui_makepad` - Makepad UI implementation
- `vb_ui_snapshot` - UI snapshot testing

### Verification/Proof Crates
- `vb_proof_kernels` - Proof kernels

---

## Dependency Graph (Key Edges)

```
velvet_ballistics (bin)
├── vb_compile
│   ├── vb_yaml
│   ├── vb_validate
│   ├── vb_codegen
│   └── vb_core
├── vb_core
├── vb_codegen
├── vb_expr
├── vb_ipc
│   ├── vb_runtime
│   │   ├── vb_core
│   │   └── vb_storage
│   └── vb_validate
├── vb_runtime
│   ├── vb_core
│   └── vb_storage
├── vb_storage
│   └── vb_core
├── vb_validate
│   └── vb_core
├── vb_yaml
└── ...

vb_compile
└── vb_codegen → vb_core

vb_storage → vb_core (fjall external)
```

---

## Key Files by Crate

### vb_core (execution core)
```
crates/vb_core/src/
├── lib.rs                 # Public API exports
├── action.rs              # Action contracts, taint propagation
├── budget.rs              # AggregateResourceBudget, boundedness
├── capability.rs          # Capability, CapabilitySet
├── diagnostic.rs          # Diagnostic, DiagnosticCode
├── engine/                # State machine execution engine
│   ├── mod.rs
│   ├── validate.rs        # CompiledWorkflow validation
│   ├── expr_eval/         # Expression evaluation
│   └── tests/
├── errors.rs              # CoreError, EngineError
├── frame.rs               # RunFrame, StepState
├── ids.rs                 # Numeric index types (WorkflowId, StepIdx, etc.)
├── replay/                # Deterministic replay
├── value.rs               # SlotValue, Taint
├── value_store.rs         # ObjectField, ValueStore
└── workflow.rs            # CompiledWorkflow, CompiledNode
```

### vb_storage (persistence)
```
crates/vb_storage/src/
├── lib.rs                 # Public API
├── admission.rs           # Artifact admission
├── artifacts.rs           # Workflow source, compiled IR records
├── codec/                 # Postcard encoding/decoding
├── journal/               # Fjall-backed journal
│   ├── admission.rs
│   ├── append.rs
│   ├── batch.rs
│   ├── core.rs
│   ├── injection.rs
│   ├── internal.rs
│   ├── mod.rs
│   ├── replay.rs
│   └── source.rs
├── recovery/              # Full journal recovery
│   ├── recover.rs
│   ├── replay/
│   └── hydrate.rs
├── snapshots.rs           # RunSnapshot
└── trimming.rs            # Log trimming
```

### vb_validate (validation gates)
```
crates/vb_validate/src/
├── lib.rs
├── gate_07_stack.rs       # Stack depth validation
├── gate_08_accessor.rs    # Accessor validation
├── gate_09_slots.rs       # Slot validation
├── gate_10_node.rs        # Node validation
├── gate_11_loop.rs       # Loop/cycle detection
├── gate_12_14_15.rs      # Resource/budget gates
├── gate_13_cycles.rs     # Cycle analysis
├── idempotency_contract.rs # Idempotency key validation
├── ref_validate.rs        # Reference validation
├── schema.rs             # Schema validation
├── taint_prop.rs         # Taint propagation
└── type_check.rs         # Type checking
```

### vb_compile (YAML → IR)
```
crates/vb_compile/src/
├── lib.rs
├── ast/                  # YAML AST
├── control_flow.rs       # CFG lowering
├── expression.rs         # Expression compilation
├── expression_bytecode.rs # Bytecode generation
├── kani/                # Kani proofs
├── lower/               # IR lowering
├── references.rs        # Reference resolution
├── schema.rs            # Schema compilation
├── strict_yaml.rs       # YAML strictness
└── type_taint.rs        # Type taint tracking
```

---

## Verification Artifacts

### TLA+ Specifications
- `specs/tla/RetryFSM.tla` - Retry finite state machine
- `specs/tla/LifecycleJournal.tla` - Lifecycle journal
- `specs/tla/AskAnswerLifecycle.tla` - Ask/answer lifecycle
- `specs/tla/RetryJournal.tla` - Retry journal

### Kani Proofs
- `kani/gate_07_stack.rs` through `kani/gate_12_14_15.rs`
- `kani/idempotency_gate_parity.rs`
- `kani/decision_table_*.rs`
- `kani/is_statically_idempotent_contract.rs`

### Verus Verification
- `verification/verus/step_budget.rs`
- `verification/verus/resource_budget.rs`
- `verification/verus/taint_lattice.rs`
- `verification/verus/budget_monotonic.rs`
- `verification/verus/recovery_verification.rs`

---

## Workspace Build Configuration

- **Edition**: 2024
- **Resolver**: 2
- **Unsafe code**: FORBIDDEN (`#![forbid(unsafe_code)]` in all crates)
- **No panic/unwrap/expect/todo/unimplemented**: Enforced via clippy
- **Key profiles**: release, hardened, maxperf

---

## Entry Points

- **Binary**: `velvet-ballistics` at `crates/velvet_ballistics/src/main.rs`
- **Library**: `velvet_ballistics` crate exposes all modules
- **Tests**: Integration tests in `tests/` root; unit tests in each crate

---

## Risk Tags (by area)

| Area | Risk | Notes |
|------|------|-------|
| `vb_core::engine` | HIGH | State machine execution; no unsafe but complex logic |
| `vb_storage::journal` | HIGH | Fjall persistence; recovery correctness critical |
| `vb_storage::recovery` | CRITICAL | Full journal replay; divergence detection |
| `vb_validate` | MEDIUM | Must catch all invalid workflows before admission |
| `vb_compile` | MEDIUM | YAML → IR lowering must preserve semantics |
| `vb_expr` | MEDIUM | Expression eval; div-by-zero, overflow bounds |
| Generated Rust mode | CRITICAL | Must preserve exact IR semantics for maxperf |

---

## Verifier Modes Required

| Crate/Area | Miri | Kani | Verus | Proptest | Loom |
|------------|------|------|-------|----------|------|
| vb_core | ✓ | ✓ | ✓ | ✓ | |
| vb_expr | ✓ | | | ✓ | |
| vb_storage | ✓ | ✓ | | ✓ | |
| vb_compile | | ✓ | | ✓ | |
| vb_validate | | ✓ | | ✓ | |
| vb_runtime | | | | ✓ | ✓ |
| vb_ipc | | | | | |

---

## Notes

- This workspace is a **femdation isolated workspace** — it has a jj workspace linked to the source checkout
- The workspace is at commit `pxulmlsp` (femdation workspace vb-qi37.2.4)
- Parent commit is `wumlxqnl` (main)
- No local modifications in this workspace beyond bead metadata
- Protected source changes are in the control checkout at `/home/lewis/src/velvet-ballistics`
