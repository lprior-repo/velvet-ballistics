bead_id: vb-qi37.7.3
bead_title: "validate: symbol/reference bounds and resource contract errors"
phase: 5
updated_at: 2026-05-09T01:10:00Z

# Test Plan: Symbol Bounds, Reference Validation & Resource Contract Errors

## Summary
- Behaviors identified: 24
- Trophy allocation: 32 unit / 6 integration / 3 proptest / 2 fuzz / 6 kani / 2 snapshot / 3 benchmark
- Mutation threshold: ≥85%

## 1. Behavior Inventory

### Symbol Bounds (validate_symbol / validate_accessor_paths)

1. [Reject] `validate_symbol(symbol, count)` → `Err(SymbolOutOfBounds)` when `symbol.get() >= count`
2. [Reject] `validate_symbol(symbol, count)` → `Ok` when `symbol.get() < count`
3. [Reject] `validate_accessor_paths` with field segment symbol >= symbols_count → `Err(SymbolOutOfBounds)`
4. [Reject] `validate_accessor_paths` with field segment symbol < symbols_count → `Ok`
5. [Reject] `validate_constants_symbols` with Symbol constant >= symbols_count → `Err(SymbolOutOfBounds)`
6. [Reject] `validate_build_object_symbols` with field symbol >= symbols_count → `Err(SymbolOutOfBounds)`
7. [Reject] `validate_gate_08_accessor_path_segments` → `Err(ValidationError::AccessorSymbolOutOfBounds)` for same case
8. [Reject] `validate_gate_08_accessor_path_segments` → `Ok` when symbol in bounds

### Resource Contract — Too Large (declared exceeds hard limit)

9.  [Reject] `max_steps` declared > `MAX_STEPS_PER_WORKFLOW` → `Err(ResourceContractTooLarge)`
10. [Reject] `max_slots` declared > `MAX_SLOTS_PER_WORKFLOW` → `Err(ResourceContractTooLarge)`
11. [Reject] `max_constants` declared > `MAX_CONSTANTS` → `Err(ResourceContractTooLarge)`
12. [Reject] `max_accessors` declared > `MAX_ACCESSORS` → `Err(ResourceContractTooLarge)`
13. [Reject] `max_expressions` declared > `MAX_EXPRESSIONS` → `Err(ResourceContractTooLarge)`
14. [Reject] `max_expr_stack` declared > `MAX_EXPRESSION_STACK` (64) → `Err(ResourceContractTooLarge)`

### Resource Contract — Exceeded (artifact exceeds declared)

15. [Reject] `max_steps` actual > declared → `Err(ResourceContractExceeded)`
16. [Reject] `max_slots` actual > declared → `Err(ResourceContractExceeded)`
17. [Reject] `max_constants` actual > declared → `Err(ResourceContractExceeded)`
18. [Reject] `max_accessors` actual > declared → `Err(ResourceContractExceeded)`
19. [Reject] `max_expressions` actual > declared → `Err(ResourceContractExceeded)`
20. [Reject] `max_expr_stack` expr.max_stack > contract.max_expr_stack → `Err(ResourceContractExceeded)`

### Pipeline Validation

21. [Reject] Pipeline with accessor out-of-bounds symbol → `Err(ValidationError::AccessorSymbolOutOfBounds)`
22. [Reject] Pipeline with expression stack too deep → `Err(ValidationError::ExpressionStackExceeded)`
23. [Reject] Pipeline with slot reference out of range → `Err(ValidationError::SlotReferenceOutOfRange)`
24. [Reject] Pipeline with all gates disabled → `Ok` for any valid parts

## 2. Trophy Allocation

| Layer | Count | Rationale |
|---|---|---|
| Unit | 32 | Pure calc layer: validate_symbol, validate_contract_limit, each gate function |
| Integration | 6 | Full pipeline validation: each error type at pipeline level |
| Proptest | 3 | Invariants: symbol < count ↔ Ok, actual ≤ declared ↔ Ok, pipeline deterministic |
| Fuzz | 2 | decode_symbol_id (u32 → SymbolId), decode_resource_contract |
| Kani | 6 | validate_contract_limit arithmetic, validate_symbol boundary, pipeline short-circuit |
| Snapshot | 2 | Complex ValidationError::AccessorSymbolOutOfBounds render, multi-error render |
| Benchmark | 3 | validate_gate_08 hot path, validate_resource_contract, full pipeline |
| **Total** | **54** | |

## 3. BDD Scenarios

### Symbol Out of Bounds

```
Given: symbols_count = 5
  and symbol = SymbolId::new(7)
When: validate_symbol(symbol, symbols_count) is called
Then: Returns Err(WorkflowError::SymbolOutOfBounds { symbol: SymbolId::new(7) })
```
Test: `validate_symbol_rejects_out_of_bounds_symbol`

### Resource Contract Too Large

```
Given: a ResourceContract with max_steps = 2000
  and MAX_STEPS_PER_WORKFLOW = 1000
When: validate_resource_contract(parts) is called
Then: Returns Err(WorkflowError::ResourceContractTooLarge { resource: "max_steps" })
```
Test: `validate_resource_contract_rejects_max_steps_too_large`

### Resource Contract Exceeded

```
Given: parts with 50 nodes
  and ResourceContract with max_steps = 40
When: validate_resource_contract(parts) is called
Then: Returns Err(WorkflowError::ResourceContractExceeded { resource: "max_steps" })
```
Test: `validate_resource_contract_rejects_max_steps_exceeded`

### Pipeline Accessor Symbol Out of Bounds

```
Given: WorkflowParts with 1 accessor using Field(SymbolId::new(99))
  and symbols_count = 10
When: ValidationPipeline::default().validate(parts) is called
Then: Returns Err(ValidationError::AccessorSymbolOutOfBounds {
    accessor_index: 0, segment_index: 0, symbol: 99, symbols_count: 10 })
```
Test: `pipeline_validate_rejects_accessor_symbol_out_of_bounds`

## 4. Proptest Invariants

### Symbol Invariant
For any `symbol: SymbolId` and `symbols_count: u32`:
- If `symbol.get() < symbols_count` → `validate_symbol` returns `Ok`
- If `symbol.get() >= symbols_count` → `validate_symbol` returns `Err(SymbolOutOfBounds)`
Strategy: `symbol in 0..(symbols_count + 100)`, `symbols_count in 1..1000`

### Resource Contract Invariant
For any `actual: usize`, `declared: usize`, `hard_limit: usize`:
- If `declared > hard_limit` → `validate_contract_limit` returns `Err(ResourceContractTooLarge)`
- Else if `actual > declared` → `Err(ResourceContractExceeded)`
- Else → `Ok`
Strategy: `prop_oneof![(0..=hard_limit, 0..=hard_limit), (hard_limit+1..hard_limit+100, 0..=hard_limit)]`

### Pipeline Determinism
For any `WorkflowParts`:
- `validate(parts)` is deterministic: same input → same output
- `validate(parts)` either returns `Ok` or the first failing gate's error (ordered: 7, 8, 9, 10, 11, 13, 14, 15)

## 5. Fuzz Targets

### Fuzz: symbol_id_boundary
Input: `(u32, u32)` interpreted as `(symbol_raw, symbols_count)`
Risk: `SymbolId::new_unchecked` construction, out-of-bounds panic
Seeds: `(0, 1)`, `(MAX, MAX)`, `(MAX, 0)`

### Fuzz: resource_contract_fields
Input: postcard-encoded `ResourceContract`
Risk: Decode produces invalid field combinations (e.g., max_steps=0, max_slots=u16::MAX)
Seeds: Default contract, all zeros, max values

## 6. Kani Harnesses

### KH-SYM-01: validate_symbol boundary
Property: `symbol.get() < symbols_count == !matches(validate_symbol, Err(SymbolOutOfBounds))`
Bound: symbol in [0, 10], symbols_count in [1, 10]

### KH-RC-01: validate_contract_limit — declared > hard_limit
Property: If `declared > hard_limit`, result is `Err(ResourceContractTooLarge)`
Bound: hard_limit in [1, 5], declared in [hard_limit..hard_limit+3]

### KH-RC-02: validate_contract_limit — actual > declared
Property: If `actual > declared` and `declared <= hard_limit`, result is `Err(ResourceContractExceeded)`
Bound: hard_limit in [5, 10], declared in [1, hard_limit], actual in [declared..declared+5]

### KH-PL-01: pipeline short-circuit order
Property: Gate 7 runs before Gate 8; if Gate 7 fails, Gate 8 does not run
Bound: Parts with both stack depth error and symbol error

### KH-PL-02: pipeline all-gates equivalent to default
Property: `ValidationPipeline::default().validate == validate`
Bound: Any valid WorkflowParts

### KH-ERR-01: error variant exactness
Property: `validate_gate_08` with symbol=count returns `Err(ValidationError::AccessorSymbolOutOfBounds { symbol: count, symbols_count: count })`
Bound: accessor_index=0, segment_index=0, count in [1, 5]

## 7. Mutation Checkpoints

Critical mutations to survive:
- `if symbol.get() < symbols_count` → invert: caught by boundary tests
- `if declared > hard_limit` → remove: caught by ResourceContractTooLarge tests
- `if actual > declared` → remove: caught by ResourceContractExceeded tests
- `parts.symbols_count` → use wrong field: caught by symbol count mismatch tests
- Pipeline gate short-circuit order wrong: caught by KH-PL-01

## 8. Benchmark Targets

### BM-01: validate_gate_08_accessor_path_segments (empty path)
Baseline: 1000 calls in 1ms → target < 500µs

### BM-02: validate_resource_contract (all within bounds)
Baseline: 1000 calls in 2ms → target < 1ms

### BM-03: Full pipeline validate (DEFAULT pipeline, minimal parts)
Baseline: 1000 calls in 5ms → target < 3ms

## Open Questions

None at contract time. The codebase structure is well-understood.
