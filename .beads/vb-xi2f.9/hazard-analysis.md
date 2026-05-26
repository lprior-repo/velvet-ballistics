# Hazard Analysis: YAML Path and Source Span Diagnostics

**Bead:** vb-xi2f.9  
**Agent:** rust-contract (State 3)  
**Schema:** 2026-05-24

---

## HA-01: Temporal Hazard — Span Propagation Gap

| Field | Detail |
|---|---|
| **Risk Tags** | `parser/codec`, `temporal`, `migration` |
| **Root Cause** | `diagnostic_from_error()` always produces `Span::ZERO` because `ValidationError` variants have no span fields. Span information exists in `SourceMap`/`SourceSpan`/`EventSpan` but is not threaded through. |
| **Trigger** | Any validation error produced during compilation. |
| **Consequence** | Users see `at byte 0..0` for every diagnostic, making YAML authoring nearly impossible. Error messages are ambiguous (same message for different locations). |
| **Affected Types** | `ValidationError` (all ~50 variants), `diagnostic_from_error()`, `Diagnostic.span` |
| **Mitigation** | Add `span: Span` to every `ValidationError` variant. Propagate through `diagnostic_from_error()`. Callers supply span via `AstMarks` or `SourceMap` lookups. |
| **Verification** | Unit tests: construct `ValidationError` with known span, assert `diagnostic_from_error(e).span == known_span`. Integration: compile YAML with known error location, assert diagnostic references correct line. |

---

## HA-02: Temporal Hazard — Canonical YAML Span Stripping

| Field | Detail |
|---|---|
| **Risk Tags** | `parser/codec`, `temporal` |
| **Root Cause** | `canonical_yaml_error()` converts `YamlError` to `CompileError::CanonicalYaml{category, message}` but discards the `SourceSpan` from `YamlError`. |
| **Trigger** | Any YAML parse error that reaches the canonical YAML bridge. |
| **Consequence** | Even though saphyr-parser provides precise line/column spans, the `CanonicalYaml` error variant always has `SourceMark::unavailable()`. |
| **Affected Types** | `canonical_yaml_error()`, `YamlError`, `CompileError::CanonicalYaml` |
| **Mitigation** | 1. Enrich `YamlError` with `span: Option<SourceSpan>`. 2. `canonical_yaml_error()` extracts `span` from `YamlError` and converts to `SourceMark`. 3. `CompileError::CanonicalYaml` gains a `mark: SourceMark` field. |
| **Verification** | Test: parse YAML with known error → `YamlError` carries span → `canonical_yaml_error()` produces `SourceMark` with that span → assert `SourceMark` matches expected line/column. |

---

## HA-03: Temporal Hazard — Tree Validation Loses Parser Spans

| Field | Detail |
|---|---|
| **Risk Tags** | `parser/codec`, `temporal` |
| **Root Cause** | `validate_strict_profile()` (part_02.rs) uses `saphyr::Yaml` tree nodes for validation. The `saphyr` tree API does not carry `saphyr-parser` event spans. |
| **Trigger** | Any validation error produced by tree-based validation. |
| **Consequence** | ~25 `CompileError` variants emitted by tree validation always get `SourceMark::unavailable()`. |
| **Affected Types** | `validate_strict_profile()`, `validate_one_node()`, `push_mapping()`, `validate_mapping_key()` |
| **Mitigation** | Backfill marks from `AstMarks` (already built from the event stream). `AstMarks::nested_key()`, `step()`, and `trigger()` provide `SourceMark` lookups by key name. Phase 2 option: refactor tree validation to use event-stream-based validation (out of scope). |
| **Verification** | Test: validate YAML with known schema error → tree validation collects error → backfill mark from `AstMarks` → assert `SourceMark.available == true` and line/column match expected. |

---

## HA-04: Invariant Hazard — Span With Unpaired Line/Column

| Field | Detail |
|---|---|
| **Risk Tags** | `invariant` |
| **Root Cause** | `Span` gains `line: Option<u32>` and `column: Option<u32>`. No compiler-enforced invariant prevents setting `line` without `column`. |
| **Trigger** | Manual or erroneous construction of `Span` with mismatched `line`/`column` presence. |
| **Consequence** | Downstream code that unwraps `line` or `column` gets inconsistent results. Diagnostic renderers may crash or produce garbled output. |
| **Affected Types** | `Span` |
| **Mitigation** | 1. `Span::with_location()` is the only public constructor for location-bearing spans; it requires both `line` and `column`. 2. `Span::new()` is byte-offset-only, setting both to `None`. 3. Doc-comment invariant: `line.is_some() == column.is_some()`. |
| **Verification** | Kani proof seed: `PS-001` — verify `Span::with_location(l, c)` produces `line == Some(l) && column == Some(c)`. Proptest: for all randomly constructed `Span`s via public constructors, `line.is_some() == column.is_some()`. |

---

## HA-05: Bounded State Hazard — usize → u32 Overflow

| Field | Detail |
|---|---|
| **Risk Tags** | `bounded-state`, `arithmetic` |
| **Root Cause** | `SourceSpan` (`vb_yaml`) and `SourceMark` (`vb_compile`) use `usize` for all fields (64-bit on x86_64). `Span` (`vb_core`) uses `u32`. Direct conversion truncates values > `u32::MAX`. |
| **Trigger** | Compiling a YAML file larger than 4 GiB or with more than 2^32 - 1 lines. |
| **Consequence** | Span byte offsets are truncated, producing incorrect diagnostic locations. Line/column numbers wrap, producing confusing output. |
| **Affected Types** | `From<SourceSpan> for Span`, `From<SourceMark> for Span` |
| **Mitigation** | 1. `YamlLimits` already caps source size (well below 4 GiB). 2. Use `u32::try_from(...)` and clamp to `u32::MAX` on overflow. 3. Line/column are bounded by source size — a 4 GiB file with 1-byte lines has ~4 billion lines, which requires ~4 * 10^9 lines, fitting in `u32` (max ~4.29 * 10^9). Practical YAML is much smaller. |
| **Verification** | Kani proof seed: `PS-002` — verify `From<SourceSpan> for Span` never panics for any `SourceSpan` within source size limits. |

---

## HA-06: Concurrency Hazard — None

| Field | Detail |
|---|---|
| **Risk Tags** | `concurrency` |
| **Assessment** | YAML parsing and compilation are **single-threaded** and **deterministic**. `Span`, `Diagnostic`, `DiagnosticCode`, `NonEmptyVec` are all `Send + Sync` (safe to transfer across threads) but are never shared mutably. No cross-thread mutation. |
| **Risk Level** | **NONE.** No concurrent access to span/diagnostic types. |

---

## HA-07: Unsafe / Provenance Hazard — None

| Field | Detail |
|---|---|
| **Risk Tags** | `unsafe`, `provenance` |
| **Assessment** | All affected modules declare `#![forbid(unsafe_code)]`. No raw pointers, no `MaybeUninit`, no FFI, no inline assembly. |
| **Risk Level** | **NONE.** |

---

## HA-08: Hostile Input Hazard — YAML Bomb with Span Overflow

| Field | Detail |
|---|---|
| **Risk Tags** | `hostile-input`, `bounded-state` |
| **Root Cause** | A malicious YAML file with deeply nested structures could generate an enormous number of `SourceSpan`s (one per node), exhausting memory during `build_source_map()`. |
| **Trigger** | YAML input with node count at or near `YamlLimits::max_nodes`. |
| **Consequence** | Memory exhaustion in `SourceMap` / `SemanticSourceMap` construction. |
| **Affected Types** | `SourceMap`, `SemanticSourceMap` — already bounded by `NodeLimitExceeded` check. |
| **Mitigation** | 1. `YamlLimits` cap node count before source map is built. 2. `SourceMap` stores `Vec<SourceSpan>` which is limited by the node limit. 3. `SemanticSourceMap` stores entries proportional to source size. |
| **Verification** | Existing test: `NodeLimitExceeded` error is produced before source map is built. No new verification needed. |

---

## HA-09: Performance Hazard — NonEmptyVec Iteration Hotspot

| Field | Detail |
|---|---|
| **Risk Tags** | `performance`, `cold-path` |
| **Root Cause** | `NonEmptyVec` wraps a `Vec` and adds a `head` field. Iterating requires an extra branch vs plain `Vec`. |
| **Trigger** | High-volume error collection in pathological YAML (many errors). |
| **Consequence** | Negligible. The compiler is a cold authoring path that runs once per workflow. Error count is bounded by `YamlLimits`. |
| **Affected Types** | `NonEmptyVec<T>` |
| **Mitigation** | `impl IntoIterator for NonEmptyVec<T>` yields `head` then delegates to `tail.into_iter()`. Accept one extra yield per collection — zero measurable impact on cold path. |
| **Verification** | Benchmark: compare `NonEmptyVec::iter()` vs `Vec::iter()` for 1..1000 elements. Must be within 5% for N=1 and equivalent for N>=10. Out of scope for this bead. |

---

## HA-10: Release / API Hazard — Breaking Change Cascade

| Field | Detail |
|---|---|
| **Risk Tags** | `public API`, `release`, `migration` |
| **Root Cause** | Adding fields to `Span`, `Diagnostic`, `ValidationError`, `CompileError`, and `YamlError` is a breaking API change for any code that pattern-matches exhaustively or constructs these types directly. |
| **Trigger** | Next crate release / CI build. |
| **Consequence** | Downstream crate builds fail. Integration tests break. |
| **Affected Types** | `Span`, `Diagnostic`, `ValidationError` (50 variants), `CompileError` (80+ variants), `YamlError` (19 variants) |
| **Mitigation** | 1. Fields are appended as the **last** field in each variant. 2. Pattern matches using `..` continue to compile. 3. Tests asserting `Span::ZERO` are updated. 4. `SourceMap` removal in `vb_core` requires callers to migrate to `vb_yaml::SourceMap`. 5. `diag_render.rs` duplicate is removed or re-exported. |
| **Verification** | CI: `moon ci` must pass. All workspace tests must pass. BDD tests for diagnostic output updated. |

---

## HA-11: Migration Hazard — Duplicate Diagnostic Conversion Drift

| Field | Detail |
|---|---|
| **Risk Tags** | `migration`, `drift` |
| **Root Cause** | Two identical-but-independent implementations of `ValidationError → Diagnostic` exist: `diagnostic.rs` (public, used by shared pipeline) and `diag_render.rs` (test-only, re-exports via `diag_tests`). |
| **Trigger** | Adding span propagation to one but not the other. |
| **Consequence** | Tests pass but production code produces stale diagnostics. Or vice versa — production works but tests fail. Silent divergence. |
| **Affected Types** | `diagnostic::diagnostic_from_error()`, `diag_render::diagnostic_from_error()` |
| **Mitigation** | Consolidate: `diagnostic.rs` is the canonical implementation. `diag_render.rs` becomes either: (a) removed entirely, or (b) a thin re-export of `diagnostic::diagnostic_from_error`. The `diag_codes` module can be absorbed or used by `diagnostic.rs`. |
| **Verification** | After consolidation, exactly one `match` on `ValidationError` exists. Grep for `fn diagnostic_from_error` returns one result. |

---

## HA-12: Model Hazard — NonEmptyVec Can Still Be Constructed Empty via Unsafe Deserialization

| Field | Detail |
|---|---|
| **Risk Tags** | `invariant`, `serialization` |
| **Root Cause** | If `NonEmptyVec<T>` implements `Deserialize`, a malicious or corrupted serialized form could contain zero elements, violating the invariant. |
| **Trigger** | Deserializing a `NonEmptyVec` from untrusted input. |
| **Consequence** | Type-level invariant (`len() >= 1`) is violated, causing downstream code to fail on `first()`. |
| **Affected Types** | `NonEmptyVec<T>` |
| **Mitigation** | `NonEmptyVec<T>` does **not** derive `Deserialize`. If serialization is needed later, implement a custom `Deserialize` that validates `len() >= 1`. Currently not needed — error accumulation is in-memory only. |
| **Verification** | `NonEmptyVec` does not have `Deserialize` in its derive list. |

---

## Summary Matrix

| Hazard ID | Risk Type | Severity | Mitigation Status |
|---|---|---|---|
| HA-01 | Temporal — Span propagation gap | **HIGH** | Type contract: TC-06 |
| HA-02 | Temporal — Canonical YAML span stripping | **HIGH** | Type contract: TC-07 |
| HA-03 | Temporal — Tree validation loses spans | **MEDIUM** | Backfill from AstMarks |
| HA-04 | Invariant — Unpaired line/column | **LOW** | Constructor discipline + doc |
| HA-05 | Bounded state — usize→u32 overflow | **LOW** | Clamping + limits check |
| HA-06 | Concurrency | **NONE** | Single-threaded |
| HA-07 | Unsafe / provenance | **NONE** | `forbid(unsafe_code)` |
| HA-08 | Hostile input — Span explosion | **LOW** | Node limit enforced |
| HA-09 | Performance — NonEmptyVec overhead | **NEGLIGIBLE** | Cold path |
| HA-10 | Release / API — Breaking changes | **MEDIUM** | Append-only + migration plan |
| HA-11 | Migration — Duplicate conversion drift | **MEDIUM** | Consolidation |
| HA-12 | Model — NonEmptyVec deserialization | **LOW** | No Deserialize derive |
