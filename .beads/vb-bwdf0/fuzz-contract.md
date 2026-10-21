# Bead vb-bwdf0: Fuzz Fixture Input-Slot Schema Repair Contract

## Objective
Repair eight stale `WorkflowParts` constructors in the `velvet-ballistics-fuzz` crate that omit the newly required `input_slots` field, blocking `fuzz-tsan` and `fuzz-proptest` compilation in the integrated CI log (`.beads/vb-xx331/evidence/current-integrated-ci/moon-ci.log` lines 2829-2875 and 3994-4040). Add only the missing field with the canonical empty-slice value `Box::from([])`; do not alter node counts, slot counts, assertions, or production code.

## Domain
- **Product:** `velvet-ballistics` (fuzz infrastructure)
- **Crate:** `velvet-ballistics-fuzz` (`fuzz/`)
- **Language:** Rust 2024 edition, `velvet-ballistics/v1` workflow ABI
- **Schema version:** post-`input_slots` addition to `WorkflowParts` (`crates/vb_core/src/workflow/mod.rs`)

## Ubiquitous Language
| Term | Definition |
|------|------------|
| `WorkflowParts` | Compiled workflow serialization parts struct with 13 fields including the mandatory `input_slots: Box<[CompiledInputSlot]>` |
| `input_slots` | Bounded box-slice of input slot declarations; empty `Box::from([])` is valid for workflows with no declared runtime inputs |
| `CompiledInputSlot` | Input slot declaration with slot index and kind (defined in `crates/vb_core/src/workflow/compiled_input_slot.rs`) |
| Fixture constructor | A `WorkflowParts` literal in fuzz test source that synthesizes workflow shape for targeted property testing |
| Fuzz target | A libfuzzer/proptest harness binary in `fuzz/fuzz_targets/` that drives arbitrary-byte mutation through a specific subsystem |
| Stale fixture | A fixture constructor written before the `input_slots` schema change, now failing compilation with `E0063: missing field input_slots` |

## Affected Fixtures (8)
1. `fuzz/src/validation_target/capability.rs:81` — `fuzz_parts_with_actions` returns a WorkflowParts missing `input_slots`
2. `fuzz/src/validation_target/verifier.rs:198` — `fuzz_gates` constructs parts missing `input_slots`
3. `fuzz/src/journal_target/admission.rs:98` — `workflow_parts` helper omits `input_slots`
4. `fuzz/src/journal_target/readback.rs:108` — `one_node_parts` helper omits `input_slots`
5. `fuzz/src/expression_target.rs:63` — expression bytecode harness omits `input_slots`
6. `fuzz/src/expression_target.rs:144` — taint harness omits `input_slots`
7. `fuzz/src/workflow_target/accessor.rs:19` — accessor traversal harness omits `input_slots`
8. `fuzz/src/workflow_target/budget.rs:48` — step budget harness omits `input_slots`

## Non-Goals
- Do not add or remove fuzz targets, properties, or assertions.
- Do not change node counts, slot counts, step names, digests, or any field other than the missing `input_slots`.
- Do not modify production code in `crates/`.
- Do not introduce `#[allow]` attributes, `unwrap`, `expect`, `panic`, or `dbg`.
- Do not run full `moon ci`; bounded compilation and targeted tests only.
- Do not push or close the bead.

## Type-Level Invariants
```rust
// WorkflowParts requires 13 fields; input_slots is mandatory post-schema-change.
// Empty slice is valid for workflows with no runtime input ports.
let parts = WorkflowParts {
    name: Box::from("fuzz_target"),
    digest: WorkflowDigest::from_bytes([0; 32]),
    nodes: Box::from([node]),
    expressions: Box::from([]),
    accessors: Box::from([]),
    constants: Box::from([]),
    slot_count: 1,
    symbols_count: 0,
    entry: StepIdx::ZERO,
    resource_contract: ResourceContract::DEFAULT,
    step_names: Box::from([]),
    input_slots: Box::from([]),  // <-- required empty slice
};
```

## Verification Scope
- `cargo check --lib` for the `velvet-ballistics-fuzz` package (bounded, no bins)
- Compilation success with exactly 0 `E0063` errors
- No new warnings or clippy violations in affected source files

## Acceptance Criteria
- All 8 WorkflowParts constructors compile with the required `input_slots` field present.
- `cargo check --lib -p velvet-ballistics-fuzz` exits 0.
- No production source changes; only test fixture literals updated.
- Full raw compiler output preserved as evidence with genuine exit status.
