# Codebase Map for vb-3ksi

## Bug Summary
Proptest `proptest_gate_08_reports_first_invalid_accessor_with_root_precedence` fails with minimal input `slot_count=2, root=0`. Function `validate_gate_08_accessor_path_segments` wrongly returns `Err(AccessorPathInvalid)` when it should return `Ok(())` since `root < slot_count`. Issue concerns "root precedence" logic in accessor validation.

---

## Relevant File Tree

```
crates/vb_validate/
├── src/
│   ├── lib.rs                           # ValidationError enum, crate root
│   ├── gates.rs                         # LIVE aggregate gate_08 function (line 148)
│   ├── gate_08_accessor.rs              # FOCUSED gate_08 module (test-only, line 10)
│   ├── gate_tests.rs                    # Integration tests importing focused gate_08
│   ├── shared.rs                        # ValidationConfig pipeline calling gates::validate_gate_08
│   ├── diagnostic.rs                    # ValidationError -> DiagnosticCode mapping
│   ├── diag_codes.rs                    # Error code constants
│   ├── diag_convert.rs                  # Error -> diagnostic conversion
│   └── diag_render.rs                   # Error rendering
└── tests/
    └── gate_08_accessor_parity.rs       # Parity tests: gates:: vs core workflow validation

crates/vb_core/
└── src/
    ├── workflow/mod.rs                  # WorkflowParts, AccessorProgram, PathSegment types
    ├── ids.rs                           # SlotIdx, SymbolId, StepIdx definitions
    └── compiled_workflow.rs             # CompiledWorkflow::try_from_parts (core validation)
```

---

## Function Signatures and Call Graph

### Gate 8: Accessor Path Validation

```rust
// crates/vb_validate/src/gates.rs:148  (LIVE / AGGREGATE)
pub fn validate_gate_08_accessor_path_segments(parts: &WorkflowParts) -> ValidationResult<()>

// crates/vb_validate/src/gate_08_accessor.rs:10  (FOCUSED / TEST-ONLY MODULE)
pub fn validate_gate_08_accessor_path_segments(parts: &WorkflowParts) -> ValidationResult<()>
```

Both implementations are **identical** in current source. Each delegates to:

```rust
fn validate_accessor_root(
    acc_index: usize,
    accessor: &AccessorProgram,
    slot_count: u16,
) -> ValidationResult<()>
// Returns Err(ValidationError::AccessorSlotOutOfRange) if root >= slot_count

fn validate_field_symbol(
    acc_index: usize,
    seg_index: usize,
    symbol: SymbolId,
    symbols_count: u32,
) -> ValidationResult<()>
// Returns Err(ValidationError::AccessorPathInvalid) if symbol >= symbols_count

fn validate_index_segment(
    acc_index: usize,
    seg_index: usize,
    idx: u32,
) -> ValidationResult<()>
// Returns Err(ValidationError::AccessorPathInvalid) if idx == u32::MAX
```

### Call Graph (Upstream)

```
velvet_ballastics / vb_compile
    |
    v
crates/vb_validate/src/shared.rs
    ValidationConfig::validate(parts)           [line 104]
    ValidationConfig::validate_with_contracts() [line 139]
        |
        +--> gates::validate_gate_08_accessor_path_segments(parts)  [line 109]
```

Re-export chain:
```
gate_08_accessor.rs  (focused, test-only)
    ^
    | pub use
    |
gates.rs             (aggregate, live)
    ^
    | pub use
    |
shared.rs            (pipeline entry)
```

---

## Error Types Involved

Defined in `crates/vb_validate/src/lib.rs`:

```rust
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValidationError {
    // Gate 8 errors
    #[error("ACCESSOR_SLOT_OUT_OF_RANGE: accessor {accessor_index}, slot {slot}, slot_count {slot_count}")]
    AccessorSlotOutOfRange {
        accessor_index: usize,
        slot: usize,
        slot_count: usize,
    },

    #[error("ACCESSOR_PATH_INVALID: accessor {accessor_index}, segment {segment_index}")]
    AccessorPathInvalid {
        accessor_index: usize,
        segment_index: usize,
    },
    // ... 40+ other variants
}

pub type ValidationResult<T> = Result<T, ValidationError>;
```

Diagnostic codes in `crates/vb_validate/src/diag_codes.rs`:
- `CODE_ACCESSOR_SLOT_OUT_OF_RANGE`
- `CODE_ACCESSOR_PATH_INVALID`

---

## Core Types

### `WorkflowParts` (`crates/vb_core/src/workflow/mod.rs:249`)
```rust
pub struct WorkflowParts {
    pub name: Box<str>,
    pub digest: WorkflowDigest,
    pub nodes: Box<[CompiledNode]>,
    pub expressions: Box<[ExprProgram]>,
    pub accessors: Box<[AccessorProgram]>,
    pub constants: Box<[ConstValue]>,
    pub slot_count: u16,
    pub symbols_count: u32,
    pub entry: StepIdx,
    pub resource_contract: ResourceContract,
    pub step_names: Box<[Box<str>]>,
}
```

### `AccessorProgram` (`crates/vb_core/src/workflow/mod.rs:276`)
```rust
pub struct AccessorProgram {
    pub root: SlotIdx,
    pub path: Box<[PathSegment]>,
}
```

### `PathSegment` (`crates/vb_core/src/workflow/mod.rs:285`)
```rust
pub enum PathSegment {
    Field(SymbolId),
    Index(u32),
}
```

### `SlotIdx` (`crates/vb_core/src/ids.rs`)
```rust
pub struct SlotIdx(u16);
impl SlotIdx {
    pub const fn new(value: u16) -> Self;
    pub fn as_usize(self) -> usize;   // usize::from(self.0)
    pub const fn get(self) -> u16;
}
```

---

## Test File Locations

### Focused Tests (import `crate::gate_08_accessor::validate_gate_08_accessor_path_segments`)
- `crates/vb_validate/src/gate_08_accessor.rs:164` -- unit tests inside the focused module
- Proptest block at `gate_08_accessor.rs:399`
- **Failing proptest**: `proptest_gate_08_reports_first_invalid_accessor_with_root_precedence` at `gate_08_accessor.rs:473`

### Aggregate Tests (use `gates::validate_gate_08_accessor_path_segments`)
- `crates/vb_validate/src/gates.rs:1449` -- inside aggregate `gates.rs` test module
- `crates/vb_validate/src/gates.rs:2136` -- adversarial edge-case tests

### Integration Tests
- `crates/vb_validate/src/gate_tests.rs:179` -- imports focused version, tests against `make_parts()` fixtures

### Parity Tests (external test binary)
- `crates/vb_validate/tests/gate_08_accessor_parity.rs` -- tests `gates::` version against `CompiledWorkflow::try_from_parts`

---

## Build / Test Commands

### Run the specific failing proptest
```bash
cargo test -p vb_validate proptest_gate_08_reports_first_invalid_accessor_with_root_precedence -- --nocapture
```

### Run all gate_08 tests
```bash
cargo test -p vb_validate gate_08 -- --nocapture
```

### Run gate_08 parity tests
```bash
cargo test -p vb_validate --test gate_08_accessor_parity
```

### Moon CI target
```bash
moon run :test
```
- Internally runs: `cargo nextest run --workspace --all-features` (10m timeout)

---

## Key Observation

There are **two copies** of `validate_gate_08_accessor_path_segments`:
1. `gate_08_accessor.rs:10` -- focused module, `#[cfg(test)]`, directly tested by proptests
2. `gates.rs:148` -- aggregate module, the live version used by `shared.rs`

The proptest in `gate_08_accessor.rs` tests the **focused** copy. The parity test in `tests/gate_08_accessor_parity.rs` tests the **aggregate** copy. Any fix must be applied to **both** copies or the focused module must be promoted to become the single source of truth.
