# Waiver Candidates: vb-xi2f.29

**Bead**: vb-xi2f.29 — Digest Covers Together Semantics
**Date**: 2026-05-24

## Summary

No waiver candidates are proposed for this bead. All 15 planned obligations are behavior-affecting or required correctness properties that can be verified by Kani (bounded model checking), Proptest (property-based testing), and unit tests. No behavior-affecting obligation requires a waiver.

## Rationale

- **All requirements are testable/verifiable**: The surface area is small (~30 lines of new code + 1 line source fix already applied). Every contract clause maps to at least one obligation with a concrete command and expected evidence.
- **No tooling gaps**: Kani and proptest are available in the workspace. The existing Kani harness infrastructure (`kani_canonical_name.rs`) proves the approach works.
- **Bounded state**: The recursion depth is bounded by `MAX_LANGUAGE_NESTING_DEPTH = 8`, making Kani verification feasible without unbounded induction.
- **Pure computation**: `canonical_digest` has no I/O, no concurrency, no temporal state — all properties are structural inclusion properties.

## Non-Behavior Exceptions

None identified. The dead code in `compile/mod.rs` (REQ-xi2f29-021) is a separate cleanup concern tracked as a monitor item, not a waiver candidate.

## Rejected Waiver Opportunities

The following were considered and rejected as unnecessary:

1. **Shallow tla-plus applicability**: TLA+ would model a trivial single-step computation. Not a waiver — it is genuinely not applicable. Documented in `verifier-lane-decisions.jsonl` vld-xi2f29-012.
2. **Verus over-engineering**: The properties in scope are structural inclusion, not deep invariants. Kani provides better coverage. Not a waiver — documented as not_applicable in vld-xi2f29-013.
3. **Cargo-fuzz for blake3**: Fuzzing `canonical_digest` would effectively fuzz blake3's implementation, not our digest code. Not a waiver — documented as not_applicable in vld-xi2f29-017.
