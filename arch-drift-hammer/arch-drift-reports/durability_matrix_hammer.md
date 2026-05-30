# ARCHITECTURAL DRIFT REPORT: `durability_matrix.rs`

**File**: `crates/vb_runtime/src/durability_matrix.rs`
**Lines**: 367 (VIOLATION: exceeds 300-line mandate)
**Severity**: CRITICAL

---

## 1. LINE COUNT VIOLATION

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 367 | 300 | **VIOLATION (+67)** |
| Module code | 268 | 300 | PASS |
| Inline tests | 99 | — | CONTRIBUTES TO DRIFT |

**ROOT CAUSE**: Tests (lines 269–367) embedded in production module. This is a
structure violation — integration tests belong in `crates/workspace_tests/`, not
inline with production code.

---

## 2. PRIMITIVE OBSESSION VIOLATIONS

### Violation A: `DurabilityRow` fields use raw strings

```rust
pub struct DurabilityRow {
    pub primitive: &'static str,           // ← primitive obsession
    pub compiled_node_kind: &'static str,  // ← primitive obsession
    pub replay_assertion: &'static str,    // ← primitive obsession
    pub test_evidence: &'static [&'static str], // ← primitive obsession
}
```

**Should be**: Named value objects with domain semantics.

```rust
// MISSING: PrimitiveName(PackageQualifier, LocalName)
// MISSING: NodeKind(CompiledNodeKind)
// MISSING: ReplayAssertion(AssertionText)
// MISSING: TestEvidence(RelativePath)
```

### Violation B: `DurabilityError` uses `String`

```rust
pub enum DurabilityError {
    MissingPrimitiveRow { primitive: String },  // ← heap allocation
    MissingReplayProof  { primitive: String, event: String },
    AckBeforePersist    { primitive: String, handler: String },
    OrphanEvent         { event: String },
}
```

**Should be**: Interned or pinned string types (`SmolStr`, `MiniString`, or
`&'static str` with a validation GADT).

### Violation C: `REQUIRED_PRIMITIVES` is `&[&str]`

```rust
pub const REQUIRED_PRIMITIVES: &[&str] = &[...];
```

**Should be**: `&'static [PrimitiveName]` with a const constructor.

---

## 3. SCOTT WLASCHIN DDD VIOLATIONS

### A. "Type Reveals Meaning" Violation

`DurabilityRow` fields are unnamed primitives that carry no domain semantics:

| Field | Domain Meaning | Current Type |
|-------|---------------|--------------|
| `primitive` | YAML primitive identifier | `&'static str` |
| `compiled_node_kind` | IR node kind label | `&'static str` |
| `journal_events` | Domain events | `&'static [RecordKind]` ✓ (this one is correct) |
| `storage_partition` | Persistence location | `StoragePartition` ✓ (this one is correct) |
| `ack_point` | Durability guarantee | `AckPoint` ✓ (this one is correct) |
| `replay_assertion` | Behavioral contract | `&'static str` |
| `test_evidence` | Evidence paths | `&'static [&'static str]` |

**Fix**: Replace the flagged string fields with domain-typed value objects.

### B. "Make Illegal States Unrepresentable" Violation

The stringly-typed `compiled_node_kind` allows values that are not valid IR node
kinds. Nothing at the type level prevents:

```rust
DurabilityRow {
    primitive: "set",
    compiled_node_kind: "NotARealNode", // ← silently wrong
    ...
}
```

### C. "A Module Should Have Cohesive Responsibilities" Violation

This module has **three** unrelated responsibility clusters:

1. **Data cluster**: `StoragePartition`, `AckPoint`, `DurabilityRow`,
   `DURABILITY_MATRIX`, `REQUIRED_PRIMITIVES`
2. **Verification cluster**: `DurabilityError`, `verify_matrix_completeness`,
   `verify_matrix_replay_proofs`, `verify_ack_after_persist`, `verify_matrix`
3. **Test cluster**: All `#[cfg(test)]` code (lines 269–367)

**Should be split into**:
- `vb_domain/` — Value objects: `StoragePartition`, `AckPoint`, `PrimitiveName`,
  `NodeKind`, `ReplayAssertion`
- `vb_runtime/` — `DurabilityRow`, `DURABILITY_MATRIX`
- `vb_runtime/` — `DurabilityVerifier` service + `DurabilityError` domain error
- `crates/workspace_tests/vb_runtime/test_durability_matrix.rs` — All tests

---

## 4. VERIFICATION FUNCTIONS AS PROCEDURAL DATA

```rust
pub fn verify_matrix_completeness() -> Result<(), DurabilityError> { ... }
pub fn verify_matrix_replay_proofs() -> Result<(), DurabilityError> { ... }
pub fn verify_ack_after_persist() -> Result<(), DurabilityError> { ... }
```

These are **naked procedural functions** — no encapsulation, no state, no
dependency injection. They belong as methods on a `DurabilityMatrixVerifier`
service struct that can be mocked/tested in isolation.

---

## 5. ENUM DISCRIMINANTS AS IMPLICIT STATE

`DurabilityError` variants carry `String` payloads but the enum itself has no
marker traits for serialization or error code semantics. Compare with proper
error domain modeling:

```rust
// CURRENT (weak)
pub enum DurabilityError {
    MissingPrimitiveRow { primitive: String },
}

// SHOULD BE (strong)
pub enum DurabilityError {
    #[error("primitive '{primitive}' has no matrix row")]
    MissingPrimitiveRow { primitive: PrimitiveName },
}
```

---

## 6. PRESCRIPTIVE REFACTORING PLAN

### Phase 1: Extract inline tests (eliminates ~99 lines, gets under 300)
Move `#[cfg(test)]` block (lines 269–367) to
`crates/workspace_tests/vb_runtime/test_durability_matrix.rs`.

### Phase 2: Introduce value objects
```rust
// In vb_domain/primitives.rs
pub struct PrimitiveName(&'static str);
pub struct NodeKind(&'static str);
pub struct ReplayAssertion(&'static str);

impl PrimitiveName {
    pub const fn new(s: &'static str) -> Self { Self(s) }
    pub const fn as_str(&self) -> &'static str { self.0 }
}
```

### Phase 3: Refactor `DurabilityRow`
```rust
pub struct DurabilityRow {
    pub primitive: PrimitiveName,
    pub compiled_node_kind: NodeKind,
    pub journal_events: &'static [RecordKind],
    pub storage_partition: StoragePartition,
    pub ack_point: AckPoint,
    pub replay_assertion: ReplayAssertion,
    pub test_evidence: &'static [TestEvidencePath],
}
```

### Phase 4: Encapsulate verification in a service
```rust
pub struct DurabilityMatrixVerifier;

impl DurabilityMatrixVerifier {
    pub fn verify_completeness(&self) -> Result<(), DurabilityError> { ... }
    pub fn verify_replay_proofs(&self) -> Result<(), DurabilityError> { ... }
    pub fn verify_ack_after_persist(&self) -> Result<(), DurabilityError> { ... }
    pub fn verify(&self) -> Result<(), DurabilityError> { ... }
}
```

### Phase 5: Enforce compile-time matrix completeness
Replace runtime `verify_matrix_completeness()` with a const evaluation:
```rust
const _: () = assert!(
    REQUIRED_PRIMITIVES.len() == DURABILITY_MATRIX.len(),
    "Every required primitive must have a matrix row"
);
```

---

## SUMMARY

| Issue | Severity | Lines Affected |
|-------|----------|----------------|
| Exceeds 300-line mandate | CRITICAL | +67 over |
| Inline tests in production module | HIGH | 99 lines |
| Primitive obsession in `DurabilityRow` | HIGH | 4 fields |
| `String` in `DurabilityError` variants | MEDIUM | 4 variants |
| Naked procedural verification functions | MEDIUM | 5 functions |
| Missing typed value objects | MEDIUM | 3 domain concepts |

**Recommendation**: Accept this file as a **RED phase artifact**. It documents
intent correctly but has structural drift. The inline test extraction (Phase 1)
is the minimum viable fix to achieve compliance. Full value-object refactoring
(Phases 2–5) should be a separate bead.
