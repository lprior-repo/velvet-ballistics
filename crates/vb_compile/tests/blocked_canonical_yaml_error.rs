// TF-VB-004: BEHAVIOR TESTS — canonical_yaml_error span preservation
// ============================================================================
//
// STATUS: IMPLEMENTATION UNBLOCKED (vb-xi2f.9 C2) — TESTS STILL PENDING
//
// Contract: C5.1-C5.3 (CANON-SPAN)
//   CompileError::CanonicalYaml now carries a `mark: SourceMark` field.
//   canonical_yaml_error() extracts SourceSpan from YamlError::span() and
//   converts it to SourceMark. When span is None, mark is SourceMark::unavailable().
//
// Implementation is complete (2026-05-25):
//   - CompileError::CanonicalYaml has `mark: SourceMark` field (kind.rs:22)
//   - canonical_yaml_error() calls error.span() and converts to SourceMark (part_01.rs:25-40)
//   - Kani harness updated for new field (kani_canonical_yaml_enrich.rs)
//
// Behavior test definitions awaiting test-writer phase:
//
// See: test-plan.md Section 8.5, behaviors B56-B60
// See: contract.md C5.1-C5.3
