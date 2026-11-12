# Verification Layers: vb-qi37.8 — Shared Validation Pipeline

## Bead Information
- **bead_id**: vb-qi37.8
- **title**: validate/compile: Prove and complete shared validation pipeline
- **state**: 3 (Contract + Proof Obligations)

---

## 1. Defense-in-Depth Architecture

```
Layer 0: Type System (Rust/Normal)
Layer 1: Unit Tests (cargo test)
Layer 2: Integration Tests (cargo test --test)
Layer 3: Property Tests (proptest)
Layer 4: Miri (Undefined Behavior)
Layer 5: Kani (Bounded Model Checking)
Layer 6: TLA+ (Temporal/Protocol)
Layer 7: Lean (Theorem Proving)
```

---

## 2. Per-Gate Verification Assignments

### 2.1 Gate 7: Expression Stack Depth

| Layer | Tool | Target | Evidence |
|-------|------|--------|----------|
| L0 | Rust types | `StackDepth` struct caps at 64 | Compile-time |
| L1 | cargo test | `tests/red_phase_validation.rs` | Test passes |
| L3 | proptest | Stack depth invariants | 1000 iterations |
| L4 | Miri | `validate_gate_07` with edge cases | No UB |
| L5 | Kani | `stack_depth < 64` for all expressions | Bounded proof |

### 2.2 Gate 8: Accessor Path Segments

| Layer | Tool | Target | Evidence |
|-------|------|--------|----------|
| L0 | Rust types | `AccessorProgram` path validation | Compile-time |
| L1 | cargo test | `tests/gate_08_accessor_parity.rs` | Test passes |
| L2 | integration | `tests/capability_contract_schema.rs` | Full path |
| L3 | proptest | Path resolution invariants | 1000 iterations |
| L4 | Miri | `validate_gate_08` with max depth | No UB |
| L5 | Kani | Symbol lookup total function | Bounded proof |

### 2.3 Gate 9: Slot References

| Layer | Tool | Target | Evidence |
|-------|------|--------|----------|
| L0 | Rust types | `Option<SlotIdx>` bounds check | Compile-time |
| L1 | cargo test | `tests/red_phase_validation.rs` | Test passes |
| L3 | proptest | Slot bounds invariants | 1000 iterations |
| L4 | Miri | `validate_gate_09` with OOB | No UB |
| L5 | Kani | `slot < slot_count` for all refs | slot_count-bounded |

### 2.4 Gate 10: Node Kind Specific

| Layer | Tool | Target | Evidence |
|-------|------|--------|----------|
| L0 | Rust types | `CompiledNodeKind` enum exhaustive | Compile-time |
| L1 | cargo test | `tests/red_phase_validation.rs` | Test passes |
| L3 | proptest | Node variant coverage | 1000 iterations |
| L4 | Miri | `validate_gate_10` all 14 variants | No UB |
| L5 | Kani | Each node kind has matching pair | 14-variant bounded |

### 2.5 Gate 11: Loop Body Graph

| Layer | Tool | Target | Evidence |
|-------|------|--------|----------|
| L0 | Rust types | `ForEachStart/Join` struct links | Compile-time |
| L1 | cargo test | `tests/red_phase_validation.rs` | Test passes |
| L3 | proptest | Loop body well-formedness | 1000 iterations |
| L4 | Miri | `validate_gate_11` iter depth | No UB |
| L5 | Kani | Finite graph traversal bounded | nodes.len()-bounded |

### 2.6 Gate 12: Action Contract Completeness

| Layer | Tool | Target | Evidence |
|-------|------|--------|----------|
| L0 | Rust types | `ActionContract` struct | Compile-time |
| L1 | cargo test | `tests/capability_contract_schema.rs` | Test passes |
| L1 | cargo test | `tests/idempotency_contract_red.rs` | Test passes |
| L2 | integration | `tests/capability_contract_schema.rs` | Full bijection |
| L3 | proptest | Contract completeness | 500 iterations |
| L5 | Kani | Bijection proof (Do ↔ Contract) | action_contracts.len()-bounded |

### 2.7 Gate 13: No Slot Cycles

| Layer | Tool | Target | Evidence |
|-------|------|--------|----------|
| L0 | Rust types | `SlotIdx` newtype | Compile-time |
| L1 | cargo test | `tests/red_phase_validation.rs` | Test passes |
| L3 | proptest | Cycle detection | 1000 iterations |
| L4 | Miri | `validate_gate_13` cycle path | No UB |
| L5 | Kani | Acyclic slot graph | slot_count-iterations bounded |
| L6 | TLA+ | G13_NoCycle invariant | TLC model check |

### 2.8 Gate 14: Slot Type Consistency

| Layer | Tool | Target | Evidence |
|-------|------|--------|----------|
| L0 | Rust types | Type tags on slots | Compile-time |
| L1 | cargo test | `tests/red_phase_validation.rs` | Test passes |
| L3 | proptest | Type compatibility | 1000 iterations |
| L4 | Miri | `validate_gate_14` type checks | No UB |
| L5 | Kani | Multi-writer compatible | type-fineness bounded |

### 2.9 Gate 15: Determinism Proof

| Layer | Tool | Target | Evidence |
|-------|------|--------|----------|
| L0 | Rust types | `IsNonDeterministic` marker | Compile-time |
| L1 | cargo test | `tests/red_phase_validation.rs` | Test passes |
| L3 | proptest | ND separation | 500 iterations |
| L4 | Miri | `validate_gate_15` graph walk | No UB |
| L5 | Kani | ND nodes separated by suspension | graph-size bounded |
| L6 | TLA+ | G15_Separated invariant | TLC model check |
| L7 | Lean | NDNodesSeparated formal proof | Theorem kernel |

---

## 3. Integration Verification

| Layer | Tool | Scope | Evidence |
|-------|------|-------|----------|
| L1 | cargo test -p vb_validate | All unit tests | 100% pass |
| L1 | cargo test -p vb_compile | Compilation tests | 100% pass |
| L2 | cargo test --test '*' | All integration tests | 100% pass |
| L4 | Miri full crate | vb_validate | No UB detected |
| L5 | Kani full crate | vb_validate | All harnesses pass |
| L6 | TLC model check | ValidationPipeline.tla | Invariants satisfied |

---

## 4. Fuzzing Integration

| Layer | Tool | Target | Coverage |
|-------|------|--------|----------|
| L3 | cargo fuzz run | `validate_with_contracts` | Corpus + generated |
| L4 | Miri + fuzz | `validate` | No UB in corpus |
| L5 | Kani | Fuzz-reduced interesting cases | Bounded proof |

---

## 5. Verification Lane Matrix

| Gate | Miri | Kani | Proptest | TLA+ | Lean |
|------|------|------|----------|------|------|
| G7 | ✓ | ✓ | ✓ | - | - |
| G8 | ✓ | ✓ | ✓ | - | - |
| G9 | ✓ | ✓ | ✓ | - | - |
| G10 | ✓ | ✓ | ✓ | - | - |
| G11 | ✓ | ✓ | ✓ | - | - |
| G12 | - | ✓ | ✓ | - | - |
| G13 | ✓ | ✓ | ✓ | ✓ | - |
| G14 | ✓ | ✓ | ✓ | - | - |
| G15 | ✓ | ✓ | ✓ | ✓ | ✓ |

---

## 6. Evidence Requirements

| Lane | Minimum Evidence | Pass Criterion |
|------|------------------|----------------|
| Unit tests | 10 tests per gate | 100% pass |
| Proptest | 500 iterations per property | No failure |
| Miri | Full crate scan | 0 UB detected |
| Kani | All harnesses | 0 failed assertions |
| TLA+ | 1-hour TLC run | 0 invariant violation |
| Lean | All theorems | 0 unproven |

---

## 7. Risk-Mitigation Verification

| Risk | Mitigation | Verification |
|------|------------|--------------|
| DRIFT-5 | vb_compile always calls vb_validate | Integration test at every call site |
| SECTION-63 | Kani bounded model checking | All 9 gates bounded-proven |
| Cold-path bypass | Design assumption documented | No runtime validation (by design) |
