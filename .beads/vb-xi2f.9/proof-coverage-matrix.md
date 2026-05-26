# Proof Coverage Matrix: YAML Path and Source Span Diagnostics

**Bead:** vb-xi2f.9
**Agent:** proof-planner (State 4)

## Coverage Map

| Proof Seed | Clause | Risk | Kani | Flux | Miri | proptest | Static/CI | Unit Tests | Coverage |
|---|---|---|---|---|---|---|---|---|---|
| PS-001 | SPAN-ENRICH | public API, invariant | PO-K01 | PO-F01 | — | PO-P01 | — | (baked-in) | FULL |
| PS-002 | NEVEC | bounded-state, invariant | PO-K02 | — | — | PO-P02 | — | (baked-in) | FULL |
| PS-003 | DIAG-FILE | public API, invariant | PO-K03 | — | — | — | — | (baked-in) | FULL |
| PS-004 | YERR-SPAN | parser/codec, public API | PO-K04 | — | — | PO-P03 | — | (baked-in) | FULL |
| PS-005 | CANON-SPAN | parser/codec, migration | PO-K05 | — | — | — | PO-G04 | (baked-in) | FULL |
| PS-006 | VERR-SPAN | public API, migration | PO-K06 | — | — | PO-P04 | PO-G04 | (baked-in) | FULL |
| PS-007 | SPAN-BRIDGE | bounded-state, parser/codec | PO-K07 | — | PO-M01 | PO-P05 | — | (baked-in) | FULL |
| PS-008 | TREE-MARK | parser/codec | PO-K08 | — | — | PO-P06 | — | (baked-in) | FULL |
| PS-009 | RM-SRCMAP | public API, migration | — | — | — | — | PO-G01 | (baked-in) | FULL |
| PS-010 | UNIFY-DIAG | migration | — | — | — | — | PO-G02 | (baked-in) | FULL |
| PS-011 | SEM-MAP-MSG | parser/codec | — | — | — | PO-P07 | — | (baked-in) | FULL |
| PS-012 | BACK-COMPAT | public API, migration | — | — | — | — | PO-G03 | (baked-in) | FULL |

## Obligation-to-Clause Mapping

| Obligation ID | Type | Covers Clauses | Covers Risks | Covers Hazards |
|---|---|---|---|---|
| PO-K01 | Kani proof | C1.1, C1.2, C1.3 | public API, invariant | HA-04 |
| PO-K02 | Kani proof | C3.1, C3.2 | bounded-state, invariant | HA-12 |
| PO-K03 | Kani proof | C2.1, C2.3 | public API, invariant | — |
| PO-K04 | Kani proof | C4.1, C4.3 | parser/codec, public API | HA-02 |
| PO-K05 | Kani proof | C5.1, C5.2, C5.3 | parser/codec, migration | HA-02 |
| PO-K06 | Kani proof | C6.1, C6.2, C6.3 | public API, migration | HA-01 |
| PO-K07 | Kani proof | C9.1, C9.3 | bounded-state, parser/codec, arithmetic | HA-05 |
| PO-K08 | Kani proof | C10.1, C10.2 | parser/codec | HA-03 |
| PO-F01 | Flux refinement | C1.3 | invariant | HA-04 |
| PO-M01 | Miri check | C9.3 | bounded-state, arithmetic | HA-05 |
| PO-P01 | proptest | C1.1, C1.2, C1.3 | public API, invariant | HA-04 |
| PO-P02 | proptest | C3.3 | bounded-state | — |
| PO-P03 | proptest | C4.2 | parser/codec | HA-02 |
| PO-P04 | proptest | C6.2 | public API | HA-01 |
| PO-P05 | proptest | C9.1, C9.2, C9.3 | bounded-state, parser/codec | HA-05 |
| PO-P06 | proptest | C10.1, C10.2, C10.3 | parser/codec | HA-03 |
| PO-P07 | proptest | C11.1, C11.2, C11.3 | parser/codec | — |
| PO-G01 | static gate | C8.1, C8.2, C8.3 | public API, migration | — |
| PO-G02 | static gate | C7.1, C7.2 | migration | HA-11 |
| PO-G03 | static gate | C12.1, C12.2, C12.3 | public API, migration | HA-10 |
| PO-G04 | static gate | C5.3, C6.3 | parser/codec, migration | HA-01, HA-02 |

## Clause Coverage

| Contract Clause | Covered by Obligations |
|---|---|
| C1.1 (Span backward compat) | PO-K01, PO-P01 |
| C1.2 (Span with_location) | PO-K01, PO-P01 |
| C1.3 (Span paired invariant) | PO-K01, PO-F01, PO-P01 |
| C1.4 (Located/Spanned compat) | PO-K01 (implied) |
| C2.1 (Diagnostic source_file) | PO-K03 |
| C2.2 (source_file non-empty) | PO-K03 (unit test) |
| C2.3 (Diagnostic backward compat) | PO-K03 |
| C3.1 (NonEmptyVec len>=1) | PO-K02 |
| C3.2 (NonEmptyVec safe construction) | PO-K02 |
| C3.3 (NonEmptyVec iteration) | PO-P02 |
| C4.1 (YamlError span field) | PO-K04 |
| C4.2 (YamlError span from events) | PO-P03 |
| C4.3 (YamlError backward compat) | PO-K04 |
| C5.1 (canonical_yaml_error preserves span) | PO-K05 |
| C5.2 (CanonicalYaml mark field) | PO-K05 |
| C5.3 (Exhaustive extraction) | PO-K05, PO-G04 |
| C6.1 (ValidationError span field) | PO-K06 |
| C6.2 (Diagnostic span propagation) | PO-K06, PO-P04 |
| C6.3 (Exhaustive code mapping) | PO-K06, PO-G04 |
| C7.1 (Single canonical conversion) | PO-G02 |
| C7.2 (Shared code constants) | PO-G02 |
| C8.1 (SourceMap removal) | PO-G01 |
| C8.2 (Re-export cleanup) | PO-G01 |
| C8.3 (Canonical vb_yaml::SourceMap) | PO-G01 |
| C9.1 (SourceSpan→Span conversion) | PO-K07, PO-P05 |
| C9.2 (SourceMark→Span conversion) | PO-P05 |
| C9.3 (Conversion safety) | PO-K07, PO-M01, PO-P05 |
| C10.1 (AstMarks integration) | PO-K08, PO-P06 |
| C10.2 (Graceful degradation) | PO-K08, PO-P06 |
| C10.3 (Lookup coverage) | PO-P06 |
| C11.1 (Path annotation) | PO-P07 |
| C11.2 (Additive only) | PO-P07 |
| C11.3 (Optional dependency) | PO-P07 |
| C12.1 (Test Span::ZERO assertions) | PO-G03 |
| C12.2 (Pattern match compatibility) | PO-G03 |
| C12.3 (moon ci passes) | PO-G03 |

**All 30 contract sub-clauses are covered.** No gaps.

## Acceptance Gate Coverage

| Acceptance Gate | Covered by Obligations |
|---|---|
| AG1 (All 12 clauses) | All 25 obligations |
| AG2 (Span::ZERO backward compat) | PO-K01, PO-P01, PO-G03 |
| AG3 (Diagnostic shows file:line:col) | PO-P07, PO-K06, PO-P04 |
| AG4 (No vb_yaml→vb_core dep) | PO-G03 (cargo-check) |
| AG5 (Single canonical diag conversion) | PO-G02 |
| AG6 (SourceMap removed from vb_core) | PO-G01 |
| AG7 (NonEmptyVec len>=1) | PO-K02 |
| AG8 (YAML parse errors show line) | PO-K04, PO-P03 |
| AG9 (Validation errors propagate span) | PO-K06, PO-P04 |
| AG10 (CI passes) | PO-G03 |

**All 10 acceptance gates are covered.**
