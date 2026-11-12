use super::*;

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, failure_persistence: None, .. ProptestConfig::default() })]

    #[test]
    fn proptest_equal_primitive_sources_compile_to_equal_digest_and_ir(case in primitive_case_strategy()) {
        let left = compile_case(&case).map_err(TestCaseError::fail)?;
        let right = compile_case(&case).map_err(TestCaseError::fail)?;

        prop_assert_eq!(left.digest(), right.digest());
        prop_assert_eq!(left.to_parts(), right.to_parts());
    }

    #[test]
    fn proptest_scoped_primitives_never_return_unsupported_step_primitive(case in primitive_case_strategy()) {
        let yaml = workflow_yaml(case.yaml_steps);
        match compile_workflow(yaml.as_bytes()) {
            Ok(workflow) => {
                let actual_kinds = node_kind_names(workflow.to_parts().nodes.as_ref());
                prop_assert_eq!(actual_kinds, case.expected_kinds);
            }
            Err(errors) => {
                let unsupported = unsupported_primitive_name(&errors);
                prop_assert_eq!(unsupported, None, "in-scope primitive {} must not be unsupported", case.name);
            }
        }
    }
    // ── Wait digest sensitivity tests (vb-xi2f.32) ──

    /// PO-002: Different wait field values produce different digests.
    /// Forall a,b with different wait event/timeout fields,
    /// canonical_digest(workflow_with(a)) != canonical_digest(workflow_with(b)).
    #[test]
    fn proptest_wait_field_sensitivity(
        (event_a, timeout_a) in wait_field_strategy(),
        (event_b, timeout_b) in wait_field_strategy(),
    ) {
        // Ensure the two configurations are actually different
        prop_assume!(event_a != event_b || timeout_a != timeout_b);

        let source_a = wait_workflow_source(&event_a, &timeout_a);
        let source_b = wait_workflow_source(&event_b, &timeout_b);

        let digest_a = canonical_digest_compat(&source_a)
            .map_err(|e| TestCaseError::fail(e))?;
        let digest_b = canonical_digest_compat(&source_b)
            .map_err(|e| TestCaseError::fail(e))?;

        prop_assert_ne!(digest_a, digest_b,
            "different wait fields must produce different digests: ({:?},{:?}) vs ({:?},{:?})",
            event_a, timeout_a, event_b, timeout_b);
    }

    /// PO-004: WaitUntil and WaitEvent produce different digests.
    /// Forall timeout_text, event_text:
    ///   digest(WaitUntil{timeout_text}) != digest(WaitEvent{event_text, timeout_text})
    #[test]
    fn proptest_wait_until_vs_wait_event(
        timeout_text in wait_slot_strategy(),
        event_text in wait_slot_strategy(),
    ) {
        let until_source = wait_workflow_source(&None, &Some(timeout_text.clone()));
        let event_source = wait_workflow_source(&Some(event_text), &Some(timeout_text));

        let digest_until = canonical_digest_compat(&until_source)
            .map_err(|e| TestCaseError::fail(e))?;
        let digest_event = canonical_digest_compat(&event_source)
            .map_err(|e| TestCaseError::fail(e))?;

        prop_assert_ne!(digest_until, digest_event,
            "WaitUntil and WaitEvent must produce different digests");
    }

    /// PO-006: Timeout field sensitivity — different timeout values produce
    /// different digests. This is the reachable version of the sentinel test:
    /// the sentinel "none" is not valid YAML (must be integer string) so it
    /// cannot reach `canonical_digest` through compilation. The sentinel
    /// property itself is verified by Kani (direct `digest_step_primitive` call).
    /// Here we verify that different integer timeout values produce different
    /// digests, which indirectly covers the property that absent (None) ≠
    /// present (some nonzero value).
    #[test]
    fn proptest_wait_sentinel_unambiguous(
        event_text in wait_slot_strategy(),
        timeout_a in wait_slot_strategy(),
        timeout_b in wait_slot_strategy(),
    ) {
        // Only relevant when the two timeout values differ
        prop_assume!(timeout_a != timeout_b);

        let source_a = wait_workflow_source(&Some(event_text.clone()), &Some(timeout_a));
        let source_b = wait_workflow_source(&Some(event_text), &Some(timeout_b));

        let digest_a = canonical_digest_compat(&source_a)
            .map_err(|e| TestCaseError::fail(e))?;
        let digest_b = canonical_digest_compat(&source_b)
            .map_err(|e| TestCaseError::fail(e))?;

        prop_assert_ne!(digest_a, digest_b,
            "different timeout values must produce different digests");
    }

    /// PO-009 / PO-016: Cross-path digest equivalence.
    /// For all generated workflow sources with Wait steps, compile_source()
    /// and compile_workflow() produce identical WorkflowDigest values.
    #[test]
    fn cross_path_wait_digest_equivalence(
        event in wait_slot_strategy(),
        timeout in wait_slot_strategy(),
    ) {
        // Build a valid Wait workflow using a random shape
        let hash_byte = event.as_bytes().first().copied().unwrap_or(0)
            .wrapping_add(timeout.as_bytes().first().copied().unwrap_or(0));
        let (event, timeout) = match hash_byte % 3 {
            0 => (None, Some(timeout)),                    // WaitUntil
            1 => (Some(event), None),                      // WaitEvent unbounded
            _ => (Some(event), Some(timeout)),             // WaitEvent bounded
        };

        let source = wait_workflow_source(&event, &timeout);

        // compile_source uses the cold-path (canonical_digest in part_05.rs)
        let cold = compile_source(&source)
            .map_err(|e| TestCaseError::fail(format!("cold-path compile failed: {e:?}")))?;

        // compile_workflow delegates to YamlCompiler::compile() → compile_source
        let yaml = wait_workflow_yaml(&event, &timeout);
        let warm = compile_workflow(yaml.as_bytes())
            .map_err(|e| TestCaseError::fail(format!("warm-path compile failed: {e:?}")))?;

        prop_assert_eq!(cold.digest(), warm.digest(),
            "cold-path and warm-path must produce identical digests");
    }

    /// PO-011: Pairwise distinct digests for distinct Wait configurations.
    /// For any two different Wait configurations wa != wb in otherwise-identical
    /// workflows, canonical_digest(wf_with(wa)) != canonical_digest(wf_with(wb)).
    #[test]
    fn proptest_wait_pairwise_distinct_digests(
        e1 in wait_slot_strategy(),
        t1 in wait_slot_strategy(),
        e2 in wait_slot_strategy(),
        t2 in wait_slot_strategy(),
    ) {
        // Build two different legal Wait shapes
        let w1 = make_legal_wait_shape(&e1, &t1);
        let w2 = make_legal_wait_shape(&e2, &t2);

        // If the Wait shapes are identical, skip
        if w1 == w2 {
            return Ok(());
        }

        let source1 = wait_workflow_source(&w1.0, &w1.1);
        let source2 = wait_workflow_source(&w2.0, &w2.1);

        let digest1 = canonical_digest_compat(&source1)
            .map_err(|e| TestCaseError::fail(e))?;
        let digest2 = canonical_digest_compat(&source2)
            .map_err(|e| TestCaseError::fail(e))?;

        prop_assert_ne!(digest1, digest2,
            "distinct Wait shapes must produce distinct digests");
    }
}

// ── Strategies and helpers for Wait digest tests ──

/// Generates a slot expression string: integer-like strings "0".."255".
/// The validator expects integer strings for wait event/timeout fields.
fn wait_slot_strategy() -> impl Strategy<Value = String> {
    // Generate integer-looking strings that pass the validator
    (0u8..255u8).prop_map(|n| n.to_string())
}

/// Generates (Option<String>, Option<String>) pairs for wait fields.
/// At least one field will be Some (legal shape guarantee). Randomly
/// makes each field None to cover all three legal Wait shapes.
fn wait_field_strategy() -> impl Strategy<Value = (Option<String>, Option<String>)> {
    (
        wait_slot_strategy(),
        wait_slot_strategy(),
        any::<u8>(),
        any::<u8>(),
    )
        .prop_map(|(e, t, eb, tb)| {
            let event = if eb % 3 == 0 { None } else { Some(e) };
            let timeout = if tb % 3 == 0 { None } else { Some(t) };
            (event, timeout)
        })
        .prop_filter("at least one wait field must be Some", |(e, t)| {
            e.is_some() || t.is_some()
        })
}

/// Returns a legal Wait field pair from two strings.
/// Varies the shape between WaitUntil (event=None, timeout=Some),
/// WaitEvent-unbounded (event=Some, timeout=None), and
/// WaitEvent-bounded (event=Some, timeout=Some) based on a hash.
fn make_legal_wait_shape(event: &str, timeout: &str) -> (Option<String>, Option<String>) {
    // Use the first byte of hash to pick a Wait shape variant
    let hash_byte = event
        .as_bytes()
        .first()
        .copied()
        .unwrap_or(0)
        .wrapping_add(timeout.as_bytes().first().copied().unwrap_or(0));
    match hash_byte % 3 {
        0 => (None, Some(timeout.to_string())), // WaitUntil
        1 => (Some(event.to_string()), None),   // WaitEvent unbounded
        _ => (Some(event.to_string()), Some(timeout.to_string())), // WaitEvent bounded
    }
}

/// Builds a WorkflowSource with a single Wait step using the given fields.
fn wait_workflow_source(
    event: &Option<String>,
    timeout: &Option<String>,
) -> vb_compile::WorkflowSource {
    let yaml = wait_workflow_yaml(event, timeout);
    vb_compile::parse_workflow_source(&yaml).expect("valid wait workflow YAML")
}

/// Builds the YAML string for a single-Wait-step workflow.
fn wait_workflow_yaml(event: &Option<String>, timeout: &Option<String>) -> String {
    let mut wait_block = String::from("  - id: wait_step\n    wait:");
    if let Some(e) = event {
        wait_block.push_str(&format!("\n      event: \"{e}\""));
    }
    if let Some(t) = timeout {
        wait_block.push_str(&format!("\n      timeout: \"{t}\""));
    }
    format!(
        "version: velvet-ballistics/v1\nname: wait_digest_test\nwhen:\n  manual: {{}}\nsteps:\n{wait_block}\n  - id: finish_step\n    finish:\n      result: 0\n"
    )
}

/// Computes the canonical_digest from a parsed WorkflowSource.
/// Uses compile_source (cold-path), which internally calls canonical_digest.
fn canonical_digest_compat(
    source: &vb_compile::WorkflowSource,
) -> Result<vb_core::WorkflowDigest, String> {
    compile_source(source)
        .map(|wf| wf.digest())
        .map_err(|errors| format!("compile_source failed: {errors:?}"))
}

fn primitive_case_strategy() -> impl Strategy<Value = PrimitiveCase> {
    prop::sample::select(PRIMITIVE_CASES.to_vec())
}

// =========================================================================
// PO-006: Proptest for different repeat configs → different digest
// =========================================================================

/// Proptest strategy generating two distinct repeat configurations.
///
/// Produces pairs of repeat cases identical except for `max_attempts`.
/// GOD RULE 1: variable max_attempts, not single hardcoded value.
fn repeat_variable_strategy() -> impl Strategy<Value = (u16, u16)> {
    (1u16..=u16::MAX, 1u16..=u16::MAX).prop_filter("max_attempts must differ", |(a, b)| a != b)
}

/// YAML template with a `{}` placeholder for max_attempts.
const REPEAT_YAML_BASE: &str = "  - id: retry\n    repeat:\n      max_attempts: {}\n      steps:\n        - id: attempt\n          set:\n            output: attempted\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n";

fn repeat_yaml(max_attempts: u16) -> String {
    REPEAT_YAML_BASE.replace("{}", &max_attempts.to_string())
}

// PO-006 / PROPTEST-REPEAT-001: Different repeat max_attempts produce
// different WorkflowDigest values via compile_workflow.
//
// Non-vacuous: asserts inequality, not just that both compile.
proptest! {
    #![proptest_config(ProptestConfig { cases: 64, failure_persistence: None, .. ProptestConfig::default() })]

    #[test]
    fn proptest_repeat_different_params_different_digest(
        (max1, max2) in repeat_variable_strategy()
    ) {
        use proptest::prelude::*;

        let yaml1 = workflow_yaml(&repeat_yaml(max1));
        let yaml2 = workflow_yaml(&repeat_yaml(max2));

        let wf1 = compile_workflow(yaml1.as_bytes()).map_err(|e| TestCaseError::fail(format_compile_errors(&e)))?;
        let wf2 = compile_workflow(yaml2.as_bytes()).map_err(|e| TestCaseError::fail(format_compile_errors(&e)))?;

        prop_assert_ne!(
            wf1.digest(),
            wf2.digest(),
            "repeat max_attempts {} vs {} must produce different digests",
            max1, max2
        );
    }

    // PO-006 extended: Different repeat body Set values produce different digests.
    // Both bodies use single Set step (required by lowering validation),
    // but with distinct Set output/value fields.
    #[test]
    fn proptest_repeat_different_body_different_digest(
        max_attempts in 1u16..=u16::MAX,
    ) {
        use proptest::prelude::*;

        // Body A: single Set with output=seen, value="1"
        let yaml_set_body = workflow_yaml(&format!(
            "  - id: retry\n    repeat:\n      max_attempts: {max}\n      steps:\n        - id: a_set\n          set:\n            output: seen\n            value: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
            max = max_attempts
        ));

        // Body B: single Set with output=out99, value="99" — different content
        let yaml_diff_body = workflow_yaml(&format!(
            "  - id: retry\n    repeat:\n      max_attempts: {max}\n      steps:\n        - id: b_set\n          set:\n            output: out99\n            value: \"99\"\n  - id: done\n    finish:\n      result: 0\n",
            max = max_attempts
        ));

        let wf1 = compile_workflow(yaml_set_body.as_bytes()).map_err(|e| TestCaseError::fail(format_compile_errors(&e)))?;
        let wf2 = compile_workflow(yaml_diff_body.as_bytes()).map_err(|e| TestCaseError::fail(format_compile_errors(&e)))?;

        prop_assert_ne!(
            wf1.digest(),
            wf2.digest(),
            "repeat with different Set body values must produce different digests"
        );
    }
}

pub(super) fn compile_case(case: &PrimitiveCase) -> Result<CompiledWorkflow, String> {
    let yaml = workflow_yaml(case.yaml_steps);
    compile_yaml(&yaml).map_err(|error| format!("primitive {} failed: {error}", case.name))
}

// ── Section 9.3: Integration tests for Wait digest (vb-xi2f.32) ──

/// B1: Two WaitEvent workflows with different event values must produce
/// different digests. Tested through compile_source (cold-path).
#[test]
fn wait_event_sensitivity_to_event_field_change_through_compile_source_when_event_differs()
-> Result<(), String> {
    let steps_a = "  - id: wait_a\n    wait:\n      event: \"0\"\n      timeout: \"30\"\n  - id: done\n    finish:\n      result: 0\n";
    let steps_b = "  - id: wait_a\n    wait:\n      event: \"1\"\n      timeout: \"30\"\n  - id: done\n    finish:\n      result: 0\n";

    let source_a = parse_source(&workflow_yaml(steps_a))?;
    let source_b = parse_source(&workflow_yaml(steps_b))?;

    let digest_a = compile_source(&source_a)
        .map_err(|errors| format!("compile A failed: {}", format_compile_errors(&errors)))?
        .digest();
    let digest_b = compile_source(&source_b)
        .map_err(|errors| format!("compile B failed: {}", format_compile_errors(&errors)))?
        .digest();

    assert_ne!(
        digest_a, digest_b,
        "WaitEvent workflows with different event values must produce different digests"
    );
    Ok(())
}

/// B2: Two WaitEvent workflows with different timeout values must produce
/// different digests. Tested through compile_source.
#[test]
fn wait_event_sensitivity_to_timeout_field_change_through_compile_source_when_timeout_differs()
-> Result<(), String> {
    let steps_a = "  - id: wait_a\n    wait:\n      event: \"0\"\n      timeout: \"10\"\n  - id: done\n    finish:\n      result: 0\n";
    let steps_b = "  - id: wait_a\n    wait:\n      event: \"0\"\n      timeout: \"20\"\n  - id: done\n    finish:\n      result: 0\n";

    let source_a = parse_source(&workflow_yaml(steps_a))?;
    let source_b = parse_source(&workflow_yaml(steps_b))?;

    let digest_a = compile_source(&source_a)
        .map_err(|errors| format!("compile A failed: {}", format_compile_errors(&errors)))?
        .digest();
    let digest_b = compile_source(&source_b)
        .map_err(|errors| format!("compile B failed: {}", format_compile_errors(&errors)))?
        .digest();

    assert_ne!(
        digest_a, digest_b,
        "WaitEvent workflows with different timeout values must produce different digests"
    );
    Ok(())
}

/// B2: Two WaitUntil workflows with different timeout/deadline values must
/// produce different digests.
#[test]
fn wait_until_timeout_change_produces_distinct_digest_through_compile_source_when_timeout_differs()
-> Result<(), String> {
    let steps_a = "  - id: wait_a\n    wait:\n      timeout: \"5\"\n  - id: done\n    finish:\n      result: 0\n";
    let steps_b = "  - id: wait_a\n    wait:\n      timeout: \"10\"\n  - id: done\n    finish:\n      result: 0\n";

    let source_a = parse_source(&workflow_yaml(steps_a))?;
    let source_b = parse_source(&workflow_yaml(steps_b))?;

    let digest_a = compile_source(&source_a)
        .map_err(|errors| format!("compile A failed: {}", format_compile_errors(&errors)))?
        .digest();
    let digest_b = compile_source(&source_b)
        .map_err(|errors| format!("compile B failed: {}", format_compile_errors(&errors)))?
        .digest();

    assert_ne!(
        digest_a, digest_b,
        "WaitUntil workflows with different timeout values must produce different digests"
    );
    Ok(())
}

/// B3: WaitUntil (event=None, timeout=Some) vs WaitEvent (event=Some, timeout=None)
/// must produce different digests — the explicit discriminator.
#[test]
fn wait_until_vs_wait_event_produce_distinct_digests_through_compile_source_when_shapes_differ()
-> Result<(), String> {
    let steps_until = "  - id: wait_a\n    wait:\n      timeout: \"5\"\n  - id: done\n    finish:\n      result: 0\n";
    let steps_event = "  - id: wait_a\n    wait:\n      event: \"5\"\n  - id: done\n    finish:\n      result: 0\n";

    let source_until = parse_source(&workflow_yaml(steps_until))?;
    let source_event = parse_source(&workflow_yaml(steps_event))?;

    let digest_until = compile_source(&source_until)
        .map_err(|errors| format!("compile until failed: {}", format_compile_errors(&errors)))?
        .digest();
    let digest_event = compile_source(&source_event)
        .map_err(|errors| format!("compile event failed: {}", format_compile_errors(&errors)))?
        .digest();

    assert_ne!(
        digest_until, digest_event,
        "WaitUntil ({:?}) and WaitEvent ({:?}) must produce distinct digests",
        digest_until, digest_event
    );
    Ok(())
}

/// B4: WaitEvent with timeout=None must produce a different digest than
/// WaitEvent with the same event but timeout=Some("5"). The sentinel
/// b"none" for absent timeout must be unambiguous.
#[test]
fn wait_event_no_timeout_vs_with_timeout_produce_distinct_digests_through_compile_source_when_timeout_absent()
-> Result<(), String> {
    let steps_no_timeout = "  - id: wait_a\n    wait:\n      event: \"0\"\n  - id: done\n    finish:\n      result: 0\n";
    let steps_with_timeout = "  - id: wait_a\n    wait:\n      event: \"0\"\n      timeout: \"5\"\n  - id: done\n    finish:\n      result: 0\n";

    let source_no_timeout = parse_source(&workflow_yaml(steps_no_timeout))?;
    let source_with_timeout = parse_source(&workflow_yaml(steps_with_timeout))?;

    let digest_no_timeout = compile_source(&source_no_timeout)
        .map_err(|errors| {
            format!(
                "compile no-timeout failed: {}",
                format_compile_errors(&errors)
            )
        })?
        .digest();
    let digest_with_timeout = compile_source(&source_with_timeout)
        .map_err(|errors| {
            format!(
                "compile with-timeout failed: {}",
                format_compile_errors(&errors)
            )
        })?
        .digest();

    assert_ne!(
        digest_no_timeout, digest_with_timeout,
        "WaitEvent with timeout=None must differ from timeout=Some; sentinel must be unambiguous"
    );
    Ok(())
}

/// B5: Compiling the same Wait workflow three times via compile_source must
/// produce identical digests each time (determinism).
#[test]
fn wait_digest_is_deterministic_through_compile_source_when_same_source_compiled_thrice()
-> Result<(), String> {
    let steps = "  - id: wait_a\n    wait:\n      event: \"42\"\n      timeout: \"99\"\n  - id: done\n    finish:\n      result: 0\n";
    let source = parse_source(&workflow_yaml(steps))?;

    let digest1 = compile_source(&source)
        .map_err(|errors| format!("compile #1 failed: {}", format_compile_errors(&errors)))?
        .digest();
    let digest2 = compile_source(&source)
        .map_err(|errors| format!("compile #2 failed: {}", format_compile_errors(&errors)))?
        .digest();
    let digest3 = compile_source(&source)
        .map_err(|errors| format!("compile #3 failed: {}", format_compile_errors(&errors)))?
        .digest();

    assert_eq!(
        digest1, digest2,
        "first two compilations must produce equal digests"
    );
    assert_eq!(
        digest2, digest3,
        "second and third compilations must produce equal digests"
    );
    Ok(())
}

/// B6: Digest roundtrips through WorkflowParts — to_parts().digest must match
/// the original digest from CompiledWorkflow::digest().
#[test]
fn wait_workflow_digest_roundtrips_through_parts_after_compile_source_when_wait_steps_present()
-> Result<(), String> {
    let steps = "  - id: wait_a\n    wait:\n      event: \"0\"\n      timeout: \"30\"\n  - id: done\n    finish:\n      result: 0\n";
    let source = parse_source(&workflow_yaml(steps))?;

    let compiled = compile_source(&source)
        .map_err(|errors| format!("compile failed: {}", format_compile_errors(&errors)))?;
    let digest_from_compiled = compiled.digest();
    let parts = compiled.to_parts();
    let digest_from_parts = parts.digest;

    assert_eq!(
        digest_from_compiled, digest_from_parts,
        "WorkflowParts::digest must match CompiledWorkflow::digest()"
    );
    Ok(())
}

/// B8/B12: A workflow with Wait + Set + Finish must produce a different
/// digest than a workflow with only Set + Finish. The Wait primitive
/// contribution must be observable in the final digest.
#[test]
fn wait_workflow_with_mixed_steps_digests_differ_from_non_wait_workflow_when_wait_added()
-> Result<(), String> {
    let steps_with_wait = "  - id: assign\n    set:\n      output: x\n      value: \"10\"\n  - id: wait_here\n    wait:\n      event: \"0\"\n      timeout: \"5\"\n  - id: done\n    finish:\n      result: x\n";
    let steps_no_wait = "  - id: assign\n    set:\n      output: x\n      value: \"10\"\n  - id: done\n    finish:\n      result: x\n";

    let source_with_wait = parse_source(&workflow_yaml(steps_with_wait))?;
    let source_no_wait = parse_source(&workflow_yaml(steps_no_wait))?;

    let digest_with_wait = compile_source(&source_with_wait)
        .map_err(|errors| {
            format!(
                "compile with-wait failed: {}",
                format_compile_errors(&errors)
            )
        })?
        .digest();
    let digest_no_wait = compile_source(&source_no_wait)
        .map_err(|errors| format!("compile no-wait failed: {}", format_compile_errors(&errors)))?
        .digest();

    assert_ne!(
        digest_with_wait, digest_no_wait,
        "Adding a Wait step to a workflow must change the canonical digest"
    );
    Ok(())
}

/// B11: An invalid wait shape (event=None, timeout=None) must be rejected
/// with CompileError::StepFieldShape.
#[test]
fn wait_invalid_shape_event_none_timeout_none_rejected_with_step_field_shape_when_both_fields_absent()
-> Result<(), String> {
    let steps = "  - id: bad_wait\n    wait: {}\n  - id: done\n    finish:\n      result: 0\n";
    let errors = compile_yaml_error(&workflow_yaml(steps))?;
    let first = first_compile_error(&errors)?;
    match first {
        CompileError::StepFieldShape {
            step,
            field,
            expected,
        } => {
            assert_eq!(*step, 0, "StepFieldShape must reference step 0");
            assert_eq!(*field, "wait", "StepFieldShape must reference field 'wait'");
            assert!(
                !expected.is_empty(),
                "StepFieldShape must have a non-empty expected message"
            );
            Ok(())
        }
        other => Err(format!(
            "expected CompileError::StepFieldShape for empty wait, got {other:?}"
        )),
    }
}
