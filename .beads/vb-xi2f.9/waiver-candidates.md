# Waiver Candidates: YAML Path and Source Span Diagnostics

**Bead:** vb-xi2f.9
**Agent:** proof-planner (State 4)
**Schema:** waiver-candidates/v1

All candidates are **non-behavior-affecting** only. No behavior-affecting waiver candidates exist in this plan.

---

## WC-01: Light Flux Refinement Depth for NonEmptyVec

| Field | Detail |
|---|---|
| **Clause** | C3.1-C3.3 (NEVEC) |
| **Proof Seed** | PS-002 |
| **Reason** | NonEmptyVec's invariant (`len() >= 1`) is structural — enforced by private fields (`head: T`, `tail: Vec<T>`) and smart constructors (`new()`, `from_vec()`). It is not a numeric refinement on a public field that Flux can express ergonomically. Flux refinements on generic containers wrapping `Vec<T>` add significant annotation complexity without benefit over the Kani bounded proof (PO-K02) and proptest round-trip (PO-P02) which already verify all invariants. |
| **Boundary Proof** | PO-K02 (Kani) verifies `from_vec(empty)==None`, `len()>=1`, `is_empty()==false`, `first()` never panics. PO-P02 (proptest) verifies element preservation across `from_vec→into_vec` round-trip. |
| **Compensating Evidence** | Kani proof result + proptest results. If Flux becomes more ergonomic for generic struct refinement, this waiver can be revisited. |
| **Owner** | proof-plan-reviewer |
| **Expiry** | bead-landing |
| **Reviewer Status** | `pending` |

---

## WC-02: Miri on All Bridge Conversions (SourceSpan→SourceMark, SourceMark→Span)

| Field | Detail |
|---|---|
| **Clause** | C9.2 (SPAN-BRIDGE) |
| **Proof Seed** | PS-007 |
| **Reason** | The `SourceSpan→SourceMark` conversion is `usize→usize` (same-width, no truncation risk). The `SourceMark→Span` conversion is `u32→u32` (byte offsets already `u32`) and `usize→u32` (line/column clamping — same path as SourceSpan→Span). Miri adds value only for the narrow `usize→u32` casting path, which is already covered by PO-M01 focused on `SourceSpan→Span`. Running Miri on same-width or already-checked conversions produces no additional evidence. |
| **Boundary Proof** | PO-M01 covers the only risky cast: `SourceSpan→Span` where `usize` offsets/line/col are clamped to `u32`. PO-K07 verifies no-panic for the same path. |
| **Compensating Evidence** | PO-M01 evidence + PO-K07 Kani proof covering all `usize→u32` paths through the shared `clamp_u32` function. |
| **Owner** | proof-plan-reviewer |
| **Expiry** | bead-landing |
| **Reviewer Status** | `pending` |

---

## WC-03: Kani for PS-009 (SourceMap Removal) and PS-010 (Diagnostic Unification)

| Field | Detail |
|---|---|
| **Clause** | C7.1-C7.2 (UNIFY-DIAG), C8.1-C8.3 (RM-SRCMAP) |
| **Proof Seed** | PS-009, PS-010 |
| **Reason** | Both proof seeds are marked `behavior_affecting: false` — SourceMap removal is dead code cleanup, and diagnostic unification is a refactoring that consolidates two identical implementations into one. Neither introduces new runtime behavior, invariants, or state transitions. Kani bounded model checking cannot add value where there are no invariants to check. Static analysis (grep, cargo-check, cargo-test) is the appropriate and sufficient verification level. |
| **Boundary Proof** | PO-G01 (grep + cargo-check for SourceMap removal) confirms no residuals. PO-G02 (grep + cargo-test for unification) confirms single canonical conversion and all tests pass. |
| **Compensating Evidence** | Grep output showing zero SourceMap references in `crates/vb_core/src/`. Grep output showing exactly one `fn diagnostic_from_error` definition. `cargo test --workspace` passing. |
| **Owner** | proof-plan-reviewer |
| **Expiry** | bead-landing |
| **Reviewer Status** | `pending` |

---

## WC-04: Kani for PS-011 (SemanticSourceMap Message Annotation)

| Field | Detail |
|---|---|
| **Clause** | C11.1-C11.3 (SEM-MAP-MSG) |
| **Proof Seed** | PS-011 |
| **Reason** | Diagnostic message rendering involves string formatting (`format!()`, string concatenation, path appending) which Kani does not model well. Kani's string support is limited to abstract representations, and assertions about string content (e.g., "message contains `$.inputs`") require concrete string operations that Kani cannot verify. proptest (PO-P07) and unit tests are the appropriate verification level — they execute real string formatting and check real output content. |
| **Boundary Proof** | PO-P07 (proptest) generates YAML with known paths and intentional errors, verifies diagnostic message contains expected path text. Unit tests verify un-annotated messages. |
| **Compensating Evidence** | proptest results showing path annotation works across generated YAML inputs. Unit test results for the no-map fallback path. |
| **Owner** | proof-plan-reviewer |
| **Expiry** | bead-landing |
| **Reviewer Status** | `pending` |

---

## Summary

| Candidate | Type | Behavior Affecting? | Severity |
|---|---|---|---|
| WC-01 | Light Flux for NonEmptyVec (covered by Kani+proptest) | NO | Low |
| WC-02 | Miri on all bridge conversions (only SourceSpan→Span path has risk) | NO | Low |
| WC-03 | Kani for PS-009, PS-010 (dead code/refactoring) | NO | Low |
| WC-04 | Kani for PS-011 (string formatting not modelable in Kani) | NO | Low |

**No behavior-affecting waiver candidates exist. All 12 proof seeds have at least one required obligation covering their behavior.**
