# Trusted Base Plan: YAML Path and Source Span Diagnostics

**Bead:** vb-xi2f.9
**Agent:** proof-planner (State 4)
**Schema:** trusted-base-plan/v1

## 1. Trusted External Libraries

| Library | Role | Trust Justification | Risk |
|---|---|---|---|
| `saphyr-parser` | YAML event stream with line/col spans | Well-established YAML parser. Event-level spans are the fundamental source of truth for all source locations in this bead. | **Accept.** Parser spans are used as-is. If parser produces wrong line/column, downstream diagnostics are wrong — but that is a pre-existing contract, not new risk from this bead. |
| `saphyr` (tree API) | YAML tree construction for validation | Known to discard parser-level spans — this is the gap this bead fills (via AstMarks backfill and YamlError span enrichment). | **Accept with mitigation.** AstMarks reconstructs span data from the event stream to compensate for the tree API's span loss. |
| `proptest` | Property-based testing framework | Standard Rust testing library. Used for input space coverage, not soundness proofs. | **Accept.** proptest generates test cases, not proof evidence. |
| `kani` (`kani_verifier`) | Bounded model checker for Rust | NASA/JPL-grade formal verification tool. Used for bounded-state invariant proofs. | **Accept.** Kani proofs are bounded to honest limits. |
| `flux-rs` | Refinement type verification | Lightweight type-level annotation checker. Used as defense-in-depth for Span paired invariant. | **Accept with caveat.** Flux RS is less mature than Kani. The Canonical proof remains the Kani obligation; Flux is complementary. |

## 2. Trusted Architectural Boundaries

| Boundary | Trust Assumption | Justification |
|---|---|---|
| `vb_yaml` does NOT depend on `vb_core` | The runtime core stays YAML-free. Bridge conversions live in `vb_compile`. | Architectural invariant verified by cargo-check. No new dependency added. boundary-map.md §172-182 |
| `#![forbid(unsafe_code)]` on all affected modules | No unsafe code in Span, Diagnostic, NonEmptyVec, YamlError, ValidationError, SourceMark, or bridge modules. | Verified by existing lint gate and cargo-geiger. HA-07. |
| Single-threaded compilation | No concurrent access to span/diagnostic types during compilation. | Pipeline is synchronous and deterministic. HA-06. |
| `YamlLimits` caps source size | YAML source is bounded below 4 GiB, making `usize→u32` overflow a theoretical concern only. | Existing `YamlLimits` enforcement at parse boundary. HA-05. Truncation/clamping is a safety net, not the primary guard. |
| `AstMarks` is correctly populated from saphyr-parser event stream | The mark lookup table built from the event stream accurately maps step/nested_key/trigger/document identifiers to source locations. | AstMarks already working (codebase-map.md §136). HA-03 uses this as the backfill mechanism for tree validation. If AstMarks is wrong, backfilled marks will be wrong — but AstMarks has existing tests. |

## 3. Trusted Internal Types and Functions

| Type/Function | Trust Assumption | Bounds |
|---|---|---|
| `Span::ZERO` | Constant `{start:0, end:0, line:None, column:None}` — never changes. | Hardcoded `const`. |
| `Span::new(start, end)` | Always produces `line: None, column: None`. | Constructor contract TC-01b. Verified by PO-K01, PO-P01. |
| `Span::with_location(start, end, line, col)` | Always produces `line: Some(line), column: Some(col)`. | Constructor contract TC-01a. Verified by PO-K01, PO-P01. |
| `NonEmptyVec::new(head)` | Always produces vec with len==1, first()==&head. | Smart constructor with private fields. Verified by PO-K02. |
| `NonEmptyVec::from_vec(vec)` | Returns `None` for empty, `Some(nev)` preserving all elements. | Verified by PO-K02, PO-P02. |
| `diagnostic_from_error(error)` | Propagates `error.span` into `Diagnostic.span`. | Verified by PO-K06, PO-P04. |
| `canonical_yaml_error(error)` | Extracts span from YamlError into CanonicalYaml mark. | Verified by PO-K05. |
| `extract_span_from_yaml_error(error)` | Exhaustively handles all 19 YamlError variants. | Verified by PO-K05, PO-G04. |
| `clamp_u32(value: usize) -> u32` | Saturated conversion, never panics, returns u32::MAX for overflow. | Verified by PO-K07, PO-M01. |
| `SourceMark::unavailable()` | Returns `{available: false, index:0, end_index:0, line:0, column:0, source_file: None}`. | Verified by existing tests. Represents "no location" sentinel. |

## 4. Model Reductions and Known Simplifications

| Reduction | Scope | Impact |
|---|---|---|
| **Kani stubs for string/heap types** | Diagnostic message strings, Box<str> source_file fields are not modeled with full heap semantics in Kani. Stubs represent abstract string values. | **Low risk.** String content does not affect invariant proofs (paired invariant, len>=1, span equality). String shapes are tested via proptest and unit tests. |
| **Kani bounded vec sizes** | NonEmptyVec tail is bounded to 0..15 elements in Kani proofs (not in proptest). | **Acceptable.** The invariant `len() >= 1` holds regardless of vec size. The element-preservation property scales linearly with size, verified by proptest for larger sizes (up to 100). |
| **Proptest YAML generation limited to strict-profile YAML** | Generated YAML must pass strict profile checks (no aliases, anchors, tags). | **Acceptable.** The bead's scope is strict-profile YAML compilation. Non-strict YAML is rejected at parse time. |
| **AstMarks lookup bounded to small tables in Kani** | Kani proofs for AstMarks backfill use small lookup tables (0..4 entries). | **Acceptable.** The property (lookup succeeds → available==true) is invariant over table size. Proptest covers larger, realistic tables. |
| **Flux refinement is light/defense-in-depth** | PO-F01 (Flux) is a complementary check, not the canonical proof. Kani provides the canonical bounded proof. | **Low risk.** If Flux cannot express the refinement due to tooling limitations, the Kani proof still covers the invariant. Flux adds value as a type-level annotation that catches regressions at compile time. |

## 5. Stubs and Harness Dependencies

| Stub | Used By | Purpose |
|---|---|---|
| `kani::Arbitrary for Span` | PO-K01, PO-K06 | Kani harness generates arbitrary Span values |
| `kani::Arbitrary for DiagnosticCode` | PO-K03 | Kani harness generates valid DiagnosticCode values |
| `saphyr event span stub` | PO-K04 | Abstract representation of saphyr-parser span for YamlError construction verification |
| `AstMarks stub with small lookup tables` | PO-K08 | Bounded AstMarks for Kani backfill verification |
| `proptest::Arbitrary for Span` | PO-P01, PO-P04 | Proptest generates randomized Span values with line/column |
| `proptest::Arbitrary for SourceSpan` | PO-P05 | Proptest generates randomized SourceSpan values (including edge case usize values) |

## 6. Trusted Base Operations

| Operation | Trust Level | Reason |
|---|---|---|
| `usize as u32` (truncating cast) | **PROVEN SAFE** in PO-K07, PO-M01 | Clamping strategy ensures no panic, no UB. Values > u32::MAX clamped to u32::MAX. |
| `Vec::push()` on tail | **TRUSTED** (stdlib) | Standard library Vec operation, not instrumented. |
| `Option::is_some()`, `Option::is_none()` | **TRUSTED** (stdlib) | Core language primitive, not instrumented. |
| `==` equality on `Span`, `Diagnostic` | **TRUSTED** (derive) | Derived PartialEq, used in assertions. |
| `format!()` / `to_string()` for diagnostic messages | **TRUSTED** (stdlib) | String formatting for diagnostic rendering. Verified by unit/proptest tests. |
| `clone()` on Copy types (Span) | **TRUSTED** (derive) | Derived Clone (Copy), trivial. |

## 7. What Is NOT Trusted

| Component | Reason | Verification |
|---|---|---|
| **Span paired invariant in arbitrary construction** | Without public-constructor-only discipline, someone could construct Span with mismatched line/column. | PO-F01 (Flux refinement) + PO-K01 (Kani proof) + code review. Not prevented by type system alone — this is a modeling hazard (HA-04). |
| **ValidationError span propagation from all code paths** | If a new code path constructs ValidationError with a span bypass, the diagnostic could have wrong span. | PO-K06 + PO-P04 verify the diagnostic_from_error function. Call-site discipline is enforced by code review. |
| **AstMarks availability for every lookup type** | The four lookup types (step, nested_key, trigger, document) must be populated during AstMarks construction. If a lookup type is implemented but never populated, backfill silently degrades to unavailable(). | PO-K08 + PO-P06 verify the lookup logic. AstMarks construction coverage is verified by existing AstMarks tests. |
| **NonEmptyVec invariant via unsafe transmutation** | If someone uses `std::mem::transmute` to construct an empty NonEmptyVec, the invariant is broken. | `#![forbid(unsafe_code)]` prevents this. Reviewed in codebase-map §158. |
| **SourceMap in vb_core after removal** | If removal is incomplete (re-export path remains), dead code lingers. | PO-G01 (grep + cargo check). |
| **Duplicate diagnostic conversion** | If diag_render.rs is not consolidated, drift risk remains. | PO-G02 (grep + cargo test). |

## 8. Trusted-Base Bounds Summary

| Dimension | Bound | Rationale |
|---|---|---|
| Span fields | u32 for offsets, Option<u32> for line/col | Type system enforces. |
| usize→u32 conversion | Clamped to u32::MAX, never panics | Verified by PO-K07, PO-M01. |
| NonEmptyVec tail | 0..N (N is physical memory) | Kani bounded to 0..15; proptest to 0..100. |
| YamlError variants | 19 total | Compile-time enum. Verified by exhaustive match in PO-K05. |
| ValidationError variants | ~50 total | Compile-time enum. Verified by exhaustive match in PO-K06. |
| AstMarks lookup entries | 0..M (M is YAML node count bounded by limits) | Kani bounded to 0..4; proptest uses realistic YAML. |
| YAML source size | Bounded by YamlLimits (<< 4 GiB) | Existing enforcement at parse boundary. |
| Compilation concurrency | 1 thread | Deterministic pipeline. |
| Unsafe code | 0 blocks | `forbid(unsafe_code)` on all affected modules. |
