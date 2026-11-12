# Test Plan: vb-qi37.7.4 — Validate accessor path segments structurally

## Review Repair Declaration

This repaired plan explicitly addresses every finding in `test-plan-review.md`:

1. Adds exact root boundary scenarios for `root == slot_count` rejection and `root == slot_count - 1` acceptance.
2. Rewrites focused/aggregate/core parity scenarios so each implementation asserts concrete `Ok(())` or concrete `Err(ValidationError::AccessorPathInvalid { accessor_index, segment_index })`; no equivalence-only oracle remains.
3. Adds Holzmann test-structure rules: no `for`, `while`, or `loop` in test bodies; use `rstest` cases or separate named tests; no shared mutable fixtures; side-effectful helpers must disclose effects in their names.
4. Adds root off-by-one mutation targets for `root < slot_count` changed to `root <= slot_count` and mutations that reject `slot_count - 1`.
5. Adds overflow-safe construction rules for every `symbols_count + 1` fixture, including parity fixtures.
6. Adds precise typed error expectations, negative paths, integration/command oracles, static panic/resource checks, and explicit test-density requirements.

## Summary

- Behaviors identified: 16.
- Planned named scenario tests: minimum 18 concrete scenario cases for 1 public API entry point; density requirement is at least 5 named behavior tests per public function and at least 1 negative-path test per error variant.
- Trophy allocation: 9 unit / 7 integration / 1 e2e-acceptance command gate / static gates mandatory.
- Proptest invariants: 5.
- Fuzz targets: 0 in bead scope; no parser/deserializer/user-text boundary is changed.
- Kani harnesses: 4.
- Mutation threshold: `cargo-mutants` or repository-approved equivalent must kill at least 90% of mutations in touched Gate 8 validation code.
- Assertion rule: no planned test may assert only `is_ok()` or `is_err()`; every assertion must compare to exact `Ok(())` or exact `ValidationError` variant with all coordinates/values.

## 1. Behavior Inventory

1. Gate 8 accepts a field segment when the symbol id is zero and `symbols_count` is one.
2. Gate 8 accepts a field segment when the symbol id equals `symbols_count - 1` and `symbols_count > 0`.
3. Gate 8 rejects a field segment with `AccessorPathInvalid` when the symbol id equals `symbols_count`.
4. Gate 8 rejects a field segment with `AccessorPathInvalid` when the symbol id is greater than `symbols_count` and the fixture is constructed without overflow.
5. Gate 8 rejects every field segment with `AccessorPathInvalid` when `symbols_count == 0`.
6. Gate 8 accepts an index segment when the index value is less than `u32::MAX`.
7. Gate 8 rejects an index segment with `AccessorPathInvalid` when the index value is `u32::MAX`.
8. Gate 8 accepts an empty accessor path when the accessor root is valid.
9. Gate 8 accepts an empty accessor collection when no accessor can violate root or segment invariants.
10. Gate 8 accepts an accessor root when `root.as_usize() == usize::from(slot_count) - 1` and `slot_count > 0`.
11. Gate 8 rejects an accessor root with `AccessorSlotOutOfRange` when `root.as_usize() == usize::from(slot_count)`.
12. Gate 8 rejects an accessor root with `AccessorSlotOutOfRange` when `root.as_usize() > usize::from(slot_count)`.
13. Gate 8 reports the first invalid accessor and segment coordinates when multiple accessors or segments exist.
14. Gate 8 checks accessor root before path segments when the same accessor contains both an invalid root and invalid path segment.
15. Focused Gate 8 and active aggregate Gate 8 produce concrete expected success/error values for valid and invalid field-symbol boundaries.
16. `vb_validate` Gate 8 and `vb_core::workflow::validate_accessor_paths` agree on concrete valid and invalid field-symbol boundary outcomes.

## 2. Trophy Allocation

| Behavior | Layer | Tool | Rationale |
|---|---|---|---|
| 1 | Unit | `#[test]` or `rstest` case near Gate 8 | Direct pure boundary behavior. |
| 2 | Unit | `#[test]` or `rstest` case | Kills off-by-one field-bound mutations. |
| 3 | Unit | `#[test]` | Exact `AccessorPathInvalid` equal-boundary negative path. |
| 4 | Unit | `#[test]` | Distinct above-bound negative path with overflow-safe fixture construction. |
| 5 | Unit | `#[test]` | Zero-symbol boundary must not underflow. |
| 6 | Unit | `#[test]`/`rstest` cases | Preserves non-sentinel index behavior at `0` and `u32::MAX - 1`. |
| 7 | Unit | `#[test]` | Preserves reserved sentinel rejection. |
| 10 | Unit | `#[test]` | Exact maximum-valid root boundary. |
| 11 | Unit | `#[test]` | Exact one-past-end root boundary. |
| 8-9 | Integration | existing public gate/aggregate validation tests | Real `WorkflowParts` fixtures through public validation surfaces. |
| 12-14 | Integration | multi-accessor fixture through public validation | Ordering and diagnostics emerge from real iteration and public errors. |
| 15 | Integration | focused Gate 8 + active aggregate Gate 8 | Detects duplicate implementation drift; assertions are concrete, not equality-only. |
| 16 | Integration | `vb_validate` + `vb_core::workflow::validate_accessor_paths` | Contract requires behavioral parity with core workflow validation. |
| All | Static | `moon ci`, clippy, policy scripts, source scans | Enforces no `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, forbidden unchecked arithmetic/casts, or forbidden nightly features. |
| All | E2E/Acceptance | non-interactive `moon ci` from workspace root | Black-box command oracle that the workspace compiles, tests, lints, and validates policies after implementation. |

## 3. BDD Scenarios

### Behavior: Gate 8 accepts field symbol zero when symbols count is one

- Test function name: `fn gate_08_accepts_field_symbol_zero_when_symbols_count_is_one()`
- Given: `WorkflowParts { symbols_count: 1, slot_count: 1, accessors: [root slot 0, path [PathSegment::Field(SymbolId::new(0))]] }`.
- When: `validate_gate_08_accessor_path_segments(&parts)` is called.
- Then: result is exactly `Ok(())`.

### Behavior: Gate 8 accepts upper-bound valid field symbol

- Test function name: `fn gate_08_accepts_field_symbol_at_symbols_count_minus_one()`
- Given: `symbols_count = N`, `N > 0`, `slot_count = 1`, root slot `0`, and path `[PathSegment::Field(SymbolId::new(N - 1))]`.
- When: focused Gate 8 validates accessor path segments.
- Then: result is exactly `Ok(())`.
- Test structure: implement `N = 1`, `N = 2`, and one larger in-range fixture as `rstest` cases or separate named tests; do not use loops in the test body.

### Behavior: Gate 8 rejects field symbol equal to symbols count

- Test function name: `fn gate_08_rejects_field_symbol_equal_to_symbols_count()`
- Given: `symbols_count = N`, `slot_count = 1`, accessor index `0`, segment index `0`, root slot `0`, and path `[PathSegment::Field(SymbolId::new(N))]`.
- When: focused Gate 8 validates accessor path segments.
- Then: result is exactly `Err(ValidationError::AccessorPathInvalid { accessor_index: 0, segment_index: 0 })`.

### Behavior: Gate 8 rejects field symbol above symbols count

- Test function name: `fn gate_08_rejects_field_symbol_above_symbols_count()`
- Given: `symbols_count = N`, `N < u32::MAX`, `above = N.checked_add(1)` succeeds, `slot_count = 1`, accessor index `0`, segment index `0`, root slot `0`, and path `[PathSegment::Field(SymbolId::new(above))]`.
- When: focused Gate 8 validates accessor path segments.
- Then: result is exactly `Err(ValidationError::AccessorPathInvalid { accessor_index: 0, segment_index: 0 })`.

### Behavior: Gate 8 rejects field segments when symbols count is zero

- Test function name: `fn gate_08_rejects_field_segment_when_symbols_count_is_zero()`
- Given: `symbols_count = 0`, `slot_count = 1`, accessor index `0`, segment index `0`, root slot `0`, and path `[PathSegment::Field(SymbolId::new(0))]`.
- When: focused Gate 8 validates accessor path segments.
- Then: result is exactly `Err(ValidationError::AccessorPathInvalid { accessor_index: 0, segment_index: 0 })`.
- And: fixture construction must not compute `symbols_count - 1`.

### Behavior: Gate 8 accepts non-sentinel index segments

- Test function names:
  - `fn gate_08_accepts_index_zero()`
  - `fn gate_08_accepts_index_u32_max_minus_one()`
- Given: a valid accessor root and one path segment, either `[PathSegment::Index(0)]` or `[PathSegment::Index(u32::MAX - 1)]`.
- When: focused Gate 8 validates accessor path segments.
- Then: each case result is exactly `Ok(())`.
- Test structure: use separate named tests or `rstest` cases; no `for`, `while`, or `loop` in the test body.

### Behavior: Gate 8 rejects sentinel index segment

- Test function name: `fn gate_08_rejects_sentinel_index_segment()`
- Given: valid root, accessor index `0`, segment index `0`, and path `[PathSegment::Index(u32::MAX)]`.
- When: focused Gate 8 validates accessor path segments.
- Then: result is exactly `Err(ValidationError::AccessorPathInvalid { accessor_index: 0, segment_index: 0 })`.

### Behavior: Gate 8 accepts empty accessor paths

- Test function name: `fn gate_08_accepts_empty_accessor_paths()`
- Given: `slot_count = 1`, one accessor rooted at slot `0`, and path `[]`.
- When: focused Gate 8 validates accessor path segments.
- Then: result is exactly `Ok(())`.

### Behavior: Gate 8 accepts empty accessor collection

- Test function name: `fn gate_08_accepts_empty_accessor_collection()`
- Given: `accessors = []`, any valid `slot_count`, and any `symbols_count`.
- When: public aggregate validation reaches Gate 8.
- Then: result is exactly `Ok(())` for the Gate 8 decision.

### Behavior: Gate 8 accepts maximum valid accessor root

- Test function name: `fn gate_08_accepts_accessor_root_at_slot_count_minus_one()`
- Given: `slot_count = N`, `N > 0`, root slot `N - 1`, `symbols_count = 1`, and an empty path or path `[PathSegment::Field(SymbolId::new(0))]`.
- When: focused Gate 8 validates accessor path segments.
- Then: result is exactly `Ok(())`.

### Behavior: Gate 8 rejects accessor root equal to slot count

- Test function name: `fn gate_08_rejects_accessor_root_equal_to_slot_count()`
- Given: `slot_count = N`, root slot `N`, accessor index `0`, and an otherwise valid empty path.
- When: focused Gate 8 validates accessor path segments.
- Then: result is exactly `Err(ValidationError::AccessorSlotOutOfRange { accessor_index: 0, slot: N as usize, slot_count: N as usize })` using repository-safe conversions for the exact expected `usize` values.

### Behavior: Gate 8 rejects accessor root greater than slot count

- Test function name: `fn gate_08_rejects_accessor_root_greater_than_slot_count()`
- Given: `slot_count = 1`, root slot `5`, accessor index `0`, and an otherwise valid empty path.
- When: focused Gate 8 validates accessor path segments.
- Then: result is exactly `Err(ValidationError::AccessorSlotOutOfRange { accessor_index: 0, slot: 5, slot_count: 1 })`.

### Behavior: Gate 8 reports invalid field segment coordinates

- Test function name: `fn gate_08_reports_invalid_field_segment_coordinates()`
- Given: `symbols_count = 2`, at least two accessors where accessor `0` is valid and accessor `1` has path `[PathSegment::Index(0), PathSegment::Field(SymbolId::new(2))]`.
- When: focused Gate 8 validates accessor path segments.
- Then: result is exactly `Err(ValidationError::AccessorPathInvalid { accessor_index: 1, segment_index: 1 })`.

### Behavior: Gate 8 checks root before path segments

- Test function name: `fn gate_08_checks_root_before_path_segments()`
- Given: `slot_count = 1`, accessor index `0`, root slot `5`, `symbols_count = 1`, and path `[PathSegment::Field(SymbolId::new(1))]`, which is also path-invalid.
- When: focused Gate 8 validates accessor path segments.
- Then: result is exactly `Err(ValidationError::AccessorSlotOutOfRange { accessor_index: 0, slot: 5, slot_count: 1 })`.
- And: the assertion must reject `Err(ValidationError::AccessorPathInvalid { accessor_index: 0, segment_index: 0 })` for this mixed-invalid case by matching the full expected value.

### Behavior: focused and aggregate Gate 8 accept the same concrete valid field boundaries

- Test function names:
  - `fn focused_and_aggregate_gate_08_accept_field_zero_when_symbols_count_is_one()`
  - `fn focused_and_aggregate_gate_08_accept_field_at_symbols_count_minus_one()`
- Given: identical valid `WorkflowParts` fixtures for each implementation:
  - case A: `symbols_count = 1`, `Field(SymbolId::new(0))`, expected `Ok(())`.
  - case B: `symbols_count = N`, `N > 0`, `Field(SymbolId::new(N - 1))`, expected `Ok(())`.
- When: focused `gate_08_accessor::validate_gate_08_accessor_path_segments` and the active aggregate Gate 8/public validation entry point validate the fixture.
- Then: focused result is exactly `Ok(())`.
- And: aggregate/public result is exactly `Ok(())`.
- Oracle rule: equality between implementations may be an additional assertion, but it is not sufficient by itself.

### Behavior: focused and aggregate Gate 8 reject the same concrete invalid field boundaries

- Test function names:
  - `fn focused_and_aggregate_gate_08_reject_field_equal_to_symbols_count()`
  - `fn focused_and_aggregate_gate_08_reject_field_above_symbols_count()`
- Given: identical invalid `WorkflowParts` fixtures for each implementation:
  - case A: `symbols_count = N`, accessor index `0`, segment index `0`, `Field(SymbolId::new(N))`, expected `Err(ValidationError::AccessorPathInvalid { accessor_index: 0, segment_index: 0 })`.
  - case B: `symbols_count = N`, `N < u32::MAX`, `above = N.checked_add(1)` succeeds, accessor index `0`, segment index `0`, `Field(SymbolId::new(above))`, expected `Err(ValidationError::AccessorPathInvalid { accessor_index: 0, segment_index: 0 })`.
- When: focused Gate 8 and the active aggregate Gate 8/public validation entry point validate the fixture.
- Then: focused result is exactly the case's concrete expected `Err(ValidationError::AccessorPathInvalid { accessor_index: 0, segment_index: 0 })`.
- And: aggregate/public result is exactly the same concrete expected `Err(ValidationError::AccessorPathInvalid { accessor_index: 0, segment_index: 0 })`.
- Oracle rule: this test must fail if both implementations incorrectly return `Ok(())` or the wrong error variant; pairwise equality alone is forbidden.

### Behavior: `vb_validate` and `vb_core` agree on concrete accessor symbol bounds

- Test function names:
  - `fn validate_gate_08_matches_core_workflow_for_valid_field_boundaries()`
  - `fn validate_gate_08_matches_core_workflow_for_invalid_field_boundaries()`
- Given: equivalent fixtures accepted by both APIs for valid case `Field(0)` with `symbols_count = 1` and invalid case `Field(symbols_count)` with accessor index `0`, segment index `0`.
- When: `vb_validate` Gate 8 and `vb_core::workflow::validate_accessor_paths` validate the fixtures.
- Then valid case: `vb_validate` result is exactly `Ok(())`, and `vb_core` result is exactly its public success value.
- Then invalid case: `vb_validate` result is exactly `Err(ValidationError::AccessorPathInvalid { accessor_index: 0, segment_index: 0 })`, and `vb_core` result is exactly its public structural path-invalid error for the same field-symbol boundary.

## 4. Proptest Invariants

### Proptest: Gate 8 accepts exactly field symbols below symbols_count

- Invariant: for any generated `symbols_count` and `field_id`, a single valid-root accessor with one `Field(field_id)` returns `Ok(())` iff `field_id < symbols_count`; otherwise it returns exactly `Err(ValidationError::AccessorPathInvalid { accessor_index: 0, segment_index: 0 })`.
- Strategy: generate `symbols_count: u32` in a bounded domain including `0`, `1`, `2`, and high representable values; generate `field_id: u32`; construct `SymbolId` through safe repository APIs.
- Anti-invariant: any `field_id >= symbols_count` must never produce `Ok(())`.

### Proptest: above-bound fixtures are overflow safe

- Invariant: generated above-bound cases only use `symbols_count.checked_add(1)` when it succeeds; when it does not, the generated case is discarded or mapped to the equal-boundary case.
- Strategy: `prop_filter_map` or equivalent checked construction for `N + 1` field ids.
- Anti-invariant: tests must never rely on wrapping arithmetic to construct `SymbolId::new(symbols_count + 1)`.

### Proptest: Gate 8 accepts exactly non-sentinel index values

- Invariant: for any generated `index: u32`, a single valid-root accessor with one `Index(index)` returns `Ok(())` iff `index != u32::MAX`; `u32::MAX` returns exactly `Err(ValidationError::AccessorPathInvalid { accessor_index: 0, segment_index: 0 })`.
- Strategy: arbitrary `u32` plus weighted cases `0`, `1`, `u32::MAX - 1`, and `u32::MAX`.
- Anti-invariant: `PathSegment::Index(u32::MAX)` must always fail.

### Proptest: Gate 8 reports the first invalid path coordinate deterministically

- Invariant: for any bounded valid-root accessor path containing at least one invalid segment, the returned `AccessorPathInvalid` coordinate equals the first invalid segment in path order.
- Strategy: generate non-empty bounded vectors of `PathSegment`; invalid means `Field(id) where id >= symbols_count` or `Index(u32::MAX)`.
- Anti-invariant: validation must not report a later invalid segment while an earlier invalid segment exists.

### Proptest: Gate 8 reports first invalid accessor deterministically with root precedence

- Invariant: for any bounded accessor list, validation returns `Ok(())` only when all roots and segments are valid; otherwise it returns the exact error for the first invalid accessor in slice order, with `AccessorSlotOutOfRange` taking precedence over path errors within that accessor.
- Strategy: generate bounded accessor lists with roots at `0`, `slot_count - 1` when `slot_count > 0`, `slot_count`, and above; generate bounded paths from field/index segments.
- Anti-invariant: an invalid later accessor must not mask an invalid earlier accessor, and an invalid path must not mask an invalid root in the same accessor.

## 5. Fuzz Targets

No fuzz target is required for this bead because the scoped API accepts already-typed `&WorkflowParts` and the contract explicitly excludes YAML, JSON, HTTP, parser, serializer, deserializer, runtime execution, and user-text behavior.

Typed adversarial coverage is mandatory through the property tests above because `WorkflowParts` is untrusted compiled IR and may contain adversarial roots, paths, field ids, zero counts, and sentinels.

## 6. Kani Harnesses

### Kani Harness: field symbol bound is complete

- Property: for bounded `symbols_count` and `field_id`, Gate 8 accepts a field segment exactly when `field_id < symbols_count` and rejects exactly when `field_id >= symbols_count` with `AccessorPathInvalid { accessor_index: 0, segment_index: 0 }`.
- Bound: `symbols_count` and `field_id` in `0..=8`; one accessor; one path segment.
- Rationale: formally proves equal-boundary and zero-boundary cases independent of random sampling.

### Kani Harness: root bound is complete

- Property: for bounded `slot_count` and `root`, Gate 8 accepts root exactly when `root < slot_count`; it rejects `root == slot_count` and `root > slot_count` with `AccessorSlotOutOfRange { accessor_index: 0, slot: root, slot_count }`.
- Bound: `slot_count` in `0..=8`, `root` in `0..=10`, one accessor, empty path.
- Rationale: formally kills root off-by-one risks required by the review.

### Kani Harness: index sentinel classification is complete

- Property: `Index(u32::MAX)` is rejected with `AccessorPathInvalid { accessor_index: 0, segment_index: 0 }`, and modeled non-sentinel indices are accepted when root is valid.
- Bound: representative set `{0, 1, 8, u32::MAX - 1, u32::MAX}` or symbolic `u32` if current Kani support permits the involved newtypes.
- Rationale: sentinel handling is a reserved-value invariant.

### Kani Harness: root-before-path error precedence

- Property: when an accessor root is out of range and its path also contains an invalid segment, Gate 8 returns exact `AccessorSlotOutOfRange`, not `AccessorPathInvalid`.
- Bound: `slot_count` in `0..=4`, root in `0..=8`, one invalid field or sentinel path segment.
- Rationale: deterministic validation order is public diagnostic behavior.

## 7. Mutation Checkpoints

Minimum threshold: mutation testing must kill at least 90% of mutations in touched Gate 8 files. Surviving mutants in changed validation logic require additional tests or documented impossibility.

Critical mutations that must be killed:

- Change `symbol.get() < symbols_count` to `symbol.get() <= symbols_count`; killed by `gate_08_rejects_field_symbol_equal_to_symbols_count`.
- Change `symbol.get() < symbols_count` to `symbol.get() > symbols_count`; killed by field-zero and upper-bound valid scenarios.
- Remove field-symbol validation; killed by all invalid field scenarios and the field-bound proptest.
- Change invalid field error to `AccessorSlotOutOfRange`; killed by exact `AccessorPathInvalid` assertions.
- Use wrong `accessor_index`; killed by `gate_08_reports_invalid_field_segment_coordinates`.
- Use wrong `segment_index`; killed by `gate_08_reports_invalid_field_segment_coordinates`.
- Change index sentinel check from `== u32::MAX` to `>= u32::MAX - 1`; killed by `gate_08_accepts_index_u32_max_minus_one`.
- Remove index sentinel rejection; killed by `gate_08_rejects_sentinel_index_segment`.
- Change root comparison from `root < slot_count` to `root <= slot_count`; killed by `gate_08_rejects_accessor_root_equal_to_slot_count`.
- Change root comparison from `root < slot_count` to `root + 1 < slot_count`, `root < slot_count - 1`, or any mutation that rejects maximum valid root; killed by `gate_08_accepts_accessor_root_at_slot_count_minus_one`.
- Remove root validation; killed by root equal/great-than slot-count rejection tests.
- Check path before root; killed by `gate_08_checks_root_before_path_segments`.
- Update focused Gate 8 but not aggregate Gate 8; killed by concrete focused/aggregate valid and invalid parity tests.
- Make both focused and aggregate implementations identically wrong and equality-only passing; killed because parity tests assert concrete `Ok(())` and concrete `AccessorPathInvalid` values for each implementation.
- Diverge from `vb_core::workflow::validate_accessor_paths`; killed by concrete core parity tests.
- Treat empty path or empty accessor collection as invalid; killed by empty-path and empty-collection scenarios.
- Introduce wrapping `symbols_count + 1`; killed by overflow-safe construction property and static arithmetic review.

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer |
|---|---|---|---|
| first field valid | `symbols_count = 1`, `Field(0)` | `Ok(())` | unit |
| upper-bound field valid | `symbols_count = N > 0`, `Field(N - 1)` | `Ok(())` | unit |
| zero symbols rejects field | `symbols_count = 0`, `Field(0)` | `Err(AccessorPathInvalid { accessor_index: 0, segment_index: 0 })` | unit |
| equal-bound field invalid | `symbols_count = N`, `Field(N)` | `Err(AccessorPathInvalid { accessor_index: 0, segment_index: 0 })` | unit |
| above-bound field invalid | `symbols_count = N`, `N < u32::MAX`, checked `Field(N + 1)` | `Err(AccessorPathInvalid { accessor_index: 0, segment_index: 0 })` | unit |
| index minimum valid | `Index(0)` | `Ok(())` | unit |
| index maximum valid | `Index(u32::MAX - 1)` | `Ok(())` | unit |
| index sentinel invalid | `Index(u32::MAX)` | `Err(AccessorPathInvalid { accessor_index: 0, segment_index: 0 })` | unit |
| empty path | valid root, `path = []` | `Ok(())` | integration |
| empty accessor collection | `accessors = []` | `Ok(())` | integration |
| max valid root | `slot_count = N > 0`, root `N - 1` | `Ok(())` | unit |
| root equal slot count | `slot_count = N`, root `N` | `Err(AccessorSlotOutOfRange { accessor_index: 0, slot: N, slot_count: N })` | unit |
| root greater than slot count | `slot_count = 1`, root `5` | `Err(AccessorSlotOutOfRange { accessor_index: 0, slot: 5, slot_count: 1 })` | integration |
| invalid segment coordinates | second accessor, second segment invalid | `Err(AccessorPathInvalid { accessor_index: 1, segment_index: 1 })` | integration |
| root before path | invalid root and invalid field in same accessor | `Err(AccessorSlotOutOfRange { accessor_index: 0, slot, slot_count })` | integration |
| focused valid parity | focused + aggregate valid fixtures | focused `Ok(())`; aggregate `Ok(())` | integration |
| focused invalid parity | focused + aggregate invalid fixtures | focused `Err(AccessorPathInvalid { 0, 0 })`; aggregate `Err(AccessorPathInvalid { 0, 0 })` | integration |
| core valid parity | `vb_validate` + `vb_core` valid field fixture | both concrete public success values | integration |
| core invalid parity | `vb_validate` + `vb_core` invalid field fixture | concrete structural path-invalid errors | integration |
| field-bound invariant | any bounded valid root + one field | accepted iff `field_id < symbols_count` | proptest |
| root-bound invariant | bounded roots around slot count | accepted iff `root < slot_count` | proptest/Kani |
| index-sentinel invariant | any bounded valid root + one index | accepted iff `index != u32::MAX` | proptest |
| first invalid segment invariant | bounded path with invalid segments | first invalid segment coordinate | proptest |
| first invalid accessor invariant | bounded accessor list | first invalid accessor error with root precedence | proptest |

## 9. Holzmann Test-Structure Rules

- No planned test body may contain `for`, `while`, or `loop`. Multi-case checks must be implemented as separate named tests or `rstest`/parameterized cases whose generated bodies remain straight-line.
- Each test creates its own `WorkflowParts` fixture. No `static mut`, mutable `lazy_static!`, mutable `once_cell`, global mutex/RwLock fixture, or shared mutable accessor/path vector is allowed.
- Fixture helper names must disclose effects. Pure builders may be named `workflow_parts_with_*`; helpers that allocate, mutate, or normalize must include that effect in the name.
- Test assertions must compare exact public state/results, not private fields or internal iteration details.
- No test may call private helper functions unless they become public contract; behavior is tested through public Gate 8/public aggregate/core validation APIs.
- Test code must avoid `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing, unchecked slicing, unchecked casts, and unchecked arithmetic. If construction can fail, use pattern matching that yields a typed test failure with context rather than unwrap/expect.

## 10. Static Resource, Panic, and Policy Checks

Downstream implementation and tests must include evidence for these static checks:

1. Source scan or clippy/policy gate proves no new `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` in production code.
2. Test code also avoids `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, and `dbg`; any unavoidable test framework macro behavior must be justified by existing repository pattern.
3. Gate 8 implementation remains single-pass over `parts.accessors` and each accessor path; no recursion and no unbounded allocation.
4. Field-symbol comparison uses `symbol.get() < parts.symbols_count` or an equivalent safe repository API; no tuple-field access and no unchecked casts.
5. `symbols_count == 0` path does not subtract one or use wrapping arithmetic.
6. All `symbols_count + 1` fixtures use `checked_add(1)` or a statically chosen constant with a written proof that it cannot overflow.
7. No JSON/YAML/HTTP/parser/deserializer behavior is introduced in runtime core.
8. Nightly feature governance remains unchanged and passes the repository nightly-feature gate if touched files are in scope.

## 11. Command Gates and Acceptance Oracles

The test writer/implementation state must capture exact command names, status, and relevant output:

1. Targeted unit/integration command for `vb_validate` Gate 8 tests using the repository's non-interactive Rust test runner pattern.
2. Targeted command for focused/aggregate/core parity tests.
3. Property-test command for field bounds, root bounds, index sentinel, and first-error determinism.
4. Mutation command over touched Gate 8 files with at least 90% killed mutations, or a repository-approved equivalent when full mutation is too slow.
5. Static policy/lint command showing no forbidden panic/resource constructs and no forbidden nightly feature drift.
6. Canonical final acceptance command: `moon ci` from workspace root.

No CLI product behavior is introduced by this bead; therefore the e2e oracle is the repository command gate, not a user-facing CLI scenario.

## Open Questions

- Confirm whether `crates/vb_validate/src/gates.rs` exposes an active independent Gate 8 function or delegates to `gate_08_accessor.rs`; if it delegates, parity tests should call the public aggregate validation path instead of private duplicate logic.
- Confirm exact public constructors for `SymbolId`, slot/root newtypes, and `WorkflowParts` fixtures before test writing; tests must use safe repository APIs and no tuple-field access or unchecked casts.
