# Architectural Drift Report: `vb_proof_kernels/src/lib.rs`

**Analyzed**: 2026-05-29  
**Crates root**: `crates/vb_proof_kernels/src/lib.rs`  
**Total lines (crate)**: 3205  
**Status**: `REFACTORED`

---

## 1. Line Count Gate

| File | Lines | Limit | Status |
|------|-------|-------|--------|
| `lib.rs` | 11 | 300 | ✅ PASS |
| `envelope_header.rs` | 579 | 300 | ❌ **VIOLATION** |
| `resource_budget.rs` | 1028 | 300 | ❌ **VIOLATION** |
| `step_state.rs` | 512 | 300 | ❌ **VIOLATION** |
| `taint.rs` | 422 | 300 | ❌ **VIOLATION** |
| `vb_kyyf_normalization.rs` | 653 | 300 | ❌ **VIOLATION** |

**Summary**: 5 of 6 files exceed the 300-line threshold. Total crate: 3205 lines.

---

## 2. DDD Cohesion Analysis

### `envelope_header.rs` (579 lines)
- **Domain concept**: Envelope header parsing and validation
- **Cohesion**: HIGH — `EnvelopeHeader` struct, `ValidationError`, `ValidationResult` all serve the same bounded context
- **Exports**: `EnvelopeHeader`, `ValidationError`, `ValidationResult`, validation helpers
- **DDD smell**: MINOR — `u32`, `u8` raw primitives for magic/version/kind but these are wire-format constants

### `resource_budget.rs` (1028 lines)
- **Domain concept**: Resource budgeting and policy enforcement
- **Cohesion**: HIGH — `Budget` aggregate, `Policy` value object, composition functions
- **Exports**: `Budget`, `Policy`, `sequential_compose`, `branch_compose`, `loop_compose`
- **DDD smell**: MINOR — 12 raw `u64` fields in `Budget` (primitive obsession) but appropriate for a proof kernel

### `step_state.rs` (512 lines)
- **Domain concept**: Step state machine with explicit state transitions
- **Cohesion**: HIGH — `StepState` enum, transition table, validation predicates
- **Exports**: `StepState`, `is_valid_transition`, `validate_transition`, `next_states`
- **DDD smell**: NONE — well-modeled state machine with explicit transition rules

### `taint.rs` (422 lines)
- **Domain concept**: Taint lattice for security classification
- **Cohesion**: HIGH — `Taint` enum, `join_taint`, lattice law proofs
- **Exports**: `Taint`, `join_taint`, `join_many`, lattice predicates
- **DDD smell**: NONE — excellent domain modeling, mathematical lattice structure

### `vb_kyyf_normalization.rs` (653 lines)
- **Domain concept**: Replay normalization and cross-run determinism comparison
- **Cohesion**: HIGH — `PublicObservation`, `NormalizedObservation`, `DigestStatus`, comparison functions
- **Exports**: `TerminalResult`, `TaintStatus`, `DeterminismError`, `DigestStatus`, `PublicObservation`, comparison functions
- **DDD smell**: MODERATE — 9 raw `u64` signature fields in `PublicObservation` (semantic_slot_signature, event_signature, etc.) could be NewTypes

---

## 3. Violations

### CRITICAL: File Size (>300 lines)

1. **`envelope_header.rs`** (579 lines)
   - Recommendation: Split at ~line 200 into `envelope_header/types.rs`, `envelope_header/validation.rs`, `envelope_header/crc.rs`

2. **`resource_budget.rs`** (1028 lines)
   - Recommendation: Split at ~line 350 into `resource_budget/budget.rs`, `resource_budget/policy.rs`, `resource_budget/composition.rs`

3. **`step_state.rs`** (512 lines)
   - Recommendation: Split at ~line 200 into `step_state/types.rs`, `step_state/transitions.rs`, `step_state/validation.rs`

4. **`taint.rs`** (422 lines)
   - Recommendation: Split at ~line 200 into `taint/types.rs`, `taint/lattice.rs`, `taint/laws.rs`

5. **`vb_kyyf_normalization.rs`** (653 lines)
   - Recommendation: Split at ~line 250 into `vb_kyyf_normalization/types.rs`, `vb_kyyf_normalization/normalize.rs`, `vb_kyyf_normalization/compare.rs`

### MODERATE: Primitive Obsession

- `resource_budget.rs`: `Budget` has 12 raw `u64` fields — acceptable for proof kernel but consider `Steps(u64)`, `Actions(u64)` NewTypes
- `vb_kyyf_normalization.rs`: `PublicObservation` has 9 `u64` signature fields — candidates for NewTypes: `EventSignature(u64)`, `SlotSignature(u64)`, `ActionSignature(u64)`

---

## 4. DDD Smell Summary

| Smell | Severity | Files |
|-------|----------|-------|
| File >300 lines | CRITICAL | 5 modules |
| Primitive obsession (u64) | MODERATE | resource_budget.rs, vb_kyyf_normalization.rs |
| Anemic validation types | LOW | envelope_header.rs (ValidationError/Result are thin) |

---

## 5. Priority

| Priority | Action | Effort |
|----------|--------|--------|
| **P0** | Split all 5 oversized modules per recommendations above | High |
| **P1** | Add NewType wrappers for signature fields in `vb_kyyf_normalization.rs` | Medium |
| **P2** | Add NewType wrappers for Budget fields in `resource_budget.rs` | Low |

---

## 6. Recommendation

The crate demonstrates strong **domain cohesion** — each module is a well-bounded proof kernel with clear purpose. However, the **file size violations are severe** (5 of 6 files over limit). The primitive obsession in `vb_kyyf_normalization.rs` is the most impactful refactoring target for type safety, while file splitting is the immediate structural fix required.
