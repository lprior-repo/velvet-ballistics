All five files are test fixtures constructing `WorkflowParts` for internal validation testing. None declare external input schemas — they test internal gates (expression stack, accessor paths, slot refs, node kinds). Empty `input_slots` is safe in every case.

**Required mutation for all five files:**
Add `input_slots: Box::new([]),` to each `WorkflowParts` struct literal.

---

**File 1: `crates/vb_validate/src/shared/tests.rs`**
```rust
fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16) -> crate::shared::WorkflowParts {
    crate::shared::WorkflowParts {
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        input_slots: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}
```
Rationale: Pipeline default/no-gates tests; purely internal scaffolding.

---

**File 2: `crates/vb_validate/src/gate_08_accessor/tests.rs`**
```rust
fn workflow_parts_with_accessors(
    slot_count: u16,
    symbols_count: u32,
    accessors: Box<[AccessorProgram]>,
) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]),
        expressions: Box::new([]),
        accessors,
        constants: Box::new([]),
        input_slots: Box::new([]),
        slot_count,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}
```
Rationale: Accessor path validation tests; no external input schema.

---

**File 3: `crates/vb_validate/src/red_phase_proptest.rs`**
```rust
fn arb_parts(
    slot_count: u16,
    symbols_count: u32,
    accessors: Vec<AccessorProgram>,
) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("prop"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: Box::new([CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
        }]),
        expressions: Box::new([]),
        accessors: accessors.into_boxed_slice(),
        constants: Box::new([]),
        input_slots: Box::new([]),
        slot_count,
        symbols_count,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}
```
Rationale: Property-based tests for gate_08 symbol bounds; internal only.

---

**File 4: `crates/vb_validate/src/gate_tests.rs`**
```rust
fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        input_slots: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        step_names: Box::new([]),
        resource_contract: ResourceContract::DEFAULT,
    }
}
```
Rationale: Multi-gate tests (stack depth, node validation); internal scaffolding.

---

**File 5: `crates/vb_validate/src/test_helpers.rs`**
```rust
pub(crate) fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts {
    WorkflowParts {
        name: Box::from("test"),
        digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([]),
        input_slots: Box::new([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    }
}
```
Rationale: Shared helper used across gate test modules; internal only.

---

**Verification:** Run `cargo test -p vb_validate` — all five test suites should compile and pass. No assertions or fixture targets altered.
Coordinator constraint: retain actual existing function bodies verbatim; illustrative reconstructed bodies above are not implementation. Apply only five missing empty input schema fields at rustc initializer byte spans. No assertions or invalid-fixture targets may change.

Implementation provenance: initial model offsets pointed before the initializer opening brace and were rejected. A fresh 5090 response supplied corrected offsets after the brace. Exactly five field insertions were mechanically applied; no other Rust lines changed.
