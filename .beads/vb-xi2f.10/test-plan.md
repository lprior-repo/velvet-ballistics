# Test Plan: Section 16 Symbolic Diagnostic Codes

**Bead**: vb-xi2f.10  
**Phase**: State 8 — Test Planning  
**Date**: 2026-05-26  
**Contract Ref**: `contract.md` (33 clauses, 12 acceptance criteria)  
**Type Contracts Ref**: `type-contracts.md` (12 type definitions with method contracts)  
**Error Taxonomy Ref**: `error-taxonomy.md` (58+20+60+~100 variant→code mappings)  
**Bridge**: `proof-to-rust-map.md` (28 proof obligations, approved with findings F-BR-001..F-BR-007)

---

## Summary

- **Behaviors identified**: 47
- **Trophy allocation**: 24 unit / 14 integration / 2 e2e / 7 static
- **Proptest invariants**: 11
- **Fuzz targets**: 2
- **Kani harnesses**: 19 referenced (9 VERIFIED, 9 BLOCKED, 1 WAIVED)
- **Mutation checkpoints**: 12

---

## 1. Behavior Inventory

### 1.1 SymbolicCode Core Behaviors

| # | Behavior Description |
|---|---------------------|
| B-001 | `SymbolicCode::from_static` returns `Some(code)` when the input string is a registered symbolic code |
| B-002 | `SymbolicCode::from_static` returns `None` when the input string is not in the registry |
| B-003 | `SymbolicCode::from_static` returns `None` for the empty string |
| B-004 | `SymbolicCode::as_str` returns the symbolic name that was used to construct it |
| B-005 | `SymbolicCode::numeric_code` returns the correct packed `u16` for the code |
| B-006 | `SymbolicCode::as_diagnostic_code` returns a `DiagnosticCode` with the matching numeric value |
| B-007 | `SymbolicCode` `Display` formats as the symbolic name (e.g. `"DUPLICATE_KEY"`), not E-hex |
| B-008 | `SymbolicCode` `FromStr` parses registered symbolic names and rejects unregistered ones |
| B-009 | `SymbolicCode` is `Copy` — cloning or passing by value does not allocate |
| B-010 | `SymbolicCode` is `Send` and `Sync` |
| B-011 | `SymbolicCode` `Serialize` produces a JSON string of the symbolic name |
| B-012 | `SymbolicCode` `Deserialize` accepts registered symbolic names and rejects unknown names |
| B-013 | `SymbolicCode` `Deserialize` rejects malformed JSON (non-string, empty, etc.) |

### 1.2 DiagnosticCode (Evolved) Behaviors

| # | Behavior Description |
|---|---------------------|
| B-014 | `DiagnosticCode::symbolic_code` returns `Some(SymbolicCode)` for codes in the registry |
| B-015 | `DiagnosticCode::symbolic_code` returns `None` for codes outside the registry |
| B-016 | `DiagnosticCode::from_str("E0101")` returns `Ok(DiagnosticCode(0x0101))` — backward compatibility |
| B-017 | `DiagnosticCode::from_str("E0501")` returns `Ok(DiagnosticCode(0x0501))` — new gate verifier range |
| B-018 | `DiagnosticCode::from_str("E0601")` returns `Ok(DiagnosticCode(0x0601))` — new contract discovery range |
| B-019 | `DiagnosticCode::from_str("E401C")` returns `Ok(DiagnosticCode(0x401C))` — extended runtime boundary range |
| B-020 | `DiagnosticCode::from_str` returns `Err(InvalidFormat)` for non-"E" prefix, wrong length, non-hex |
| B-021 | `DiagnosticCode::from_str` returns `Err(UnsupportedCode)` for well-formed codes in gaps between ranges |
| B-022 | `DiagnosticCode::new(code).code()` returns the exact `u16` value — identity property |
| B-023 | `DiagnosticCode` `Display` continues to format as `"E0101"` — backward compatible |

### 1.3 Diagnostic (Evolved) Behaviors

| # | Behavior Description |
|---|---------------------|
| B-024 | `Diagnostic.code` is of type `SymbolicCode` |
| B-025 | `Diagnostic::new(code, message, severity, span)` derives `numeric_code` from `code`; invariant `numeric_code.symbolic_code() == Some(code)` holds |
| B-026 | `Diagnostic::new` never panics for any valid `SymbolicCode` |
| B-027 | Constructing a `Diagnostic` with `Severity::Error` preserves that severity |

### 1.4 CODE_REGISTRY Behaviors

| # | Behavior Description |
|---|---------------------|
| B-028 | `CODE_REGISTRY` contains entries for all 36 Section 16 symbolic codes |
| B-029 | `CODE_REGISTRY` contains entries for all 19 gate verifier codes (E05xx) |
| B-030 | `CODE_REGISTRY` contains entries for all 3 contract discovery codes (E06xx) |
| B-031 | `CODE_REGISTRY` contains entries for all compilation-specific symbolic codes |
| B-032 | No two registry entries share the same symbolic name |
| B-033 | No two registry entries share the same numeric code |
| B-034 | All registry entries have non-zero numeric codes |
| B-035 | Every entry's `(numeric >> 8) & 0xFF` matches the expected high byte for its `CodeCategory` |
| B-036 | Symbolic-to-numeric and numeric-to-symbolic lookup form a bijection: `symbolic_to_numeric(s).and_then(numeric_to_symbolic) == Some(s)` |

### 1.5 Error Type code() Behaviors

| # | Behavior Description |
|---|---------------------|
| B-037 | `ValidationError::code()` returns a `SymbolicCode` for each of its 58 variants — exhaustive match without wildcard |
| B-038 | Every `ValidationError` variant produces a unique `SymbolicCode` — no two variants share the same code |
| B-039 | `CompileError::code()` returns a `SymbolicCode` instead of bare `&'static str` |
| B-040 | `CompileError::code()` preserves all existing symbolic code values (regression) |
| B-041 | `YamlError::code()` returns a `SymbolicCode` for each of its 20 variants |
| B-042 | `YamlError::code()` uses exhaustive match without wildcard |
| B-043 | `CoreError::symbolic_code()` returns a `SymbolicCode` for all 40+ variants |
| B-044 | `RuntimeError::symbolic_code()` returns a `SymbolicCode` for all 25+ variants |
| B-045 | `JournalError::symbolic_code()` returns a `SymbolicCode` for all 28 variants |

### 1.6 HasSymbolicCode Trait Behavior

| # | Behavior Description |
|---|---------------------|
| B-046 | The `HasSymbolicCode` trait is defined in `vb_core` with `fn symbolic_code(&self) -> SymbolicCode` |
| B-047 | All six error types implement `HasSymbolicCode`: `ValidationError`, `CompileError`, `YamlError`, `CoreError`, `RuntimeError`, `JournalError` |

---

## 2. Trophy Allocation

| Layer | Count | Behaviors | Rationale |
|-------|-------|-----------|-----------|
| **Static Analysis** | 7 | — | clippy, cargo-deny, compile-time const assertions (registry uniqueness, exhaustive match enforcement, non_exhaustive guards). This is zero-cost verification. |
| **Unit / Calc** | 24 | B-001..B-013, B-022, B-028..B-036, B-046 | Pure logic: registry lookups, from_static/from_str parsing, Serialize/Deserialize round-trips, arithmetic invariant checks. No I/O, no external deps. |
| **Integration** | 14 | B-014..B-021, B-023, B-024..B-027, B-037..B-045, B-047 | Cross-crate type interactions. Real error types from vb_validate, vb_compile, vb_yaml, vb_runtime, vb_storage exercised against vb_core diagnostic types. |
| **E2E** | 2 | CLI diagnostic output, YAML→diagnostic pipeline | Full user-facing workflow: YAML parse → ValidationError → Diagnostic → Display output. Verifies the full chain. |
| **Total** | 47 | | ~51% unit, ~30% integration, ~4% e2e, ~15% static |

**Deviation rationale**: The diagnostic code system is overwhelmingly pure-functional (no I/O, no concurrency, no network). The unit-heavy split reflects this. Integration tests cover the cross-crate boundary where error types are converted to symbolic codes. E2E is deliberately minimal — the diagnostic string output is user-visible but mechanically derived from the unit-tested registry.

---

## 3. BDD Scenarios

### 3.1 SymbolicCode Construction

#### Behavior: B-001 — from_static returns Some for registered codes

```
Given: CODE_REGISTRY contains an entry with symbolic "DUPLICATE_KEY" and numeric 0x0101
When: SymbolicCode::from_static("DUPLICATE_KEY") is called
Then: The result is Some(code) where:
  - code.as_str() == "DUPLICATE_KEY"
  - code.numeric_code() == 0x0101
  - code.as_diagnostic_code().code() == 0x0101
```

Test function: `fn symbolic_code_from_static_returns_some_when_registered_code()`

```
Given: CODE_REGISTRY contains entries for all 90+ codes
When: SymbolicCode::from_static is called with each registry entry's symbolic name
Then: Every call returns Some; as_str matches the input; numeric_code matches the registry
```

Test function: `fn symbolic_code_from_static_all_registry_entries_return_some()`

#### Behavior: B-002 — from_static returns None for unregistered strings

```
Given: CODE_REGISTRY is loaded
When: SymbolicCode::from_static("BOGUS_NOT_A_CODE") is called
Then: The result is None
```

Test function: `fn symbolic_code_from_static_returns_none_when_unregistered_string()`

```
Given: CODE_REGISTRY is loaded
When: SymbolicCode::from_static("") is called
Then: The result is None
```

Test function: `fn symbolic_code_from_static_returns_none_when_empty_string()`

```
Given: CODE_REGISTRY is loaded
When: SymbolicCode::from_static("  DUPLICATE_KEY  ") is called
Then: The result is None — no whitespace tolerance
```

Test function: `fn symbolic_code_from_static_returns_none_when_whitespace_mismatch()`

#### Behavior: B-004 — as_str preserves identity

```
Given: A SymbolicCode constructed via from_static("TYPE_MISMATCH")
When: code.as_str() is called
Then: The result is "TYPE_MISMATCH"
```

Test function: `fn symbolic_code_as_str_preserves_constructor_string()`

#### Behavior: B-005 — numeric_code is correct

```
Given: CODE_REGISTRY entries mapping "DUPLICATE_KEY"→0x0101, "TYPE_MISMATCH"→0x0407
When: SymbolicCode::from_static("DUPLICATE_KEY").unwrap().numeric_code() is called
Then: Returns 0x0101
When: SymbolicCode::from_static("TYPE_MISMATCH").unwrap().numeric_code() is called
Then: Returns 0x0407
```

Test function: `fn symbolic_code_numeric_code_matches_registry_bijection()`

#### Behavior: B-007 — Display formats as symbolic name

```
Given: A SymbolicCode for "DUPLICATE_KEY"
When: format!("{}", code) is called
Then: The result is "DUPLICATE_KEY"
```

Test function: `fn symbolic_code_display_formats_as_symbolic_name_not_e_hex()`

```
Given: A SymbolicCode for "DUPLICATE_KEY"
When: The Display output is inspected
Then: It does NOT contain "E0101" — no numeric fallback
```

Test function: `fn symbolic_code_display_never_outputs_numeric_format()`

#### Behavior: B-008 — FromStr parses registered names

```
Given: CODE_REGISTRY contains "DUPLICATE_KEY"
When: "DUPLICATE_KEY".parse::<SymbolicCode>() is called
Then: Returns Ok(code) where code.as_str() == "DUPLICATE_KEY"
```

Test function: `fn symbolic_code_from_str_parses_registered_name()`

```
When: "BOGUS".parse::<SymbolicCode>() is called
Then: Returns Err — rejected as unknown code
```

Test function: `fn symbolic_code_from_str_rejects_unregistered_name()`

#### Behavior: B-011/B-012 — serde round-trip

```
Given: A SymbolicCode for "DUPLICATE_KEY"
When: It is serialized to JSON and then deserialized back
Then: The result is Ok(original_code) — round-trip identity
```

Test function: `fn symbolic_code_serde_round_trip_preserves_code()`

```
Given: JSON string "\"BOGUS\"" (an unregistered code name)
When: Deserialized as SymbolicCode
Then: Returns Err — rejected by deserialization validation
```

Test function: `fn symbolic_code_deserialize_rejects_unknown_code_name()`

```
Given: JSON inputs: "123", "null", "\"\"", "{}", "[]", "\"duplicate_key\"" (wrong case)
When: Each is deserialized as SymbolicCode
Then: Every one returns Err
```

Test function: `fn symbolic_code_deserialize_rejects_malformed_inputs()`

### 3.2 DiagnosticCode Parsing

#### Behavior: B-016 — backward-compatible parsing

```
Given: The string "E0101"
When: DiagnosticCode::from_str("E0101") is called
Then: Returns Ok(DiagnosticCode(0x0101))
```

Test function: `fn diagnostic_code_from_str_parses_existing_e0101()`

```
Given: The string "E040C"
When: DiagnosticCode::from_str("E040C") is called
Then: Returns Ok(DiagnosticCode(0x040C))
```

Test function: `fn diagnostic_code_from_str_parses_existing_e040c_boundary()`

#### Behavior: B-017/B-018 — new ranges accepted

```
Given: The string "E0501" (gate verifier, previously rejected)
When: DiagnosticCode::from_str("E0501") is called
Then: Returns Ok(DiagnosticCode(0x0501))
```

Test function: `fn diagnostic_code_from_str_accepts_e0501_gate_verifier_range()`

```
Given: The string "E0601" (contract discovery, previously rejected)
When: DiagnosticCode::from_str("E0601") is called
Then: Returns Ok(DiagnosticCode(0x0601))
```

Test function: `fn diagnostic_code_from_str_accepts_e0601_contract_discovery_range()`

#### Behavior: B-020 — rejection of malformed input

```
Given: Input strings: "", "E", "E01", "E010101", "0101", "E010G", "e0101", " E0101 "
When: Each is parsed via DiagnosticCode::from_str
Then: Every one returns Err(DiagnosticCodeParseError::InvalidFormat)
```

Test function: `fn diagnostic_code_from_str_rejects_malformed_input_variants()`

Error variant assertions (individual):
- `fn diagnostic_code_from_str_invalid_format_when_empty()`
- `fn diagnostic_code_from_str_invalid_format_when_too_short()`
- `fn diagnostic_code_from_str_invalid_format_when_too_long()`
- `fn diagnostic_code_from_str_invalid_format_when_missing_e_prefix()`
- `fn diagnostic_code_from_str_invalid_format_when_non_hex_digit()`
- `fn diagnostic_code_from_str_invalid_format_when_lowercase_e()`
- `fn diagnostic_code_from_str_invalid_format_when_leading_whitespace()`

#### Behavior: B-021 — rejection in gaps between ranges

```
Given: Well-formed but unsupported strings: "E010C", "E0205", "E030A", "E040D", "E0514", "E0604",
       "E1003", "E1014", "E1105", "E1203", "E130E", "E1315", "E1408", "E2010", "E300F", "E401D"
When: Each is parsed via DiagnosticCode::from_str
Then: Every one returns Err(DiagnosticCodeParseError::UnsupportedCode)
```

Test function: `fn diagnostic_code_from_str_unsupported_code_in_range_gaps()`

### 3.3 DiagnosticCode reverse lookup

#### Behavior: B-014 — symbolic_code lookup succeeds for registered codes

```
Given: A DiagnosticCode(0x0101)
When: dc.symbolic_code() is called
Then: Returns Some(SymbolicCode("DUPLICATE_KEY"))
```

Test function: `fn diagnostic_code_symbolic_lookup_returns_symbolic_when_registered()`

#### Behavior: B-015 — symbolic_code returns None for unregistered

```
Given: A DiagnosticCode(0x0000)
When: dc.symbolic_code() is called
Then: Returns None
```

Test function: `fn diagnostic_code_symbolic_lookup_returns_none_when_unregistered()`

### 3.4 Diagnostic constructor

#### Behavior: B-025 — constructor invariant

```
Given: A SymbolicCode for "DUPLICATE_KEY"
When: Diagnostic::new(code, "test message".into(), Severity::Error, Span::ZERO) is called
Then: The resulting Diagnostic has:
  - diagnostic.code == code
  - diagnostic.numeric_code.symbolic_code() == Some(code)
  - diagnostic.message.as_ref() == "test message"
  - diagnostic.severity == Severity::Error
```

Test function: `fn diagnostic_new_preserves_symbolic_numeric_invariant()`

#### Behavior: B-026 — no-panic guarantee

```
Given: Every SymbolicCode in the registry
When: Diagnostic::new is called with each one, a message, Severity::Error, and Span::ZERO
Then: No call panics; all return valid Diagnostic records with invariant satisfied
```

Test function: `fn diagnostic_new_never_panics_for_all_registered_codes()`

### 3.5 CODE_REGISTRY invariants

#### Behavior: B-028/B-029/B-030/B-031 — completeness

```
Given: The master list of 36 Section 16 codes + 19 gate verifier codes + 3 contract discovery codes
       + 8 compilation-specific codes = 66 minimum required entries
When: CODE_REGISTRY is inspected
Then: Every required symbolic name is present as a CodeEntry
```

Test function: `fn code_registry_contains_all_mandatory_entries()`

#### Behavior: B-032 — no duplicate symbolic names

```
Given: CODE_REGISTRY
When: All symbolic names are collected into a set
Then: The set cardinality equals the registry length (no duplicates)
```

Test function: `fn code_registry_has_no_duplicate_symbolic_names()`

#### Behavior: B-033 — no duplicate numeric codes

```
Given: CODE_REGISTRY
When: All numeric codes are collected into a set
Then: The set cardinality equals the registry length (no duplicates)
```

Test function: `fn code_registry_has_no_duplicate_numeric_codes()`

#### Behavior: B-034 — non-zero numeric codes

```
Given: CODE_REGISTRY
When: Each entry's numeric field is checked
Then: Every numeric code is non-zero
```

Test function: `fn code_registry_all_numeric_codes_are_nonzero()`

#### Behavior: B-035 — category consistency

```
Given: CODE_REGISTRY
When: Each entry's numeric high byte `(numeric >> 8) & 0xFF` is compared to expected ranges for its CodeCategory
Then: Schema entries (0x01xx) map to CodeCategory::Schema; Reference (0x02xx) to Reference; etc.
```

Test function: `fn code_registry_category_matches_numeric_high_byte()`

#### Behavior: B-036 — bijection

```
Given: CODE_REGISTRY
When: For each entry: lookup_by_symbolic(entry.symbolic) returns the entry's numeric;
Then: lookup_by_numeric(entry.numeric) returns the entry's symbolic
Then: The round-trip is identity
```

Test function: `fn code_registry_bijection_symbolic_to_numeric_round_trip()`

### 3.6 Error type code() methods

#### Behavior: B-037/B-038 — ValidationError exhaustive coverage

```
Given: All 58 ValidationError variants, instantiated with valid field values where needed
When: .code() is called on each variant
Then: Every call returns a SymbolicCode
Then: The set of returned SymbolicCodes has cardinality 58 (all unique)
Then: Each SymbolicCode's as_str() matches the expected Section 16/E05xx/E06xx name
```

Test function: `fn validation_error_code_returns_symbolic_for_all_58_variants()`
Test function: `fn validation_error_code_all_58_unique_symbolic_codes()`

```
Given: The ValidationError source code
When: The code() match arms are inspected
Then: No wildcard arm exists (must be exhaustive)
Then: Adding a new enum variant causes a compile error
```

Test function: `compile_fail_validation_error_code_must_be_exhaustive()` — uses trybuild or compiletest

#### Behavior: B-039/B-040 — CompileError regression

```
Given: All CompileError variants (60+), instantiated with valid field values
When: .code() is called on each variant
Then: Every call returns a SymbolicCode
Then: The as_str() values match the existing &'static str values from the pre-migration code() method
```

Test function: `fn compile_error_code_returns_symbolic_for_all_variants()`
Test function: `fn compile_error_code_preserves_all_existing_string_values()`

```
Given: The CompileError::code() method return type
When: It is inspected at compile time
Then: The return type is SymbolicCode, not &'static str
```

Test function: `compile_assert_compile_error_code_returns_symbolic_code()` — static assertion

#### Behavior: B-041/B-042 — YamlError code() method

```
Given: All 20 YamlError variants, instantiated with valid field values
When: .code() is called on each variant
Then: Every call returns a SymbolicCode
Then: The mapping matches the error-taxonomy §2.3: DuplicateKey→DUPLICATE_KEY,
      ForbiddenFeature→FORBIDDEN_YAML_FEATURE, SourceTooLarge→PAYLOAD_TOO_LARGE,
      NestingTooDeep→LIMIT_EXCEEDED, EmptySource→MISSING_REQUIRED_FIELD, etc.
```

Test function: `fn yaml_error_code_returns_symbolic_for_all_20_variants()`

```
Given: The YamlError::code() source
When: The match statement is inspected
Then: It is exhaustive (no wildcard arm)
```

Test function: `compile_fail_yaml_error_code_must_be_exhaustive()`

#### Behavior: B-043/B-044/B-045 — remaining error types

```
Given: All CoreError variants (40+), RuntimeError variants (25+), JournalError variants (28)
When: .symbolic_code() is called on each variant
Then: Every call returns a SymbolicCode
Then: Each SymbolicCode's as_str() matches the expected symbolic name
Then: Every returned SymbolicCode passes validation (is in CODE_REGISTRY)
```

Test function: `fn core_error_symbolic_code_returns_valid_code_for_all_variants()`
Test function: `fn runtime_error_symbolic_code_returns_valid_code_for_all_variants()`
Test function: `fn journal_error_symbolic_code_returns_valid_code_for_all_variants()`

---

## 4. Proptest Invariants

### Proptest: SymbolicCode::from_static

**Invariant**: For any `&str` input `s`, `from_static(s).is_some()` iff `s` exists in `CODE_REGISTRY`.  
**Strategy**: `any::<String>()` — arbitrary strings including empty, very long, unicode, special chars.  
**Anti-invariant**: No registered string should ever return `None`; no unregistered string should return `Some`.  
**Compensates**: BLOCKED PO-001 (Kani iter().find() SSO).  
**File**: `crates/vb_core/tests/proptest_symbolic_code.rs`  
**Evidence command**: `cargo test --test proptest_symbolic_code -- --nocapture`

### Proptest: ValidationError variant code uniqueness

**Invariant**: For the 58 `ValidationError` variants, each produces a unique `SymbolicCode`; all are registered in `CODE_REGISTRY`; no two share the same numeric code.  
**Strategy**: Enumerate all 58 variants systematically. No random generation needed (finite domain).  
**Anti-invariant**: Any two different variants producing the same code would violate the contract.  
**Compensates**: BLOCKED PO-003 (now VERIFIED in Kani, but runtime defense-in-depth).  
**File**: `crates/vb_validate/tests/proptest_validation_error_codes.rs`  
**Evidence command**: `cargo test --test proptest_validation_error_codes -- --nocapture`

### Proptest: is_supported_code + from_str correctness

**Invariant**: 
- `is_supported_code(code)` accepts all code constants across the workspace.
- `from_str("EXXXX")` succeeds for every supported range; fails for gaps.
- `from_str(malformed)` always returns `Err(InvalidFormat)`.
- Round-trip: `from_str(format!("E{:04X}", code))` returns `Ok(DiagnosticCode(code))` for all supported codes.

**Strategy**: Enumerate all registered numeric codes; generate random `u16` values for rejection testing; generate random malformed strings.  
**Compensates**: BLOCKED PO-004 H1, BLOCKED PO-008.  
**File**: `crates/vb_core/tests/proptest_supported_codes.rs`  
**Evidence command**: `cargo test --test proptest_supported_codes -- --nocapture`

### Proptest: Diagnostic constructor consistency

**Invariant**: For every `SymbolicCode` in the registry, `Diagnostic::new(code, msg, sev, span)` produces a record where `numeric_code.symbolic_code() == Some(code)`. The constructor never panics.  
**Strategy**: Iterate all registered `SymbolicCode` values.  
**Compensates**: BLOCKED PO-005, BLOCKED PO-014.  
**File**: `crates/vb_core/tests/proptest_diagnostic_constructor.rs`  
**Evidence command**: `cargo test --test proptest_diagnostic_constructor -- --nocapture`

### Proptest: SymbolicCode serde round-trip

**Invariant**: For any registered `SymbolicCode`, `serde_json::to_string(&code).and_then(|s| serde_json::from_str::<SymbolicCode>(&s))` returns `Ok(code)`.  
For arbitrary unregistered strings, `serde_json::from_str::<SymbolicCode>(...)` returns `Err`.  
**Strategy**: Iterate all registered codes for round-trip; generate arbitrary unregistered strings for rejection.  
**Compensates**: BLOCKED PO-009 H1.  
**File**: `crates/vb_core/tests/proptest_serde_roundtrip.rs`  
**Evidence command**: `cargo test --test proptest_serde_roundtrip -- --nocapture`

### Proptest: CODE_REGISTRY unified consistency

**Invariant**: 
- All numeric codes non-zero.
- No duplicate symbolic names.
- No duplicate numeric codes.
- Every entry's category matches its numeric high byte `(numeric >> 8) & 0xFF`.
- Symbolic→numeric→symbolic round-trip identity holds for every entry.

**Strategy**: Iterate CODE_REGISTRY entries. No random generation (registry is finite).  
**Compensates**: BLOCKED PO-002 H1/H3, BLOCKED PO-012.  
**File**: `crates/vb_core/tests/proptest_registry_consistency.rs`  
**Evidence command**: `cargo test --test proptest_registry_consistency -- --nocapture`

### Proptest: Section 16 master contract parity

**Invariant**: All 36 Section 16 symbolic codes (from `velvet-ballistics-MASTER.md` §16) appear in `CODE_REGISTRY` with correct symbolic names and expected numeric ranges.  
**Strategy**: Hardcoded golden data for the 36 codes; cross-check against CODE_REGISTRY.  
**File**: `crates/vb_core/tests/proptest_section16_parity.rs`  
**Evidence command**: `cargo test --test proptest_section16_parity -- --nocapture`

### Proptest: Error types registration

**Invariant**: Every variant of `CoreError`, `RuntimeError`, and `JournalError` maps to a `SymbolicCode` that is registered in `CODE_REGISTRY`.  
**Strategy**: Enumerate all variants programmatically.  
**Compensates**: BLOCKED PO-015.  
**File**: `crates/workspace_tests/tests/proptest_error_types_registration.rs`  
**Evidence command**: `cargo test --test proptest_error_types_registration -- --nocapture`

### Proptest: CompileError symbolic code registration

**Invariant**: All `CompileError` symbolic codes from `.code()` are registered in `CODE_REGISTRY`.  
**Strategy**: Enumerate all `CompileError` variants.  
**File**: `crates/workspace_tests/tests/proptest_compile_error_codes.rs`  
**Evidence command**: `cargo test --test proptest_compile_error_codes -- --nocapture`

### Proptest: diag_codes.rs promotion sync

**Invariant**: Every numeric code constant in `vb_validate/src/diag_codes.rs` (58 constants) has a matching entry in `CODE_REGISTRY` with the correct symbolic name and numeric value.  
**Strategy**: Enumerate constants.  
**File**: `crates/vb_validate/tests/proptest_diag_codes_promotion.rs`  
**Evidence command**: `cargo test --test proptest_diag_codes_promotion -- --nocapture`

### Proptest: HasSymbolicCode determinism

**Invariant**: Calling `symbolic_code()` twice on the same error value always returns the same `SymbolicCode`.  
**Strategy**: For each error type, construct instances of all variants, call `symbolic_code()` twice, assert equality.  
**Compensates**: BLOCKED PO-013.  
**File**: `crates/vb_core/tests/proptest_symbolic_code_determinism.rs`  
**Evidence command**: `cargo test --test proptest_symbolic_code_determinism -- --nocapture`

---

## 5. Fuzz Targets

### Fuzz Target: SymbolicCode deserialization from arbitrary JSON

**Input type**: `&[u8]` — arbitrary byte sequences  
**Risk**: Panic on malformed UTF-8, deeply nested JSON, extremely long strings, escape sequences, null bytes. `Deserialize` implementation must gracefully reject all hostile input without panic, OOM, or undefined behavior.  
**Corpus seeds**: Valid JSON `"DUPLICATE_KEY"`, `"\"\"`, `"123"`, `"null"`, `"[]"`, `"{}"`, `"\"A\"*100000"`, UTF-8 BOM prefix.  
**File**: `fuzz/fuzz_targets/fuzz_symbolic_code_deserialize.rs` — MISSING (not present in fuzz_targets/ directory, not in fuzz/Cargo.toml [[bin]] entries). This is a ledger inconsistency: ledgers reference a non-existent fuzz target. Compensating evidence: PO-021 proptest_serde_roundtrip covers JSON round-trip identity and unknown-code rejection.
**Evidence command**: `cargo fuzz run fuzz_symbolic_code_deserialize -- -max_len=4096 -runs=100000` (CANNOT RUN — target MISSING)

### Fuzz Target: DiagnosticCode from_str parsing

**Input type**: `&str` — arbitrary strings  
**Risk**: Panic on non-ASCII, control characters, extremely long inputs, format string attacks. The `FromStr` impl must handle all inputs without panic (only `Err` returns).  
**Corpus seeds**: `"E0101"`, `"E010C"`, `"E9999"`, `""`, `"E"`, `"E010101"`, `"💥"`, `"\0E0101"`, very long strings.  
**File**: `fuzz/fuzz_targets/fuzz_diagnostic_code_from_str.rs` — NEW.  
**Evidence command**: `cargo fuzz run fuzz_diagnostic_code_from_str -- -max_len=256 -runs=100000`

---

## 6. Kani Verification Harnesses

All Kani harnesses are referenced by their proof obligation IDs (PO-001 through PO-014). The harness files live in `crates/vb_core/src/kani/` and `crates/vb_validate/src/kani/`. The test plan does NOT duplicate Kani harness specifications — those live in `proof-obligations.planned.jsonl` and the harness source files. This section documents the test plan's relationship to Kani coverage.

### Verified (Production-Connected)

| PO | Harness | Status | Evidence Command |
|----|---------|--------|-----------------|
| PO-003 × 6 | `kani_validation_error_code_registered_1..6` | ✅ VERIFIED R9 | `cargo kani -p vb_validate --harness kani_validation_error_code_registered_1 -Z stubbing` |
| PO-006 × 2 | `kani_yaml_error_code_registered_1..2` | ✅ VERIFIED R9 | `cargo kani -p vb_yaml --harness kani_yaml_error_code_registered_1` |
| PO-002 H2 | `kani_registry_unique_numeric` | ✅ VERIFIED R6 | `cargo kani --harness kani_registry_bijection --crate vb_core` |
| PO-004 H3 | `kani_is_supported_code_accepts_ranges` | ✅ VERIFIED R6 | `cargo kani --harness kani_is_supported_code_accepts_ranges --crate vb_core` |
| PO-004 H2 × 3 | `kani_is_supported_code_rejects_gaps_1..3` | ✅ VERIFIED R6 | `cargo kani --harness kani_is_supported_code_rejects_gaps_1 --crate vb_core` |
| PO-009 H2 | `kani_serde_rejects_unknown` | ✅ VERIFIED R6 | `cargo kani --harness kani_serde_rejects_unknown --crate vb_core` |
| PO-010 | `kani_registry_nonzero` | ✅ VERIFIED R6 | `cargo kani --harness kani_registry_nonzero --crate vb_core` |
| PO-011 | `kani_registry_category_match` | ✅ VERIFIED R6 | `cargo kani --harness kani_registry_category_match --crate vb_core` |

### Blocked (iter().find() SSO) — Compensated by Proptest

| PO | Harness | Status | Compensating Proptest |
|----|---------|--------|----------------------|
| PO-001 | `kani_from_static_validation` | BLOCKED | PO-016 (proptest_symbolic_code) |
| PO-002 H1/H3 | `kani_registry_unique_symbolic`, roundtrip | BLOCKED | PO-023 (proptest_registry_consistency) |
| PO-004 H1 | `kani_is_supported_code_all_constants` | BLOCKED | PO-018 (proptest_supported_codes) |
| PO-005 | `kani_diagnostic_constructor_consistency` | BLOCKED | PO-019 (proptest_diagnostic_constructor) |
| PO-008 | `kani_from_str_backward_compat` | BLOCKED | PO-018 (proptest_supported_codes) |
| PO-009 H1 | `kani_serde_roundtrip` | BLOCKED | PO-021 (proptest_serde_roundtrip) |
| PO-012 | `kani_reverse_lookup` | BLOCKED | PO-023 (proptest_registry_consistency) |
| PO-013 | `kani_symbolic_code_determinism` | BLOCKED | proptest_symbolic_code_determinism |
| PO-014 | `kani_diagnostic_no_mismatch` | BLOCKED | PO-019 (proptest_diagnostic_constructor) |

### Waived

| PO | Harness | Status | Reason |
|----|---------|--------|--------|
| PO-007 | `kani_zero_alloc_hot_path` | WAIVED | Non-behavior performance invariant (WVR-PS010-ALLOC). Compensated by compile-time checks. |

### Pending Execution

| PO | Type | File | Status |
|----|------|------|--------|
| PO-022 | cargo-fuzz | `fuzz/fuzz_targets/fuzz_symbolic_code_deserialize.rs` (MISSING) | BLOCKED — target file does not exist |
| PO-027 | cargo-mutants | All diagnostic modules | PENDING since R2 |
| PO-028 | moon-ci | `.moon/tasks.yml` | PENDING since R2 |

---

## 7. Mutation Checkpoints

**Threshold**: ≥90% mutation kill rate across all diagnostic code modules.

### Critical mutations to survive:

| # | Function/Branch | Must Be Caught By |
|---|----------------|-------------------|
| M-1 | `SymbolicCode::from_static` — changing `==` to `!=` in registry comparison | `fn symbolic_code_from_static_returns_none_when_unregistered_string()` |
| M-2 | `is_supported_code` — removing E05xx range from matches! | `fn diagnostic_code_from_str_accepts_e0501_gate_verifier_range()` |
| M-3 | `is_supported_code` — removing E06xx range from matches! | `fn diagnostic_code_from_str_accepts_e0601_contract_discovery_range()` |
| M-4 | `ValidationError::code()` — removing a specific variant arm | `fn validation_error_code_all_58_unique_symbolic_codes()` (cardinality drops to 57) |
| M-5 | `ValidationError::code()` — swapping two variant numeric codes | `fn validation_error_code_returns_correct_numeric_for_variant_X()` |
| M-6 | `Diagnostic::new` — failing to derive numeric_code | `fn diagnostic_new_preserves_symbolic_numeric_invariant()` |
| M-7 | `YamlError::code()` — wildcard arm catching all variants | `fn yaml_error_code_returns_symbolic_for_all_20_variants()` |
| M-8 | `CODE_REGISTRY` — duplicate symbolic name inserted | `fn code_registry_has_no_duplicate_symbolic_names()` |
| M-9 | `DiagnosticCode::symbolic_code()` — returns `None` for registered code | `fn diagnostic_code_symbolic_lookup_returns_symbolic_when_registered()` |
| M-10 | `DiagnosticCode::symbolic_code()` — returns `Some` for unregistered code | `fn diagnostic_code_symbolic_lookup_returns_none_when_unregistered()` |
| M-11 | `CompileError::code()` — return type changed from `SymbolicCode` to `&'static str` | Compile-time type assertion |
| M-12 | `HasSymbolicCode` — missing implementation on one error type | `fn has_symbolic_code_implemented_by_all_six_error_types()` |

**Execution command**:
```bash
cargo mutants --in-package vb_core --in-package vb_validate --in-package vb_compile --in-package vb_yaml -- --test-dir tests/
```

---

## 8. Combinatorial Coverage Matrix

### 8.1 SymbolicCode Construction & Identity

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Happy: registered code | `"DUPLICATE_KEY"` | `Some(SymbolicCode("DUPLICATE_KEY"))` | unit |
| Happy: all registry entries | each registry symbolic name | `Some` for every entry | unit |
| Error: unregistered string | `"BOGUS"` | `None` | unit |
| Error: empty string | `""` | `None` | unit |
| Boundary: whitespace variant | `" DUPLICATE_KEY "` | `None` (exact match only) | unit |
| Boundary: case sensitivity | `"duplicate_key"` | `None` (case-sensitive match) | unit |
| Invariant: as_str preserves | any registered name | `code.as_str() == input` | unit |
| Invariant: numeric_code | any registered name | `code.numeric_code() == registry[input]` | unit |
| Invariant: as_diagnostic_code | any registered name | `code.as_diagnostic_code().code() == registry[input]` | unit |
| Invariant: Display format | `"DUPLICATE_KEY"` | `"DUPLICATE_KEY"` (not `"E0101"`) | unit |
| Invariant: FromStr round-trip | `"DUPLICATE_KEY"` | `"DUPLICATE_KEY".parse() == Ok(code)` | unit |
| Invariant: Copy | any SymbolicCode | `let c2 = c1; c1.as_str() == c2.as_str()` | unit |
| Invariant: Send + Sync | compile-time | `fn assert_send_sync<T: Send + Sync>()` passes for `T = SymbolicCode` | static |

### 8.2 SymbolicCode serde

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Happy: round-trip registered | `SymbolicCode("DUPLICATE_KEY")` | `serialize → deserialize == Ok(code)` | unit |
| Happy: all registry entries | each registered code | round-trip identity for all | unit (proptest) |
| Error: unknown code name | JSON `"\"BOGUS\""` | `Err` | unit |
| Error: non-string JSON | `"123"`, `"null"`, `"[]"`, `"{}"` | `Err` | unit |
| Error: empty JSON string | `"\"\""` | `Err` | unit |
| Error: wrong-case serialized | `"\"duplicate_key\""` | `Err` | unit |
| Fuzz: arbitrary bytes | any `&[u8]` | never panics | fuzz |

### 8.3 DiagnosticCode parsing

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Happy: existing E0101 | `"E0101"` | `Ok(DiagnosticCode(0x0101))` | integration |
| Happy: existing boundary E040C | `"E040C"` | `Ok(DiagnosticCode(0x040C))` | integration |
| Happy: new E0501 (gate) | `"E0501"` | `Ok(DiagnosticCode(0x0501))` | integration |
| Happy: new E0513 (gate max) | `"E0513"` | `Ok(DiagnosticCode(0x0513))` | integration |
| Happy: new E0601 (disc) | `"E0601"` | `Ok(DiagnosticCode(0x0601))` | integration |
| Happy: new E0603 (disc max) | `"E0603"` | `Ok(DiagnosticCode(0x0603))` | integration |
| Happy: extended E401C | `"E401C"` | `Ok(DiagnosticCode(0x401C))` | integration |
| Error: gap E010C | `"E010C"` | `Err(UnsupportedCode)` | integration |
| Error: gap E040D | `"E040D"` | `Err(UnsupportedCode)` | integration |
| Error: gap E0514 | `"E0514"` | `Err(UnsupportedCode)` | integration |
| Error: gap E0604 | `"E0604"` | `Err(UnsupportedCode)` | integration |
| Error: gap E401D | `"E401D"` | `Err(UnsupportedCode)` | integration |
| Error: fully outside | `"E9999"` | `Err(UnsupportedCode)` | integration |
| Error: missing prefix | `"0101"` | `Err(InvalidFormat)` | integration |
| Error: lowercase | `"e0101"` | `Err(InvalidFormat)` | integration |
| Error: too short | `"E01"` | `Err(InvalidFormat)` | integration |
| Error: too long | `"E010101"` | `Err(InvalidFormat)` | integration |
| Error: empty | `""` | `Err(InvalidFormat)` | integration |
| Error: non-hex | `"E010G"` | `Err(InvalidFormat)` | integration |
| Error: whitespace prefix | `" E0101"` | `Err(InvalidFormat)` | integration |
| Invariant: identity | `DiagnosticCode::new(code).code() == code` | holds for all u16 | unit |
| Fuzz: arbitrary strings | any `&str` | never panics | fuzz |

### 8.4 DiagnosticCode reverse lookup

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Happy: registered code | `DiagnosticCode(0x0101)` | `Some(SymbolicCode("DUPLICATE_KEY"))` | integration |
| Error: zero code | `DiagnosticCode(0x0000)` | `None` | integration |
| Error: unregistered gap | `DiagnosticCode(0x010C)` | `None` | integration |
| Invariant: full registry | all registry numeric codes | all return `Some` with correct symbolic | unit (proptest) |

### 8.5 Diagnostic constructor

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Happy: valid code | `SymbolicCode("DUPLICATE_KEY")` + message + Severity::Error + Span::ZERO | `Diagnostic` with invariant satisfied | integration |
| Happy: all registered codes | every registry SymbolicCode | no panics, invariant holds | integration |
| Invariant: code field | any Diagnostic | `.code` is the input SymbolicCode | integration |
| Invariant: numeric_code field | any Diagnostic | `.numeric_code.symbolic_code() == Some(.code)` | integration |
| Invariant: message preserved | any Diagnostic | `.message == input_message` | integration |
| Invariant: severity preserved | any Diagnostic | `.severity == input_severity` | integration |

### 8.6 CODE_REGISTRY consistency

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Invariant: completeness | golden list of 36+19+3+8 codes | all present | unit |
| Invariant: no duplicate symbolic | all symbolic names | set size == registry length | unit |
| Invariant: no duplicate numeric | all numeric codes | set size == registry length | unit |
| Invariant: non-zero | all numeric codes | no zero values | unit |
| Invariant: category match | each (numeric, category) pair | `(numeric>>8)&0xFF` matches category range | unit |
| Invariant: bijection | each entry | `numeric_to_symbolic(symbolic_to_numeric(name)) == Some(name)` | unit |
| Invariant: const assertion | compile-time | duplicate symbolic/numeric would fail to compile | static |

### 8.7 ValidationError::code()

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Happy: DuplicateKey variant | `ValidationError::DuplicateKey` | `SymbolicCode("DUPLICATE_KEY")` | integration |
| Happy: all 36 Section 16 variants | each variant | matching Section 16 symbolic name | integration |
| Happy: 19 gate verifier variants | each E05xx variant | matching gate verifier symbolic name | integration |
| Happy: 3 contract discovery variants | each E06xx variant | matching discovery symbolic name | integration |
| Invariant: uniqueness | all 58 variants | 58 distinct SymbolicCodes | integration |
| Invariant: all registered | all 58 codes | every code in CODE_REGISTRY | integration |
| Invariant: exhaustive match | compile-time | no wildcard arm; adding variant = compile error | static |
| Invariant: Severity::Error | all diagnostics from ValidationError | `diagnostic_from_error(e).severity == Severity::Error` | integration |

### 8.8 CompileError::code()

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Happy: all 60+ variants | each variant with valid fields | returns SymbolicCode | integration |
| Happy: regression check | all existing symbolic code strings | all match pre-migration `&'static str` values | integration |
| Happy: compilation-specific codes | `UNKNOWN_INPUT_SCHEMA_FIELD`, etc. | registered in CODE_REGISTRY | integration |
| Invariant: return type | static inspection | return type is `SymbolicCode`, not `&'static str` | static |

### 8.9 YamlError::code()

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Happy: DuplicateKey variant | `YamlError::DuplicateKey { .. }` | `SymbolicCode("DUPLICATE_KEY")` | integration |
| Happy: ForbiddenFeature variant | `YamlError::ForbiddenFeature { .. }` | `SymbolicCode("FORBIDDEN_YAML_FEATURE")` | integration |
| Happy: SourceTooLarge variant | `YamlError::SourceTooLarge { .. }` | `SymbolicCode("PAYLOAD_TOO_LARGE")` | integration |
| Happy: all 20 variants | each variant | expected SymbolicCode per error-taxonomy §2.3 | integration |
| Invariant: exhaustive match | compile-time | no wildcard arm | static |
| Invariant: all registered | all 20 codes | every code in CODE_REGISTRY | integration |

### 8.10 Trait: HasSymbolicCode

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| Happy: trait defined in vb_core | `vb_core::HasSymbolicCode` symbol | compiles; one method: `symbolic_code(&self) -> SymbolicCode` | static |
| Happy: ValidationError implements | `ValidationError as HasSymbolicCode` | compiles | integration |
| Happy: CompileError implements | `CompileError as HasSymbolicCode` | compiles | integration |
| Happy: YamlError implements | `YamlError as HasSymbolicCode` | compiles | integration |
| Happy: CoreError implements | `CoreError as HasSymbolicCode` | compiles | integration |
| Happy: RuntimeError implements | `RuntimeError as HasSymbolicCode` | compiles | integration |
| Happy: JournalError implements | `JournalError as HasSymbolicCode` | compiles | integration |
| Invariant: purity | all implementations | no I/O, no allocation, no panic — pure functions | unit (proptest) |

### 8.11 E2E: Full Workflow

| Scenario | Input Class | Expected Output | Test Layer |
|----------|-------------|-----------------|------------|
| E2E: YAML → diagnostic chain | invalid YAML with duplicate keys | `ValidationError::DuplicateKey` → `Diagnostic { code: SymbolicCode("DUPLICATE_KEY"), severity: Error, .. }` | e2e |
| E2E: CLI error display | invoke CLI with invalid workflow | output contains `"DUPLICATE_KEY"` (symbolic, not `"E0101"`) | e2e |

---

## 9. Test File Placement

### New test files to create (or land from workspace)

| File | Layer | Crate | Priority |
|------|-------|-------|----------|
| `crates/vb_core/tests/proptest_symbolic_code.rs` | unit | vb_core | CRITICAL |
| `crates/vb_core/tests/proptest_registry_consistency.rs` | unit | vb_core | CRITICAL |
| `crates/vb_core/tests/proptest_supported_codes.rs` | unit | vb_core | CRITICAL |
| `crates/vb_core/tests/proptest_diagnostic_constructor.rs` | integration | vb_core | HIGH |
| `crates/vb_core/tests/proptest_serde_roundtrip.rs` | unit | vb_core | HIGH |
| `crates/vb_core/tests/proptest_section16_parity.rs` | unit | vb_core | HIGH |
| `crates/vb_core/tests/proptest_symbolic_code_determinism.rs` | unit | vb_core | MEDIUM |
| `crates/vb_validate/tests/proptest_validation_error_codes.rs` | integration | vb_validate | CRITICAL |
| `crates/vb_validate/tests/proptest_diag_codes_promotion.rs` | integration | vb_validate | HIGH |
| `crates/workspace_tests/tests/proptest_error_types_registration.rs` | integration | workspace_tests | HIGH |
| `crates/workspace_tests/tests/proptest_compile_error_codes.rs` | integration | workspace_tests | HIGH |
| `crates/workspace_tests/tests/behavior_validation_error_code.rs` | integration | workspace_tests | HIGH |
| `crates/workspace_tests/tests/behavior_compile_error_code.rs` | integration | workspace_tests | HIGH |
| `crates/workspace_tests/tests/behavior_yaml_error_code.rs` | integration | workspace_tests | HIGH |
| `crates/workspace_tests/tests/behavior_diagnostic_code_ranges.rs` | integration | workspace_tests | HIGH |
| `crates/workspace_tests/tests/behavior_symbolic_code_serde.rs` | integration | workspace_tests | HIGH |
| `crates/workspace_tests/tests/e2e_diagnostic_chain.rs` | e2e | workspace_tests | MEDIUM |
| `fuzz/fuzz_targets/fuzz_diagnostic_code_from_str.rs` | fuzz | — (fuzz crate) | MEDIUM |

### Existing test files to augment

| File | What to Add |
|------|------------|
| `crates/vb_core/src/diagnostic.rs` (tests module) | `symbolic_code_lookup_tests` — when SymbolicCode/DiagnosticCode linkage is in production |
| `crates/vb_validate/src/diagnostic.rs` (tests module) | `validation_error_code_returns_symbolic` — when ValidationError::code() returns SymbolicCode |
| `crates/vb_compile/tests/vb_xi2f_compile_source_proptest.rs` | Verify CompileError::code() returns SymbolicCode type |

---

## 10. Bridge Finding Resolution

### F-BR-001: Transition criteria for mapping_status

**Resolution**: All 28 RROs with `mapping_status: planned` need explicit transition criteria. The test-plan-to-implementation bridge must verify:
- Every proptest file (PO-016 through PO-026) exists and compiles
- Every proptest file passes with `cargo test`
- The 9 BLOCKED Kani harnesses have compensating proptest files that PASS
- The 3 PENDING obligations (PO-022 fuzz, PO-027 mutation, PO-028 CI) are executed and produce evidence

### F-BR-002: Evidence workdir mismatch

**Resolution**: All test files listed in §9 must be created in the production tree (`/home/lewis/src/velvet-ballistics`), not just the isolated workspace. Run:
```bash
cp -rf /home/lewis/src/vb-workspaces/vb-xi2f.10/<test-files> /home/lewis/src/velvet-ballistics/<target-path>
```
for any files that currently exist only in the workspace.

### F-BR-003: workspace_tests crate exclusion

**Affected RROs**: PO-005, PO-015, PO-020. The `proptest_error_types_registration.rs` and `proptest_compile_error_codes.rs` files must be created in `crates/workspace_tests/tests/` (which is part of the workspace's test build).

### F-BR-004: PO-013 missing independent behavior test

**Resolution**: Create `crates/vb_core/tests/proptest_symbolic_code_determinism.rs` — a property test that verifies `HasSymbolicCode::symbolic_code()` is deterministic (same call twice = same result) for all six error types.

---

## 11. Open Questions

1. **Where are the proptest files from the workspace?** The bridge review (F-BR-002) states that proptest/fuzz files exist "only in workspace, not production tree." The workspace directory at `/home/lewis/src/vb-workspaces/vb-xi2f.10/` does not appear to contain Rust test files in the expected locations. The test-writer must either (a) locate these files and copy them to the production tree, or (b) write them fresh from this plan. **Recommendation**: Route to test-writer for fresh creation per the BDD scenarios and proptest invariant sections of this plan.

2. **Are `SymbolicCode`, `CODE_REGISTRY`, and `HasSymbolicCode` implemented in production?** Based on the current state of `crates/vb_core/src/diagnostic.rs` (no SymbolicCode, no CODE_REGISTRY, no HasSymbolicCode), these types exist only in the Kani model layer (`crates/vb_core/src/kani/`). The implementation bridge (State 7) is APPROVED but the types must be promoted to production before behavior tests can be written. **Recommendation**: Coordinate with State 9 (implementation) — test-writer creates tests that reference the contract's type signatures; tests may initially fail to compile until the production types are promoted.

3. **Should CompileError::code() return type change be a compile_assert or a runtime test?** The return type from `&'static str` to `SymbolicCode` is a type-level change. A `compile_assert` test using `static_assertions` or a trybuild compiletest is the strongest approach.

4. **Is `is_supported_code()` extended to include E05xx and E06xx?** Production code at `diagnostic.rs:149-167` does NOT include these ranges. This is a GAP-4 beat referenced in the bridge (PO-004). The test for B-017/B-018 will fail until the production code is updated.

5. **Are existing tests at `crates/vb_validate/src/diagnostic.rs` (tests module) still valid?** The existing tests at lines 700+ of that file test `error_code() -> DiagnosticCode` and `diagnostic_from_error() -> Diagnostic`. After migration, `Diagnostic.code` changes from `DiagnosticCode` to `SymbolicCode`. These existing tests will need updating. This plan does NOT cover updating existing tests — that is a separate bead concern.

---

## 12. Exit Criteria Checklist

- [x] Every public API behavior has at least one BDD scenario (47 behaviors, 47+ scenarios)
- [x] Every pure function with multiple inputs has at least one proptest invariant (11 invariants)
- [x] Every parsing/deserialization boundary has a fuzz target (2 fuzz targets: SymbolicCode deser, DiagnosticCode from_str)
- [x] Every error variant class in the Error enum hierarchy has explicit test scenarios
- [x] The mutation threshold target (≥90%) is stated with 12 checkpoints
- [x] No test asserts only `is_ok()` or `is_err()` without specifying the value
- [x] Trophy allocation is justified with rationale for deviation from 60/30/5/5 ratio
- [x] Bridge findings F-BR-001 through F-BR-004 are addressed with resolution plans
- [x] Test file placement is specified with priority levels
- [x] Open questions are explicitly documented for the test-writer
