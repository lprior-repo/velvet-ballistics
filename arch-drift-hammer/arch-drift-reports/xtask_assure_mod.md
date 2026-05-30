# Architectural Drift Report: `xtask/src/assure/`

**Analyzed Path:** `/home/lewis/src/velvet-ballistics/xtask/src/assure/`
**Report Generated:** 2026-05-29
**Priority:** HIGH

---

## 1. Line Count Analysis

| File | Lines |
|------|-------|
| `contract.rs` | 180 |
| `ir.rs` | 191 |
| `path.rs` | 206 |
| `policy.rs` | 201 |
| `schema.rs` | 213 |
| **`mod.rs`** | **MISSING** |
| **TOTAL** | **991** |

**Status:** ❌ FAILS - Total exceeds 300 line limit (991 > 300)

---

## 2. DDD Cohesion Analysis

### Domain Boundary Review

The `assure/` directory implements an **Assurance Contract** bounded context with the following entities:

| Entity | Responsibility | File |
|--------|----------------|------|
| `AssuranceIr` | IR builder: contract → typed IR | `ir.rs` |
| `ContractClause` | Decision table clause (when/then) | `schema.rs` |
| `TypedExpr` | Expression AST for conditions | `schema.rs` |
| `DomainType` | Finite domain variants | `schema.rs` |
| `PathCheckResult` | Decision path validation | `path.rs` |
| `PolicyViolation` | Static policy AST-grep rules | `policy.rs` |
| `OracleRecord` | Provenance tracking | `schema.rs` |
| `EvidenceRecord` | Content-addressed evidence | `schema.rs` |

### Cohesion Issues

1. **Missing Module Entry Point (CRITICAL)**
   - No `mod.rs` exists to declare submodules
   - `ir.rs`, `path.rs`, `policy.rs` all use `super::error::AssurecError` but `error.rs` does not exist
   - The module cannot compile as-is

2. **God Module Tendency**
   - `schema.rs` at 213 lines contains 15+ distinct types
   - Should be split into: `expr.rs`, `clause.rs`, `oracle.rs`, `evidence.rs`

3. **Impedance Mismatch**
   - `contract.rs` implements tenant-specific business logic (`tenant_claim_rel_domain`, `membership_fact_domain`)
   - `policy.rs` implements generic AST-grep rules
   - These belong in separate bounded contexts

---

## 3. Violations

### Critical (Must Fix)

| # | Violation | Location | Description |
|---|-----------|----------|-------------|
| 1 | **Missing `mod.rs`** | `xtask/src/assure/` | Directory module has no entry point; cannot compile |
| 2 | **Missing `error.rs`** | `xtask/src/assure/` | Referenced as `super::error::AssurecError` but never defined |
| 3 | **Cross-module import error** | `ir.rs:4`, `path.rs:4`, `policy.rs:3` | Import `super::error::AssurecError` but no error module exists |

### High (Should Fix)

| # | Violation | Location | Description |
|---|-----------|----------|-------------|
| 4 | **File size: schema.rs** | 213 lines | Exceeds 300-line limit per file |
| 5 | **File size: path.rs** | 206 lines | Exceeds 300-line limit per file |
| 6 | **File size: policy.rs** | 201 lines | Exceeds 300-line limit per file |
| 7 | **Single Responsibility Violation** | `schema.rs` | 15+ types bundled; violates SRP |

### Medium (Consider Fixing)

| # | Violation | Location | Description |
|---|-----------|----------|-------------|
| 8 | **Leaky Abstraction** | `contract.rs` | Hardcoded tenant domain logic leaks into generic IR |
| 9 | **Temporal Coupling** | `ir.rs:181-184` | Test depends on `crate::assure::contract::` being defined |
| 10 | **DDD Context Misalignment** | `policy.rs` | AST-grep rules are infrastructure, not domain |

---

## 4. DDD Smell Assessment

| Smell | Severity | Evidence |
|-------|----------|----------|
| **Anemic Domain Model** | Medium | `schema.rs` is data-only; behavior scattered in `ir.rs`, `path.rs`, `policy.rs` |
| **Feature Envy** | Medium | `ir.rs` `from_contract()` extracts variables from clauses but doesn't own them |
| **Shotgun Surgery** | Low | Adding a new `TypedExpr` variant requires editing `ir.rs`, `path.rs`, `schema.rs` |
| **God Module** | High | `schema.rs` owns 15+ types spanning multiple subdomains |

---

## 5. Recommended Fixes

### Immediate (Priority 1)

```rust
// xtask/src/assure/mod.rs (CREATE THIS FILE)
pub mod contract;
pub mod error;  // CREATE error.rs
pub mod ir;
pub mod path;
pub mod policy;
pub mod schema;
```

```rust
// xtask/src/assure/error.rs (CREATE THIS FILE)
#[derive(Debug, Clone)]
pub enum AssurecError {
    Ir(String),
    Contract(String),
    Policy(String),
}
```

### Short-term (Priority 2)

1. Split `schema.rs` into:
   - `expr.rs` (TypedExpr, DomainType)
   - `clause.rs` (ContractClause, EffectOutcome, PathOutcome)
   - `oracle.rs` (OracleRecord, OracleProvenance, OracleProvenanceKind)
   - `evidence.rs` (EvidenceRecord, GeneratedArtifact, RepairPacket, etc.)

2. Move `policy.rs` to `xtask/src/policy/` or similar infrastructure module

### Long-term (Priority 3)

- Extract tenant-specific contract logic (`contract.rs`) into `vb_tenant_access` crate
- Generalize IR builder to be domain-agnostic

---

## 6. Summary

| Metric | Value | Threshold | Status |
|--------|-------|-----------|--------|
| Total Lines | 991 | < 300 | ❌ FAIL |
| Max File Size | 213 | < 300 | ❌ FAIL |
| Module Entry Point | MISSING | REQUIRED | ❌ FAIL |
| Error Module | MISSING | REQUIRED | ❌ FAIL |
| DDD Cohesion | LOW | HIGH | ❌ FAIL |
| **Overall** | — | — | **❌ CRITICAL** |

**Recommendation:** Do not merge until `mod.rs` and `error.rs` are created and the missing cross-module imports are resolved. The current state will not compile.
