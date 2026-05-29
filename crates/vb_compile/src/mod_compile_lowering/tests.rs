//! Digest coverage tests for Collect field hashing.
//!
//! PO: PO-003, PO-004, PO-005, PO-006, PO-007, PO-014
//! Bead: vb-xi2f.38
//!
//! These tests verify that different Collect field values produce different
//! digests when compiled through `compute_compiled_digest`.
//!
//! Note: `compute_compiled_digest` in mod_compile_core.rs is `blake3::hash(source)`.
//! The bug in `digest_step_primitive` (part_05.rs:158-160) is in the internal
//! `canonical_digest` function which is not publicly accessible. These tests
//! verify the public API digest behavior via `compute_compiled_digest`.
//!
//! The DIRECT tests (below) call `digest_step_primitive` directly and actually
//! verify that Collect fields contribute to the digest. These are the tests
//! that black-hat required: tests that call `digest_step_primitive` with Collect
//! input, NOT blake3::hash of YAML bytes.
//!
//! The Kani harnesses in `verification/kani/collect_field_coverage.rs` provide
//! formal verification of the same property.

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

// ─────────────────────────────────────────────────────────────────
// vb-awhr: choose otherwise handling and fanout limit
// ─────────────────────────────────────────────────────────────────

use crate::mod_compile_lowering::part_01::choose_width;
use crate::mod_compile_lowering::part_02::lower_canonical_choose;

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
    assert_eq!(builder.nodes[1].id, vb_core::ids::StepIdx::new(1));
    assert_eq!(builder.nodes[1].next, Some(vb_core::ids::StepIdx::new(2)));
}

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
