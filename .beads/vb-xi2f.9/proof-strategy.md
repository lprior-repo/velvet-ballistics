# Proof Strategy: YAML Path and Source Span Diagnostics

**Bead:** vb-xi2f.9
**Agent:** proof-planner (State 4)
**Schema:** proof-strategy/v1
**Date:** 2026-05-24

## Executive Summary

This bead enriches diagnostics across the velvet-ballistics compiler pipeline with YAML source paths, line/column spans, and non-empty error accumulation. The proof strategy emphasizes **lightweight formal methods** (Kani for bounded state verification, proptest for broad input-space coverage) and **CI-level gates** (cargo-check, cargo-test, grep) given the diagnostic nature of the changes. Heavyweight formal methods (TLA+, Verus) are not applicable — no temporal workflows, no concurrency, no unsafe code, no safety-critical deep invariants that require full functional proof.

## Risk Classification Summary

| Risk Category | Applicable? | Primary Verifier |
|---|---|---|
| temporal/state-machine | NO | — (single-threaded pipeline, HA-06) |
| Rust-local invariant | PARTIAL | Kani (bounded proofs), proptest (property coverage) |
| bounded state | YES | Kani (usize→u32 clamping, NonEmptyVec limits) |
| refinement/type-state | LIMITED | Flux (one light obligation for Span paired invariant) |
| concurrency | NO | — (HA-06: no concurrent access, HA-07: no unsafe) |
| unsafe/UB | NO | — (`#![forbid(unsafe_code)]`, Miri sanity only) |
| untrusted input | PARTIAL | Existing fuzz infra; no new fuzz needed |
| dependency/supply-chain | NO | — (no new deps) |
| performance | NO | — (cold authoring path, HA-09) |
| release-critical gate | YES | moon ci + cargo test workspace |

## Strategy Pillars

### Pillar 1: Kani Bounded Proofs (Primary Formal Method)

Kani covers the critical bounded-state invariants: Span paired invariant, NonEmptyVec invariant, Diagnostic source_file invariant, YamlError span construction, canonical_yaml_error span extraction across all 19 variants, ValidationError span propagation across ~50 variants, and the usize→u32 bridge panic-freedom.

**Bounds:** All Kani proofs use honest bounds — u32::MAX for Span fields, reasonable vec sizes (0..16 elements) for NonEmptyVec, realistic YAML error variant counts (19 max), ValidationError variant counts (50 max). No bounded-unbounded cheating.

**Proofs planned:**
- PO-K01: Span paired invariant — Kani proof that `Span::with_location(l,c)` produces `line==Some(l) && column==Some(c)` and `Span::new(s,e)` produces `line==None && column==None`
- PO-K02: NonEmptyVec invariants — Kani proof that `from_vec(empty)==None`, `new(x).first()==x`, invariants hold
- PO-K03: Diagnostic source_file — Kani proof that `Diagnostic::new(..., Span::ZERO)` produces `source_file: None`
- PO-K04: YamlError span construction — Kani proof that each variant can be constructed with `span: None` without panic
- PO-K05: canonical_yaml_error span extraction — Kani proof for each YamlError variant that `extract_span` yields correct mark
- PO-K06: ValidationError span propagation — Kani proof that `diagnostic_from_error(x)` when x has `span: s` yields `diagnostic.span == s`
- PO-K07: usize→u32 bridge panic-freedom — Kani proof that any usize produces safe conversion for `SourceSpan→Span`
- PO-K08: AstMarks backfill — Kani proof that when AstMarks has matching entry, error's mark is available

### Pillar 2: Proptest Property Coverage

Proptest covers the broad input-space behaviors that bounded Kani proofs can't catch in practice: round-trip preservation for NonEmptyVec, semantic source map integration, diagnostic message path annotation, and backward compatibility across random YAML inputs.

**Properties planned:**
- PO-P01: Span constructors for-all `(start, end, line, col)` — paired invariant holds
- PO-P02: NonEmptyVec round-trip `from_vec → into_vec` preserves all elements
- PO-P03: YamlError event-stream errors produce matching spans
- PO-P04: ValidationError for-all `(variant, span)` — `diagnostic.span == error.span`
- PO-P05: SourceSpan→SourceMark round-trip preserves offsets/lines/cols
- PO-P06: AstMarks from known YAML — errors have correct line/column
- PO-P07: SemanticSourceMap path annotation in diagnostic messages

### Pillar 3: CI and Static Gates

Non-behavior changes (dead code removal, diagnostic unification) are verified through static analysis.

**Gates planned:**
- PO-G01: SourceMap removal — `grep -r SourceMap crates/vb_core/src/` returns no results
- PO-G02: Diagnostic unification — `grep -r "fn diagnostic_from_error"` returns exactly one definition
- PO-G03: moon ci passes — all workspace tests, clippy, cargo check
- PO-G04: cargo test --workspace passes — no test regressions

### Pillar 4: Miri Safety Check

A lightweight Miri run on bridge conversion code ensures no UB in usize→u32 conversions (despite `forbid(unsafe_code)`, Miri catches integer overflow in debug mode that `as` casts don't).

**Check planned:**
- PO-M01: Miri run on `SourceSpan→Span` conversion with edge-case usize values

### Pillar 5: Flux Refinement (Light)

One Flux obligation for the Span paired invariant as a refinement type annotation, primarily as a defense-in-depth check that the Kani proof aligns with type-level specification.

**Refinement planned:**
- PO-F01: Flux refinement on `Span` that `line.is_some() == column.is_some()` for all constructor outputs


## Non-Vacuity Plan

### Purpose

Every Kani assumption, proptest strategy, stub, and model reduction carries the risk of vacuity — proving a property about an assumption rather than about reality. This section documents each assumption, why it is non-vacuous, and how its truth is independently validated.

### Kani Assumption Audit

| Obligation | Assumption | Non-Vacuity Check |
|---|---|---|
| PO-K01 | Span line/col are Option<u32> | True by type system; Rust compiler enforces Option<u32> layout. Kani proves no panic for all u32 values generated by `kani::any()`. |
| PO-K01 | u32 values bounded to [0, u32::MAX] | `kani::any::<u32>()` exhaustively explores the full u32 range (bounded model checking). No values excluded. |
| PO-K01 | start <= end enforced by constructor (not checked in this proof) | Constructor is a precondition, not an assumption of the proof. Span::new panics on invalid bounds in debug; Kani can separately verify this. |
| PO-K02 | Tail vec bounded to 0..15 for Kani | Model reduction, not assumption. Invariant `len() >= 1` holds regardless of size. Proptest PO-P02 covers up to 100 elements with round-trip preservation. |
| PO-K02 | T implements kani::Arbitrary | Generic type parameter — property holds for all T. Vacuity impossible: Kani generates arbitrary values for T within honest bounds. |
| PO-K03 | String allocation succeeds (abstract representation) | Heap string modeling limitation in Kani. Compensated by proptest PO-P01 (covers string content shapes) and unit tests for `source_file: Some(s)` where `s` is non-empty. |
| PO-K03 | DiagnosticCode::from_u16 for valid codes | DiagnosticCode is a finite enum. Kani harness generates valid codes via `kani::Arbitrary`. Invalid codes produce DiagnosticCode::Unknown (backward compat). |
| PO-K04 | YamlError has exactly 19 variants (as of contract date) | Compile-time enum; verified by exhaustive match in PO-K05 and PO-G04 unit tests. If a 20th variant is added, the Kani harness (exhaustive match) will fail to compile — catchable before CI. |
| PO-K05 | extract_span_from_yaml_error covers all 19 variants | Exhaustive match construction; Rust compiler enforces match completeness. PO-G04 unit tests use macro-generated exhaustive assertions. |
| PO-K06 | ValidationError has exactly ~50 variants | Compile-time enum; exhaustive match in PO-K06 harness + PO-G04 unit test. Same compile-time safety net as PO-K04. |
| PO-K07 | SourceSpan offsets are usize (64-bit target) | Architectural invariant; target is x86_64. Kani verifies for any usize::any() value — including usize::MAX — that clamp_u32/clamping produces no panic. |
| PO-K08 | AstMarks lookup bounded to 0..4 entries | Model reduction. Property (lookup succeeds → available==true) is invariant over table size. Proptest PO-P06 covers realistic YAML with larger lookup tables. |

### Proptest Strategy Edge-Case Coverage

| Obligation | Strategy | Edge-Case Coverage |
|---|---|---|
| PO-P01 | u32::ANY for start, end, line, col; filter start<=end | Covers u32::MAX, u32::MIN, 0, 1, and all intermediate values through proptest's uniform distribution. Shrinker finds minimal counterexamples. |
| PO-P02 | Vec<T> with 0..100 elements; round-trip from_vec→into_vec | Covers empty vec (expect None), single element, up to 100 elements. Shrinker reduces large vecs to minimal failing case. |
| PO-P03 | Event-stream errors with known EventSpan→SourceSpan conversion | Covers all 5 event-stream error variants (ParseError, AnchorAliasMerge, CustomTag, BinaryScalar, AmbiguousScalar) with randomized span values. |
| PO-P04 | For-all (ValidationError variant, Span value) | Covers all ~50 variants through exhaustive variant enumeration × random Span. Edge cases: Span::ZERO, Span with u32::MAX offsets, Span with Some(0) and Some(u32::MAX) line/col. |
| PO-P05 | SourceSpan with usize::ANY offsets | Covers usize::MAX, u32::MAX, u32::MAX+1, 0, 1. Round-trip SourceSpan→SourceMark preserves all fields. Shrinker identifies minimal offset values that trigger clamping. |
| PO-P06 | Known YAML with AstMarks backfill | Covers step-level errors, nested_key errors, trigger errors, document errors. Intentionally includes YAML with missing entries (graceful degradation to unavailable()). |
| PO-P07 | YAML with known paths (1..4 nesting) + intentional errors | Covers unknown fields, duplicate keys, empty map, absent SemanticSourceMap. Verifies path annotation is appended (not replacement). |

### Stub and Model Reduction Independence

| Trusted Base Entry | Model Reduction | Independent Validation |
|---|---|---|
| TB-022 (Kani string stubs) | Heap strings not modeled in Kani | Proptest PO-P01 + unit tests cover Diagnostic source_file shapes; string content is irrelevant to invariant proofs (paired field equality, len>=1) |
| TB-023 (bounded NonEmptyVec) | Tail bounded to 0..15 in Kani | Proptest PO-P02 covers 0..100 elements; invariant len()>=1 holds for all sizes |
| TB-024 (strict-profile YAML) | Proptest YAML limited to strict profile | Bead scope is strict-profile YAML only; non-strict YAML is rejected by parser before reaching enriched diagnostics |
| TB-025 (bounded AstMarks) | Lookup tables 0..4 in Kani | Proptest PO-P06 covers realistic YAML with large AstMarks; property is invariant over table size |
| TB-026 (Flux light refinement) | Complementary, not canonical | Kani PO-K01 is the canonical bounded proof; Flux adds type-level annotation for compile-time regression catching |

### Production Implementation Binding

All `kani::proof` harnesses (PO-K01 through PO-K08) exercise the **actual production implementation** via `#[kani::proof]` functions that call the same code paths used at runtime. No harness replaces production code with simplified stubs except where explicitly documented in the trusted-base ledger:

- Kani harnesses import and exercise `vb_core::span::Span::new`, `Span::with_location`, `Span::ZERO` — same functions called by the compiler pipeline
- Kani harnesses import and exercise `vb_core::non_empty_vec::NonEmptyVec::from_vec`, `::new`, `::with_tail` — same smart constructors
- Kani harnesses import and exercise `vb_compile` bridge conversion functions — same From implementations used at runtime
- The only exceptions are abstract representations of `saphyr-parser` event stream values (TB-029, TB-037) and `AstMarks` lookup tables (TB-025, TB-038), which are independently validated by proptest against realistic inputs


## Non-Applicable Lanes

| Verifier | Reason | Evidence |
|---|---|---|
| TLA+ | No temporal workflows, retries, leases, queues, or distributed protocols. Pipeline is single-threaded, deterministic, no async. | HA-06, workflow-model.md §5, §6 |
| Verus | Invariants are simple (paired fields, len>=1) and well-covered by Kani + proptest. No safety-critical deep functional proofs needed. Verus overhead (ghost code, spec fns) is unwarranted. | Domain-model.md §2, hazard-analysis.md |
| Loom | No threads, atomics, channels, locks, or async shutdown. Pipeline is pure synchronous compilation. | HA-06, HA-07, boundary-map.md §170 |
| cargo-fuzz | No new untrusted input parsing boundary. YAML parsing already has existing fuzz targets. Span enrichment is additive payload, not new parsing logic. | delivery-scope.jsonl, existing fuzz/ directory |
| cargo-mutants | Diagnostic payload changes are additive (new optional fields). Mutation testing on existing logic would not find new bugs. Not cost-effective for this bead. | hazard-analysis.md §HA-10, §HA-11 |

## Waiver Candidates

1. **WC-01: Flux depth for NonEmptyVec** — The NonEmptyVec invariant (`len()>=1`) is structural (guaranteed by private fields + smart constructors), not a numeric refinement on a public field. Flux refinements on generic containers with Vec internals add complexity without benefit over Kani + unit tests.
2. **WC-02: Miri on all bridge conversions** — Only the usize→u32 conversion path (`SourceSpan→Span`) involves potentially lossy integer conversion. The `SourceSpan→SourceMark` and `SourceMark→Span` paths are same-width conversions (usize→usize, u32→u32) that Miri cannot add value to.
3. **WC-03: Kani for PS-009 (SourceMap removal) and PS-010 (diagnostic unification)** — These are pure refactoring changes (behavior_affecting: false). Static analysis (grep, cargo-check, cargo-test) is the appropriate verification level.

## Trusted Base

See `trusted-base-plan.md` for detailed assumptions, stubs, and trusted surfaces.

## Blockers

**None.** All required tools are available in the workspace. No external dependencies or unavailable tooling.

## Traceability

All 12 proof seeds (PS-001 through PS-012) map to at least one obligation. All 30 traceability entries (TR-001 through TR-030) are covered. No orphaned requirements.
