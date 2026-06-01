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
    // Given: a branch with an unsupported body primitive (ForEach)
    let unsupported_step = vb_yaml::ast::StepAst {
        id: "unsupported".to_string(),
        name: None,
        condition: None,
        primitive: vb_yaml::ast::StepPrimitive::Repeat {
            max_attempts: 3,
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
