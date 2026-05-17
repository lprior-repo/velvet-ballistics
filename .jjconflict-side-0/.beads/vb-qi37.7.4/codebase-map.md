# Codebase Map: vb-qi37.7.4

## Bead

- Title: `ir: Validate accessor path segments structurally`
- Pipeline state: State 2 artifact retry for `go-skill`
- Scope: investigation artifact only; no production code, tests, or bead database writes performed.

## Relevant Files

- `crates/vb_validate/src/gate_08_accessor.rs`
- Primary Gate 8 validation implementation.
- Existing behavior checks accessor root bounds and rejects `PathSegment::Index(u32::MAX)`.
- Existing gap: `PathSegment::Field(SymbolId)` is accepted without checking the field symbol id is `< symbols_count`.

- `crates/vb_validate/src/gates.rs`
- Duplicate or aggregate Gate 8 logic, reported around lines 139-163.
- Must remain behaviorally synchronized with `gate_08_accessor.rs` if both implementations are still active.
- Existing gap mirrors primary file: field path segments need structural symbol bounds validation.

- `crates/vb_core/src/workflow/mod.rs`
- Reuse pattern: `validate_accessor_paths` already enforces the desired structure.
- Known checks to mirror: maximum path depth, field symbol `< symbols_count`, reject sentinel index `u32::MAX`.
- This is the best local reference for semantics and diagnostic intent.

- `crates/vb_validate/src/gate_tests.rs`
- Validation gate tests likely covering aggregate gate behavior.
- Add or extend cases for invalid `PathSegment::Field(SymbolId)` where the symbol id is out of bounds.

- `crates/vb_validate/src/gate_08_accessor.rs`
- Also contains or neighbors unit tests for focused Gate 8 behavior.
- Add focused tests for valid field symbols, boundary valid symbol `symbols_count - 1`, invalid symbol `symbols_count`, and invalid symbol above bounds.

- `crates/vb_core/src/workflow/tests.rs`
- Existing workflow tests can confirm parity with the core validation pattern.
- Use as reference only unless contract work finds an actual cross-crate regression gap.

## Patterns To Reuse

- Reuse the `validate_accessor_paths` semantics from `crates/vb_core/src/workflow/mod.rs` rather than inventing new validation rules.
- Preserve existing Gate 8 rejection for `PathSegment::Index(u32::MAX)`.
- Preserve existing accessor root bounds behavior.
- Keep validation structural: field path segment ids must refer to declared symbols, so `field.0 < symbols_count` or the equivalent accessor for `SymbolId` should hold.
- Prefer existing `ValidationError::AccessorPathInvalid` if it already represents structurally invalid accessor paths cleanly.
- Add a more specific diagnostic only if current error shape cannot identify the segment/root/path enough for contract tests.

## Suspected Touchpoints

- Add field symbol bounds check in `crates/vb_validate/src/gate_08_accessor.rs` within the path segment validation loop or helper.
- Add the same check in `crates/vb_validate/src/gates.rs` unless later investigation proves it delegates to the primary Gate 8 implementation.
- Ensure both paths use the same `symbols_count` source used by root bounds validation.
- If `SymbolId` is a newtype, use its existing safe accessor/conversion pattern; avoid unchecked casts or direct field access if the crate has a preferred API.
- Extend tests in `crates/vb_validate/src/gate_tests.rs` and/or focused tests in `gate_08_accessor.rs`.

## Test Locations

- `crates/vb_validate/src/gate_tests.rs`
- Aggregate validation behavior, including failure propagation from Gate 8.

- `crates/vb_validate/src/gate_08_accessor.rs`
- Focused Gate 8 unit coverage if tests are colocated there.

- `crates/vb_core/src/workflow/tests.rs`
- Reference expectations for workflow-level accessor path validation.

## Suggested Test Cases

- Valid accessor path with `PathSegment::Field(SymbolId(0))` when `symbols_count > 0` passes.
- Valid accessor path with `PathSegment::Field(SymbolId(symbols_count - 1))` passes.
- Invalid accessor path with `PathSegment::Field(SymbolId(symbols_count))` fails.
- Invalid accessor path with `PathSegment::Field(SymbolId(symbols_count + 1))` fails.
- Existing invalid `PathSegment::Index(u32::MAX)` failure still fails.
- Existing root out-of-bounds failure still fails.
- If max depth belongs to Gate 8 parity, verify it is still enforced or explicitly out of scope.

## Risks And Dependencies

- Risk: two Gate 8 implementations drift if only one file is updated.
- Risk: choosing a new error variant may break callers or snapshot-style assertions; prefer existing invalid-accessor diagnostic unless specificity is required.
- Risk: `SymbolId` conversion may tempt unchecked casts; follow existing crate-safe conversion/accessor patterns.
- Risk: `symbols_count == 0` must reject all field segments without underflow in tests or implementation.
- Dependency: confirm whether `gates.rs` is still active production code or a legacy aggregate path before changing only one implementation.
- Dependency: confirm exact `SymbolId` construction/access API before writing contract tests.

## Next-State Notes For rust-contract

- Contract invariant: every `PathSegment::Field(symbol)` in every accessor path must satisfy `symbol < symbols_count`.
- Contract invariant: every accessor root remains within declared accessor/root bounds according to current Gate 8 rules.
- Contract invariant: `PathSegment::Index(u32::MAX)` remains a reserved sentinel and is always invalid.
- Contract boundary case: `symbols_count == 0` means no field segment is valid.
- Contract boundary case: `symbol == symbols_count - 1` is valid when `symbols_count > 0`.
- Contract boundary case: `symbol == symbols_count` is invalid.
- Contract parity requirement: `gate_08_accessor.rs`, `gates.rs`, and `vb_core::workflow::validate_accessor_paths` must agree on field segment validity.
- Acceptance proof should include focused Gate 8 tests plus aggregate validation tests if both entry points exist.
