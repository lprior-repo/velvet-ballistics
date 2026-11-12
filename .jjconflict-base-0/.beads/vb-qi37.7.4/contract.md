# Contract: vb-qi37.7.4 - Validate accessor path segments structurally

## Scope

This bead specifies Gate 8 validation for accessor path segment structure in `vb_validate`.

In scope:
- `crates/vb_validate/src/gate_08_accessor.rs::validate_gate_08_accessor_path_segments`
- Any active duplicate/aggregate Gate 8 entry point in `crates/vb_validate/src/gates.rs`
- Behavioral parity with `crates/vb_core/src/workflow/mod.rs::validate_accessor_paths`
- Validation that each `PathSegment::Field(SymbolId)` refers to a declared symbol: `symbol.get() < parts.symbols_count`
- Preservation of existing Gate 8 root and reserved-index checks

Out of scope:
- Runtime accessor execution semantics
- Symbol interning table construction
- YAML, JSON, HTTP, parser, or serializer behavior
- New performance claims or benchmark changes
- Production code or test implementation in this State 3 artifact

## Domain terms

- `WorkflowParts`: untrusted compiled workflow IR accepted by validation gates.
- `AccessorProgram`: a root slot plus a bounded path used to select nested values.
- `PathSegment::Field(SymbolId)`: object-field traversal by interned symbol identifier.
- `PathSegment::Index(u32)`: list-index traversal.
- `symbols_count`: declared upper bound for valid interned symbol identifiers.
- `slot_count`: declared upper bound for valid runtime slot roots.
- `Gate 8`: cold-path structural validation of accessor root and path segment shape.

## Preconditions

- Input is an immutable borrowed `&WorkflowParts`.
- `WorkflowParts` may be adversarial; validation must not assume any field was previously checked.
- `parts.accessors`, each accessor `path`, `parts.slot_count`, and `parts.symbols_count` are the only data required for this bead's Gate 8 decision.
- Symbol comparison must use the repository-safe accessor for `SymbolId` (`get()` as seen in `vb_core::workflow::validate_symbol`) rather than unchecked casts or tuple-field access.
- Validation must be bounded by total accessor path segment count and must not allocate unbounded memory.

## Postconditions

- Returns `Ok(())` only if all accessor roots and all path segments satisfy Gate 8 invariants.
- Returns `Err(ValidationError::AccessorSlotOutOfRange { accessor_index, slot, slot_count })` when the first invalid accessor root has `root.as_usize() >= usize::from(parts.slot_count)`.
- Returns `Err(ValidationError::AccessorPathInvalid { accessor_index, segment_index })` when the first invalid path segment is found, including:
  - `PathSegment::Field(symbol)` where `symbol.get() >= parts.symbols_count`
  - `PathSegment::Index(u32::MAX)`
- Error indices must identify the accessor and path segment that caused rejection.
- Validation order remains deterministic: accessors are checked in slice order, root before path segments, and segments in path order.
- No mutation of `WorkflowParts` or any global state occurs.

## Invariants

- Every accepted accessor root satisfies `accessor.root.as_usize() < usize::from(parts.slot_count)`.
- Every accepted field segment satisfies `symbol.get() < parts.symbols_count`.
- If `parts.symbols_count == 0`, no `PathSegment::Field(_)` is valid.
- If `parts.symbols_count > 0`, `PathSegment::Field(SymbolId::new(parts.symbols_count - 1))` is valid.
- `PathSegment::Field(SymbolId::new(parts.symbols_count))` is invalid.
- `PathSegment::Field(SymbolId::new(parts.symbols_count + 1))` is invalid when constructible without overflow.
- `PathSegment::Index(u32::MAX)` is always invalid because it is a reserved sentinel.
- `PathSegment::Index(n)` where `n < u32::MAX` remains structurally valid for Gate 8.
- Empty accessor slices and empty paths remain valid when all roots are valid.
- `gate_08_accessor.rs`, active Gate 8 logic in `gates.rs`, and `vb_core::workflow::validate_accessor_paths` must agree on field-symbol validity.

## Typed error taxonomy

Preferred existing variants:

```rust
pub type ValidationResult<T> = Result<T, ValidationError>;

pub enum ValidationError {
    AccessorSlotOutOfRange {
        accessor_index: usize,
        slot: usize,
        slot_count: usize,
    },
    AccessorPathInvalid {
        accessor_index: usize,
        segment_index: usize,
    },
}
```

Error mapping:
- Invalid root slot: `AccessorSlotOutOfRange`.
- Invalid field symbol id: `AccessorPathInvalid` unless maintainers require a more specific diagnostic.
- Reserved index sentinel: `AccessorPathInvalid`.

Constraint on new errors:
- Do not add a new error variant unless existing diagnostics cannot support acceptance tests with accessor and segment location.
- If a new variant is introduced later, it must remain railway-oriented (`Result<T, ValidationError>`) and diagnostics in `diagnostic.rs`, `diag_render.rs`, and `diag_convert.rs` must be updated in the same change.

## Contract signatures

Existing public contract to preserve:

```rust
pub fn validate_gate_08_accessor_path_segments(parts: &WorkflowParts) -> ValidationResult<()>;
```

Recommended internal helper contract if implementation is factored:

```rust
fn validate_accessor_path_segment(
    accessor_index: usize,
    segment_index: usize,
    segment: &PathSegment,
    symbols_count: u32,
) -> ValidationResult<()>;
```

Helper preconditions:
- `accessor_index` and `segment_index` are caller-provided diagnostic coordinates from enumeration.
- `symbols_count` is copied from `parts.symbols_count`.

Helper postconditions:
- Returns `Ok(())` only for valid field/index segment structure.
- Returns `AccessorPathInvalid` with the supplied coordinates for invalid fields or sentinel index.

## Acceptance criteria

- `gate_08_accessor.rs` rejects `PathSegment::Field(symbol)` when `symbol.get() >= parts.symbols_count`.
- `gates.rs` rejects the same invalid field segments if its Gate 8 function remains active or exported.
- Existing root bounds behavior is unchanged.
- Existing `u32::MAX` index rejection is unchanged.
- Valid boundary field `symbols_count - 1` passes when `symbols_count > 0`.
- `symbols_count == 0` rejects every field segment.
- Tests prove parity between focused Gate 8 and aggregate Gate 8 entry points where both exist.
- `moon ci` remains the canonical final quality gate for implementation State; contract State does not run or require implementation gates.
- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` are introduced by downstream implementation.
- No runtime JSON/YAML/HTTP core behavior is introduced by downstream implementation.

## Martin Fowler Given/When/Then scenarios

### Scenario: valid first symbol is accepted
Given a `WorkflowParts` value with `symbols_count = 1`, `slot_count = 1`, and one accessor rooted at slot `0`
And the accessor path contains `PathSegment::Field(SymbolId::new(0))`
When Gate 8 validates accessor path segments
Then validation returns `Ok(())`.

### Scenario: valid upper-bound symbol is accepted
Given `symbols_count = N` where `N > 0`
And an accessor path contains `PathSegment::Field(SymbolId::new(N - 1))`
When Gate 8 validates accessor path segments
Then validation returns `Ok(())`.

### Scenario: field equal to symbols_count is rejected
Given `symbols_count = N`
And an accessor path contains `PathSegment::Field(SymbolId::new(N))`
When Gate 8 validates accessor path segments
Then validation returns `Err(ValidationError::AccessorPathInvalid { accessor_index, segment_index })`
And the error coordinates identify that field segment.

### Scenario: field above symbols_count is rejected
Given `symbols_count = N`
And an accessor path contains `PathSegment::Field(SymbolId::new(N + 1))` where this value is constructible
When Gate 8 validates accessor path segments
Then validation returns `Err(ValidationError::AccessorPathInvalid { accessor_index, segment_index })`.

### Scenario: zero symbols rejects fields
Given `symbols_count = 0`
And an accessor path contains any `PathSegment::Field(_)`
When Gate 8 validates accessor path segments
Then validation returns `Err(ValidationError::AccessorPathInvalid { accessor_index, segment_index })`
And no subtraction underflow is required to make the decision.

### Scenario: non-sentinel index remains accepted
Given a valid accessor root
And an accessor path contains `PathSegment::Index(0)` or another `n < u32::MAX`
When Gate 8 validates accessor path segments
Then validation returns `Ok(())` for that segment.

### Scenario: sentinel index remains rejected
Given a valid accessor root
And an accessor path contains `PathSegment::Index(u32::MAX)`
When Gate 8 validates accessor path segments
Then validation returns `Err(ValidationError::AccessorPathInvalid { accessor_index, segment_index })`.

### Scenario: root out of range is still rejected before path checks
Given `slot_count = 1`
And an accessor has root `SlotIdx::new(5)` and any path
When Gate 8 validates accessor path segments
Then validation returns `Err(ValidationError::AccessorSlotOutOfRange { accessor_index, slot: 5, slot_count: 1 })`
And path segment validation for that accessor is not required to run.

### Scenario: duplicate Gate 8 implementations stay synchronized
Given the same `WorkflowParts` value with an invalid field segment
When both `gate_08_accessor::validate_gate_08_accessor_path_segments` and the active `gates::validate_gate_08_accessor_path_segments` entry point validate it
Then both return an error for invalid accessor path structure.

## Test plan names

Happy path:
- `gate_08_accepts_field_symbol_zero_when_symbols_count_is_one`
- `gate_08_accepts_field_symbol_at_symbols_count_minus_one`
- `gate_08_accepts_non_sentinel_index_segments`
- `gate_08_accepts_empty_accessor_paths`

Error path:
- `gate_08_rejects_field_symbol_equal_to_symbols_count`
- `gate_08_rejects_field_symbol_above_symbols_count`
- `gate_08_rejects_field_segment_when_symbols_count_is_zero`
- `gate_08_rejects_sentinel_index_segment`
- `gate_08_rejects_accessor_root_out_of_range`

Contract/parity:
- `gate_08_reports_invalid_field_segment_coordinates`
- `gate_08_checks_root_before_path_segments`
- `gate_08_aggregate_entry_point_matches_focused_entry_point_for_field_bounds`
- `gate_08_matches_core_workflow_accessor_path_symbol_bounds`

## Proof obligations for implementation State

- Show the field-symbol check uses `symbol.get() < parts.symbols_count` or an equivalent safe repository API.
- Show no unchecked arithmetic is needed for `symbols_count == 0`.
- Show all Gate 8 entry points that can be called by production validation are updated or explicitly delegate to one implementation.
- Show existing diagnostics remain compatible, or document and update all diagnostic conversions if a new error is introduced.
- Show bounded-resource behavior: single pass over `parts.accessors` and each path; no recursion; no unbounded allocation.
- Show all new and existing Gate 8 tests compile and run under the repository's normal gates, with `moon ci` as final canonical CI.

## Risk notes

- Drift risk: `gate_08_accessor.rs` and `gates.rs` contain separate Gate 8 logic; updating only one leaves inconsistent behavior.
- Diagnostic risk: reusing `AccessorPathInvalid` is least disruptive but less specific than a dedicated symbol-out-of-range error.
- Boundary risk: tests that compute `symbols_count - 1` must guard `symbols_count > 0`.
- Conversion risk: direct casts or tuple-field access on `SymbolId` may violate local patterns; use `get()`.
- Compatibility risk: existing tests currently accept `SymbolId::new(1)` while helper fixtures default `symbols_count = 0`; downstream implementation must update fixtures to declare sufficient symbols for valid field tests.
