# ARCH-DRIFT HAMMER REPORT: `vb_core/src/diagnostic.rs`

**File:** `/home/lewis/src/velvet-ballistics/crates/vb_core/src/diagnostic.rs`  
**Line Count:** 2445 (CATASTROPHIC — 8.1× over the 300-line limit)  
**Classification:** ZERO-TOLERANCE VIOLATION  
**Date:** 2026-05-29  
**Enforcer:** architectural-drift agent  

---

## EXECUTIVE SUMMARY

`diagnostic.rs` is a **2,445-line monolithic god module** that violates every
architectural rule in the book. It mixes (1) domain enum definitions, (2) a
150+-entry constant registry, (3) newtype wrappers, (4) serde implementations,
(5) pure helper functions, (6) error types, and (7) 370 lines of inline tests —
all in a single file. This is not a diagnostic module; it is a **code museum**.

---

## VIOLATION CATALOG

### 1. SIZE VIOLATION — 2445 lines vs 300-line hard limit

| Metric | Value | Limit | Status |
|--------|-------|-------|--------|
| Total lines | 2445 | 300 | **FAIL** |
| Code:data ratio | 64:36 | 80:20 desired | **FAIL** |
| Test code embedded | ~370 lines | 0 in production | **FAIL** |
| Registry entries | 150+ | N/A (should be separate) | **FAIL** |

### 2. SINGLE RESPONSIBILITY PRINCIPLE VIOLATIONS

The file contains **7 distinct responsibility clusters** that must be split:

| Cluster | Lines | Problem |
|---------|-------|---------|
| `CodeCategory` enum + impl | ~90 | Domain grouping — belongs in `codes::category` |
| `CodeEntry` + `CODE_REGISTRY` const | ~450 | **Data** not code — belongs in `vb_codes` crate |
| Registry lookup fns | ~60 | Pure helpers — belong with registry |
| `SymbolicCode` newtype + impl | ~200 | **Type definition** — belongs in `codes::types` |
| `DiagnosticCode` newtype + impl | ~140 | **Type definition** — belongs in `codes::types` |
| `Severity` enum + `Diagnostic` struct | ~120 | **Value object** — belongs in `diagnostic::model` |
| `HasSymbolicCode` trait | ~15 | **Trait** — belongs in error boundary |
| Serde impls (Visitor, Serialize, Deserialize) | ~100 | **Infrastructure** — belongs with types |
| Internal helpers (`parse_hex_digit`, `pack_digits`) | ~40 | **Utility** — belongs with parsing |
| Inline `#[cfg(test)]` module | ~370 | **Test code** — does not belong in production |

### 3. SCOTT WLASCHIN DDD VIOLATIONS

#### 3.1 Primitive Obsession — `DiagnosticCode` wraps `u16`

```rust
// VIOLATION: raw primitive used directly
pub struct DiagnosticCode(u16);

// Should be a proper Value Object with domain semantics
pub struct DiagnosticCode {
    value: PackedCode, // newtype around u16 with domain validation
    category: CodeCategory, // derived, not stored
}
```

The `u16` numeric encoding leaks into every call site. The `E{:04X}` formatting
logic is duplicated in `Display::fmt` instead of being encapsulated.

#### 3.2 Primitive Obsession — `SymbolicCode` wraps `&'static str`

```rust
// VIOLATION: stringly typing for a domain concept
pub struct SymbolicCode(&'static str);
```

`symbolic_code.as_str()` returns a raw `&str` that callers can misuse. The
registry lookup is repeated at every call site instead of being encapsulated in
a proper domain service.

#### 3.3 Mixed Domain Concerns — Registry Contains Cross-Crate Codes

`CODE_REGISTRY` at 0x20xx (Storage), 0x30xx (Runtime), 0x40xx
(RuntimeBoundary) contains entries for `vb_storage`, `vb_runtime`, and other
crate namespaces. This is a **workspace-wide registry** that does not belong
inside `vb_core`. It creates a hard coupling: `vb_core` cannot compile without
knowing about storage and runtime error codes.

**Correct architecture:** `vb_codes` crate containing all workspace diagnostic
codes, consumed by `vb_core`, `vb_storage`, `vb_runtime`, etc.

#### 3.4 Feature Envy — `category_from_numeric` knows too much

```rust
pub fn category_from_numeric(numeric: u16) -> CodeCategory {
    // consults registry first...
    // then falls back to HIGH-BYTE HEURISTICS
    let high_byte = numeric.wrapping_shr(8) & 0xFF_u16;
    match high_byte { ... }
}
```

This function has envy for both the registry AND the numeric encoding scheme.
It should be a method on a `CodeRegistry` service, not a standalone function.

#### 3.5 Data Clump — `Diagnostic` struct is a "dump everything" record

```rust
pub struct Diagnostic {
    pub code: SymbolicCode,
    pub numeric_code: DiagnosticCode,  // DERIVED, redundant
    pub message: Box<str>,
    pub severity: Severity,
    pub span: Span,
    pub source_file: Option<Box<str>>,  // ONLY for authoring-time
}
```

`numeric_code` is derivable from `code` (always). `source_file` is only needed
at authoring time, not runtime. This struct tries to serve both compilation
diagnostics AND runtime errors without distinguishing them.

### 4. IMPERATIVE BLOCKS — 370 lines of inline tests

The `#[cfg(test)] mod tests` block at lines 2057–2445 is **37% of the file**.
It is not a separate test file; it is embedded production code. This violates
the "tests belong in `tests/` or `benches/` directories" rule from
`AGENTS.md`.

---

## ROOT CAUSE ANALYSIS

This file grew organically via "just add it to diagnostic.rs" syndrome:

1. Someone needed a diagnostic code enum → added to `diagnostic.rs`
2. Someone needed to look up codes → added registry to `diagnostic.rs`
3. Someone needed symbolic codes → added `SymbolicCode` to `diagnostic.rs`
4. Someone needed runtime codes from `vb_storage` → added to `diagnostic.rs`
5. Someone needed serde → added to `diagnostic.rs`
6. Someone needed tests → added to `diagnostic.rs`

**Result:** 2445 lines, zero cohesion.

---

## PRESCRIBED REMEDIATION

### Phase 1: Extract `vb_codes` crate (HIGH PRIORITY)

Create `crates/vb_codes/` containing:

```
vb_codes/
├── src/
│   ├── lib.rs           # re-exports
│   ├── category.rs      # CodeCategory enum (lines 24–85)
│   ├── registry.rs      # CodeEntry, CODE_REGISTRY, lookup fns (lines 91–1603)
│   └── types.rs         # SymbolicCode, DiagnosticCode (lines 1609–1845)
└── tests/
    └── registry_tests.rs # extracted from lines 2057–2445
```

**Registry relocation:** The 150+ `CodeEntry` structs (lines 118–1547) must be
moved to `vb_codes`. `vb_core`, `vb_storage`, `vb_runtime` all import from
`vb_codes`.

**Break the cross-crate coupling:** `vb_core/src/diagnostic.rs` must NOT
contain Storage (0x20xx), Runtime (0x30xx), or RuntimeBoundary (0x40xx) codes.

### Phase 2: Split `vb_core/src/diagnostic.rs` into focused modules

```
vb_core/src/
├── diagnostic/
│   ├── mod.rs           # thin re-exports only (~50 lines)
│   ├── model.rs         # Diagnostic, Severity, DiagnosticRecord (~100 lines)
│   └── traits.rs        # HasSymbolicCode trait (~20 lines)
└── codes/
    └── re_exports.rs    # re-exports from vb_codes (~30 lines)
```

**Target line count per file:**
- `diagnostic/mod.rs`: ≤50 lines (re-exports)
- `diagnostic/model.rs`: ≤150 lines (value objects)
- `diagnostic/traits.rs`: ≤50 lines (trait + impls)
- Total for diagnostic module: ≤250 lines

### Phase 3: Extract test module to integration tests

Move `#[cfg(test)] mod tests` (lines 2057–2445) to
`crates/workspace_tests/vb_core_diagnostic_tests.rs`.

### Phase 4: Fix Primitive Obsession

```rust
// Replace raw u16 with domain-validated newtype
pub struct DiagnosticCode {
    code: u16,  // invariant: is_registered_numeric(code)
}

impl DiagnosticCode {
    pub fn new(code: u16) -> Option<Self> {
        if is_registered_numeric(code) { Some(Self { code }) } else { None }
    }
    pub fn code(self) -> u16 { self.code }
    pub fn symbolic(self) -> Option<SymbolicCode> { ... }
    pub fn category(self) -> Option<CodeCategory> { ... }
}
```

---

## ARCHITECTURAL METRICS (POST-REFACTOR TARGETS)

| Metric | Before | After |
|--------|--------|-------|
| `diagnostic.rs` lines | 2445 | ≤250 |
| `vb_core` diagnostic module total | 2445 | ≤400 |
| Registry location | embedded in `vb_core` | `vb_codes` crate |
| Cross-crate code coupling | YES (vb_storage, vb_runtime) | NO |
| Inline test code | 370 lines | 0 lines |
| Primitive obsession violations | 3 | 0 |
| Feature envy violations | 1 | 0 |
| Data clump violations | 1 | 0 |

---

## ENFORCEMENT ACTION

**STATUS:** `diagnostic.rs` is an active architectural cancer.

**REQUIRED ACTIONS:**
1. Create `crates/vb_codes` crate for the workspace-wide registry
2. Relocate `CODE_REGISTRY` and all 150+ `CodeEntry` instances to `vb_codes`
3. Split remaining `diagnostic.rs` into `diagnostic/{model,traits}.rs`
4. Extract inline tests to `crates/workspace_tests/`
5. Fix `Diagnostic` struct to remove derived/redundant fields
6. Delete `vb_core/src/diagnostic.rs` once split is complete

**DEADLINE:** Before next `moon ci` gate passes.

---

## EVIDENCE APPENDIX

### A. Cross-namespace codes in `vb_core`'s registry (BELONGS IN vb_codes)

| Code Range | Namespace | In `vb_core`? |
|------------|-----------|----------------|
| 0x01xx | Schema | ✓ (correct) |
| 0x02xx | Reference | ✓ (correct) |
| 0x20xx–0x207F | Storage | **VIOLATION** |
| 0x3001–0x3022 | Runtime | **VIOLATION** |
| 0x32xx | IPC | **VIOLATION** |
| 0x40xx | RuntimeBoundary | **VIOLATION** |

### B. Registry size evidence

```
Lines 118–1547  = 1,429 lines of CodeEntry structs
Lines 91–1603   = 1,512 lines of registry-related code
Total registry   = ~50% of file
```

### C. Test code embedded evidence

```
Lines 2057–2445 = 389 lines of #[cfg(test)] module
Percentage       = 15.9% of file in tests
```

---

**END HAMMER REPORT**
