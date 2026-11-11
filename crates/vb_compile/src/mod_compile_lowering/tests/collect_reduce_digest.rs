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
fn direct_digest_collect_pages_none_equiv_some1() {
    // pages: None vs pages: Some(1)
    let collect_a = make_collect("x", "items", None, Some(50), vec![]);
    let collect_b = make_collect("x", "items", Some(1), Some(50), vec![]);

    let digest_a = digest_primitive(&collect_a);
    let digest_b = digest_primitive(&collect_b);

    // None and Some(1) both lower to `limit = pages.unwrap_or(1)` and must
    // share a digest (canonical equivalence).
    assert_eq!(
        digest_a, digest_b,
        "pages None and Some(1) lower to limit=1 and must share a digest"
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
fn direct_digest_collect_items_none_equiv_some1() {
    // items: None vs items: Some(1)
    let collect_a = make_collect("x", "items", Some(10), None, vec![]);
    let collect_b = make_collect("x", "items", Some(10), Some(1), vec![]);

    let digest_a = digest_primitive(&collect_a);
    let digest_b = digest_primitive(&collect_b);

    // None and Some(1) both lower to `page_size = items.unwrap_or(1)` and
    // must share a digest (canonical equivalence).
    assert_eq!(
        digest_a, digest_b,
        "items None and Some(1) lower to page_size=1 and must share a digest"
    );
}

// ─────────────────────────────────────────────────────────────────
// Body recursive hashing
// ─────────────────────────────────────────────────────────────────

fn make_set_step(id: &str, output: &str, value: &str) -> StepAst {
    StepAst {
        id: id.to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: output.to_string(),
            value: value.to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }
}

#[test]
fn direct_digest_collect_body_recursive() {
    // Two Collect primitives identical except body content differs.
    let body_a = vec![make_set_step("step_a", "a", "1")];
    let body_b = vec![make_set_step(
        "step_b", // different id
        "b",      // different output
        "1",
    )];

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
    let body = vec![make_set_step("inner", "x", "1")];

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

/// Helper: create a minimal StepPrimitive::Aggregate (reduce) with given field overrides.
fn make_reduce(variable: &str, input: &str, initial: &str, body: Vec<StepAst>) -> StepPrimitive {
    StepPrimitive::Aggregate {
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
    let body_a = vec![make_set_step("step_a", "x", "1")];
    let body_b = vec![make_set_step("step_b", "y", "2")];

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
    let body = vec![make_set_step("inner", "x", "1")];

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
        "same StepPrimitive::Aggregate must produce same digest (idempotence)"
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
    let aggregate = StepPrimitive::Aggregate {
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

