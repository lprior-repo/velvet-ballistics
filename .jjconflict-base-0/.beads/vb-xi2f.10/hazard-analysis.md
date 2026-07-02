# Hazard Analysis — Section 16 Symbolic Diagnostic Codes

**Bead**: vb-xi2f.10  
**Phase**: State 3 — Rust Contract

---

## 1. Temporal Hazards

| ID | Hazard | Likelihood | Impact | Mitigation |
|----|--------|-----------|--------|------------|
| T-H1 | **Registry drift across releases**: A new error variant is added in one crate but not registered in `vb_core::CODE_REGISTRY`. | Medium | `SymbolicCode::from_static()` returns `None`, error variant has no code. | Audit workflow (see workflow-model §6). CI gate via exhaustive variant test. |
| T-H2 | **Symbolic name change**: A symbolic name changes between releases, breaking deserialization of previously serialized diagnostics. | Low | Consumers reject unknown codes. Old diagnostics become unreadable. | Policy: symbolic names are append-only. Use deprecation, never rename. |
| T-H3 | **Numeric code reassignment**: A numeric code is reused for a different symbolic name. | Low | Consumers with numeric-only code see wrong symbolic name. Registry bijection breaks. | Policy: numeric codes are append-only. `CODE_REGISTRY` uniqueness assertions at compile time. |
| T-H4 | **Unregistered code emitted at runtime**: An error path that was previously unreachable becomes reachable via a new code path, and the error variant has no registered code. | Low | `code()` returns a fallback code or panics. | Exhaustive match on error variants. All variants must be covered in `code()`. No wildcard arms. |
| T-H5 | **is_supported_code() falls behind**: `is_supported_code()` is not updated when new numeric codes are added to the registry. | Medium | `DiagnosticCode::from_str("E0512")` returns `Err(UnsupportedCode)` for a valid in-use code. | CI test: for every numeric code in the registry, assert `is_supported_code()` returns `true`. |

---

## 2. Rust-Core Invariant Hazards

| ID | Hazard | Likelihood | Impact | Mitigation |
|----|--------|-----------|--------|------------|
| I-H1 | **`DiagnosticCode::new(code)` bypasses registry**: A `DiagnosticCode` can be constructed from an arbitrary `u16` without registry validation. | High (by design) | A numeric code with no symbolic equivalent enters the system. `symbolic_code()` returns `None`. | `DiagnosticCode::new()` remains a low-level constructor. All public API paths that produce `Diagnostic` records go through `SymbolicCode` which validates. |
| I-H2 | **`SymbolicCode` constructed from bare `&'static str`**: If the newtype is not properly encapsulated, code can construct `SymbolicCode("BOGUS_CODE")`. | Medium | Unregistered code enters the system. | Module-level visibility: `SymbolicCode(&'static str)` field is private. Only `from_static()` pub. |
| I-H3 | **Bijection invariant broken**: Two symbolic codes map to the same numeric code, or vice versa. | Low | Lookups return wrong code. Category inference is wrong. | `CODE_REGISTRY` is a const slice. `const` assertions verify uniqueness at compile time. Runtime tests double-check. |
| I-H4 | **CodeCategory mismatch**: A code's numeric high byte does not match its declared category. | Low | Category-based filtering gives wrong results. | Const assertion in `CODE_REGISTRY`: for each entry, `(numeric >> 8) & 0xFF` matches the category's expected high byte. |
| I-H5 | **Numeric code zero**: A code with value `0x0000` is registered. | Low | Ambiguous with "no error" or default value. | `CODE_REGISTRY` const assertion: all numeric codes are non-zero. |

---

## 3. Bounded State Hazards

| ID | Hazard | Likelihood | Impact | Mitigation |
|----|--------|-----------|--------|------------|
| B-H1 | **Registry too large for const evaluation**: 90+ entries in `CODE_REGISTRY` may hit const evaluation limits. | Low | Compile failure. | Rust const evaluation handles slices of 100+ items easily. Monitor if approaching 1000+. |
| B-H2 | **u16 numeric space exhaustion**: All 65536 values are used. | Very Low | Cannot register new codes. | Current usage: ~100 codes. u16 space has 65536 slots. No practical limit. |
| B-H3 | **Symbolic name length**: Symbolic names are `&'static str` with no explicit length bound. | Low | Very long names bloat binary. | Names are human-readable identifiers; typical length is 10–40 chars. Acceptable. |

---

## 4. Refinement Hazards

| ID | Hazard | Likelihood | Impact | Mitigation |
|----|--------|-----------|--------|------------|
| R-H1 | **Error variant not covered in code()**: A new `ValidationError` variant is added but `code()` is not updated. | Medium | Compile error (exhaustive match). | `code()` uses `match self { ... }` without wildcard. Adding a variant forces compilation failure until handled. |
| R-H2 | **Numeric encoding mismatch between crates**: `vb_validate` and `vb_compile` independently define different numeric codes for the same symbolic name. | Low | Inconsistent behavior. | All numeric codes derived from `CODE_REGISTRY` in `vb_core`. No crate defines its own numeric constants. |
| R-H3 | **Symbolic code string literal typo**: `"DUPLICATE_KEY"` vs `"DUPLICATE_KEYS"`. | Medium | Code not found in registry; `from_static()` returns `None`. | Tests: every symbolic code used in `code()` is asserted to be in the registry. |

---

## 5. Concurrency Hazards

| ID | Hazard | Likelihood | Impact | Mitigation |
|----|--------|-----------|--------|------------|
| C-H1 | **Race on registry**: Two threads attempt to read different entries of `CODE_REGISTRY` simultaneously. | N/A | None. | `CODE_REGISTRY` is a `const &[CodeEntry]` — immutable static data. No synchronization needed. |
| C-H2 | **Concurrent code resolution**: Two threads call `error.code()` on different error values simultaneously. | N/A | None. | `code()` methods are pure functions with no mutable state. Thread-safe by construction. |
| C-H3 | **Concurrent diagnostic emission**: Two threads construct `Diagnostic` records simultaneously. | Low | `Box<str>` allocation may contend on allocator. | Allocation is in cold path only. Standard allocator handles this. No data race. |

---

## 6. Unsafe/Provenance Hazards

| ID | Hazard | Likelihood | Impact | Mitigation |
|----|--------|-----------|--------|------------|
| U-H1 | **Unsafe code in diagnostic system**: `unsafe` blocks in `DiagnosticCode`, `SymbolicCode`, or registry. | None | N/A | All diagnostic modules use `#![forbid(unsafe_code)]`. |
| U-H2 | **Pointer provenance in `&'static str`**: Converting a bare `&'static str` could carry invalid provenance. | Very Low | UB if string is not actually static. | `CODE_REGISTRY` entries are string literals — always valid `'static` lifetime. `SymbolicCode::from_static()` only accepts values from the registry. |

---

## 7. Hostile Input Hazards

| ID | Hazard | Likelihood | Impact | Mitigation |
|----|--------|-----------|--------|------------|
| X-H1 | **Malformed E-style code string**: `FromStr` receives `"E01ZZ"`, `""`, `"E01"`, `"E010101"`. | Medium | Parse error → `InvalidFormat`. | All paths checked. Validation function returns `Result`. No panic. |
| X-H2 | **E-style code outside supported range**: `FromStr` receives `"E9999"`. | Medium | Parse error → `UnsupportedCode`. | `is_supported_code()` guard. |
| X-H3 | **Unknown symbolic code**: `FromStr` receives `"BOGUS_CODE"`. | Medium | Parse error → `UnknownCode`. | Registry lookup. |
| X-H4 | **Extremely long symbolic name**: Deserialization receives a 10MB symbolic code string. | Low | Memory exhaustion or parse failure. | `serde` deserialization should enforce a maximum length. Symbolic codes are bounded by registry entries (max ~50 chars). |
| X-H5 | **Malformed numeric in serialization**: JSON contains `"code": 12345` (integer, not string `"DUPLICATE_KEY"`). | Low | Deserialization error. | Type mismatch caught by serde. |

---

## 8. Performance Hazards

| ID | Hazard | Likelihood | Impact | Mitigation |
|----|--------|-----------|--------|------------|
| P-H1 | **Registry lookup in hot path**: `SymbolicCode::from_static()` or `symbolic_to_numeric()` called in tight loop. | Medium | Linear scan of 90+ entries. | Registry is small (90 entries). Linear scan is O(90) = negligible. If needed, generate a `phf` map at compile time. |
| P-H2 | **Allocation in error code resolution**: `.to_string()` or `format!()` in `code()` method. | High (if done) | Violates "no heap allocation in hot path" contract. | All `code()` methods return `SymbolicCode` (Copy) or `&'static str`. No allocation. |
| P-H3 | **`Box<str>` creation for every error**: Constructing `Diagnostic` allocates. | Expected | One allocation per diagnostic emitted. | Diagnostics are cold path. Allocation acceptable. |
| P-H4 | **Large `Display` format strings**: `DiagnosticCode::Display` writes `"E0101"` (5 chars, stack-allocated). | None | No allocation. | `write!(f, "E{:04X}", self.0)` — formatted on stack. |

---

## 9. Release/API Hazards

| ID | Hazard | Likelihood | Impact | Mitigation |
|----|--------|-----------|--------|------------|
| A-H1 | **Breaking change to `Diagnostic.code` type**: Changing from `DiagnosticCode(u16)` to `SymbolicCode` breaks consumers that access `.code.code()`. | Certain (if done) | Compile errors in `vb_cli`, `vb_runtime`, `vb_storage`, and external consumers. | Two-phase migration: (1) Add `symbolic_code: SymbolicCode` field alongside existing `code: DiagnosticCode`. (2) In a future release, deprecate numeric field. OR: switch immediately with clear migration guide. |
| A-H2 | **`ValidationError` gains new variants**: Non-exhaustive enum (`#[non_exhaustive]`), but consumers with wildcard match arms may not handle new variants. | Medium | Consumer behavior may change. | `#[non_exhaustive]` already prevents exhaustive external match. New variants must have `code()` entries. |
| A-H3 | **`DiagnosticCode` is `repr(transparent)` over `u16`**: Changing the internal representation would be ABI-breaking. | Low | FFI consumers break. | Retain `repr(transparent)` over `u16`. No change to internal layout. |
| A-H4 | **Serialization format change**: Switching from numeric `"E0101"` to symbolic `"DUPLICATE_KEY"` in JSON. | High | Existing JSON consumers cannot parse old-format diagnostics. | Dual serialization or versioned format. Accept both `"E0101"` and `"DUPLICATE_KEY"` during deserialization for a transition period. |

---

## 10. Hazard Severity Matrix

| Severity | Hazards |
|----------|---------|
| **CRITICAL** | A-H1 (breaking Diagnostic.code type change) |
| **HIGH** | T-H1 (registry drift), I-H1 (DiagnosticCode bypasses registry), A-H4 (serialization format change) |
| **MEDIUM** | T-H5 (is_supported_code falls behind), R-H1 (variant not covered), R-H3 (string typo), X-H1–X-H3 (hostile input), P-H1 (lookup in hot path) |
| **LOW** | T-H2–T-H4 (rename/reassign), I-H2–I-H5 (invariant break), B-H1–B-H3 (bounded state), C-H3 (allocator contention), X-H4–X-H5 (malformed input) |
| **NONE** | C-H1–C-H2 (concurrency — no shared mutable state), U-H1–U-H2 (no unsafe code) |

---

## 11. Residual Risks After Mitigation

| Risk | Residual Concern |
|------|-----------------|
| A-H1 | The `.code` field type change is a planned breaking change. Must be clearly communicated and migration path documented. |
| A-H4 | Serialization format migration requires dual-format acceptance during transition. Cannot be done purely in type system — needs runtime format detection or versioning. |
| T-H1 | Registry drift can only be mitigated by CI gates, not prevented at the type level. A const assertion that counts registry entries against error variants is possible but brittle. |
