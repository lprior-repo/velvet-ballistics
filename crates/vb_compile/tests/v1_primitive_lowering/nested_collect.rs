use super::*;

#[test]
fn compile_workflow_reports_missing_nested_collect_steps() -> Result<(), String> {
    let yaml = workflow_yaml(
        "  - id: collect_pages\n    collect:\n      variable: batch\n      source: \"0\"\n      pages: 1\n      items: 1\n      steps:\n        - id: inner\n          collect:\n            variable: inner_page\n            source: \"1\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let errors = compile_yaml_error(&yaml)?;
    let first = first_compile_error(&errors)?;
    match first {
        CompileError::MissingStepField { step, field } => {
            assert_eq!((*step, *field), (0, "collect.steps"));
            Ok(())
        }
        other => Err(format!(
            "expected MissingStepField for nested collect.steps, got {other:?}"
        )),
    }
}

#[test]
fn compile_workflow_accepts_nested_collect_body() -> Result<(), String> {
    // A collect nested inside a collect body lowers recursively: the inner
    // region (nodes 1..=4) sits inside the outer body interval (1..=5).
    let yaml = workflow_yaml(
        "  - id: collect_pages\n    collect:\n      variable: batch\n      source: \"0\"\n      pages: 1\n      items: 1\n      steps:\n        - id: inner\n          collect:\n            variable: inner_page\n            source: \"1\"\n            pages: 1\n            items: 1\n            steps:\n              - id: leaf\n                set:\n                  output: picked\n                  value: \"7\"\n  - id: done\n    finish:\n      result: 0\n",
    );
    let workflow = compile_yaml(&yaml)?;
    let nodes = workflow.to_parts().nodes;
    assert_eq!(nodes.len(), 8, "nested collect must emit 8 nodes");
    let specs = [
        (
            0u16,
            CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: 1,
                page_size: 1,
                body: StepIdx::new(1),
                done: StepIdx::new(6),
            },
            Some(2u16),
            None,
        ),
        (
            1u16,
            CompiledNodeKind::CollectStart {
                source: SlotIdx::new(1),
                limit: 1,
                page_size: 1,
                body: StepIdx::new(2),
                done: StepIdx::new(4),
            },
            Some(4u16),
            None,
        ),
        (
            3u16,
            CompiledNodeKind::CollectNext {
                collector_slot: SlotIdx::new(4),
                body: StepIdx::new(2),
                done: StepIdx::new(4),
            },
            None,
            None,
        ),
        (
            4u16,
            CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(4),
            },
            Some(5u16),
            Some(5u16),
        ),
        (
            5u16,
            CompiledNodeKind::CollectNext {
                collector_slot: SlotIdx::new(2),
                body: StepIdx::new(1),
                done: StepIdx::new(6),
            },
            None,
            None,
        ),
        (
            6u16,
            CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(2),
            },
            Some(3u16),
            Some(7u16),
        ),
    ];
    for (id, kind, output, next) in specs {
        let actual = node_at(&nodes, usize::from(id))?;
        let expected = node(
            StepIdx::new(id),
            kind,
            output.map(SlotIdx::new),
            next.map(StepIdx::new),
        );
        if actual != &expected {
            return Err(format!(
                "node {} mismatch: actual={:?} expected={:?}",
                id, actual, expected
            ));
        }
    }
    Ok(())
}
