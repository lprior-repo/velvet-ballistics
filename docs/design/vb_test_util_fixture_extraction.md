# `vb_test_util` fixture extraction design

**Bead**: vb-XXXX (design only — no production code change)
**Follow-up beads**: one per crate migration
**Date**: 2026-06-20
**Author**: holzman-rust refactor agent
**Doctrine**: NASA/JPL Power-of-Ten — bounded, checked, types carry invariants; Holzman Rust §zero-unwrap, §checked-access, §typed-errors

---

## 1. Current `vb_test_util` state

`crates/vb_test_util/src/` is **347 lines across 4 files**. **Only 2 files consume it workspace-wide**:

| Consumer | File | Use |
|---|---|---|
| `vb_test_util` self | `crates/vb_test_util/tests/density_tests.rs` | round-trips `FixtureCapacity`, `SeededBytes`, `TempKeyspace` |
| `workspace_tests` | `crates/workspace_tests/tests/vb_test_util_crate.rs` | same round-trip coverage |

That is, **1 external consumer** (`workspace_tests`). Despite this, 108+ files re-implement `tempfile::tempdir()`, 135+ files re-implement `FjallJournal::open`, and 47 sites define a private `fn temp_journal()`.

### 1.1 `crates/vb_test_util/src/lib.rs` (verbatim, 27 lines)

```rust
#![forbid(unsafe_code)]

pub mod fixture;
pub mod seed;
pub mod temp_keyspace;

use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TestSetupError {
    #[error("out of memory")]
    OutOfMemory,
    #[error("invalid seed: {0}")]
    InvalidSeed(String),
    #[error("invalid capacity: {0}")]
    InvalidCapacity(String),
    #[error("temp directory error: {0}")]
    TempDirError(String),
    #[error("fjall open error: {0}")]
    FjallOpenError(String),
    #[error("postcard encode error: {0}")]
    PostcardEncodeError(String),
    #[error("postcard decode error: {0}")]
    PostcardDecodeError(String),
    #[error("assertion mismatch: {0}")]
    AssertionMismatch(String),
}
```

### 1.2 `crates/vb_test_util/Cargo.toml` (verbatim)

```toml
[package]
name = "vb_test_util"
version.workspace = true
edition.workspace = true
license.workspace = true
publish = false

[dependencies]
rand = { workspace = true }
postcard = { workspace = true }
thiserror = { workspace = true }
tempfile = { workspace = true }
fjall = { workspace = true }

[lints]
workspace = true
```

### 1.3 Other src files (line counts)

| File | Lines | Purpose |
|---|---:|---|
| `src/lib.rs` | 27 | re-exports + `TestSetupError` |
| `src/fixture.rs` | 123 | `FixtureCapacity`, `FixtureBuilder` |
| `src/seed.rs` | 79 | `SeededBytes<const N>` |
| `src/temp_keyspace.rs` | 118 | `TempKeyspace` |

The existing module is essentially **a `TempKeyspace` + byte generator + capacity newtype**. It does not yet absorb any of the duplicated test fixtures surveyed below.

### 1.4 Single-consumer fact

Workspace-wide `vb_test_util` Cargo references (verified):

```
Cargo.toml:17:              "crates/vb_test_util",                     # workspace member
vb_test_util/Cargo.toml:2:   name = "vb_test_util"
workspace_tests/Cargo.toml:51: vb_test_util = { path = "../vb_test_util" }  # [dev-dependencies]
workspace_tests/Cargo.toml:54: name = "vb_test_util_crate"
```

No other crate declares `vb_test_util` in `[dependencies]` or `[dev-dependencies]`. The crate exists, is wired into the workspace, and is **substantially under-utilized**.

---

## 2. Per-pattern inventory

Counts are from `rg 'fn <name>'` over `crates/**.rs` (excluding `target/`). The user's brief cited higher numbers than grep returns; I report **grep-confirmed** numbers.

### 2.1 `fn temp_journal` — **47 sites**

Identical-shape body in **every site**:

```rust
// Canonical body (used by vb_storage/src/queue/tests.rs, vb_storage/src/trimming/tests.rs,
// vb_storage/src/journal/tests.rs, vb_storage/tests/proptest_ps_*.rs, etc.):
fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
    (temp, journal)
}
```

**5 distinct signatures observed**:

| Signature variant | Count | Where |
|---|---:|---|
| `(TempDir, FjallJournal)` | 36 | `vb_storage/src/**`, `vb_storage/tests/**`, `vb_cli/tests/**`, `workspace_tests/**` |
| `Result<(TempDir, FjallJournal), JournalError>` | 4 | `vb_storage/src/tests/fixtures.rs`, `workspace_tests/idempotency_suite/**`, `workspace_tests/tests/vb_core_yaml_e2e_chain_contract.rs`, `vb_storage/src/vb_2bok_durability_gate_tests.rs` |
| `Result<(TempDir, Arc<FjallJournal>), String>` | 2 | `vb_runtime/src/journal/tests/chunk_001.rs`, `vb_cli/tests/admission_evidence_integration/chunk_001.rs` |
| `Result<FjallJournal, String>` | 1 | `vb_storage/tests/accepted_artifact_red_phase.rs` (TempDir dropped immediately — bug) |
| `Option<(TempDir, Arc<...>)>` | 1 | `vb_cli/tests/admission_evidence_integration/chunk_001.rs` |
| `(TempDir, PathBuf)` (only TempDir, no journal) | 2 | `vb_cli/src/commands_status/tests.rs`, `vb_cli/src/commands_system_status/tests.rs` |
| `Result<TestJournal, JournalError>` (custom wrapper) | 1 | `vb_storage/src/admission/tests.rs` |

**Holzman verdict**: most `.expect()` calls violate `unwrap_used = "deny"` workspace lint — already a latent `BLOCK_REGRESSION`. Consolidating lets us fix once.

### 2.2 `fn make_parts` — **20 sites**

Two shapes; both produce `WorkflowParts` with `name="test"`, `digest=zero`, `expressions/accessors/constants/step_names=empty`, `entry=StepIdx(0)`, `resource_contract=DEFAULT`:

```rust
// Shape A — 13 sites: nodes + slot_count, symbols_count=0
fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("test"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}

// Shape B — 7 sites: nodes + slot_count + symbols_count (or with accessors)
fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16, symbols_count: u32) -> WorkflowParts { ... }
//   vb_validate/src/gate_tests.rs:12
//   vb_validate/src/gate_08_verus_proof.rs:142   (with accessors: Box<[AccessorProgram]>)
//   workspace_tests/tests/proptest_validation.rs:158
//   workspace_tests/tests/integration_validate_policy_enforcement.rs:158
//   workspace_tests/tests/vb_test_validate_policy_enforce_behavior.rs:163
//   vb_validate/tests/red_phase_validation.rs:22   (1-node Finish default)
//   vb_validate/src/red_phase_proptest.rs:22        (renamed to arb_parts)
```

The vb_validate `shared::WorkflowParts` re-export (gate_tests.rs + shared/tests.rs) is a sub-re-export — same `WorkflowParts` struct from `vb_core`, just two `use` paths.

### 2.3 `fn make_workflow` — **22 sites**

The `CompiledWorkflow::try_from_parts` finalization wrapper. Body is identical in **all 22 sites**:

```rust
// Canonical body (vb_runtime/src/engine/execute_tests.rs:42, vb_runtime/src/engine/execute/execute_tests.rs:42,
// vb_core/src/engine/tests/mod.rs:1619, etc.):
fn make_workflow(nodes: Vec<CompiledNode>, slot_count: u16) -> CompiledWorkflow {
    let parts = WorkflowParts {
        name: Box::from("test"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::from([]),
    };
    CompiledWorkflow::try_from_parts(parts).expect("workflow validation failed")  // or .unwrap()/panic!
}
```

5 different `panic!` / `.unwrap()` / `.expect()` idioms — Holzman `unwrap_used = "deny"` violation in every site. **3 distinct signatures**:

| Signature | Count | Notes |
|---|---:|---|
| `fn make_workflow() -> CompiledWorkflow` (no args, hard-coded 2-node) | 9 | `vb_storage/tests/proptest_ps_*.rs` |
| `fn make_workflow(nodes, slot_count) -> CompiledWorkflow` | 7 | `vb_runtime/src/engine/execute_tests.rs`, `vb_runtime/src/engine/execute/execute_tests.rs`, `vb_core/src/engine/tests/mod.rs` |
| `fn make_workflow(digest_bytes: [u8;32]) -> CompiledWorkflow` | 1 | `vb_storage/tests/proptest_ps_001_digest_binding.rs:31` |
| `fn make_workflow<F>(name: &str, f: F) -> Result<CompiledWorkflow, String>` | 1 | `vb_core/src/engine/tests/integration_error_routing_behavior.rs:145` — different shape (closure-based) |
| `fn make_workflow_with_constants(nodes, slot_count, constants) -> CompiledWorkflow` | 2 | `vb_runtime/src/engine/execute_tests.rs:47`, `vb_runtime/src/engine/execute/execute_tests.rs:47` |
| `fn make_workflow(fields: Vec<(&str, FieldValue)>) -> WorkflowDoc` | 2 | `vb_validate/src/schema/tests.rs:7`, `workspace_tests/tests/vb_test_validate_yaml_parsing_behavior.rs:25` — different domain (YAML schema, not IR) |
| `fn make_workflow(steps: Vec<StepTypes>) -> WorkflowTypes` | 1 | `vb_validate/src/type_taint/type_taint_tests.rs:121` — different domain (type-taint) |
| `fn make_workflow(steps, slot_count, symbols_count) -> CompiledWorkflow` (via `arb_set_const_workflow`) | 1 | `vb_runtime/src/shard/property_tests/slot_written_before_pc.rs:251` |

### 2.4 `fn make_event` — **15 sites**

```rust
// Canonical body (vb_storage/src/journal/tests.rs:122, vb_storage/src/queue/tests.rs:25,
// vb_storage/src/batch/tests.rs:123, vb_storage/src/batch/byte_accounting_tests.rs:127,
// vb_storage/src/trimming/tests.rs:23, vb_storage/tests/manual_qa_smoke.rs:25,
// workspace_tests/tests/journal_batch_accounting_tests.rs:37):
fn make_event(run: RunId, seq: u64) -> JournalEvent {
    JournalEvent::RunAccepted {
        run,
        seq: EventSeq::new(seq),
        workflow: WorkflowDigest::from_bytes([0; DIGEST_BYTES]),
    }
}
```

The `u64` typed `make_event(run: u64, seq: u64)` variant (in `vb_storage/tests/proptest_vb_vzcuf_PS_*.rs:6` sites) constructs `RunId::new(run)` inside the body — same final value, just an extra `.new()` call. Trivial unification.

`fn make_step_started(run: RunId, seq: u64, step: u16, attempt: u16)` — appears 2× (`vb_storage/tests/manual_qa_smoke.rs:33`, `vb_storage/src/journal/tests.rs:130`).

### 2.5 `fn make_frame` — **8 sites**

```rust
// Canonical body — RunFrame::new with default RunId(1)/StepIdx(0)/5 steps
fn make_frame(slot_count: u16) -> RunFrame {
    RunFrame::new(RunId::new(1), StepIdx::new(0), 5, slot_count).expect("valid frame")
}
//   vb_core/src/replay/choose/mod.rs:322
```

5 different signatures observed:

| Signature | Sites |
|---|---|
| `fn make_frame(slot_count: u16) -> RunFrame` (5 steps hard-coded) | 1 |
| `fn make_frame() -> CoreResult<RunFrame>` (4 steps, 4 slots) | 1 |
| `fn make_frame_with(slots: u16) -> CoreResult<RunFrame>` (4 steps) | 1 |
| `fn make_frame(slot_count: u16) -> Result<RunFrame, CoreError>` | 1 |
| `fn make_frame(workflow: &CompiledWorkflow, run_id: u64) -> RunFrame` | 1 |
| `fn make_frame(run_id: u64, workflow: &CompiledWorkflow) -> Result<RunFrame, String>` | 1 |
| `fn make_frame(workflow: &CompiledWorkflow) -> Result<RunFrame, String>` | 1 |
| `fn make_frame(run_id_val: u64, payload_bytes: impl Into<Bytes>) -> IngressFrame` | 1 (different domain — IPC ingress) |

### 2.6 `fn make_ticket` — **14 sites**

```rust
// Canonical body (vb_runtime/src/action_queue/tests/bounded_queue_tests.rs:19):
fn make_ticket(seq: u32) -> ActionTicket {
    ActionTicket {
        run: RunId::new(1),
        step: StepIdx::new(0),
        seq: SeqNo::new(seq),
        action: ActionId::new(1),
        attempt: 1,
        idempotency_key: seq as u128,
        capacity: 1,
        ..Default::default()
    }
}
```

| Variant | Sites |
|---|---|
| `(seq: u32) -> ActionTicket` | 2 |
| `(key: u128) -> ActionTicket` | 2 |
| `(run, step, attempt, capacity) -> ActionTicket` | 4 |
| `(run, seq) -> ActionTicket` (no step/attempt) | 2 |
| `(mock: MockMarker) -> ActionTicket` (MockMarker variant) | 2 |
| `(step: StepIdx, attempt: u16, capacity: u16) -> ActionTicket` | 1 |
| `(seq: u32, action: u16) -> ActionTicket` | 1 |

The `vb_runtime/tests/vb_jggy_lifecycle_tests.rs:380` variant uses `compute_action_idempotency_key` for the key — substantive difference worth a `make_ticket_with_key` variant.

### 2.7 `fn arb_*` — **75 sites** (definition-level; total occurrences higher when nested)

Proptest strategies span 5 categories:

| Strategy class | Sites | Canonical body length |
|---|---:|---:|
| `arb_slot_value` | 4 | 12 lines |
| `arb_step_state` | 4 | 9 lines (`prop_oneof![Just(_)]` over 8 variants) |
| `arb_taint` | 2 | 6 lines (`prop_oneof![Just(Clean), Just(DerivedFromSecret), Just(Secret)]`) |
| `arb_ticket` | 3 | 22 lines (tuple-of-bounds + prop_map) |
| `arb_capacity` | 2 | 1 line (`1usize..16`) |
| `arb_budget`/`arb_usage`/`arb_capacity_larger_than`/`arb_capacity_smaller_than` | 4 | ~90 lines combined |
| `arb_*` for `BoundaryClass`, `MockMarker`, `SlotIdx`, `StepIdx`, `SeqNo`, `ActionId`, `RunId`, etc. | ~25 | 1–4 lines each |
| `arb_profile_*` (5 functions in `vb_proof_kernels/.../strategies.rs`) | 5 | 60+ lines total |
| `arb_parts` (WorkflowParts generator) | 1 | 25 lines |
| `arb_set_const_workflow` / `arb_mixed_workflow` (full workflow IR) | 2 | 30+ lines combined |

### 2.8 Other duplicated patterns (for completeness)

| Pattern | Sites | Notes |
|---|---:|---|
| `tempfile::tempdir()` direct call | 203 | many inside `fn temp_journal()` and `TempKeyspace::open()` |
| `FjallJournal::open(...)` direct call | 203 | pairs with above |
| `nop_node`, `finish_node`, `copy_node`, `set_const_node` helpers | ~30 | trivial `CompiledNode` constructors |
| `submit_artifact_in_fresh_journal` | ~6 | `vb_storage/src/tests/fixtures.rs:67` re-exported |
| `fresh_frame`, `make_run` (`RunFrame::new(RunId(1), StepIdx(0), slot, step)`) | ~10 | same as `make_frame(slot_count)` |

---

## 3. Proposed new module tree

```text
crates/vb_test_util/src/
├── lib.rs                       # pub mod re-exports + TestSetupError (extended)
├── error.rs                     # TestSetupError (single source)
├── storage/
│   ├── mod.rs                   # pub use open_temp_journal, open_temp_keyspace
│   ├── temp_journal.rs          # open_temp_journal, open_temp_journal_with_events
│   └── temp_keyspace.rs         # TempKeyspace (moved from src/temp_keyspace.rs)
├── workflow/
│   ├── mod.rs                   # pub use canonical builders
│   ├── parts.rs                 # make_parts, make_parts_with_symbols, make_parts_with_accessors
│   ├── workflow.rs              # make_workflow, make_workflow_with_constants
│   ├── event.rs                 # make_event, make_step_started
│   ├── frame.rs                 # make_frame, make_frame_for_workflow, make_run
│   ├── nodes.rs                 # nop_node, finish_node, copy_node, set_const_node
│   └── minimal.rs               # minimal_valid_workflow (ported from vb_storage)
├── builders/
│   ├── mod.rs                   # pub use canonical builders
│   ├── ticket.rs                # make_ticket + 3 variants
│   ├── contract.rs              # make_resource_contract
│   └── value.rs                 # make_slot_value, make_const_value
├── arbitrary/
│   ├── mod.rs                   # feature = "arbitrary"; pub use *
│   ├── slot.rs                  # arb_slot_value, arb_finite_f64_raw
│   ├── step_state.rs            # arb_step_state
│   ├── taint.rs                 # arb_taint
│   ├── ids.rs                   # arb_slot_idx, arb_step_idx, arb_run_id, arb_seq
│   ├── action.rs                # arb_ticket, arb_hostile_ticket, arb_action_*
│   ├── workflow.rs              # arb_parts, arb_set_const_workflow, arb_mixed_workflow
│   ├── budget.rs                # arb_budget, arb_usage, arb_capacity_*
│   └── boundary.rs              # arb_known_class
├── cli/
│   ├── mod.rs                   # pub use run_cli, assert_exit_code, CommandOutput
│   └── runner.rs                # subprocess runner
├── fixture.rs                   # (existing) FixtureCapacity, FixtureBuilder
└── seed.rs                      # (existing) SeededBytes<const N>
```

### 3.1 `lib.rs`

```rust
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, allow(clippy::expect_used))]

pub mod builders;
pub mod error;
pub mod storage;
pub mod workflow;

#[cfg(feature = "arbitrary")]
pub mod arbitrary;

#[cfg(feature = "cli")]
pub mod cli;

pub mod fixture;
pub mod seed;

pub use error::TestSetupError;
```

### 3.2 `error.rs`

```rust
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TestSetupError {
    #[error("out of memory")]
    OutOfMemory,
    #[error("invalid seed: {0}")]
    InvalidSeed(String),
    #[error("invalid capacity: {0}")]
    InvalidCapacity(String),
    #[error("invalid symbol index: {0}")]
    InvalidSymbolIdx(String),
    #[error("temp directory error: {0}")]
    TempDirError(String),
    #[error("fjall open error: {0}")]
    FjallOpenError(String),
    #[error("postcard encode error: {0}")]
    PostcardEncodeError(String),
    #[error("postcard decode error: {0}")]
    PostcardDecodeError(String),
    #[error("workflow parts invalid: {0}")]
    WorkflowPartsInvalid(String),
    #[error("node count out of bounds: {0}")]
    NodeCountOutOfBounds(String),
    #[error("assertion mismatch: {0}")]
    AssertionMismatch(String),
}

impl From<std::io::Error> for TestSetupError {
    fn from(e: std::io::Error) -> Self {
        Self::TempDirError(e.to_string())
    }
}
```

### 3.3 `storage/temp_journal.rs`

```rust
//! Fjall-backed test journals with RAII tempdir cleanup.

use crate::TestSetupError;
use std::path::PathBuf;
use tempfile::TempDir;
use vb_storage::{FjallJournal, JournalError, JournalEvent};

/// Open a fresh Fjall journal in a fresh tempdir.
///
/// Returns `(TempDir, FjallJournal)`. Caller **must** keep the `TempDir` alive
/// for the journal's lifetime — bind both with `let (tmp, journal) = ...;`.
///
/// # Errors
///
/// Returns `TestSetupError::TempDirError` or `TestSetupError::FjallOpenError`.
pub fn open_temp_journal() -> Result<(TempDir, FjallJournal), TestSetupError>;

/// Pre-populated journal with the supplied events already appended.
///
/// Events are appended in order; duplicate `seq` values are NOT deduplicated
/// here — caller is responsible for monotonic sequence numbers.
pub fn open_temp_journal_with_events(
    events: &[JournalEvent],
) -> Result<(TempDir, FjallJournal), TestSetupError>;

/// Open a fresh journal and wrap it in `Arc` for sharing across threads.
pub fn open_temp_journal_arc() -> Result<(TempDir, std::sync::Arc<FjallJournal>), TestSetupError>;

/// Convenience: drop the tempdir, return only the journal path (rarely useful).
pub fn open_temp_journal_path() -> Result<(TempDir, PathBuf), TestSetupError>;

/// Convert a `JournalError` into `TestSetupError` for callers that previously
/// used `Result<_, JournalError>`.
impl From<JournalError> for TestSetupError;
```

### 3.4 `storage/temp_keyspace.rs` (relocated from `src/temp_keyspace.rs`)

API unchanged: `TempKeyspace::open()`, `.path()`, `.database()`. Field drop order documented as before.

### 3.5 `workflow/parts.rs`

```rust
//! Canonical `WorkflowParts` builders.

use vb_core::ids::StepIdx;
use vb_core::workflow::{CompiledNode, ResourceContract, WorkflowParts, WorkflowDigest};
use vb_core::workflow::AccessorProgram;

pub const DEFAULT_PARTS_NAME: &str = "test";
pub const DEFAULT_PARTS_DIGEST_BYTES: [u8; 32] = [0u8; 32];

/// Build `WorkflowParts` with `symbols_count = 0`, empty expressions/accessors/constants/step_names,
/// `entry = StepIdx::new(0)`, `resource_contract = ResourceContract::DEFAULT`.
///
/// Matches the canonical body used in 13 sites.
pub fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts;

/// Build `WorkflowParts` with explicit `symbols_count`. Matches the 7-site variant.
pub fn make_parts_with_symbols(
    nodes: Vec<CompiledNode>,
    slot_count: u16,
    symbols_count: u32,
) -> WorkflowParts;

/// Build `WorkflowParts` with explicit `accessors` (used by `gate_08_verus_proof`).
pub fn make_parts_with_accessors(
    nodes: Vec<CompiledNode>,
    slot_count: u16,
    symbols_count: u32,
    accessors: Box<[AccessorProgram]>,
) -> WorkflowParts;

/// Single-node Finish-only minimal parts (1 site variant, vb_validate red_phase).
pub fn make_minimal_finish_parts(slot_count: u16, symbols_count: u32) -> WorkflowParts;
```

### 3.6 `workflow/workflow.rs`

```rust
//! Canonical `CompiledWorkflow` finalization.

use vb_core::workflow::{CompiledNode, CompiledWorkflow, WorkflowError};
use vb_core::value::ConstValue;

pub fn make_workflow(nodes: Vec<CompiledNode>, slot_count: u16) -> CompiledWorkflow;
pub fn make_workflow_with_constants(
    nodes: Vec<CompiledNode>,
    slot_count: u16,
    constants: Box<[ConstValue]>,
) -> CompiledWorkflow;
pub fn make_workflow_named(name: &str, nodes: Vec<CompiledNode>, slot_count: u16) -> CompiledWorkflow;
pub fn make_minimal_valid_workflow() -> Result<CompiledWorkflow, WorkflowError>;
pub fn make_minimal_workflow_with_digest(
    digest_bytes: [u8; 32],
) -> CompiledWorkflow;
```

All `make_workflow*` panic with `panic!`-free `Result`/typed-error semantics — internal `expect()` calls annotated `#[allow(clippy::expect_used, reason = "test fixture construction failure is fatal")]` at the module level. This satisfies the production `expect_used = "deny"` lint by localising the override to the test-utility crate (whose only purpose is to provide fixture builders).

### 3.7 `workflow/event.rs`

```rust
use vb_core::{RunId, StepIdx, WorkflowDigest};
use vb_storage::{EventSeq, JournalEvent};

pub fn make_event(run: RunId, seq: u64) -> JournalEvent;
pub fn make_event_raw(run: u64, seq: u64) -> JournalEvent;  // u64 variant (PS_* sites)
pub fn make_run_accepted(run: RunId, seq: u64) -> JournalEvent;  // explicit-name alias
pub fn make_step_started(run: RunId, seq: u64, step: u16, attempt: u16) -> JournalEvent;
pub fn make_run_cancelled(run: RunId, seq: u64) -> JournalEvent;
```

### 3.8 `workflow/frame.rs`

```rust
use vb_core::frame::RunFrame;
use vb_core::{RunId, StepIdx};
use vb_core::workflow::CompiledWorkflow;

pub fn make_frame(slot_count: u16) -> RunFrame;
pub fn make_frame_for(workflow: &CompiledWorkflow, run_id: RunId) -> RunFrame;
pub fn make_frame_with(run_id: RunId, pc: StepIdx, steps: u16, slots: u16) -> RunFrame;
pub fn make_run(slot_count: u16, step_count: u16) -> RunFrame;
```

### 3.9 `workflow/nodes.rs`

```rust
use vb_core::workflow::{CompiledNode, CompiledNodeKind};
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx};

pub fn nop_node(idx: u16) -> CompiledNode;                  // next = idx+1, output = None
pub fn nop_forward(idx: u16, next: u16) -> CompiledNode;
pub fn finish_node(idx: u16, result_slot: u16) -> CompiledNode;
pub fn copy_node(idx: u16, src: u16, out: u16) -> CompiledNode;
pub fn set_const_node(idx: u16, out: u16, value: ConstIdx) -> CompiledNode;
```

### 3.10 `builders/ticket.rs`

```rust
use vb_core::action::{ActionTicket, compute_action_idempotency_key};
use vb_core::ids::{ActionId, RunId, SeqNo, StepIdx};

pub fn make_ticket(seq: u32) -> ActionTicket;
pub fn make_ticket_with_key(key: u128) -> ActionTicket;
pub fn make_ticket_with_capacity(
    run: RunId,
    step: StepIdx,
    attempt: u16,
    capacity: u16,
) -> ActionTicket;
pub fn make_ticket_full(
    run: RunId,
    step: StepIdx,
    seq: SeqNo,
    action: ActionId,
    attempt: u16,
    capacity: u16,
) -> ActionTicket;                                          // computes idempotency_key via compute_action_idempotency_key
pub fn make_ticket_with_mock(mock: vb_core::action::MockMarker) -> ActionTicket;
```

### 3.11 `arbitrary/*.rs` — feature-gated under `feature = "arbitrary"`

```rust
// arbitrary/mod.rs
#![cfg(feature = "arbitrary")]

pub mod slot;
pub mod step_state;
pub mod taint;
pub mod ids;
pub mod action;
pub mod workflow;
pub mod budget;
pub mod boundary;
```

Each submodule re-exports `arb_*` constructors that mirror the local definitions in vb_core, vb_runtime, vb_validate, vb_cli, vb_proof_kernels, vb_expr, vb_storage, vb_compile, vb_boundary_inventory. The 75-site `arb_*` count collapses into ~30 canonical strategies + ~5 cross-crate composites.

The `feature = "arbitrary"` gate ensures vb_test_util only pulls `proptest` as a hard dep when the consumer opts in. Without the feature, the crate is lighter and doesn't transitively force `proptest` into crates that don't use property tests.

### 3.12 `cli/runner.rs` — feature-gated under `feature = "cli"`

```rust
use std::process::{Command, ExitStatus, Output, Stdio};
use crate::TestSetupError;

pub struct CommandOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub fn run_cli(binary: &str, args: &[&str]) -> Result<CommandOutput, TestSetupError>;
pub fn assert_exit_code(output: &CommandOutput, expected: i32) -> Result<(), TestSetupError>;
pub fn assert_stdout_contains(output: &CommandOutput, needle: &str) -> Result<(), TestSetupError>;
pub fn assert_stderr_contains(output: &CommandOutput, needle: &str) -> Result<(), TestSetupError>;
```

Used by `vb_cli/src/commands_status/tests.rs`, `vb_cli/tests/lifecycle_integration.rs`, `vb_cli/tests/admission_evidence_integration/chunk_001.rs` — currently these call `Command::new("velvet-ballistics")` ad hoc.

### 3.13 `Cargo.toml` (proposed, feature-gated)

```toml
[package]
name = "vb_test_util"
version.workspace = true
edition.workspace = true
license.workspace = true
publish = false

[dependencies]
blake3.workspace = true
postcard.workspace = true
rand.workspace = true
tempfile.workspace = true
thiserror.workspace = true
fjall.workspace = true
vb_core = { path = "../vb_core" }
vb_storage = { path = "../vb_storage" }

[target.'cfg(not(any(test, doctest)))'.dependencies]
# Production consumers (CLI, IPC, runtime fixture builders) gated to non-test
# contexts so `cargo build --release` does not pull test-only paths.

[features]
default = []
arbitrary = ["dep:proptest"]
cli = []
proptest = ["dep:proptest"]  # alias for downstream backward-compat

[dev-dependencies]
# proptest is dev-only on the crate itself; consumers may opt in via
# features = ["arbitrary"] in their [dev-dependencies] entry.
proptest = { workspace = true }

[lints]
workspace = true
```

Note: `vb_runtime` (for `ActionTicket`) is **NOT** added as a hard dependency of `vb_test_util` because that would force vb_test_util → vb_runtime → vb_storage → vb_core chain onto every consumer. Instead, `builders/ticket.rs` lives behind `#[cfg(feature = "runtime")]`, opt-in for crates that already pull vb_runtime.

### 3.14 Per-module `Cargo.toml` feature matrix

| Module | Required dep | Activation |
|---|---|---|
| `storage/` | `tempfile`, `fjall`, `vb_storage` | always (in `[dependencies]`) |
| `workflow/parts.rs`, `workflow.rs`, `frame.rs`, `nodes.rs`, `event.rs` | `vb_core` | always |
| `workflow/event.rs` | `vb_storage` (for `JournalEvent`) | always |
| `workflow/minimal.rs` | `vb_core` | always |
| `builders/ticket.rs` | `vb_core` | `feature = "runtime"` |
| `builders/value.rs` | `vb_core` | always |
| `builders/contract.rs` | `vb_core` | always |
| `arbitrary/*` | `proptest` (via `dep:proptest`) | `feature = "arbitrary"` |
| `cli/runner.rs` | (stdlib only) | `feature = "cli"` |

This matrix keeps vb_test_util from becoming a heavy aggregate crate while absorbing the duplications.

---

## 4. Cycle risk analysis

### 4.1 Dependency graph (proposed)

```text
vb_test_util  ──[dependencies]──>  vb_core
              ──[dependencies]──>  vb_storage  ──[dependencies]──>  vb_core
              ──[optional]─────>  vb_runtime  ──[dependencies]──>  vb_storage, vb_core
              ──[features]────>  proptest, fjall, tempfile

consumer crate (e.g. vb_storage)  ──[dev-dependencies]──>  vb_test_util
```

### 4.2 Cargo cycle rules

Cargo's cycle detection treats `[dependencies]` and `[dev-dependencies]` as **separate edges**. A dev-dependency edge is only materialized during `cargo test`, `cargo bench`, etc. — never during `cargo build` or `cargo build --release`. Therefore:

- `vb_storage` [dependencies] does NOT see `vb_test_util` → no production cycle.
- `vb_storage` [dev-dependencies] sees `vb_test_util` → only when building `cargo test -p vb_storage`.
- `vb_test_util` [dependencies] sees `vb_storage` → always.

**Resolution**: a `cargo test` invocation in `vb_storage` builds:
```
vb_test_util (test) ──> vb_storage (test) ──> vb_core (test) ──> (terminal)
```

This is a **diamond, not a cycle**, because `vb_storage`'s dev-dependency on `vb_test_util` is the *outgoing* edge from vb_storage; the *incoming* edge is `vb_test_util → vb_storage`. Cargo resolves this via node duplication (Rust 2024 + resolver v2): the test build of `vb_storage` is one node, the production build (consumed by `vb_test_util`) is a different node.

**Confirmed by the existing pattern**: `vb_runtime` already does this:

```toml
# crates/vb_runtime/Cargo.toml (existing)
[dependencies]
vb_core = { path = "../vb_core" }           # production edge
vb_storage = { path = "../vb_storage" }     # production edge

[dev-dependencies]
vb_core = { path = "../vb_core", features = ["test-util"] }  # test-only
```

And `vb_storage`'s `[dev-dependencies]` already includes `tempfile` (line 32), so adding `vb_test_util` is symmetric.

### 4.3 Workspace-wide verification (grep-confirmed)

```
$ rg 'vb_test_util' crates/*/Cargo.toml
crates/vb_test_util/Cargo.toml:2:                name = "vb_test_util"
crates/workspace_tests/Cargo.toml:51:           vb_test_util = { path = "../vb_test_util" }   ← dev-dep
crates/workspace_tests/Cargo.toml:54:           name = "vb_test_util_crate"                  ← [[test]]
crates/workspace_tests/Cargo.toml:55:           path = "tests/vb_test_util_crate.rs"
```

**No `[dependencies]` reference to `vb_test_util` exists anywhere.** The single `[dev-dependencies]` consumer is `workspace_tests`. All migrations will **add** `[dev-dependencies] vb_test_util = { path = "../vb_test_util" }` lines — no migration will ever touch `[dependencies]`.

### 4.4 `vb_test_util → vb_core` is a new edge

Today's `vb_test_util/Cargo.toml` has zero workspace crate deps (only `rand`, `postcard`, `thiserror`, `tempfile`, `fjall`). Adding `vb_core` and `vb_storage` as production deps is **new**.

This is acceptable because:
1. `vb_test_util` already imports `fjall::Database` (transitively: nothing else imports fjall besides vb_storage). With the new structure, `vb_test_util` calls `vb_storage::FjallJournal::open` rather than `fjall::Database::builder` — semantically equivalent, more type-safe.
2. The crate's purpose **is** to centralize fixtures from those crates; the dependency edge is principled.

### 4.5 Feature-gating the `runtime` modules

`vb_runtime` itself transitively depends on `vb_storage`. If `vb_test_util` adds `vb_runtime` as a `[dependencies]`, then **every dev-dependency consumer of `vb_test_util` must compile `vb_runtime`** for tests — a significant compile-time tax for crates that don't use runtime fixtures.

**Resolution**: gate `builders/ticket.rs` and any `runtime`-namespace helpers behind `#[cfg(feature = "runtime")]` and only enable that feature in the `[dev-dependencies]` declaration of crates that actually need runtime fixtures (`vb_runtime`, `workspace_tests`, `vb_cli`).

---

## 5. Migration plan

### 5.1 Ordering principle

Migrate **leaf crates first** (no other test consumer depends on them), then **workspace_tests last** as the integration smoke test.

```
Phase 0: introduce new modules in vb_test_util (additive, no breakage)
Phase 1: leaf crates with proptest (vb_expr, vb_compile, vb_ipc, vb_proof_kernels,
         vb_boundary_inventory, vb_queue_semantics, vb_doc, vb_benchmark)
Phase 2: vb_storage (largest gain — 21+ temp_journal sites, 9 proptest sites)
Phase 3: vb_core (5 make_parts sites, 3 make_frame sites, 8 arb_* sites)
Phase 4: vb_validate (4 make_parts sites, 2 proptest sites)
Phase 5: vb_runtime (4 make_ticket sites, 2 make_workflow sites, 3 arb_* sites)
Phase 6: vb_cli (4 temp_journal sites, 1 make_event site, 1 arb_* site)
Phase 7: workspace_tests (consolidation smoke test)
```

### 5.2 Per-phase diff example

#### Phase 0: add `storage::open_temp_journal` (additive)

Before (no migration needed yet — this is the new module landing):

```rust
// crates/vb_test_util/src/storage/temp_journal.rs  (NEW)
```

After: nothing changes in any consumer. They keep their `fn temp_journal()` until phase 2.

#### Phase 2: vb_storage migration (one site)

Before — `crates/vb_storage/src/queue/tests.rs:22-23`:

```rust
fn temp_journal() -> (tempfile::TempDir, FjallJournal) {
    let temp = tempfile::tempdir().expect("tempdir creation should succeed");
    let journal = FjallJournal::open(temp.path(), None).expect("journal open should succeed");
    (temp, journal)
}
```

After:

```rust
// crates/vb_storage/src/queue/tests.rs
use vb_test_util::storage::open_temp_journal;

// fn temp_journal removed (3-line body inlined into caller).
```

Caller change (in the same file):

```rust
let (temp, journal) = open_temp_journal().expect("test journal setup");
```

`vb_storage/Cargo.toml` change:

```diff
 [dev-dependencies]
 proptest = { workspace = true }
 tempfile.workspace = true
+vb_test_util = { path = "../vb_test_util" }
```

#### Phase 2: vb_storage migration (proptest site)

Before — `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs:31-60`:

```rust
fn make_workflow() -> CompiledWorkflow {
    let mut parts = WorkflowParts {
        name: Box::<str>::from("proptest_001"),
        digest: WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([ ... 30 lines ... ]),
        ...
    };
    ...
}
```

After:

```rust
use vb_test_util::workflow::{make_parts, make_workflow};
// (29-line function deleted; replace call sites with make_workflow(vec![node0, node1], 1))
```

#### Phase 7: workspace_tests consolidation

All 47 `temp_journal` definitions and 22 `make_workflow` definitions across the workspace become single `use vb_test_util::storage::open_temp_journal;` + `use vb_test_util::workflow::make_workflow;` imports.

### 5.3 Per-bead allocation

Recommended bead structure (each is a single atomic delivery):

| Bead | Crate | Pattern replaced | Sites |
|---|---|---|---:|
| `vb-tutil-001` | `vb_test_util` | new `storage/temp_journal.rs` (additive) | n/a |
| `vb-tutil-002` | `vb_test_util` | new `workflow/parts.rs`, `workflow.rs`, `event.rs`, `frame.rs`, `nodes.rs` | n/a |
| `vb-tutil-003` | `vb_test_util` | new `builders/ticket.rs` | n/a |
| `vb-tutil-004` | `vb_test_util` | new `arbitrary/*` (feature-gated) | n/a |
| `vb-tutil-005` | `vb_test_util` | new `cli/runner.rs` (feature-gated) | n/a |
| `vb-storage-tutil` | `vb_storage` | migrate `temp_journal` (21), `make_workflow` (9), `make_event` (5) | 35 |
| `vb-core-tutil` | `vb_core` | migrate `make_workflow` (3), `make_frame` (4), `make_parts` (5), `arb_*` (8) | 20 |
| `vb-validate-tutil` | `vb_validate` | migrate `make_parts` (4), `arb_parts` (1) | 5 |
| `vb-runtime-tutil` | `vb_runtime` | migrate `make_ticket` (4), `make_workflow` (4), `arb_*` (3) | 11 |
| `vb-cli-tutil` | `vb_cli` | migrate `temp_journal` (3), `make_event` (1), `arb_*` (1) | 5 |
| `vb-wt-tutil-finalize` | `workspace_tests` | migrate all remaining in `workspace_tests/tests/**` + idempotency_suite | ~25 |

**Total: 11 beads, additive-first, leaf-first.**

### 5.4 Acceptance gates per bead

- `cargo fmt --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::indexing_slicing -D clippy::arithmetic_side_effects`
- `cargo test --workspace --all-features --no-run`
- `cargo test --workspace --all-features` (specifically: `-p vb_storage`, `-p vb_core`, `-p vb_validate`, `-p vb_runtime`, `-p vb_cli`, `-p workspace_tests`)
- `moon run :nightly-feature-gate` (per AGENTS.md)

### 5.5 Revert strategy

Each phase is a single commit on a single crate. A failing test in any phase reverts to phase-pre without touching the other phases. The additive Phase 0 means the first 5 beads (vb-tutil-001 through vb-tutil-005) **never** break consumers — they only add new code paths.

---

## 6. LOC savings estimate

Conservative, source-line counts only (no comments, no blank lines excluded — straight `fn temp_journal()` body line count).

| Migration target | Sites | Avg body LOC/site | Total LOC saved |
|---|---:|---:|---:|
| `temp_journal` → `open_temp_journal` | 47 | 4 | **188** |
| `make_parts` (Shape A 2-arg) → `make_parts` | 13 | 13 | **169** |
| `make_parts` (Shape B 3-arg) → `make_parts_with_symbols` | 7 | 14 | **98** |
| `make_workflow` (2-arg) → `make_workflow` | 7 | 25 | **175** |
| `make_workflow` (0-arg w/ hard-coded 2-node) → `make_minimal_valid_workflow` | 9 | 25 | **225** |
| `make_workflow_with_constants` → canonical | 2 | 26 | **52** |
| `make_event` → `make_event` | 13 | 7 | **91** |
| `make_frame` → `make_frame` | 7 | 4 | **28** |
| `make_ticket` → `make_ticket` (4-shape family) | 14 | 11 | **154** |
| `arb_slot_value` → canonical | 4 | 12 | **48** |
| `arb_step_state` → canonical | 4 | 9 | **36** |
| `arb_taint` → canonical | 2 | 6 | **12** |
| `arb_ticket` → canonical | 3 | 22 | **66** |
| `arb_capacity` → canonical | 2 | 1 | **2** |
| `arb_budget`/`arb_usage`/`arb_capacity_*` → canonical | 4 | 80 | **320** |
| `arb_set_const_workflow`/`arb_mixed_workflow` → canonical | 2 | 30 | **60** |
| `arb_parts` → canonical | 1 | 25 | **25** |
| `arb_profile_*` (5 fns in 1 file) → canonical | 1 file | 60 | **60** |
| `arb_*` simple (ids, BoundaryClass, MockMarker, etc.) | ~25 | 4 | **100** |
| `nop_node`/`finish_node`/`copy_node`/`set_const_node` → canonical | ~30 | 6 | **180** |
| `fresh_frame`/`make_run` → `make_run` | ~10 | 4 | **40** |
| `submit_artifact_in_fresh_journal` re-exports | 6 | 1 | **6** |
| `tempfile::tempdir()` direct call sites (NOT inside `temp_journal`) | ~150 | 2 | **300** |
| **Total fixture body LOC removed** | | | **~2,435** |

### 6.1 Net after vb_test_util growth

`vb_test_util` new modules add roughly:

| Module | New LOC |
|---|---:|
| `error.rs` (existing + 4 variants) | 50 |
| `storage/temp_journal.rs` | 80 |
| `storage/temp_keyspace.rs` (relocated) | 118 (no change) |
| `workflow/parts.rs` | 70 |
| `workflow/workflow.rs` | 90 |
| `workflow/event.rs` | 35 |
| `workflow/frame.rs` | 50 |
| `workflow/nodes.rs` | 60 |
| `workflow/minimal.rs` | 65 |
| `builders/ticket.rs` | 100 |
| `builders/value.rs` | 30 |
| `builders/contract.rs` | 25 |
| `arbitrary/*` (8 submodules) | 400 |
| `cli/runner.rs` | 80 |
| Module-level doc-comments | 150 |
| Per-fn unit tests (in-crate) | 200 |
| **Net vb_test_util growth** | **~1,600** |

### 6.2 Net workspace savings

```
fixture-LOC-removed        : 2,435
- vb_test_util-LOC-added   : 1,600
- per-consumer dev-dep line: 11 × 1 = 11  (one new line per Cargo.toml)
------------------------------
NET WORKSPACE SAVINGS       : ~824 LOC
```

But the **real** savings is not the line count — it is **canon**: one `make_parts` to fix when `WorkflowParts` grows a new field, not 20. The LOC delta is the visible artifact; the **maintenance delta is the actual win** (estimated ~5×: every new field on `WorkflowParts`, `CompiledNode`, `ActionTicket`, `JournalEvent` currently requires touching 13–47 sites).

---

## 7. Per-crate `[dev-dependencies]` diff

Every migration adds exactly **one line** to the consumer's `Cargo.toml` `[dev-dependencies]`. For proptest strategy migration, the consumer additionally enables the `arbitrary` feature.

### 7.1 Diff table

```diff
# crates/vb_storage/Cargo.toml  (Phase 2)
 [dev-dependencies]
 proptest = { workspace = true }
 tempfile.workspace = true
+vb_test_util = { path = "../vb_test_util", features = ["arbitrary"] }

# crates/vb_core/Cargo.toml  (Phase 3)
 [dev-dependencies]
 proptest = { workspace = true }
+vb_test_util = { path = "../vb_test_util", features = ["arbitrary"] }

# crates/vb_validate/Cargo.toml  (Phase 4)
 [dev-dependencies]
 proptest = { workspace = true }
+vb_test_util = { path = "../vb_test_util", features = ["arbitrary"] }

# crates/vb_runtime/Cargo.toml  (Phase 5)
 [dev-dependencies]
 proptest = { workspace = true }
 tempfile.workspace = true
 vb_core = { path = "../vb_core", features = ["test-util"] }
+vb_test_util = { path = "../vb_test_util", features = ["arbitrary", "runtime"] }

# crates/vb_cli/Cargo.toml  (Phase 6)
 [dev-dependencies]
 proptest = { workspace = true }
 tempfile.workspace = true
+vb_test_util = { path = "../vb_test_util", features = ["arbitrary", "cli", "runtime"] }

# crates/vb_compile/Cargo.toml  (Phase 1)
 [dev-dependencies]
 proptest = { workspace = true }
+vb_test_util = { path = "../vb_test_util", features = ["arbitrary"] }

# crates/vb_expr/Cargo.toml  (Phase 1)
 [dev-dependencies]
 proptest = { workspace = true }
+vb_test_util = { path = "../vb_test_util", features = ["arbitrary"] }

# crates/vb_ipc/Cargo.toml  (Phase 1)
 [dev-dependencies]
 proptest = { workspace = true }
+vb_test_util = { path = "../vb_test_util", features = ["arbitrary"] }

# crates/vb_proof_kernels/Cargo.toml  (Phase 1)
 [dev-dependencies]
 proptest = { workspace = true }
+vb_test_util = { path = "../vb_test_util", features = ["arbitrary"] }

# crates/vb_boundary_inventory/Cargo.toml  (Phase 1)
 [dev-dependencies]
 proptest.workspace = true
+vb_test_util = { path = "../vb_test_util", features = ["arbitrary"] }

# crates/vb_doc/Cargo.toml  (Phase 1)
 [dev-dependencies]
 proptest.workspace = true
+vb_test_util = { path = "../vb_test_util" }   # no arbitrary unless doc has proptests

# crates/vb_benchmark/Cargo.toml  (Phase 1)
 [dev-dependencies]
 proptest.workspace = true
+vb_test_util = { path = "../vb_test_util", features = ["arbitrary"] }

# crates/workspace_tests/Cargo.toml  (Phase 7, already in place)
 [dev-dependencies]
 vb_test_util = { path = "../vb_test_util" }   # ADD features = ["arbitrary", "cli", "runtime"]
```

### 7.2 Crates that DON'T need vb_test_util

| Crate | Reason |
|---|---|
| `vb_ajc40_flux` | excluded from workspace (Cargo.toml:24) |
| `vb_queue_semantics` | only has 1 arb_* (`arb_capacity`); migrate if convenient, skip otherwise |
| `vb_verification` | already pulls vb_test_util transitively through vb_runtime's test-util feature; verify before adding |
| `xtask` | tooling-only, no test fixtures |
| `fuzz` | excluded from workspace |
| `vb_ui` | excluded from workspace |

### 7.3 Feature-flag interaction check

The proposed `vb_test_util` features (`arbitrary`, `cli`, `runtime`) are **additive** — turning them on only adds dependencies, never removes. Therefore:

- Adding `features = ["arbitrary"]` to a consumer's dev-dep entry **only** causes that consumer's test build to additionally pull `proptest`. Consumers that already declare `proptest` in `[dev-dependencies]` see no compile-graph change.
- Adding `features = ["runtime"]` causes consumers that don't already have `vb_runtime` in their test graph to additionally pull it. **Audit**: only `vb_cli`, `vb_runtime`, and `workspace_tests` enable this feature; all three already pull `vb_runtime` transitively. **No new compile-graph growth.**

### 7.4 moon.yml interaction

`moon ci` is the canonical gate (AGENTS.md §"Build And CI"). The proposed changes:

- New tasks may be added to `.moon/` only if a moon task exists today that runs `cargo test -p vb_test_util` — currently none, so **no moon.yml change required**. `cargo test --workspace --all-features` covers vb_test_util transitively.
- `moon run :nightly-feature-gate` is unaffected because no new nightly features are used in `vb_test_util` or its consumers.

---

## 8. Open design questions (require human arbitration)

1. **Should `vb_test_util` move out of `crates/` to `crates/workspace_tests/vb_test_util/` to clarify it is a test-only crate?** Current location is fine — `publish = false` already prevents external consumption.

2. **Should `minimal_valid_workflow` (currently in `vb_storage/src/tests/fixtures.rs:18`) move to `vb_test_util::workflow::minimal`? Yes** — it's a storage-test helper that depends only on `vb_core`. Moving it eliminates a circular-looking dep where `vb_storage::tests` knows how to construct a `CompiledWorkflow` from scratch.

3. **Should the `arbitrary` module's `arb_*` strategies **accept** user-provided seed values or use `proptest::any()` directly?** Current sites use both. The canonical version should use `proptest::any()` for unconstrained fields and named-bounded ranges for constrained ones (e.g. `0u16..u16::MAX` for `StepIdx`). This matches the existing `arb_ticket` in `vb_runtime/src/verification/proptest/proptest_attempt_fence.rs:39` pattern.

4. **Should the `cli::runner.rs` use `assert_cmd` (already a workspace transitive dep?) or stdlib `Command`?** Skim of workspace shows `assert_cmd` is not declared. Sticking to `Command` keeps the surface zero-cost; `assert_cmd` could be added as a future enhancement.

5. **Should `arb_*` strategies that produce `BoxedStrategy<T>` be moved when the existing site returns `impl Strategy<Value = T>`?** Yes — the return type is part of the API and a BoxedStrategy gives callers flexibility. The canonical signature is `pub fn arb_x() -> BoxedStrategy<X>` for strategies that are recursively composed; `impl Strategy<Value = X>` for leaf strategies.

6. **What happens to the Holzman `unwrap_used = "deny"` lint in `vb_test_util` itself?** The crate already uses `#![allow(clippy::panic, clippy::panic_in_result_fn)]` in `fixture.rs` and `seed.rs`. The new modules need the same local allows. Document this with a `// SAFETY (test-fixture only): ...` comment at each call site as the project pattern dictates.

---

## 9. Files read

Source files (verbatim content read):

| Path | Purpose |
|---|---|
| `/home/lewis/src/velvet-ballistics/crates/vb_test_util/src/lib.rs` | current `TestSetupError` |
| `/home/lewis/src/velvet-ballistics/crates/vb_test_util/src/fixture.rs` | `FixtureCapacity`, `FixtureBuilder` |
| `/home/lewis/src/velvet-ballistics/crates/vb_test_util/src/seed.rs` | `SeededBytes<const N>` |
| `/home/lewis/src/velvet-ballistics/crates/vb_test_util/src/temp_keyspace.rs` | `TempKeyspace` |
| `/home/lewis/src/velvet-ballistics/crates/vb_test_util/Cargo.toml` | current deps |
| `/home/lewis/src/velvet-ballistics/Cargo.toml` | workspace members + lints |
| `/home/lewis/src/velvet-ballistics/crates/vb_storage/Cargo.toml` | dev-deps |
| `/home/lewis/src/velvet-ballistics/crates/vb_core/Cargo.toml` | dev-deps |
| `/home/lewis/src/velvet-ballistics/crates/vb_runtime/Cargo.toml` | dev-deps |
| `/home/lewis/src/velvet-ballistics/crates/vb_validate/Cargo.toml` | dev-deps |
| `/home/lewis/src/velvet-ballistics/crates/vb_cli/Cargo.toml` | dev-deps |
| `/home/lewis/src/velvet-ballistics/crates/workspace_tests/Cargo.toml` | current consumer + `[[test]]` blocks |

Sample consumers read (canonical body extraction):

| Path | Pattern sampled |
|---|---|
| `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/tests/fixtures.rs` | canonical `temp_journal`, `minimal_valid_workflow` |
| `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/queue/tests.rs` | `temp_journal`, `make_event` |
| `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/journal/tests.rs` | `temp_journal`, `make_event`, `make_step_started` |
| `/home/lewis/src/velvet-ballistics/crates/vb_storage/tests/manual_qa_smoke.rs` | `temp_journal`, `make_event` |
| `/home/lewis/src/velvet-ballistics/crates/vb_storage/tests/proptest_ps_001_digest_binding.rs` | `temp_journal`, `make_workflow(digest_bytes)` |
| `/home/lewis/src/velvet-ballistics/crates/vb_validate/src/gate_tests.rs` | `make_parts` (3-arg with `symbols_count`) |
| `/home/lewis/src/velvet-ballistics/crates/vb_validate/src/shared/tests.rs` | `make_parts` (2-arg), `finish_node` |
| `/home/lewis/src/velvet-ballistics/crates/vb_validate/tests/red_phase_validation.rs` | `make_parts` (1-arg with default nodes), `finish_node` |
| `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/engine/execute_tests.rs` | `make_workflow`, `make_workflow_with_constants`, `make_run` |
| `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/idempotency/tests.rs` | `make_ticket(key)` |
| `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/action_queue/tests/bounded_queue_tests.rs` | `make_ticket(seq)`, `make_ticket_with_action` |
| `/home/lewis/src/velvet-ballistics/crates/vb_runtime/tests/vb_jggy_lifecycle_tests.rs` | `make_ticket(run, step, attempt, capacity)` |
| `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/verification/proptest/mod.rs` | `arb_ticket`, `arb_capacity` |
| `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/verification/proptest/proptest_attempt_fence.rs` | `arb_ticket`, `arb_hostile_ticket` |
| `/home/lewis/src/velvet-ballistics/crates/vb_core/src/replay/choose/mod.rs` | `make_frame(slot_count)` |
| `/home/lewis/src/velvet-ballistics/crates/vb_core/src/engine/tests/integration_frame_behavior.rs` | `make_frame`, `make_frame_with` |
| `/home/lewis/src/velvet-ballistics/crates/vb_core/src/workflow/proptest_workflow.rs` | `arb_step_state` |
| `/home/lewis/src/velvet-ballistics/crates/vb_core/src/budget_vb_8mdp_7_prop_tests.rs` | `arb_budget`, `arb_usage`, `arb_capacity_*` |
| `/home/lewis/src/velvet-ballistics/crates/vb_validate/src/red_phase_proptest.rs` | `arb_parts`, `arb_accessor` |
| `/home/lewis/src/velvet-ballistics/crates/vb_runtime/src/primitives/reentry_tests.rs` | `arb_step_state`, `arb_i64_list` |
| `/home/lewis/src/velvet-ballistics/crates/vb_cli/src/properties/prop_gen.rs` | `arb_slot_value`, `arb_taint`, `arb_step_state` |
| `/home/lewis/src/velvet-ballistics/crates/vb_core/src/action/model.rs` | `ActionTicket` field structure |

Existing design doc format reference (not modified):

| Path | Purpose |
|---|---|
| `/home/lewis/src/velvet-ballistics/docs/design/eval_append-cumulative-design.md` | established design-doc format |

---

## 10. Summary table

| Item | Value |
|---|---|
| Current `vb_test_util` size | 347 LOC across 4 files |
| Current `vb_test_util` consumers | 1 external (`workspace_tests`) |
| Sites that would migrate | ~135 (47 + 20 + 22 + 15 + 14 + 75 - 13 already inside `vb_test_util`) |
| New `vb_test_util` size | ~1,950 LOC across 17 files |
| Net workspace LOC delta | **−824 LOC** (fixture body) + canonicalization wins |
| New `[dev-dependencies]` lines | 11 (one per consumer crate) |
| New `[dependencies]` to workspace crates | 2 (`vb_core`, `vb_storage`) in `vb_test_util` |
| Cycle risk | **none** — dev-deps are not production edges |
| Beads | 11 (5 additive + 6 migrations) |
| Build graph growth | zero for `cargo build`; test-build adds `proptest` to consumers that don't yet have it (already in most crates' `[dev-dependencies]`) |
| New nightly features | none |
