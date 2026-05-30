// Behavior tests: structural/identity fields in canonical_digest (B15, B16, B17, B18, B19)
// Bead: vb-xi2f.33 / P1: digest covers ask semantics
//
// B15: Digest includes version, name, trigger, step IDs, and primitive fields.
// B16: Source with zero steps produces valid digest.
// B17: Changing trigger type changes the digest.
// B18: Changing version or name changes the digest.
// B19: Changing step order changes the digest.
//
// Verifies POST-006, WF-INV-001, WF-INV-002, WF-INV-004.

#![forbid(unsafe_code)]

mod common;
use common::{
    ask_source, empty_source, named_source, set_source, triggered_source, versioned_source,
};
use vb_compile::canonical_digest;
use vb_yaml::ast::{ScalarValue, StepAst, StepPrimitive, TriggerAst};

// ── B16: Empty source (zero steps) ──

#[test]
fn canonical_digest_produces_valid_digest_when_source_has_no_steps() {
    // Given: source with zero steps
    let source = empty_source();
    // When
    let digest = canonical_digest(&source).expect("valid test input");
    // Then: must produce a valid 32-byte digest without panic
    assert_eq!(
        digest.as_bytes().len(),
        32,
        "WF-INV-001: source with no steps must produce valid 32-byte digest"
    );
    let all_zero = digest.as_bytes().iter().all(|b| *b == 0);
    assert!(
        !all_zero,
        "WF-INV-001: source with no steps must produce non-zero digest"
    );
}

#[test]
fn canonical_digest_is_deterministic_when_source_has_no_steps() {
    // Given
    let source = empty_source();
    // When: called twice
    let d1 = canonical_digest(&source).expect("valid test input");
    let d2 = canonical_digest(&source).expect("valid test input");
    // Then
    assert_eq!(
        d1, d2,
        "WF-INV-001: empty source digest must be deterministic"
    );
}

// ── B15 / B18: Version sensitivity ──

#[test]
fn canonical_digest_produces_distinct_digests_when_version_differs() {
    // Given: two sources differing only in version
    let steps = vec![StepAst {
        id: "step_1".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Finish {
            result: ScalarValue::String("done".to_string()),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    let source_a = versioned_source("velvet-ballistics/v1", "test", steps.clone());
    let source_b = versioned_source("velvet-ballistics/v2", "test", steps);
    // When
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then
    assert_ne!(
        digest_a, digest_b,
        "WF-INV-004: different version strings must produce distinct digests"
    );
}

// ── B15 / B18: Name sensitivity ──

#[test]
fn canonical_digest_produces_distinct_digests_when_name_differs() {
    // Given: two sources differing only in name
    let source_a = ask_source("hello", None);
    // Same Ask config but named differently
    let steps = vec![
        StepAst {
            id: "ask_1".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::Ask {
                prompt: "hello".to_string(),
                timeout: None,
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        },
        StepAst {
            id: "finish_1".to_string(),
            name: None,
            condition: None,
            primitive: StepPrimitive::Finish {
                result: ScalarValue::String("done".to_string()),
            },
            with: None,
            retry: None,
            on_error: None,
            then: None,
        },
    ];
    let source_b = named_source("different_workflow_name", steps);
    // When
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then
    assert_ne!(
        digest_a, digest_b,
        "WF-INV-004: different workflow names must produce distinct digests"
    );
}

// ── B15 / B17: Trigger sensitivity ──

#[test]
fn canonical_digest_produces_distinct_digests_when_trigger_is_manual_vs_webhook() {
    // Given: two sources differing only in trigger type
    let steps = vec![StepAst {
        id: "step_1".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Finish {
            result: ScalarValue::String("done".to_string()),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    let source_a = triggered_source(TriggerAst::Manual, steps.clone());
    let source_b = triggered_source(TriggerAst::Webhook, steps);
    // When
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then
    assert_ne!(
        digest_a, digest_b,
        "WF-INV-004: different trigger types (Manual vs Webhook) must produce distinct digests"
    );
}

#[test]
fn canonical_digest_produces_distinct_digests_when_trigger_schedule_cron_differs() {
    // Given: two schedule triggers with different cron expressions
    let steps = vec![StepAst {
        id: "step_1".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Finish {
            result: ScalarValue::String("done".to_string()),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    let source_a = triggered_source(
        TriggerAst::Schedule {
            cron: "*/5 * * * *".to_string(),
        },
        steps.clone(),
    );
    let source_b = triggered_source(
        TriggerAst::Schedule {
            cron: "0 0 * * *".to_string(),
        },
        steps,
    );
    // When
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then
    assert_ne!(
        digest_a, digest_b,
        "WF-INV-004: different schedule cron expressions must produce distinct digests"
    );
}

#[test]
fn canonical_digest_produces_distinct_digests_when_trigger_is_manual_vs_event() {
    // Given: Manual vs Event trigger
    let steps = vec![StepAst {
        id: "step_1".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Finish {
            result: ScalarValue::String("done".to_string()),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    let source_a = triggered_source(TriggerAst::Manual, steps.clone());
    let source_b = triggered_source(
        TriggerAst::Event {
            event_type: "custom.event".to_string(),
        },
        steps,
    );
    // When
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then
    assert_ne!(
        digest_a, digest_b,
        "WF-INV-004: Manual vs Event trigger must produce distinct digests"
    );
}

// ── B15: Step ID sensitivity ──

#[test]
fn canonical_digest_produces_distinct_digests_when_step_id_differs() {
    // Given: two sources with different step IDs but same primitive content
    use vb_yaml::ast::{WorkflowSource, WorkflowSourceParts};
    let steps_a = vec![StepAst {
        id: "step_alpha".to_string(),
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
    let steps_b = vec![StepAst {
        id: "step_beta".to_string(),
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
    let source_a = WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "test_workflow".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: steps_a,
        result: None,
        examples: vec![],
    });
    let source_b = WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "test_workflow".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: steps_b,
        result: None,
        examples: vec![],
    });
    // When
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then: step ID is hashed, so different IDs → different digests
    assert_ne!(
        digest_a, digest_b,
        "POST-006: different step IDs must produce distinct digests"
    );
}

// ── B19: Step order sensitivity ──

#[test]
fn canonical_digest_produces_distinct_digests_when_step_order_differs_ask_set_vs_set_ask() {
    // Given: sources with [Ask, Set] and [Set, Ask] (same steps, different order)
    use vb_yaml::ast::{WorkflowSource, WorkflowSourceParts};
    let ask_step = StepAst {
        id: "ask_1".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Ask {
            prompt: "hello".to_string(),
            timeout: None,
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };
    let set_step = StepAst {
        id: "set_1".to_string(),
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
    };
    let source_a = WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "test_workflow".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![ask_step.clone(), set_step.clone()],
        result: None,
        examples: vec![],
    });
    let source_b = WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "test_workflow".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![set_step, ask_step],
        result: None,
        examples: vec![],
    });
    // When
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then: different step order → different digest
    assert_ne!(
        digest_a, digest_b,
        "WF-INV-002: different step order must produce distinct digests"
    );
}

#[test]
fn canonical_digest_produces_distinct_digests_when_step_order_differs_within_same_type() {
    // Given: two Set steps with different IDs, swapped order
    use vb_yaml::ast::{WorkflowSource, WorkflowSourceParts};
    let set_a = StepAst {
        id: "set_a".to_string(),
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
    };
    let set_b = StepAst {
        id: "set_b".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "b".to_string(),
            value: "2".to_string(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };
    let source_a = WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "test_workflow".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![set_a.clone(), set_b.clone()],
        result: None,
        examples: vec![],
    });
    let source_b = WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "test_workflow".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![set_b, set_a],
        result: None,
        examples: vec![],
    });
    // When
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then
    assert_ne!(
        digest_a, digest_b,
        "WF-INV-002: reversed step order must produce distinct digest"
    );
}

// ── B15/B19: Adding/removing steps changes digest ──

#[test]
fn canonical_digest_produces_distinct_digests_when_step_is_added() {
    // Given: source with 1 step vs source with 2 steps
    let source_a = set_source("x", "1");
    let source_b = set_finish_source_test();
    // When
    let digest_a = canonical_digest(&source_a).expect("valid test input");
    let digest_b = canonical_digest(&source_b).expect("valid test input");
    // Then: different step counts → different digests
    assert_ne!(
        digest_a, digest_b,
        "WF-INV-002: adding a step must change the digest"
    );
}

/// Build a source with Set + Finish steps (same as set_finish_source but different step IDs
/// to avoid collision).
fn set_finish_source_test() -> vb_yaml::ast::WorkflowSource {
    use vb_yaml::ast::{WorkflowSource, WorkflowSourceParts};
    WorkflowSource::new(WorkflowSourceParts {
        version: "velvet-ballistics/v1".to_string(),
        name: "test_set_workflow".to_string(),
        trigger: TriggerAst::Manual,
        inputs: vec![],
        vars: vec![],
        secrets: vec![],
        steps: vec![
            StepAst {
                id: "set_1".to_string(),
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
            },
            StepAst {
                id: "finish_1".to_string(),
                name: None,
                condition: None,
                primitive: StepPrimitive::Finish {
                    result: ScalarValue::String("done".to_string()),
                },
                with: None,
                retry: None,
                on_error: None,
                then: None,
            },
        ],
        result: None,
        examples: vec![],
    })
}
