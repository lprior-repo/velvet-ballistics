# Domain Model — Section 16 Symbolic Diagnostic Codes

**Bead**: vb-xi2f.10  
**Phase**: State 3 — Rust Contract  
**Date**: 2026-05-24

---

## 1. Ubiquitous Language

| Term | Definition | Current State | Target State |
|------|-----------|---------------|-------------|
| **Symbolic Diagnostic Code** | A stable, human-readable, machine-consumable error identifier (e.g., `DUPLICATE_KEY`). The **primary** public contract. | Exists only as thiserror `#[error("DUPLICATE_KEY")]` display strings and `CompileError::code() -> &'static str`. Not a first-class type. | First-class type `SymbolicCode` that cannot represent invalid codes at the type level. |
| **Numeric Diagnostic Code** | An internal packed `u16` encoding in E-hex format (e.g., `E0101` = `0x0101`). Category-encoding via high byte. | `DiagnosticCode(u16)` — the **only** stable public type. Publically visible in `Diagnostic` struct, parsable from string, and used by `CoreError`, `JournalError`, `RuntimeError`. | Internal implementation detail. Retained for wire/serialization efficiency but hidden from symbolic-code public contract. |
| **Diagnostic Record** | A user-facing error record: symbolic code + message + severity + source span. | `Diagnostic { code: DiagnosticCode(u16), message: Box<str>, severity: Severity, span: Span }`. Public type. | `Diagnostic` carries symbolic code as primary identifier; numeric code is derived/optional. |
| **Code Registry** | The authoritative, single-source-of-truth mapping between symbolic names and numeric encodings. | None. Each crate defines its own constants. Two crates (`vb_validate/diagnostic.rs`, `vb_validate/diag_codes.rs`) duplicate the same 58 constants. | One `vb_core::diagnostic::CODE_MAP` or equivalent const-validated registry. |
| **Supported Code Range** | The set of numeric codes that `is_supported_code()` considers valid for parsing. | E01xx, E02xx, E03xx, E04xx, E10xx–E14xx, E20xx, E30xx, E40xx. Missing E05xx, E06xx, codes above 0x401B. | All existing code ranges present in the codebase. |

---

## 2. Value Objects

### 2.1 `SymbolicCode`

**Purpose**: The primary, stable public diagnostic code type. Carries a symbolic string from the Section 16/17 matrix.

```rust
/// Stable symbolic diagnostic code (e.g., "DUPLICATE_KEY").
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SymbolicCode(&'static str);
```

**Invariants**:
- INV-SYM-1: The contained `&'static str` must be a member of the canonical code registry.
- INV-SYM-2: Every `SymbolicCode` must round-trip to exactly one numeric `u16` encoding and back.
- INV-SYM-3: `SymbolicCode` cannot be constructed from an arbitrary `&'static str` outside the registry (smart constructor required).
- INV-SYM-4: `SymbolicCode` is `Copy`, `Eq`, `Ord`, `Hash`, `Send`, `Sync`, with zero heap allocation.

**Serialization**: The string representation is the symbolic name (e.g., `"DUPLICATE_KEY"`), not the E-hex form. The E-hex form is available via a separate `numeric_code() -> u16` method.

### 2.2 `DiagnosticCode` (evolved)

**Purpose**: Retained as an internal encoding type. Extended to carry an optional symbolic reference.

**Invariants** (existing plus new):
- INV-DC-1 (existing): The packed `u16` is the canonical numeric encoding.
- INV-DC-2 (new): Every `DiagnosticCode` must be constructable from its corresponding `SymbolicCode` and vice versa.
- INV-DC-3 (new): `is_supported_code()` must include ALL numeric codes used by any error type in the workspace.
- INV-DC-4 (existing retained): `DiagnosticCode::new(0)` is valid only if it maps to a supported code (currently zero is not supported; this invariant is preserved).

### 2.3 `Diagnostic` Record (evolved)

**Purpose**: User-facing error record.

**Change**: The `code` field transitions from `DiagnosticCode` (numeric-only) to `SymbolicCode` (symbolic) as the primary identifier. A `numeric_code: DiagnosticCode` field may be derived for backward-compatible consumers.

### 2.4 `CodeRegistry` (new)

**Purpose**: The single source of truth for all known diagnostic codes.

**Shape**: A const-validated mapping from symbolic name → numeric encoding. Could be an enum, a const `phf` map, or a generated match function.

**Invariants**:
- INV-REG-1: Every symbolic code in the registry has exactly one numeric encoding.
- INV-REG-2: Every numeric encoding in the registry has exactly one symbolic code.
- INV-REG-3: The registry is exhaustive for all Section 16 (36 codes), Section 17 (33 codes), gate verifier (19+ codes), contract-discovery (3+ codes), and storage (28 codes).
- INV-REG-4: The registry is defined in `vb_core` to serve as the cross-crate authority.

---

## 3. Entities and Aggregates

### 3.1 Error-Code Aggregate

Each error type forms an aggregate with its diagnostic code:

```
ValidationError ──owns──→ SymbolicCode
CompileError    ──owns──→ SymbolicCode  (already has code() -> &'static str)
YamlError       ──owns──→ SymbolicCode  (new)
CoreError       ──owns──→ SymbolicCode  (has numeric; needs symbolic)
RuntimeError    ──owns──→ SymbolicCode  (has runtime_code() for Section 17; map to SymbolicCode)
JournalError    ──owns──→ SymbolicCode  (has numeric; needs symbolic)
```

**Aggregate invariant**: For every error variant, `error.symbolic_code()` returns a `SymbolicCode` that is (a) a member of the registry, (b) consistent with the variant's semantics, and (c) stable across releases.

---

## 4. Commands and Events

### Commands (what the system is asked to do)

| Command | Description |
|---------|-------------|
| `ResolveSymbolicCode(error)` | Given an error value, return its `SymbolicCode`. |
| `FormatDiagnostic(code, message, span)` | Produce a user-facing diagnostic record. |
| `ParseNumericCode(input: &str)` | Parse a `"E0101"`-style string into a `DiagnosticCode`. |
| `LookupSymbolicCode(numeric: u16)` | Reverse-lookup: numeric → symbolic. |
| `ValidateCodeConsistency()` | Assert registry bijection holds. |

### Events (what happened)

| Event | Description |
|-------|-------------|
| `DiagnosticEmitted { code: SymbolicCode, severity, span }` | A diagnostic was produced and logged/displayed. |
| `CodeRegistryValidated { total_codes, range_count }` | The registry passed a consistency audit. |

---

## 5. Policies

| Policy | Rule |
|--------|------|
| **Stable Contract**: SymbolicCode | The set of symbolic codes is append-only. Existing codes never change their symbolic name or numeric encoding. |
| **No Stringly Errors**: Error variants | Every error variant exposed to consumers must carry a `SymbolicCode`. No parsing of display strings to extract codes. |
| **Single Registry**: Registry location | `vb_core` owns the canonical code registry. All other crates reference it; none may define independent registries. |
| **Backward Compat**: DiagnosticCode | The existing `DiagnosticCode(u16)` type must remain functional and parseable. Consumers that receive numeric codes continue to work. |
| **Zero-Alloc Hot Path**: DiagnosticCode | Constructing a `DiagnosticCode` or `SymbolicCode` in the hot path must not allocate. |
| **Exhaustive Match**: Error → SymbolicCode | Every error variant must have a deterministic mapping to exactly one `SymbolicCode`. |

---

## 6. Invariants (Global)

| ID | Invariant |
|----|-----------|
| G-INV-1 | `SymbolicCode` registry is a bijection between symbolic names and numeric codes. |
| G-INV-2 | Every error type that appears in the public API surface has a public `symbolic_code() -> SymbolicCode` method. |
| G-INV-3 | The `Diagnostic` struct carries symbolic code as its primary code field. |
| G-INV-4 | All 36 Section 16 symbolic codes, 33 Section 17 codes, 19+ gate-verifier codes, 3+ contract-discovery codes, and 28 storage codes are registered in `vb_core`. |
| G-INV-5 | `is_supported_code()` accepts exactly the union of all numeric codes used in the workspace. No valid numeric code is rejected; no invalid numeric code is accepted. |
| G-INV-6 | Backward compatibility: `DiagnosticCode::from_str("E0101")` returns Ok, and `error_code(&ValidationError::DuplicateKey).code()` returns `0x0101`. |
| G-INV-7 | `SymbolicCode::from("DUPLICATE_KEY")` is the canonical way to obtain the symbolic code; any future code added to the registry makes this compile/parse. |

---

## 7. Forbidden States (Made Unrepresentable)

| Forbidden State | How Prevented |
|----------------|---------------|
| SymbolicCode containing an unregistered string | Smart constructor or const-validated compile-time check. |
| Numeric code without a symbolic counterpart | Registry bijection enforced by tests and compile-time checks. |
| Two symbolic codes mapping to the same numeric code | Registry uniqueness assertions. |
| ValidationError variant with no code() method | Trait or inherent method on the enum; exhaustive match guarantees coverage. |
| is_supported_code() rejecting a valid in-use code | Test encodes every used code constant and asserts all are accepted. |
| Parsing an unsupported E-style string succeeding | `FromStr` checks `is_supported_code()` guard. |

---

## 8. Open Domain Questions

1. **SymbolicCode construction safety**: Should `SymbolicCode` be an enum (exhaustive compile-time check) or a newtype with a runtime `try_from`? An enum provides compile-time exhaustiveness but requires all consumers to match. A newtype with a const function constructor is more ergonomic but requires the registry to be const-checked.

2. **Diagnostic struct migration**: Should `Diagnostic.code` change type from `DiagnosticCode(u16)` to `SymbolicCode`, or should the `Diagnostic` have both `code: SymbolicCode` and `numeric_code: DiagnosticCode`? The former is cleaner but breaks consumers that access `.code.code()`. The latter preserves compatibility.

3. **YamlError symbolic codes**: Which Section 16 codes map to YamlError variants? Some YamlError variants (e.g., `DuplicateKey`, `ForbiddenFeature`) have direct Section 16 equivalents. Others (e.g., `AmbiguousScalar`, `BinaryScalar`) may map to `FORBIDDEN_YAML_FEATURE`. Need explicit mapping design.

4. **Section 17 runtime codes**: Should `SymbolicCode` cover Section 17 codes (e.g., `INPUT_MAPPING_FAILED`, `QUEUE_FULL`) or should those remain a separate namespace? Given they are diagnostic codes with numeric mappings, a unified `SymbolicCode` is coherent.

5. **CompileError code() granularity**: Currently `CompileError::code()` maps multiple error variants to the same symbolic code (e.g., 8 variants map to `"FORBIDDEN_YAML_FEATURE"`). Is this the desired contract, or should each variant have a distinct symbolic code?
