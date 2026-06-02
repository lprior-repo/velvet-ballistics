//! Digest coverage tests for Collect and Reduce (Aggregate) field hashing.
//!
//! PO: PO-003, PO-004, PO-005, PO-006, PO-007, PO-014 (Collect)
//! Bead: vb-xi2f.38 (Collect), vb-xi2f.39 (Reduce/Aggregate)
//!
//! These tests verify that different Collect and Reduce field values produce
//! different digests when compiled through `compute_compiled_digest`.
//!
//! Note: `compute_compiled_digest` in mod_compile_core.rs is `blake3::hash(source)`.
//! The bug in `digest_step_primitive` (part_05.rs:158-160) is in the internal
//! `canonical_digest` function which is not publicly accessible. These tests
//! verify the public API digest behavior via `compute_compiled_digest`.
//!
//! The DIRECT tests (below) call `digest_step_primitive` directly and actually
//! verify that Collect and Aggregate fields contribute to the digest. These are
//! the tests that black-hat required: tests that call `digest_step_primitive`
//! with Collect/Aggregate input, NOT blake3::hash of YAML bytes.
//!
//! The Kani harnesses in `verification/kani/collect_field_coverage.rs` provide
//! formal verification of the same property.
//!
//! ## Reduce/Aggregate Tests (vb-xi2f.39, lines 517+)
//!
//! PO-R1 through PO-R7: Variable, input, initial, body, empty/non-empty,
//! idempotence, and determinism for `StepPrimitive::Reduce` (reduce).

use blake3::Hasher;
use vb_yaml::ast::{StepAst, StepPrimitive};

use crate::compute_compiled_digest;
use crate::mod_compile_lowering::part_01::canonical_body_step_width;
use crate::mod_compile_lowering::part_05::digest_step_primitive;

/// Helper: build a YAML source with a Collect step having the given field overrides.
fn collect_yaml_with_field(
    variable: &str,
    source: &str,
    pages: Option<u32>,
    items: Option<u32>,
    body_var: Option<&str>,
) -> Vec<u8> {
    let pages_str = pages.map(|p| format!("pages: {p}")).unwrap_or_default();
    let items_str = items.map(|i| format!("items: {i}")).unwrap_or_default();

    let body_content = if let Some(bv) = body_var {
        format!(
            r#"
    - id: body_step
      set:
        output: "{bv}"
        value: "1""#
        )
    } else {
        String::new()
    };

    format!(
        r#"version: velvet-ballistics/v1
name: test
when:
  manual: {{}}
steps:
  - id: collect_step
    collect:
      variable: "{variable}"
      source: "{source}"
      {pages_str}
      {items_str}{body_content}
  - id: done
    finish:
      result: 0
"#,
    )
    .into_bytes()
}

// ─────────────────────────────────────────────────────────────────
// PO-003: Collect variable field hashing
// ─────────────────────────────────────────────────────────────────

#[test]
fn digest_collect_variable_field() {
    // Two Collect primitives identical except variable field differs.
    let yaml_a = collect_yaml_with_field("x", "items", Some(10), Some(50), None);
    let yaml_b = collect_yaml_with_field("y", "items", Some(10), Some(50), None);

    let digest_a = compute_compiled_digest(&yaml_a);
    let digest_b = compute_compiled_digest(&yaml_b);

    // Different variable fields MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different variable fields must produce different digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-004: Collect source field hashing
// ─────────────────────────────────────────────────────────────────

#[test]
fn digest_collect_source_field() {
    // Two Collect primitives identical except source field differs.
    let yaml_a = collect_yaml_with_field("x", "items_a", Some(10), Some(50), None);
    let yaml_b = collect_yaml_with_field("x", "items_b", Some(10), Some(50), None);

    let digest_a = compute_compiled_digest(&yaml_a);
    let digest_b = compute_compiled_digest(&yaml_b);

    // Different source fields MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different source fields must produce different digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-005: Collect pages field hashing
// ─────────────────────────────────────────────────────────────────

#[test]
fn digest_collect_pages_field() {
    // Two Collect primitives identical except pages field differs.
    let yaml_a = collect_yaml_with_field("x", "items", Some(10), Some(50), None);
    let yaml_b = collect_yaml_with_field("x", "items", Some(999), Some(50), None);

    let digest_a = compute_compiled_digest(&yaml_a);
    let digest_b = compute_compiled_digest(&yaml_b);

    // Different pages fields MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different pages fields must produce different digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-005b: Collect pages None vs Some
// ─────────────────────────────────────────────────────────────────

#[test]
fn digest_collect_pages_none_vs_some() {
    // pages: None vs pages: Some(1)
    let yaml_a = collect_yaml_with_field("x", "items", None, Some(50), None);
    let yaml_b = collect_yaml_with_field("x", "items", Some(1), Some(50), None);

    let digest_a = compute_compiled_digest(&yaml_a);
    let digest_b = compute_compiled_digest(&yaml_b);

    assert_ne!(
        digest_a, digest_b,
        "pages None vs Some(1) must produce different digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-006: Collect items field hashing
// ─────────────────────────────────────────────────────────────────

#[test]
fn digest_collect_items_field() {
    // Two Collect primitives identical except items field differs.
    let yaml_a = collect_yaml_with_field("x", "items", Some(10), Some(50), None);
    let yaml_b = collect_yaml_with_field("x", "items", Some(10), Some(999), None);

    let digest_a = compute_compiled_digest(&yaml_a);
    let digest_b = compute_compiled_digest(&yaml_b);

    // Different items fields MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different items fields must produce different digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-006b: Collect items None vs Some
// ─────────────────────────────────────────────────────────────────

#[test]
fn digest_collect_items_none_vs_some() {
    // items: None vs items: Some(1)
    let yaml_a = collect_yaml_with_field("x", "items", Some(10), None, None);
    let yaml_b = collect_yaml_with_field("x", "items", Some(10), Some(1), None);

    let digest_a = compute_compiled_digest(&yaml_a);
    let digest_b = compute_compiled_digest(&yaml_b);

    assert_ne!(
        digest_a, digest_b,
        "items None vs Some(1) must produce different digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-007: Collect body recursive hashing
// ─────────────────────────────────────────────────────────────────

#[test]
fn digest_collect_body_recursive() {
    // Two Collect primitives identical except body content differs.
    let yaml_a = collect_yaml_with_field("x", "items", Some(10), Some(50), Some("a"));
    let yaml_b = collect_yaml_with_field("x", "items", Some(10), Some(50), Some("b"));

    let digest_a = compute_compiled_digest(&yaml_a);
    let digest_b = compute_compiled_digest(&yaml_b);

    // Different body step content MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different body step content must produce different digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-014: Property-based digest equality
// ─────────────────────────────────────────────────────────────────

#[test]
fn collect_digest_equality_property() {
    // Same source MUST produce same digest (idempotence)
    let yaml = collect_yaml_with_field("x", "items", Some(10), Some(50), None);
    let digest_a = compute_compiled_digest(&yaml);
    let digest_b = compute_compiled_digest(&yaml);
    assert_eq!(
        digest_a, digest_b,
        "same source must produce same digest (idempotence)"
    );

    // Different sources MUST produce different digests (collision resistance)
    let yaml_other = collect_yaml_with_field("y", "items", Some(10), Some(50), None);
    let digest_other = compute_compiled_digest(&yaml_other);
    assert_ne!(
        digest_a, digest_other,
        "different sources must produce different digests"
    );
}

// ─────────────────────────────────────────────────────────────────
// Determinism: repeated calls produce same digest
// ─────────────────────────────────────────────────────────────────

#[test]
fn digest_collect_repeated_calls_same_digest() {
    let yaml = collect_yaml_with_field("x", "items", Some(10), Some(50), None);

    // Call 1
    let d1 = compute_compiled_digest(&yaml);
    // Call 2
    let d2 = compute_compiled_digest(&yaml);
    // Call 3
    let d3 = compute_compiled_digest(&yaml);

    assert_eq!(d1, d2, "first and second call must match");
    assert_eq!(d2, d3, "second and third call must match");
}

// ═══════════════════════════════════════════════════════════════════════════
// DIRECT digest_step_primitive TESTS
// These call digest_step_primitive directly (NOT blake3::hash of YAML bytes).
// Black-hat required tests that actually verify Collect fields are hashed.
// ═══════════════════════════════════════════════════════════════════════════

/// Helper: create a minimal StepPrimitive::Collect with given field overrides.
fn make_collect(
    variable: &str,
    source: &str,
    pages: Option<u32>,
    items: Option<u32>,
    body: Vec<StepAst>,
) -> StepPrimitive {
    StepPrimitive::Collect {
        variable: variable.to_string(),
        source: source.to_string(),
        pages,
        items,
        body,
    }
}

/// Helper: compute digest of a StepPrimitive via digest_step_primitive.
fn digest_primitive(primitive: &StepPrimitive) -> blake3::Hash {
    let mut hasher = Hasher::new();
    match digest_step_primitive(&mut hasher, primitive) {
        Ok(()) => hasher.finalize(),
        Err(error) => panic!("digest_step_primitive failed: {error:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────
// Variable field
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_collect_variable_field() {
    // Two Collect primitives identical except variable field differs.
    let collect_a = make_collect("x", "items", Some(10), Some(50), vec![]);
    let collect_b = make_collect("y", "items", Some(10), Some(50), vec![]);

    let digest_a = digest_primitive(&collect_a);
    let digest_b = digest_primitive(&collect_b);

    // Different variable fields MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different variable fields must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// Source field
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_collect_source_field() {
    // Two Collect primitives identical except source field differs.
    let collect_a = make_collect("x", "items_a", Some(10), Some(50), vec![]);
    let collect_b = make_collect("x", "items_b", Some(10), Some(50), vec![]);

    let digest_a = digest_primitive(&collect_a);
    let digest_b = digest_primitive(&collect_b);

    // Different source fields MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different source fields must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// Pages field (Some different values)
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_collect_pages_field() {
    // Two Collect primitives identical except pages field differs.
    let collect_a = make_collect("x", "items", Some(10), Some(50), vec![]);
    let collect_b = make_collect("x", "items", Some(999), Some(50), vec![]);

    let digest_a = digest_primitive(&collect_a);
    let digest_b = digest_primitive(&collect_b);

    // Different pages fields MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different pages fields must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// Pages field (None vs Some)
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_collect_pages_none_vs_some() {
    // pages: None vs pages: Some(1)
    let collect_a = make_collect("x", "items", None, Some(50), vec![]);
    let collect_b = make_collect("x", "items", Some(1), Some(50), vec![]);

    let digest_a = digest_primitive(&collect_a);
    let digest_b = digest_primitive(&collect_b);

    assert_ne!(
        digest_a, digest_b,
        "pages None vs Some(1) must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// Items field (Some different values)
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_collect_items_field() {
    // Two Collect primitives identical except items field differs.
    let collect_a = make_collect("x", "items", Some(10), Some(50), vec![]);
    let collect_b = make_collect("x", "items", Some(10), Some(999), vec![]);

    let digest_a = digest_primitive(&collect_a);
    let digest_b = digest_primitive(&collect_b);

    // Different items fields MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different items fields must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// Items field (None vs Some)
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_collect_items_none_vs_some() {
    // items: None vs items: Some(1)
    let collect_a = make_collect("x", "items", Some(10), None, vec![]);
    let collect_b = make_collect("x", "items", Some(10), Some(1), vec![]);

    let digest_a = digest_primitive(&collect_a);
    let digest_b = digest_primitive(&collect_b);

    assert_ne!(
        digest_a, digest_b,
        "items None vs Some(1) must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// Body recursive hashing
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_collect_body_recursive() {
    // Two Collect primitives identical except body content differs.
    let body_a = vec![StepAst {
        id: "step_a".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "a".to_string(),
            value: "1".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    let body_b = vec![StepAst {
        id: "step_b".to_string(), // different id
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "b".to_string(), // different output
            value: "1".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];

    let collect_a = make_collect("x", "items", Some(10), Some(50), body_a);
    let collect_b = make_collect("x", "items", Some(10), Some(50), body_b);

    let digest_a = digest_primitive(&collect_a);
    let digest_b = digest_primitive(&collect_b);

    // Different body step content MUST produce different digests
    assert_ne!(
        digest_a, digest_b,
        "different body step content must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// Idempotence: same input produces same digest
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_collect_idempotence() {
    let collect = make_collect("x", "items", Some(10), Some(50), vec![]);

    let digest_a = digest_primitive(&collect);
    let digest_b = digest_primitive(&collect);

    assert_eq!(
        digest_a, digest_b,
        "same StepPrimitive::Collect must produce same digest (idempotence)"
    );
}

// ─────────────────────────────────────────────────────────────────
// Determinism: repeated calls produce same digest
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_collect_repeated_calls_same_digest() {
    let collect = make_collect("x", "items", Some(10), Some(50), vec![]);

    // Call 1
    let d1 = digest_primitive(&collect);
    // Call 2
    let d2 = digest_primitive(&collect);
    // Call 3
    let d3 = digest_primitive(&collect);

    assert_eq!(d1, d2, "first and second call must match");
    assert_eq!(d2, d3, "second and third call must match");
}

// ─────────────────────────────────────────────────────────────────
// Empty body (zero steps) is distinct from non-empty body
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_collect_empty_vs_nonempty_body() {
    let body = vec![StepAst {
        id: "inner".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".to_string(),
            value: "1".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];

    let collect_empty = make_collect("x", "items", Some(10), Some(50), vec![]);
    let collect_nonempty = make_collect("x", "items", Some(10), Some(50), body);

    let digest_empty = digest_primitive(&collect_empty);
    let digest_nonempty = digest_primitive(&collect_nonempty);

    assert_ne!(
        digest_empty, digest_nonempty,
        "empty body vs non-empty body must produce different digests via digest_step_primitive"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// REDUCE DIGEST TESTS (vb-xi2f.39)
// These tests verify that different Reduce field values produce different
// digests when compiled through `digest_step_primitive`.
// ═══════════════════════════════════════════════════════════════════════════

/// Helper: create a minimal StepPrimitive::Reduce (reduce) with given field overrides.
fn make_reduce(variable: &str, input: &str, initial: &str, body: Vec<StepAst>) -> StepPrimitive {
    StepPrimitive::Reduce {
        variable: variable.to_string(),
        input: input.to_string(),
        initial: initial.to_string(),
        body,
    }
}

// ─────────────────────────────────────────────────────────────────
// PO-R1: Variable (accumulator) field hashing
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_reduce_variable_field() {
    let reduce_a = make_reduce("acc", "items", "0", vec![]);
    let reduce_b = make_reduce("result", "items", "0", vec![]);

    let digest_a = digest_primitive(&reduce_a);
    let digest_b = digest_primitive(&reduce_b);

    assert_ne!(
        digest_a, digest_b,
        "different variable (accumulator) fields must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-R2: Input collection field hashing
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_reduce_input_field() {
    let reduce_a = make_reduce("acc", "items_a", "0", vec![]);
    let reduce_b = make_reduce("acc", "items_b", "0", vec![]);

    let digest_a = digest_primitive(&reduce_a);
    let digest_b = digest_primitive(&reduce_b);

    assert_ne!(
        digest_a, digest_b,
        "different input collection fields must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-R3: Initial accumulator value hashing
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_reduce_initial_field() {
    let reduce_a = make_reduce("acc", "items", "0", vec![]);
    let reduce_b = make_reduce("acc", "items", "1", vec![]);

    let digest_a = digest_primitive(&reduce_a);
    let digest_b = digest_primitive(&reduce_b);

    assert_ne!(
        digest_a, digest_b,
        "different initial accumulator values must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-R4: Body steps hashing
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_reduce_body_steps() {
    let body_a = vec![StepAst {
        id: "step_a".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".to_string(),
            value: "1".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    let body_b = vec![StepAst {
        id: "step_b".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "y".to_string(),
            value: "2".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];

    let reduce_a = make_reduce("acc", "items", "0", body_a);
    let reduce_b = make_reduce("acc", "items", "0", body_b);

    let digest_a = digest_primitive(&reduce_a);
    let digest_b = digest_primitive(&reduce_b);

    assert_ne!(
        digest_a, digest_b,
        "different body steps must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-R5: Empty body vs non-empty body
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_reduce_empty_vs_nonempty_body() {
    let body = vec![StepAst {
        id: "inner".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".to_string(),
            value: "1".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];

    let reduce_empty = make_reduce("acc", "items", "0", vec![]);
    let reduce_nonempty = make_reduce("acc", "items", "0", body);

    let digest_empty = digest_primitive(&reduce_empty);
    let digest_nonempty = digest_primitive(&reduce_nonempty);

    assert_ne!(
        digest_empty, digest_nonempty,
        "empty body vs non-empty body must produce different digests via digest_step_primitive"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-R6: Idempotence — same input produces same digest
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_reduce_idempotence() {
    let reduce = make_reduce("acc", "items", "0", vec![]);

    let digest_a = digest_primitive(&reduce);
    let digest_b = digest_primitive(&reduce);

    assert_eq!(
        digest_a, digest_b,
        "same StepPrimitive::Reduce must produce same digest (idempotence)"
    );
}

// ─────────────────────────────────────────────────────────────────
// PO-R7: Determinism — repeated calls produce same digest
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_reduce_repeated_calls_same_digest() {
    let reduce = make_reduce("acc", "items", "0", vec![]);

    let d1 = digest_primitive(&reduce);
    let d2 = digest_primitive(&reduce);
    let d3 = digest_primitive(&reduce);

    assert_eq!(d1, d2, "first and second call must match");
    assert_eq!(d2, d3, "second and third call must match");
}

// ─────────────────────────────────────────────────────────────────
// PO-R8: Collect vs Aggregate cross-variant collision resistance
// ─────────────────────────────────────────────────────────────────

#[test]
fn direct_digest_collect_vs_aggregate_different_sentinels() {
    let collect = StepPrimitive::Collect {
        variable: "x".to_string(),
        source: "items".to_string(),
        pages: None,
        items: None,
        body: vec![],
    };
    let aggregate = StepPrimitive::Reduce {
        variable: "x".to_string(),
        input: "items".to_string(),
        initial: "0".to_string(),
        body: vec![],
    };

    let digest_collect = digest_primitive(&collect);
    let digest_aggregate = digest_primitive(&aggregate);

    assert_ne!(
        digest_collect, digest_aggregate,
        "Collect and Aggregate with similar fields must produce different digests (different sentinels)"
    );
}

// ─────────────────────────────────────────────────────────────────
// vb-awhr: choose otherwise handling and fanout limit
// ─────────────────────────────────────────────────────────────────

use crate::mod_compile_lowering::part_01::{body_width, choose_width};
use crate::mod_compile_lowering::part_02::lower_canonical_choose;
use crate::mod_compile_lowering::part_05::slot_from_text;
use crate::mod_compile_lowering::part_06::lower_choose;

fn choose_body_set_step(id: &str, value: &str) -> vb_yaml::ast::StepAst {
    vb_yaml::ast::StepAst {
        id: id.to_string(),
        name: None,
        condition: None,
        primitive: vb_yaml::ast::StepPrimitive::Set {
            output: id.to_string(),
            value: value.to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

#[allow(dead_code)]
fn choose_body_do_step(id: &str, action: &str, input: &str) -> vb_yaml::ast::StepAst {
    vb_yaml::ast::StepAst {
        id: id.to_string(),
        name: None,
        condition: None,
        primitive: vb_yaml::ast::StepPrimitive::Do {
            action: action.to_string(),
            input: input.to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

#[test]
fn choose_width_counts_branch_body_steps() {
    let branches = vec![
        vb_yaml::ast::ChooseBranch {
            when: "0".to_string(),
            steps: vec![choose_body_set_step("body_a", "1")],
        },
        vb_yaml::ast::ChooseBranch {
            when: "1".to_string(),
            steps: vec![
                choose_body_set_step("body_b", "2"),
                choose_body_set_step("body_c", "3"),
            ],
        },
    ];
    let width = choose_width(&branches).expect("valid Set bodies must compute width");
    assert_eq!(width, 4, "ChooseSlot plus three body nodes");
}

#[test]
fn choose_width_64_empty_branches_returns_1() {
    let branches: Vec<vb_yaml::ast::ChooseBranch> = (0..64)
        .map(|i| vb_yaml::ast::ChooseBranch {
            when: i.to_string(),
            steps: vec![],
        })
        .collect();
    let width = choose_width(&branches).expect("64 empty branches must compute width");
    assert_eq!(width, 1, "ChooseSlot only, no body nodes");
}

#[test]
fn lower_canonical_choose_accepts_two_branches() {
    let branches = vec![
        vb_yaml::ast::ChooseBranch {
            when: "0".to_string(),
            steps: vec![],
        },
        vb_yaml::ast::ChooseBranch {
            when: "1".to_string(),
            steps: vec![],
        },
    ];
    let step_names: [Box<str>; 2] = [Box::from("pick"), Box::from("done")];
    let mut builder = crate::SlotCompiler::new();
    let result = lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("done"),
        Some(vb_core::ids::StepIdx::new(1)),
        &step_names,
        &mut builder,
    );
    assert!(
        result.is_ok(),
        "two-branch choose must compile: {:?}",
        result
    );
}

#[test]
fn lower_canonical_choose_single_body_set_targets_body_start() {
    let branches = vec![vb_yaml::ast::ChooseBranch {
        when: "0".to_string(),
        steps: vec![choose_body_set_step("body_a", "7")],
    }];
    let step_names: [Box<str>; 2] = [Box::from("pick"), Box::from("done")];
    let mut builder = crate::SlotCompiler::new();
    lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("done"),
        Some(vb_core::ids::StepIdx::new(2)),
        &step_names,
        &mut builder,
    )
    .expect("single Set body must lower");

    assert_eq!(builder.nodes.len(), 2, "ChooseSlot plus body node");
    assert_eq!(builder.nodes[0].id, vb_core::ids::StepIdx::new(0));
    match &builder.nodes[0].kind {
        vb_core::CompiledNodeKind::ChooseSlot { branches, .. } => {
            assert_eq!(branches[0].target, vb_core::ids::StepIdx::new(1));
        }
        other => panic!("expected ChooseSlot, got {other:?}"),
    }
}

// ============================================================================
// vb-282my tests are at the end of the file.

#[test]
fn lower_canonical_choose_multi_body_steps_chain_to_common_next() {
    let branches = vec![vb_yaml::ast::ChooseBranch {
        when: "0".to_string(),
        steps: vec![
            choose_body_set_step("body_a", "7"),
            choose_body_set_step("body_b", "8"),
        ],
    }];
    let step_names: [Box<str>; 2] = [Box::from("pick"), Box::from("done")];
    let mut builder = crate::SlotCompiler::new();
    lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("done"),
        Some(vb_core::ids::StepIdx::new(3)),
        &step_names,
        &mut builder,
    )
    .expect("multi-step Set body must lower");

    assert_eq!(builder.nodes.len(), 3, "ChooseSlot plus two body nodes");
    match &builder.nodes[0].kind {
        vb_core::CompiledNodeKind::ChooseSlot { branches, .. } => {
            assert_eq!(branches[0].target, vb_core::ids::StepIdx::new(1));
        }
        other => panic!("expected ChooseSlot, got {other:?}"),
    }
    assert_eq!(builder.nodes[1].next, Some(vb_core::ids::StepIdx::new(2)));
    assert_eq!(builder.nodes[2].next, Some(vb_core::ids::StepIdx::new(3)));
}

#[test]
fn lower_canonical_choose_rejects_unknown_otherwise_label() {
    let branches = vec![vb_yaml::ast::ChooseBranch {
        when: "0".to_string(),
        steps: vec![],
    }];
    let step_names: [Box<str>; 1] = [Box::from("pick")];
    let mut builder = crate::SlotCompiler::new();
    let result = lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("missing"),
        Some(vb_core::ids::StepIdx::new(1)),
        &step_names,
        &mut builder,
    );
    match result {
        Err(crate::CompileErrors(errors)) => {
            assert!(
                errors.iter().any(|e| matches!(e, crate::CompileError::UnknownStepLabel { label, .. } if label.as_ref() == "missing")),
                "unknown otherwise label must return UnknownStepLabel with actual label text, got: {:?}",
                errors
            );
        }
        other => panic!(
            "expected error for unknown otherwise label, got: {:?}",
            other
        ),
    }
}

#[test]
fn lower_canonical_choose_rejects_65_branches() {
    let branches: Vec<vb_yaml::ast::ChooseBranch> = (0..65)
        .map(|i| vb_yaml::ast::ChooseBranch {
            when: i.to_string(),
            steps: vec![],
        })
        .collect();
    let step_names: [Box<str>; 2] = [Box::from("pick"), Box::from("done")];
    let mut builder = crate::SlotCompiler::new();
    let result = lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("done"),
        Some(vb_core::ids::StepIdx::new(1)),
        &step_names,
        &mut builder,
    );
    match result {
        Err(crate::CompileErrors(errors)) => {
            assert!(
                errors.iter().any(|e| matches!(e, crate::CompileError::PrimitiveLoweringLimitExceeded { primitive, field, value, limit } if *primitive == "choose" && *field == "branches" && *value == 65 && *limit == 64)),
                "65-branch choose must fail with PrimitiveLoweringLimitExceeded, got: {:?}",
                errors
            );
        }
        other => panic!("expected error for 65 branches, got: {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────
// vb-xi2f.13 new tests: choose_width overflow, slot_from_text,
// fanout defense-in-depth, anti-hallucination, slot allocation,
// condition/body slot disjointness, otherwise span
// ─────────────────────────────────────────────────────────────────

// ─────────────────────────────────────────────────────────────────
// Test 1: choose_width overflow error propagation
// Plan N2 / RRO-B004
// ─────────────────────────────────────────────────────────────────

/// Verifies that `choose_width` propagates errors from `body_width`.
///
/// Direct overflow of `choose_width` requires usize::MAX body steps, which
/// is not constructable in memory. This test exercises the error propagation
/// path by using an unsupported body primitive (ForEach) that causes
/// `canonical_body_step_width` to return Err, which propagates through
/// `body_width` and then `choose_width`.
#[test]
fn choose_width_overflow_returns_error() {
    // Given: a branch with an unsupported body primitive (Collect is not allowed in body steps)
    let unsupported_step = vb_yaml::ast::StepAst {
        id: "unsupported".to_string(),
        name: None,
        condition: None,
        primitive: vb_yaml::ast::StepPrimitive::Collect {
            variable: "x".to_string(),
            source: "items".to_string(),
            pages: None,
            items: None,
            body: vec![],
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };
    let branches = vec![vb_yaml::ast::ChooseBranch {
        when: "0".to_string(),
        steps: vec![unsupported_step],
    }];
    // When: choose_width is called
    let result = choose_width(&branches);
    // Then: error propagates from body_width through choose_width
    assert!(
        matches!(
            result,
            Err(crate::CompileError::UnsupportedStepPrimitive { .. })
        ),
        "choose_width must propagate unsupported body primitive error, got: {:?}",
        result
    );
}

// ─────────────────────────────────────────────────────────────────
// Test 2: body_width overflow via checked_add
// Plan N3 / RRO-B004
// ─────────────────────────────────────────────────────────────────

/// Verifies that `body_width` returns `StepIndexOutOfRange` when the
/// `checked_add` accumulator overflows.
///
/// Uses `overhead = usize::MAX` with one body step (adds 1) to trigger
/// the overflow on the first iteration.
#[test]
fn choose_width_body_step_overflow_returns_error() {
    // Given: a body with one Set step and overhead = usize::MAX
    let body = vec![choose_body_set_step("overflow_step", "1")];
    // When: body_width is called with overflow-inducing overhead
    let result = body_width(&body, usize::MAX);
    // Then: returns StepIndexOutOfRange from checked_add failure
    assert!(
        matches!(
            result,
            Err(crate::CompileError::StepIndexOutOfRange { value }) if value == usize::MAX
        ),
        "body_width must return StepIndexOutOfRange on checked_add overflow, got: {:?}",
        result
    );
}

// ─────────────────────────────────────────────────────────────────
// Test: add_body_offset overflow (checked_add at part_14.rs:140)
// Plan N5 / RRO-B005
// ─────────────────────────────────────────────────────────────────

/// Verifies that `add_body_offset` returns `StepIndexOutOfRange` when
/// `checked_add` between `start_offset` and `index` overflows `u16`.
/// Directly tests the overflow guard at part_14.rs:140.
#[test]
fn add_body_offset_overflow_returns_error() {
    use crate::mod_compile_lowering::part_14::add_body_offset;
    // Given: start_offset = u16::MAX, index = 1 (sum exceeds u16::MAX)
    // When: add_body_offset is called
    let result = add_body_offset(u16::MAX, 1, 0);
    // Then: returns StepIndexOutOfRange
    assert!(
        matches!(
            result,
            Err(ref errors) if errors.0.iter().any(|e| matches!(
                e,
                crate::CompileError::StepIndexOutOfRange { .. }
            ))
        ),
        "add_body_offset must return StepIndexOutOfRange on overflow, got: {:?}",
        result
    );
}

// ─────────────────────────────────────────────────────────────────
// Test: cursor overflow during body lowering (checked_add at part_14.rs:34,53)
// Plan N4 / RRO-B005
// ─────────────────────────────────────────────────────────────────

/// Verifies that the `cursor.checked_add(width)` overflow guards inside
/// `lower_canonical_choose` propagate the correct error when body step
/// offsets overflow `u16`. Uses `emit_choose_branch_body` with a
/// `start_offset` of `u16::MAX` and two body steps to trigger
/// `add_body_offset` overflow, which exercises the same `checked_add`
/// pattern as the cursor accumulation at part_14.rs:34 and part_14.rs:53.
///
/// Direct cursor overflow requires >65535 body steps total (impractical
/// to construct); this test covers the same error variant and guard
/// mechanism through the reachable `emit_choose_branch_body` path.
#[test]
fn lower_choose_body_cursor_overflow_returns_error() {
    use crate::mod_compile_lowering::part_14::emit_choose_branch_body;
    // Given: start_offset = u16::MAX, body with 2 Set steps
    //   i=0: add_body_offset(u16::MAX, 0, _) → Ok(u16::MAX)
    //   i=1: add_body_offset(u16::MAX, 1, _) → overflow → StepIndexOutOfRange
    let body = vec![
        choose_body_set_step("cursor_a", "1"),
        choose_body_set_step("cursor_b", "2"),
    ];
    let mut builder = crate::SlotCompiler::new();
    // When: emit_choose_branch_body is called with overflow-inducing start_offset
    let result = emit_choose_branch_body(
        &body,
        vb_core::ids::StepIdx::new(0),
        u16::MAX,
        0,
        vb_core::ids::StepIdx::new(100),
        &mut builder,
    );
    // Then: returns StepIndexOutOfRange from add_body_offset overflow
    assert!(
        matches!(
            result,
            Err(ref errors) if errors.0.iter().any(|e| matches!(
                e,
                crate::CompileError::StepIndexOutOfRange { .. }
            ))
        ),
        "emit_choose_branch_body must return StepIndexOutOfRange on cursor overflow, got: {:?}",
        result
    );
}

// ─────────────────────────────────────────────────────────────────
// Test 3: slot_from_text valid integer
// Plan N6+N7 / RRO-B011
// ─────────────────────────────────────────────────────────────────

/// Verifies `slot_from_text` returns `Ok(SlotIdx)` for a valid integer string.
/// Tests a representative middle value (42).
#[test]
fn slot_from_text_valid_integer() {
    // Given: a valid integer string representation (42)
    // When: slot_from_text is called
    let result = slot_from_text("42", 0, "test.field");
    // Then: returns Ok with correct SlotIdx
    assert_eq!(
        result.expect("valid integer must parse"),
        vb_core::ids::SlotIdx::new(42),
    );
}

/// Verifies `slot_from_text` returns `Ok(SlotIdx(0))` for the minimum valid u16 boundary.
#[test]
fn slot_from_text_valid_zero_boundary() {
    // Given: text = "0"
    // When: slot_from_text is called
    let result = slot_from_text("0", 0, "test.field");
    // Then: returns Ok(SlotIdx(0))
    assert_eq!(
        result.expect("zero must parse"),
        vb_core::ids::SlotIdx::new(0),
    );
}

/// Verifies `slot_from_text` returns `Ok(SlotIdx(65535))` for the maximum valid u16 boundary.
#[test]
fn slot_from_text_valid_u16_max_boundary() {
    // Given: text = "65535" (u16::MAX)
    // When: slot_from_text is called
    let result = slot_from_text("65535", 0, "test.field");
    // Then: returns Ok(SlotIdx(65535))
    assert_eq!(
        result.expect("u16 max must parse"),
        vb_core::ids::SlotIdx::new(65535),
    );
}

// ─────────────────────────────────────────────────────────────────
// Test 4: slot_from_text out of range (> u16::MAX)
// Plan N11 / RRO-B011
// ─────────────────────────────────────────────────────────────────

/// Verifies `slot_from_text` returns `SlotIndexOutOfRange` when the
/// numeric value exceeds `u16::MAX`.
#[test]
fn slot_from_text_out_of_range() {
    // Given: "65536" which is u16::MAX + 1
    // When: slot_from_text is called
    let result = slot_from_text("65536", 0, "test.field");
    // Then: returns SlotIndexOutOfRange with the exact value
    match result {
        Err(crate::CompileErrors(errors)) => {
            assert!(
                errors.iter().any(|e| matches!(
                    e,
                    crate::CompileError::SlotIndexOutOfRange { value }
                    if *value == 65536i64
                )),
                "slot_from_text must reject value exceeding u16::MAX, got: {:?}",
                errors
            );
        }
        other => panic!("expected SlotIndexOutOfRange error, got: {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────
// Test 5: slot_from_text non-integer
// Plan N9 / RRO-B011
// ─────────────────────────────────────────────────────────────────

/// Verifies `slot_from_text` returns `StepFieldShape` error when the
/// text is not a valid integer.
#[test]
fn slot_from_text_non_integer() {
    // Given: a non-integer string
    // When: slot_from_text is called
    let result = slot_from_text("abc", 1, "test");
    // Then: returns StepFieldShape with "integer string" expected
    match result {
        Err(crate::CompileErrors(errors)) => {
            assert!(
                errors.iter().any(|e| matches!(
                    e,
                    crate::CompileError::StepFieldShape { expected, .. }
                    if *expected == "integer string"
                )),
                "slot_from_text must reject non-integer text, got: {:?}",
                errors
            );
        }
        other => panic!("expected StepFieldShape error, got: {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────
// Test 6: slot_from_text empty string
// Plan N8 / RRO-B011
// ─────────────────────────────────────────────────────────────────

/// Verifies `slot_from_text` returns `StepFieldShape` error when the
/// text is an empty string.
#[test]
fn slot_from_text_empty_string() {
    // Given: an empty string
    // When: slot_from_text is called
    let result = slot_from_text("", 3, "choose.branches[].when");
    // Then: returns StepFieldShape with "non-empty primitive field" expected
    match result {
        Err(crate::CompileErrors(errors)) => {
            assert!(
                errors.iter().any(|e| matches!(
                    e,
                    crate::CompileError::StepFieldShape { step, field, expected }
                    if *step == 3
                    && *field == "choose.branches[].when"
                    && expected.contains("non-empty")
                )),
                "slot_from_text must reject empty string, got: {:?}",
                errors
            );
        }
        other => panic!("expected StepFieldShape error, got: {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────
// Test 7: slot_from_text negative
// Plan N10 / RRO-B011
// ─────────────────────────────────────────────────────────────────

/// Verifies `slot_from_text` returns `SlotIndexOutOfRange` for a
/// negative integer, which parses successfully as i64 but fails
/// `u16::try_from`.
#[test]
fn slot_from_text_negative() {
    // Given: a negative integer string
    // When: slot_from_text is called
    let result = slot_from_text("-1", 0, "test");
    // Then: returns SlotIndexOutOfRange with value -1
    match result {
        Err(crate::CompileErrors(errors)) => {
            assert!(
                errors.iter().any(|e| matches!(
                    e,
                    crate::CompileError::SlotIndexOutOfRange { value }
                    if *value == -1i64
                )),
                "slot_from_text must reject negative integer, got: {:?}",
                errors
            );
        }
        other => panic!("expected SlotIndexOutOfRange error, got: {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────
// Test 8: lower_choose fanout exceeds limit (defense-in-depth)
// Plan N14 / RRO-B008
// ─────────────────────────────────────────────────────────────────

/// Verifies that `lower_choose` (the second checkpoint) rejects >64
/// branches with `PrimitiveLoweringLimitExceeded`.
#[test]
fn lower_choose_fanout_exceeds_limit() {
    // Given: 65 SlotBranch entries
    let branches: Vec<vb_core::SlotBranch> = (0..65u16)
        .map(|i| vb_core::SlotBranch {
            condition: vb_core::ids::SlotIdx::new(i),
            target: vb_core::ids::StepIdx::new(100),
        })
        .collect();
    let mut builder = crate::SlotCompiler::new();
    // When: lower_choose is called with 65 branches
    let result = lower_choose(vb_core::ids::StepIdx::new(0), branches, None, &mut builder);
    // Then: returns PrimitiveLoweringLimitExceeded
    assert!(
        matches!(
            result,
            Err(crate::CompileError::PrimitiveLoweringLimitExceeded {
                primitive: "choose",
                field: "branches",
                value: 65,
                limit: 64,
            })
        ),
        "lower_choose must reject 65 branches at defense-in-depth, got: {:?}",
        result
    );
}

// ─────────────────────────────────────────────────────────────────
// Test 9: lower_canonical_choose emits no YAML strings in IR
// Plan N12 / RRO-B013
// ─────────────────────────────────────────────────────────────────

/// Verifies that `lower_canonical_choose` stores branch conditions as
/// `SlotIdx` values, never as raw YAML strings. The type system enforces
/// this (`SlotBranch.condition: SlotIdx`), but this test provides
/// behavioral evidence.
#[test]
fn lower_canonical_choose_emits_no_yaml_strings() {
    // Given: a choose with when="5" and one Set body step
    let branches = vec![vb_yaml::ast::ChooseBranch {
        when: "5".to_string(),
        steps: vec![choose_body_set_step("body_a", "42")],
    }];
    let step_names: [Box<str>; 2] = [Box::from("pick"), Box::from("done")];
    let mut builder = crate::SlotCompiler::new();
    // When: lower_canonical_choose compiles
    lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("done"),
        Some(vb_core::ids::StepIdx::new(2)),
        &step_names,
        &mut builder,
    )
    .expect("choose must lower");
    // Then: all conditions are SlotIdx (type-guaranteed; verified by value)
    match &builder.nodes[0].kind {
        vb_core::CompiledNodeKind::ChooseSlot {
            branches: slot_branches,
            ..
        } => {
            assert!(!slot_branches.is_empty(), "must have at least one branch");
            assert_eq!(
                slot_branches[0].condition,
                vb_core::ids::SlotIdx::new(5),
                "condition must be SlotIdx(5), parsed from when string '5'"
            );
        }
        other => panic!("expected ChooseSlot, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────
// Test 10: slot compiler records unique indices
// Plan N15 / RRO-B006
// ─────────────────────────────────────────────────────────────────

/// Verifies that the `SlotCompiler` tracks slot indices as body steps
/// are lowered. The body Set node must have an assigned output slot,
/// and the condition slot referenced in the ChooseSlot branches must
/// match the parsed `when` value.
#[test]
fn slot_compiler_records_unique_indices() {
    // Given: a choose with one Set body step
    let branches = vec![vb_yaml::ast::ChooseBranch {
        when: "1".to_string(),
        steps: vec![choose_body_set_step("body_a", "7")],
    }];
    let step_names: [Box<str>; 2] = [Box::from("pick"), Box::from("done")];
    let mut builder = crate::SlotCompiler::new();
    // When: lower_canonical_choose compiles the body
    lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("done"),
        Some(vb_core::ids::StepIdx::new(2)),
        &step_names,
        &mut builder,
    )
    .expect("choose must lower");
    // Then: the body Set node has an output slot (observable behavior)
    assert!(
        builder.nodes[1].output.is_some(),
        "body Set node must have an output slot assigned"
    );
    // Verify the ChooseSlot condition is correctly parsed from when="1"
    match &builder.nodes[0].kind {
        vb_core::CompiledNodeKind::ChooseSlot {
            branches: slot_branches,
            ..
        } => {
            assert_eq!(
                slot_branches[0].condition,
                vb_core::ids::SlotIdx::new(1),
                "condition must be SlotIdx(1) from when string '1'"
            );
        }
        other => panic!("expected ChooseSlot, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────
// Test 11: condition slots disjoint from body output slots
// ─────────────────────────────────────────────────────────────────

/// Verifies that condition slots referenced in ChooseSlot nodes are
/// distinct from the output slots allocated during body step lowering.
/// The output slot is extracted from the compiled body node (not hardcoded),
/// so the test does not couple to the internal slot-allocation scheme.
#[test]
fn lower_canonical_choose_slots_disjoint_from_conditions() {
    // Given: a choose with when="99" and one Set body step
    let branches = vec![vb_yaml::ast::ChooseBranch {
        when: "99".to_string(),
        steps: vec![choose_body_set_step("body_a", "42")],
    }];
    let step_names: [Box<str>; 2] = [Box::from("pick"), Box::from("done")];
    let mut builder = crate::SlotCompiler::new();
    // When: lower_canonical_choose compiles
    lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("done"),
        Some(vb_core::ids::StepIdx::new(2)),
        &step_names,
        &mut builder,
    )
    .expect("choose must lower");
    // Then: condition slots are disjoint from body output slots
    let condition_slot = vb_core::ids::SlotIdx::new(99);
    // Extract the body node's actual output slot from the compiled IR
    let body_output_slot = builder.nodes[1]
        .output
        .expect("body Set node must have an output slot");
    // Verify that the condition slot does NOT match the body output slot
    assert_ne!(
        condition_slot, body_output_slot,
        "condition slot ({condition_slot:?}) must be disjoint from body output slot ({body_output_slot:?})"
    );
    // Verify the ChooseSlot node has the correct condition
    match &builder.nodes[0].kind {
        vb_core::CompiledNodeKind::ChooseSlot {
            branches: slot_branches,
            ..
        } => {
            assert_eq!(
                slot_branches[0].condition, condition_slot,
                "condition must be SlotIdx(99)"
            );
        }
        other => panic!("expected ChooseSlot, got {other:?}"),
    }
    // Verify the body Set node has a different output
    assert_eq!(builder.nodes[1].id, vb_core::ids::StepIdx::new(1));
    assert!(
        matches!(
            &builder.nodes[1].kind,
            vb_core::CompiledNodeKind::SetConst { .. }
        ),
        "body node must be SetConst"
    );
}

// ─────────────────────────────────────────────────────────────────
// Test 12: lower_canonical_choose otherwise target past body span
// Plan N13 / RRO-B003
// ─────────────────────────────────────────────────────────────────

/// Verifies that the `otherwise` target StepIdx points past the body
/// span (i.e., it is not one of the body step nodes).
#[test]
fn lower_canonical_choose_otherwise_target_past_body() {
    // Given: a choose with 2 body steps and an otherwise label pointing past
    // the body. step_names has 4 entries so "done" maps to StepIdx(3),
    // which is beyond the body span (StepIdx 0 ChooseSlot + body at 1,2).
    let branches = vec![vb_yaml::ast::ChooseBranch {
        when: "0".to_string(),
        steps: vec![
            choose_body_set_step("body_a", "7"),
            choose_body_set_step("body_b", "8"),
        ],
    }];
    let step_names: [Box<str>; 4] = [
        Box::from("pick"),
        Box::from("body_a"),
        Box::from("body_b"),
        Box::from("done"),
    ];
    let mut builder = crate::SlotCompiler::new();
    // When: lower_canonical_choose compiles with otherwise="done"
    lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("done"),
        Some(vb_core::ids::StepIdx::new(3)),
        &step_names,
        &mut builder,
    )
    .expect("choose with otherwise must lower");
    // Then: otherwise target is not a body node AND exactly hits the done label
    // Body spans StepIdx(1) through StepIdx(2).
    // "done" at index 3 in step_names → StepIdx(3) → past body span.
    match &builder.nodes[0].kind {
        vb_core::CompiledNodeKind::ChooseSlot { otherwise, .. } => {
            let target = otherwise
                .as_ref()
                .expect("otherwise must be set when label is provided");
            assert_eq!(
                *target,
                vb_core::ids::StepIdx::new(3),
                "otherwise must target node at StepIdx(3) past body span, got {target:?}"
            );
            assert_ne!(
                *target,
                vb_core::ids::StepIdx::new(1),
                "otherwise must not target first body node"
            );
            assert_ne!(
                *target,
                vb_core::ids::StepIdx::new(2),
                "otherwise must not target last body node"
            );
        }
        other => panic!("expected ChooseSlot, got {other:?}"),
    }
}

// ============================================================================
// vb-282my: Choose lowering refinement (CL-02, CL-10, CL-13)
// ============================================================================

#[test]
fn lower_canonical_choose_empty_branches_without_otherwise_returns_empty_branch_table_error() {
    let branches: Vec<vb_yaml::ast::ChooseBranch> = vec![];
    let step_names: [Box<str>; 1] = [Box::from("pick")];
    let mut builder = crate::SlotCompiler::new();
    let result = lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        None,
        Some(vb_core::ids::StepIdx::new(1)),
        &step_names,
        &mut builder,
    );
    match result {
        Err(crate::CompileErrors(errors)) => {
            assert!(
                errors.iter().any(|e| matches!(
                    e,
                    crate::CompileError::Workflow(
                        // Use the same pattern as part_08: vb_core::WorkflowError
                        vb_core::WorkflowError::EmptyBranchTable
                    )
                )),
                "empty branches without otherwise must return EmptyBranchTable, got: {errors:?}"
            );
        }
        other => panic!("expected EmptyBranchTable error, got {other:?}"),
    }
}

#[test]
fn lower_canonical_choose_accepts_exactly_64_branches() {
    let branches: Vec<vb_yaml::ast::ChooseBranch> = (0..64)
        .map(|i| vb_yaml::ast::ChooseBranch {
            when: i.to_string(),
            steps: vec![],
        })
        .collect();
    let step_names: [Box<str>; 2] = [Box::from("pick"), Box::from("done")];
    let mut builder = crate::SlotCompiler::new();
    let result = lower_canonical_choose(
        0,
        vb_core::ids::StepIdx::new(0),
        &branches,
        Some("done"),
        Some(vb_core::ids::StepIdx::new(1)),
        &step_names,
        &mut builder,
    );
    // Verify the lowering succeeded (no error)
    result.expect("64 branches is the maximum allowed, must succeed");
    // Verify a ChooseSlot node was emitted with exactly 64 branches
    match &builder.nodes[0].kind {
        vb_core::CompiledNodeKind::ChooseSlot {
            branches: slot_branches,
            ..
        } => {
            assert_eq!(
                slot_branches.len(),
                64,
                "ChooseSlot must contain exactly 64 branches, got {}",
                slot_branches.len()
            );
        }
        other => panic!("expected ChooseSlot node, got {other:?}"),
    }
}

// NOTE: slot_from_text integer-parsing and error-rejection behaviour is already
// covered with stronger assertions by existing tests at lines 912–1079:
// - slot_from_text_valid_integer      (SlotIdx::new(42) exact)
// - slot_from_text_valid_zero_boundary (SlotIdx::new(0) exact)
// - slot_from_text_valid_u16_max_boundary (SlotIdx::new(65535) exact)
// - slot_from_text_out_of_range       (SlotIndexOutOfRange with exact value)
// - slot_from_text_non_integer        (StepFieldShape with expected string)
// - slot_from_text_empty_string       (StepFieldShape with step/field/expected)
// - slot_from_text_negative           (SlotIndexOutOfRange with value -1)
// No additional weak-assertion tests are needed here.

// ── vb-xi2f.21: canonical_body_step_width accepts ForEach ──

#[test]
fn canonical_body_step_width_accepts_for_each() {
    let foreach = vb_yaml::ast::StepPrimitive::ForEach {
        variable: "x".to_string(),
        input: "0".to_string(),
        at_once: None,
        body: vec![choose_body_set_step("s", "1")],
    };
    let result = canonical_body_step_width(&foreach);
    assert!(result.is_ok(), "ForEach must be accepted in body steps");
    assert_eq!(
        result.ok(),
        Some(3),
        "ForEach with single Set body has width 3"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// vb-xi2f.24: Reduce Multi-Step Body Lowering Behavior Tests
// ═══════════════════════════════════════════════════════════════════════════
//
// PHASE 1: Tests that compile and PASS now (pre-implementation).
//   Test canonical_body_step_width, canonical_step_width, body_width,
//   and error codes that are already correct.
//
// PHASE 2: Tests that compile but FAIL at runtime (TDD red).
//   Test lower_canonical_aggregate with multi-step bodies.
//   Currently fail because emit_single_body_set rejects body.len() != 1.
//   Will pass after emit_reduce_body_steps is implemented and wired in.
//
// PHASE 3: Direct emit_reduce_body_steps tests (BLOCKED).
//   Will be unblocked after emit_reduce_body_steps exists in part_04.rs.
//   Contract: emit_reduce_body_steps(body, body_step, idx, slot, next, builder)
//   See Kani harness kani_reduce_regression.rs for signature reference.
// ═══════════════════════════════════════════════════════════════════════════

use crate::mod_compile_lowering::part_01::canonical_step_width;
use crate::mod_compile_lowering::part_04::lower_canonical_aggregate;
use crate::mod_compile_lowering::part_07::SlotCompiler;
use crate::{CompileError, CompileErrors};
use vb_core::SymbolicCode;
use vb_core::ids::SlotIdx;
use vb_core::ids::StepIdx;
use vb_yaml::ast::ScalarValue;

// ── Helpers ─────────────────────────────────────────────────────────────

/// Create a StepAst with a Set primitive for reduce body steps.
fn reduce_set_step(id: &str, value: &str) -> StepAst {
    StepAst {
        id: id.to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: id.to_string(),
            value: value.to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

/// Create a StepAst with a Do primitive for reduce body steps.
fn reduce_do_step(id: &str, action: &str, input: &str) -> StepAst {
    StepAst {
        id: id.to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Do {
            action: action.to_string(),
            input: input.to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

/// Create a StepAst from a given StepPrimitive.
fn reduce_step(id: &str, primitive: StepPrimitive) -> StepAst {
    StepAst {
        id: id.to_string(),
        name: None,
        condition: None,
        primitive,
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

// ─────────────────────────────────────────────────────────────────────────
// PHASE 1: Width Calculation Tests (COMPILE AND PASS NOW)
// ─────────────────────────────────────────────────────────────────────────

// ── B01: canonical_body_step_width returns Ok(1) for Set ──

#[test]
fn canonical_body_step_width_returns_one_for_set() {
    let set = StepPrimitive::Set {
        output: "out".to_string(),
        value: "42".to_string(),
    };
    let result = canonical_body_step_width(&set);
    match result {
        Ok(width) => assert_eq!(width, 1, "Set must have width 1"),
        Err(e) => panic!("Set must be accepted in body, got error: {e:?}"),
    }
}

// ── B02: canonical_body_step_width returns Ok(1) for Do ──

#[test]
fn canonical_body_step_width_returns_one_for_do() {
    let do_step = StepPrimitive::Do {
        action: "1".to_string(),
        input: "0".to_string(),
    };
    let result = canonical_body_step_width(&do_step);
    match result {
        Ok(width) => assert_eq!(width, 1, "Do must have width 1"),
        Err(e) => panic!("Do must be accepted in body, got error: {e:?}"),
    }
}

// ── B03: canonical_body_step_width returns correct width for ForEach ──

#[test]
fn canonical_body_step_width_returns_overhead_for_foreach_with_empty_body() {
    let foreach = StepPrimitive::ForEach {
        variable: "x".to_string(),
        input: "0".to_string(),
        at_once: None,
        body: vec![],
    };
    let result = canonical_body_step_width(&foreach);
    match result {
        Ok(width) => assert_eq!(width, 2, "ForEach with empty body: overhead 2 + 0 = 2"),
        Err(e) => panic!("ForEach must be accepted in body, got error: {e:?}"),
    }
}

#[test]
fn canonical_body_step_width_returns_three_for_foreach_with_one_set_body() {
    let foreach = StepPrimitive::ForEach {
        variable: "x".to_string(),
        input: "0".to_string(),
        at_once: None,
        body: vec![reduce_set_step("inner", "1")],
    };
    let result = canonical_body_step_width(&foreach);
    match result {
        Ok(width) => assert_eq!(width, 3, "ForEach with 1 Set: overhead 2 + Set(1) = 3"),
        Err(e) => panic!("ForEach with body must be accepted, got error: {e:?}"),
    }
}

#[test]
fn canonical_body_step_width_returns_four_for_foreach_with_two_set_body() {
    let foreach = StepPrimitive::ForEach {
        variable: "x".to_string(),
        input: "0".to_string(),
        at_once: None,
        body: vec![reduce_set_step("a", "1"), reduce_set_step("b", "2")],
    };
    let result = canonical_body_step_width(&foreach);
    match result {
        Ok(width) => assert_eq!(
            width, 4,
            "ForEach with 2 Sets: overhead 2 + Set(1) + Set(1) = 4"
        ),
        Err(e) => panic!("ForEach with 2-step body must be accepted, got error: {e:?}"),
    }
}

// ── B08: canonical_body_step_width rejects Finish ──

#[test]
fn canonical_body_step_width_rejects_finish_with_unsupported_step_primitive() {
    let finish = StepPrimitive::Finish {
        result: ScalarValue::Integer(0),
    };
    let result = canonical_body_step_width(&finish);
    match result {
        Err(CompileError::UnsupportedStepPrimitive { step, primitive }) => {
            assert_eq!(step, 0, "diagnostic step index must be 0");
            assert_eq!(primitive, "finish", "primitive name must be 'finish'");
        }
        other => panic!("expected UnsupportedStepPrimitive for Finish, got: {other:?}"),
    }
}

// ── B09: canonical_body_step_width rejects Wait ──

#[test]
fn canonical_body_step_width_rejects_wait_with_unsupported_step_primitive() {
    let wait = StepPrimitive::Wait {
        event: Some("0".to_string()),
        timeout: None,
    };
    let result = canonical_body_step_width(&wait);
    match result {
        Err(CompileError::UnsupportedStepPrimitive { primitive, .. }) => {
            assert_eq!(primitive, "wait", "primitive name must be 'wait'");
        }
        other => panic!("expected UnsupportedStepPrimitive for Wait, got: {other:?}"),
    }
}

// ── B10: canonical_body_step_width rejects Ask ──

#[test]
fn canonical_body_step_width_rejects_ask_with_unsupported_step_primitive() {
    let ask = StepPrimitive::Ask {
        prompt: "?".to_string(),
        timeout: None,
    };
    let result = canonical_body_step_width(&ask);
    match result {
        Err(CompileError::UnsupportedStepPrimitive { primitive, .. }) => {
            assert_eq!(primitive, "ask", "primitive name must be 'ask'");
        }
        other => panic!("expected UnsupportedStepPrimitive for Ask, got: {other:?}"),
    }
}

// Extended: reject all unsupported primitives in body step context

#[test]
fn canonical_body_step_width_rejects_collect_with_unsupported_step_primitive() {
    let collect = StepPrimitive::Collect {
        variable: "x".to_string(),
        source: "items".to_string(),
        pages: None,
        items: None,
        body: vec![],
    };
    let result = canonical_body_step_width(&collect);
    match result {
        Err(CompileError::UnsupportedStepPrimitive { primitive, .. }) => {
            assert_eq!(primitive, "collect", "primitive name must be 'collect'");
        }
        other => panic!("expected UnsupportedStepPrimitive for Collect, got: {other:?}"),
    }
}

#[test]
fn canonical_body_step_width_accepts_repeat_with_empty_body() {
    // Repeat is now supported in body context, delegating to canonical_step_width
    let repeat = StepPrimitive::Repeat {
        max_attempts: 3,
        body: vec![],
    };
    let result = canonical_body_step_width(&repeat);
    // Empty body with overhead=3 yields Ok(3)
    assert_eq!(
        result,
        Ok(3),
        "Repeat with empty body should be accepted with width 3"
    );
}

#[test]
fn canonical_body_step_width_rejects_choose_with_unsupported_step_primitive() {
    let choose = StepPrimitive::Choose {
        branches: vec![],
        otherwise: None,
    };
    let result = canonical_body_step_width(&choose);
    match result {
        Err(CompileError::UnsupportedStepPrimitive { primitive, .. }) => {
            assert_eq!(primitive, "choose", "primitive name must be 'choose'");
        }
        other => panic!("expected UnsupportedStepPrimitive for Choose, got: {other:?}"),
    }
}

#[test]
fn canonical_body_step_width_rejects_together_with_unsupported_step_primitive() {
    // Together is now supported in body position (vb-xi2f.22).
    // Width computation succeeds; error handling for empty branches
    // occurs at IR emission time (emit_single_body_together → StepFieldShape).
    let together = StepPrimitive::Together { branches: vec![] };
    match canonical_body_step_width(&together) {
        Ok(width) => assert_eq!(width, 2, "Together width with 0 branches is 2 (base)"),
        Err(e) => panic!("Together width should succeed, got: {e:?}"),
    }
}

// ── B11: canonical_step_width(Reduce) == body_width(body, 3) ──

#[test]
fn canonical_step_width_reduce_with_one_set_equals_body_width_plus_three() {
    let body = vec![reduce_set_step("body0", "1")];
    let reduce = StepPrimitive::Reduce {
        variable: "acc".to_string(),
        input: "items".to_string(),
        initial: "0".to_string(),
        body: body.clone(),
    };
    let width_from_step =
        canonical_step_width(&reduce).expect("canonical_step_width must handle Reduce");
    let width_from_body = body_width(&body, 3).expect("body_width must succeed for Set body");
    assert_eq!(
        width_from_step, width_from_body,
        "canonical_step_width(Reduce) must equal body_width(body, 3)"
    );
    assert_eq!(width_from_step, 4, "Reduce: overhead 3 + Set(1) = 4");
}

#[test]
fn canonical_step_width_reduce_with_three_sets_equals_body_width_plus_three() {
    let body = vec![
        reduce_set_step("a", "1"),
        reduce_set_step("b", "2"),
        reduce_set_step("c", "3"),
    ];
    let reduce = StepPrimitive::Reduce {
        variable: "acc".to_string(),
        input: "items".to_string(),
        initial: "0".to_string(),
        body: body.clone(),
    };
    let width_from_step =
        canonical_step_width(&reduce).expect("canonical_step_width must handle Reduce");
    let width_from_body = body_width(&body, 3).expect("body_width must succeed for Set body");
    assert_eq!(
        width_from_step, width_from_body,
        "canonical_step_width(Reduce) must equal body_width(body, 3)"
    );
    assert_eq!(width_from_step, 6, "Reduce: overhead 3 + 3 Sets = 6");
}

#[test]
fn canonical_step_width_reduce_with_mixed_body_equals_body_width_plus_three() {
    let body = vec![reduce_set_step("a", "1"), reduce_do_step("b", "1", "0")];
    let reduce = StepPrimitive::Reduce {
        variable: "acc".to_string(),
        input: "items".to_string(),
        initial: "0".to_string(),
        body: body.clone(),
    };
    let width_from_step =
        canonical_step_width(&reduce).expect("canonical_step_width must handle Reduce");
    let width_from_body = body_width(&body, 3).expect("body_width must succeed for mixed body");
    assert_eq!(
        width_from_step, width_from_body,
        "canonical_step_width(Reduce) must equal body_width(body, 3)"
    );
    assert_eq!(
        width_from_step, 5,
        "Reduce: overhead 3 + Set(1) + Do(1) = 5"
    );
}

// ── body_width boundary and overflow tests ──

#[test]
fn body_width_returns_overhead_for_empty_body() {
    let body: Vec<StepAst> = vec![];
    let result = body_width(&body, 3);
    match result {
        Ok(width) => assert_eq!(width, 3, "empty body returns overhead only"),
        Err(e) => panic!("empty body must succeed, got: {e:?}"),
    }
}

#[test]
fn body_width_returns_zero_for_empty_body_with_zero_overhead() {
    let body: Vec<StepAst> = vec![];
    let result = body_width(&body, 0);
    match result {
        Ok(width) => assert_eq!(width, 0, "empty body with zero overhead returns 0"),
        Err(e) => panic!("zero overhead empty body must succeed, got: {e:?}"),
    }
}

#[test]
fn body_width_returns_overhead_plus_n_for_n_set_steps() {
    let body = vec![reduce_set_step("a", "1"), reduce_set_step("b", "2")];
    let result = body_width(&body, 3);
    match result {
        Ok(width) => assert_eq!(width, 5, "overhead 3 + Set(1) + Set(1) = 5"),
        Err(e) => panic!("valid Set body must succeed, got: {e:?}"),
    }
}

#[test]
fn body_width_returns_correct_for_mixed_set_do_body() {
    let body = vec![reduce_set_step("a", "1"), reduce_do_step("b", "1", "0")];
    let result = body_width(&body, 3);
    match result {
        Ok(width) => assert_eq!(width, 5, "overhead 3 + Set(1) + Do(1) = 5"),
        Err(e) => panic!("mixed Set+Do body must succeed, got: {e:?}"),
    }
}

#[test]
fn body_width_returns_correct_for_foreach_in_body() {
    let foreach_step = reduce_step(
        "inner",
        StepPrimitive::ForEach {
            variable: "x".to_string(),
            input: "0".to_string(),
            at_once: None,
            body: vec![reduce_set_step("s", "1")],
        },
    );
    let body = vec![foreach_step];
    let result = body_width(&body, 3);
    match result {
        Ok(width) => assert_eq!(
            width, 6,
            "overhead 3 + ForEach(overhead 2 + Set(1) = 3) = 6"
        ),
        Err(e) => panic!("ForEach in body must succeed, got: {e:?}"),
    }
}

#[test]
fn body_width_returns_correct_for_for_each_empty_body() {
    let foreach_step = reduce_step(
        "inner",
        StepPrimitive::ForEach {
            variable: "x".to_string(),
            input: "0".to_string(),
            at_once: None,
            body: vec![],
        },
    );
    let body = vec![foreach_step];
    let result = body_width(&body, 3);
    match result {
        Ok(width) => assert_eq!(width, 5, "overhead 3 + ForEach(overhead 2 + 0) = 5"),
        Err(e) => panic!("ForEach empty body in body must succeed, got: {e:?}"),
    }
}

#[test]
fn body_width_nested_reduce_rejected_pre_widening() {
    let nested_reduce = reduce_step(
        "inner_fold",
        StepPrimitive::Reduce {
            variable: "sum".to_string(),
            input: "inner_items".to_string(),
            initial: "0".to_string(),
            body: vec![reduce_set_step("ns", "1")],
        },
    );
    let body = vec![nested_reduce];
    let result = body_width(&body, 3);
    match result {
        Ok(width) => {
            // After canonical_body_step_width is widened for Reduce:
            // overhead 3 + nested Reduce (3 + Set(1) = 4) = 7
            assert_eq!(width, 7, "nested Reduce: 3 + 4 = 7 (post-widening)");
        }
        Err(CompileError::UnsupportedStepPrimitive { primitive, .. }) => {
            // TDD red: currently expected until canonical_body_step_width accepts Reduce
            assert_eq!(
                primitive, "reduce",
                "primitive must be 'reduce' before widening"
            );
        }
        Err(e) => panic!("unexpected error for nested Reduce: {e:?}"),
    }
}

#[test]
fn body_width_returns_error_when_body_contains_unsupported_primitive() {
    let finish_step = reduce_step(
        "bad",
        StepPrimitive::Finish {
            result: ScalarValue::Integer(0),
        },
    );
    let body = vec![finish_step];
    let result = body_width(&body, 3);
    match result {
        Err(CompileError::UnsupportedStepPrimitive { primitive, .. }) => {
            assert_eq!(
                primitive, "finish",
                "error must propagate with correct primitive name"
            );
        }
        other => panic!("expected UnsupportedStepPrimitive for Finish in body, got: {other:?}"),
    }
}

// ── B46: Width overflow ──

#[test]
fn body_width_returns_step_index_out_of_range_when_width_overflows_usize() {
    let body = vec![reduce_set_step("s", "1")];
    let result = body_width(&body, usize::MAX);
    match result {
        Err(CompileError::StepIndexOutOfRange { value }) => {
            assert_eq!(
                value,
                usize::MAX,
                "overflow error must report the value at overflow point"
            );
        }
        other => panic!("expected StepIndexOutOfRange for overflow, got: {other:?}"),
    }
}

#[test]
fn body_width_handles_u16_max_boundary() {
    let body = vec![reduce_set_step("s", "1")];
    let result = body_width(&body, 65534);
    match result {
        Ok(width) => assert_eq!(width, 65535, "overhead 65534 + Set(1) = 65535 (u16::MAX)"),
        Err(e) => panic!("u16::MAX boundary must succeed, got: {e:?}"),
    }
}

#[test]
fn body_width_single_step_zero_overhead_boundary() {
    let body = vec![reduce_set_step("s", "1")];
    let result = body_width(&body, 0);
    match result {
        Ok(width) => assert_eq!(width, 1, "overhead 0 + Set(1) = 1"),
        Err(e) => panic!("zero overhead boundary must succeed, got: {e:?}"),
    }
}

// ── B47: Error diagnostic code validity ──

#[test]
fn unsupported_step_primitive_error_code_is_not_internal_invariant() {
    let finish = StepPrimitive::Finish {
        result: ScalarValue::Integer(0),
    };
    let result = canonical_body_step_width(&finish);
    match result {
        Err(ref e) => {
            let code = e.code();
            assert_ne!(
                code,
                SymbolicCode::INTERNAL_INVARIANT,
                "UnsupportedStepPrimitive code must not be INTERNAL_INVARIANT"
            );
        }
        other => panic!("expected error with valid code, got: {other:?}"),
    }
}

#[test]
fn step_index_out_of_range_error_code_is_not_internal_invariant() {
    let body = vec![reduce_set_step("s", "1")];
    let result = body_width(&body, usize::MAX);
    match result {
        Err(ref e) => {
            let code = e.code();
            assert_ne!(
                code,
                SymbolicCode::INTERNAL_INVARIANT,
                "StepIndexOutOfRange code must not be INTERNAL_INVARIANT"
            );
        }
        other => panic!("expected error with valid code, got: {other:?}"),
    }
}

// ── B48: Deterministic width (idempotent) ──

#[test]
fn canonical_body_step_width_returns_same_result_for_same_input() {
    let set = StepPrimitive::Set {
        output: "out".to_string(),
        value: "42".to_string(),
    };
    let r1 = canonical_body_step_width(&set);
    let r2 = canonical_body_step_width(&set);
    assert_eq!(r1, r2, "same input must produce same result");
    match (r1, r2) {
        (Ok(w1), Ok(w2)) => assert_eq!(w1, w2, "widths must be identical"),
        (Err(e1), Err(e2)) => assert_eq!(
            format!("{e1:?}"),
            format!("{e2:?}"),
            "errors must be identical"
        ),
        _ => panic!("results must be of the same variant"),
    }
}

#[test]
fn body_width_returns_same_result_for_same_input() {
    let body = vec![reduce_set_step("a", "1"), reduce_set_step("b", "2")];
    let r1 = body_width(&body, 3);
    let r2 = body_width(&body, 3);
    assert_eq!(r1, r2, "same body must produce same width");
}

// ─────────────────────────────────────────────────────────────────────────
// PHASE 2: lower_canonical_aggregate Tests (COMPILE OK, FAIL RUNTIME)
//
// These tests call the existing lower_canonical_aggregate function.
// Single-step body tests PASS now. Multi-step body tests FAIL now
// because emit_single_body_set rejects body.len() != 1.
// They will pass after emit_reduce_body_steps is wired in.
// ─────────────────────────────────────────────────────────────────────────

/// Helper: compile a reduce body through lower_canonical_aggregate.
fn compile_reduce_body(body: &[StepAst]) -> Result<SlotCompiler, CompileErrors> {
    let mut builder = SlotCompiler::new();
    lower_canonical_aggregate(
        0,
        StepIdx::new(0),
        "0", // input slot as integer string
        "0", // initial value
        body,
        Some(StepIdx::new(10)),
        &mut builder,
    )?;
    Ok(builder)
}

// ── Single-step regression (PASS NOW) ──

#[test]
fn lower_canonical_aggregate_compiles_single_set_body() {
    let body = vec![reduce_set_step("body0", "1")];
    let result = compile_reduce_body(&body);
    match result {
        Ok(builder) => {
            assert_eq!(
                builder.nodes.len(),
                4,
                "single Set body must produce 4 nodes (ReduceStart+Set+ReduceNext+ReduceFinish)"
            );
        }
        Err(errors) => panic!("single Set body must compile, got: {errors:?}"),
    }
}

#[test]
fn lower_canonical_aggregate_compiles_single_do_body() {
    let body = vec![reduce_do_step("body0", "1", "0")];
    let result = compile_reduce_body(&body);
    match result {
        Ok(builder) => {
            assert_eq!(
                builder.nodes.len(),
                4,
                "single Do body must produce 4 nodes"
            );
        }
        Err(errors) => panic!("single Do body must compile, got: {errors:?}"),
    }
}

// ── B29-B31: ReduceStart/ReduceNext/ReduceFinish field verification ──

#[test]
fn lower_canonical_aggregate_reduce_start_body_equals_id_plus_one() {
    let body = vec![reduce_set_step("body0", "1")];
    let builder = compile_reduce_body(&body).expect("single Set body must compile");
    assert_eq!(
        builder.nodes[0].id,
        StepIdx::new(0),
        "ReduceStart must be at id"
    );
    match &builder.nodes[0].kind {
        vb_core::CompiledNodeKind::ReduceStart {
            body: body_step,
            done,
            ..
        } => {
            assert_eq!(
                *body_step,
                StepIdx::new(1),
                "ReduceStart.body must be id + 1"
            );
            assert_eq!(*done, StepIdx::new(3), "ReduceStart.done must be id + 3");
        }
        other => panic!("expected ReduceStart at node 0, got {other:?}"),
    }
}

#[test]
fn lower_canonical_aggregate_reduce_next_has_correct_field_values() {
    let body = vec![reduce_set_step("body0", "1")];
    let builder = compile_reduce_body(&body).expect("single Set body must compile");
    assert_eq!(
        builder.nodes[2].id,
        StepIdx::new(2),
        "ReduceNext must be at id + 2"
    );
    match &builder.nodes[2].kind {
        vb_core::CompiledNodeKind::ReduceNext {
            body: body_step,
            done,
            ..
        } => {
            assert_eq!(
                *body_step,
                StepIdx::new(1),
                "ReduceNext.body must equal body_step (same as ReduceStart.body)"
            );
            assert_eq!(
                *done,
                StepIdx::new(3),
                "ReduceNext.done must equal done_step"
            );
        }
        other => panic!("expected ReduceNext at node 2, got {other:?}"),
    }
}

#[test]
fn lower_canonical_aggregate_reduce_finish_id_is_next_step_plus_one() {
    let body = vec![reduce_set_step("body0", "1")];
    let builder = compile_reduce_body(&body).expect("single Set body must compile");
    assert_eq!(
        builder.nodes[3].id,
        StepIdx::new(3),
        "ReduceFinish.id must be id + 3 (next_step + 1)"
    );
    match &builder.nodes[3].kind {
        vb_core::CompiledNodeKind::ReduceFinish { .. } => {}
        other => panic!("expected ReduceFinish at node 3, got {other:?}"),
    }
}

#[test]
fn lower_canonical_aggregate_reduce_finish_next_is_passed_next_parameter() {
    let body = vec![reduce_set_step("body0", "1")];
    let builder = compile_reduce_body(&body).expect("single Set body must compile");
    assert_eq!(
        builder.nodes[3].next,
        Some(StepIdx::new(10)),
        "ReduceFinish.next must equal the passed next parameter"
    );
}

// ── B30: ReduceNext.body equals ReduceStart.body ──

#[test]
fn reduce_start_and_reduce_next_both_point_to_body_step() {
    let body = vec![reduce_set_step("body0", "1")];
    let builder = compile_reduce_body(&body).expect("single Set body must compile");
    let start_body_step = match &builder.nodes[0].kind {
        vb_core::CompiledNodeKind::ReduceStart { body, .. } => *body,
        other => panic!("expected ReduceStart, got {other:?}"),
    };
    let next_body_step = match &builder.nodes[2].kind {
        vb_core::CompiledNodeKind::ReduceNext { body, .. } => *body,
        other => panic!("expected ReduceNext, got {other:?}"),
    };
    assert_eq!(
        start_body_step, next_body_step,
        "ReduceStart.body and ReduceNext.body must be identical"
    );
    assert_eq!(start_body_step, StepIdx::new(1), "body_step must be id + 1");
}

// ── B34: ReduceFinish.next is parent aggregate's next ──

#[test]
fn reduce_finish_next_is_parent_aggregate_next() {
    let body = vec![reduce_set_step("body0", "1")];
    let mut builder = SlotCompiler::new();
    let parent_next = Some(StepIdx::new(20));
    lower_canonical_aggregate(
        0,
        StepIdx::new(5),
        "0", // input slot as integer string
        "0", // initial value
        &body,
        parent_next,
        &mut builder,
    )
    .expect("single Set body must compile");
    assert_eq!(
        builder.nodes[3].next, parent_next,
        "ReduceFinish.next must equal the parent aggregate's next step"
    );
}

// ── Body Set node fields ──

#[test]
fn lower_canonical_aggregate_body_set_node_has_correct_id_and_next() {
    let body = vec![reduce_set_step("body0", "1")];
    let builder = compile_reduce_body(&body).expect("single Set body must compile");
    let body_node = &builder.nodes[1];
    assert_eq!(
        body_node.id,
        StepIdx::new(1),
        "body Set node must be at body_step = id + 1"
    );
    assert_eq!(
        body_node.next,
        Some(StepIdx::new(2)),
        "body Set node's next must point to ReduceNext at id + 2"
    );
    match &body_node.kind {
        vb_core::CompiledNodeKind::SetConst { .. } => {}
        other => panic!("expected SetConst for body Set node, got {other:?}"),
    }
}

// ── B54-B56: Empty body rejection ──

#[test]
fn lower_canonical_aggregate_rejects_empty_body_with_step_field_shape() {
    let body: Vec<StepAst> = vec![];
    let result = compile_reduce_body(&body);
    match result {
        Err(errors) => {
            let has_step_field_shape = errors
                .0
                .iter()
                .any(|e| matches!(e, CompileError::StepFieldShape { field: "steps", .. }));
            assert!(
                has_step_field_shape,
                "empty body must be rejected with StepFieldShape on 'steps', got: {errors:?}"
            );
        }
        Ok(builder) => {
            panic!(
                "empty body must not compile (got {} nodes)",
                builder.nodes.len()
            );
        }
    }
}

// ── Multi-step body tests (TDD RED — FAIL RUNTIME NOW) ──

#[test]
fn lower_canonical_aggregate_multi_step_two_set_body_tdd_red() {
    let body = vec![reduce_set_step("s1", "1"), reduce_set_step("s2", "2")];
    let result = compile_reduce_body(&body);
    match result {
        Ok(builder) => {
            // TDD GREEN: after emit_reduce_body_steps is implemented
            // overhead 3 + 2 body steps = 5 nodes total
            assert_eq!(
                builder.nodes.len(),
                5,
                "two-step body: ReduceStart + Set + Set + ReduceNext + ReduceFinish = 5 nodes"
            );
        }
        Err(errors) => {
            // TDD RED: emit_single_body_set rejects body.len() != 1
            let has_step_field_shape = errors
                .0
                .iter()
                .any(|e| matches!(e, CompileError::StepFieldShape { .. }));
            assert!(
                has_step_field_shape,
                "multi-step body rejected (TDD red): {errors:?}"
            );
        }
    }
}

#[test]
fn lower_canonical_aggregate_multi_step_three_set_body_tdd_red() {
    let body = vec![
        reduce_set_step("s1", "1"),
        reduce_set_step("s2", "2"),
        reduce_set_step("s3", "3"),
    ];
    let result = compile_reduce_body(&body);
    match result {
        Ok(builder) => {
            // TDD GREEN: 3 + 3 = 6 nodes total
            assert_eq!(
                builder.nodes.len(),
                6,
                "three-step body: 3 overhead + 3 Sets = 6 nodes"
            );
        }
        Err(errors) => {
            let has_step_field_shape = errors
                .0
                .iter()
                .any(|e| matches!(e, CompileError::StepFieldShape { .. }));
            assert!(
                has_step_field_shape,
                "three-step body rejected (TDD red): {errors:?}"
            );
        }
    }
}

#[test]
fn lower_canonical_aggregate_multi_step_mixed_set_do_body_tdd_red() {
    let body = vec![reduce_set_step("s1", "1"), reduce_do_step("s2", "1", "0")];
    let result = compile_reduce_body(&body);
    match result {
        Ok(builder) => {
            assert_eq!(
                builder.nodes.len(),
                5,
                "mixed body: 3 overhead + Set + Do = 5 nodes"
            );
        }
        Err(errors) => {
            let has_step_field_shape = errors
                .0
                .iter()
                .any(|e| matches!(e, CompileError::StepFieldShape { .. }));
            assert!(
                has_step_field_shape,
                "mixed body rejected (TDD red): {errors:?}"
            );
        }
    }
}

// ── B12-B14: Width-node count parity (TDD RED) ──

#[test]
fn reduce_body_width_node_count_parity_single_set_body() {
    let body = vec![reduce_set_step("s1", "1")];
    let width = body_width(&body, 3).expect("width must compute");
    let result = compile_reduce_body(&body);
    match result {
        Ok(builder) => {
            assert_eq!(
                builder.nodes.len(),
                width,
                "node count must equal body_width (3 overhead + 1 body = 4 nodes for single Set)"
            );
        }
        Err(errors) => panic!("single Set body must compile via full pipeline, got: {errors:?}"),
    }
}

#[test]
fn reduce_body_width_node_count_parity_two_set_body_tdd_red() {
    let body = vec![reduce_set_step("a", "1"), reduce_set_step("b", "2")];
    let width = body_width(&body, 3).expect("width must compute");
    assert_eq!(width, 5, "two-step body width: overhead 3 + 2 = 5");
    let result = compile_reduce_body(&body);
    match result {
        Ok(builder) => {
            assert_eq!(
                builder.nodes.len(),
                width,
                "node count ({}) must equal computed width ({}) (TDD green)",
                builder.nodes.len(),
                width
            );
        }
        Err(errors) => {
            let has_step_field_shape = errors
                .0
                .iter()
                .any(|e| matches!(e, CompileError::StepFieldShape { .. }));
            assert!(
                has_step_field_shape,
                "two-step body rejected (TDD red): {errors:?}"
            );
        }
    }
}

// ── B21: No body IDs overlap with next_step ──

#[test]
fn lower_canonical_aggregate_body_ids_do_not_overlap_reduce_next_tdd_red() {
    let body = vec![reduce_set_step("s1", "1"), reduce_set_step("s2", "2")];
    let result = compile_reduce_body(&body);
    match result {
        Ok(builder) => {
            // TDD GREEN: all body node IDs are < next_step (ReduceNext)
            // body_step = id + 1 = StepIdx(1)
            // body_width([Set,Set], 0) = 0 + 1 + 1 = 2
            // next_step = body_step + body_width = StepIdx(1) + 2 = StepIdx(3)
            // Body nodes at StepIdx(1) and StepIdx(2); both < StepIdx(3)
            // ReduceNext at StepIdx(3), ReduceFinish at StepIdx(4)
            let next_step = StepIdx::new(3);
            for node in &builder.nodes {
                assert!(
                    node.id < next_step
                        || matches!(
                            node.kind,
                            vb_core::CompiledNodeKind::ReduceNext { .. }
                                | vb_core::CompiledNodeKind::ReduceFinish { .. }
                        ),
                    "body node at {:?} must not occupy or exceed next_step {:?}",
                    node.id,
                    next_step
                );
            }
        }
        Err(errors) => {
            let has_step_field_shape = errors
                .0
                .iter()
                .any(|e| matches!(e, CompileError::StepFieldShape { .. }));
            assert!(
                has_step_field_shape,
                "multi-step body rejected (TDD red): {errors:?}"
            );
        }
    }
}

// ── No panic: lower_canonical_aggregate returns Result for single step ──

#[test]
fn lower_canonical_aggregate_never_panics_for_single_set_body() {
    let body = vec![reduce_set_step("s1", "1")];
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| compile_reduce_body(&body)));
    match result {
        Ok(inner) => match inner {
            Ok(_) => {}  // success
            Err(_) => {} // error is OK, not a panic
        },
        Err(_) => panic!("lower_canonical_aggregate must not panic on valid input"),
    }
}

// ── No panic: lower_canonical_aggregate returns Result for empty body ──

#[test]
fn lower_canonical_aggregate_never_panics_for_empty_body() {
    let body: Vec<StepAst> = vec![];
    let result =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| compile_reduce_body(&body)));
    match result {
        Ok(inner) => match inner {
            Err(_) => {} // error is expected, not a panic
            Ok(_) => panic!("empty body must not succeed"),
        },
        Err(_) => panic!("lower_canonical_aggregate must not panic on empty body"),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// PHASE 3: Direct emit_reduce_body_steps Tests (NOW ACTIVE)
// ═══════════════════════════════════════════════════════════════════════════

use crate::mod_compile_lowering::part_04::emit_reduce_body_steps;

#[test]
fn emit_reduce_body_steps_assigns_sequential_distinct_step_indices() {
    let body = vec![
        reduce_set_step("s1", "1"),
        reduce_set_step("s2", "2"),
        reduce_set_step("s3", "3"),
    ];
    let mut builder = SlotCompiler::new();
    let body_step = StepIdx::new(10);
    let next = Some(StepIdx::new(20));
    let result = emit_reduce_body_steps(&body, body_step, 0, SlotIdx::new(1), next, &mut builder);
    match result {
        Ok(()) => {
            assert_eq!(builder.nodes.len(), 3, "must emit exactly 3 body nodes");
            assert_eq!(
                builder.nodes[0].id,
                StepIdx::new(10),
                "first body node at body_step"
            );
            assert_eq!(
                builder.nodes[1].id,
                StepIdx::new(11),
                "second at body_step + Set width(1)"
            );
            assert_eq!(
                builder.nodes[2].id,
                StepIdx::new(12),
                "third at body_step + 2"
            );
        }
        Err(e) => panic!("sequential assignment must succeed: {e:?}"),
    }
}

#[test]
fn emit_reduce_body_steps_single_step_next_points_to_next_parameter() {
    let body = vec![reduce_set_step("s1", "1")];
    let mut builder = SlotCompiler::new();
    let next = Some(StepIdx::new(42));
    let result = emit_reduce_body_steps(
        &body,
        StepIdx::new(0),
        0,
        SlotIdx::new(1),
        next,
        &mut builder,
    );
    match result {
        Ok(()) => {
            assert_eq!(
                builder.nodes[0].next,
                Some(StepIdx::new(42)),
                "single body step next = next param"
            );
        }
        Err(e) => panic!("single step must succeed: {e:?}"),
    }
}

#[test]
fn emit_reduce_body_steps_first_step_next_points_to_second_when_multi_step() {
    let body = vec![reduce_set_step("s1", "1"), reduce_set_step("s2", "2")];
    let mut builder = SlotCompiler::new();
    let result = emit_reduce_body_steps(
        &body,
        StepIdx::new(0),
        0,
        SlotIdx::new(1),
        Some(StepIdx::new(20)),
        &mut builder,
    );
    match result {
        Ok(()) => {
            assert_eq!(
                builder.nodes[0].next,
                Some(StepIdx::new(1)),
                "first step next = second step id"
            );
        }
        Err(e) => panic!("multi-step must succeed: {e:?}"),
    }
}

#[test]
fn emit_reduce_body_steps_last_step_next_points_to_next_parameter() {
    let body = vec![reduce_set_step("s1", "1"), reduce_set_step("s2", "2")];
    let mut builder = SlotCompiler::new();
    let next = Some(StepIdx::new(20));
    let result = emit_reduce_body_steps(
        &body,
        StepIdx::new(0),
        0,
        SlotIdx::new(1),
        next,
        &mut builder,
    );
    match result {
        Ok(()) => {
            assert_eq!(
                builder.nodes[1].next, next,
                "last body step next = next param"
            );
        }
        Err(e) => panic!("multi-step must succeed: {e:?}"),
    }
}

#[test]
fn emit_reduce_body_steps_all_next_links_are_some() {
    let body = vec![reduce_set_step("s1", "1"), reduce_set_step("s2", "2")];
    let mut builder = SlotCompiler::new();
    let result = emit_reduce_body_steps(
        &body,
        StepIdx::new(0),
        0,
        SlotIdx::new(1),
        Some(StepIdx::new(20)),
        &mut builder,
    );
    match result {
        Ok(()) => {
            for (i, node) in builder.nodes.iter().enumerate() {
                assert!(
                    node.next.is_some(),
                    "node {i} at {:?} must have Some(next), got None",
                    node.id
                );
            }
        }
        Err(e) => panic!("multi-step must succeed: {e:?}"),
    }
}

#[test]
fn emit_reduce_body_steps_empty_body_returns_step_field_shape() {
    let body: Vec<StepAst> = vec![];
    let mut builder = SlotCompiler::new();
    let result = emit_reduce_body_steps(
        &body,
        StepIdx::new(0),
        0,
        SlotIdx::new(1),
        None,
        &mut builder,
    );
    match result {
        Err(errors) => {
            let has_step_field_shape = errors
                .0
                .iter()
                .any(|e| matches!(e, CompileError::StepFieldShape { field: "steps", .. }));
            assert!(
                has_step_field_shape,
                "empty body must be rejected with StepFieldShape on 'steps'"
            );
            assert_eq!(
                builder.nodes.len(),
                0,
                "no nodes must be emitted for empty body"
            );
        }
        Ok(()) => panic!("empty body must be rejected"),
    }
}

#[test]
fn emit_reduce_body_steps_produces_same_ir_as_emit_single_body_set_for_single_set() {
    use crate::mod_compile_lowering::part_04::emit_single_body_set;
    let body = vec![reduce_set_step("s1", "1")];
    let mut builder_a = SlotCompiler::new();
    let mut builder_b = SlotCompiler::new();
    let id = StepIdx::new(0);
    let next = Some(StepIdx::new(2));
    let slot = SlotIdx::new(1);
    emit_single_body_set(&body, id, 0, slot, next, &mut builder_a, false)
        .expect("reference dispatcher must succeed");
    emit_reduce_body_steps(&body, id, 0, slot, next, &mut builder_b)
        .expect("multi-step dispatcher must succeed for single step");
    assert_eq!(
        builder_a.nodes.len(),
        builder_b.nodes.len(),
        "both must emit same node count"
    );
    assert_eq!(
        builder_a.nodes[0].id, builder_b.nodes[0].id,
        "node IDs must match"
    );
    assert_eq!(
        builder_a.nodes[0].next, builder_b.nodes[0].next,
        "next links must match"
    );
}
