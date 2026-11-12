fn nested_workflow_nodes(collect_limit: u32, max_attempts: u16) -> [CompiledNode; 7] {
    [
        node(
            StepIdx::new(0),
            CompiledNodeKind::CollectStart {
                source: SlotIdx::new(0),
                limit: collect_limit.into(),
                page_size: 2,
                body: StepIdx::new(1),
                done: StepIdx::new(6),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::TogetherStart {
                branches: Box::from([StepIdx::new(2), StepIdx::new(4)]),
                join: StepIdx::new(5),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(2),
            CompiledNodeKind::RepeatStart {
                max_attempts: max_attempts.into(),
                body: StepIdx::new(3),
                done: StepIdx::new(5),
            },
            None,
            None,
        ),
        node(
            StepIdx::new(3),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(5)),
        ),
        node(
            StepIdx::new(4),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(5)),
        ),
        node(
            StepIdx::new(5),
            CompiledNodeKind::CollectFinish {
                collector_slot: SlotIdx::new(0),
            },
            None,
            Some(StepIdx::new(6)),
        ),
        node(
            StepIdx::new(6),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ]
}

proptest::proptest! {
    #[test]
    fn whole_workflow_budget_within_policy(
        collect_limit in 1u32..=64,
        max_attempts in 1u16..=8,
    ) {
        use proptest::prop_assert_eq;

        let nodes = nested_workflow_nodes(collect_limit, max_attempts);
        let validated = WholeWorkflowBudget::compute(
            &nodes,
            StepIdx::new(0),
            &ResourceContract::DEFAULT,
        )
        .map(|budget| BoundednessPolicy::DEFAULT.validate(&budget));
        prop_assert_eq!(validated, Ok(Ok(())));
    }
}

#[test]
fn budget_bounded_default() {
    let nodes = [
        node(
            StepIdx::new(0),
            CompiledNodeKind::Nop,
            None,
            Some(StepIdx::new(1)),
        ),
        node(
            StepIdx::new(1),
            CompiledNodeKind::Finish {
                result: SlotIdx::new(0),
            },
            None,
            None,
        ),
    ];
    let validated = WholeWorkflowBudget::compute(
        &nodes,
        StepIdx::new(0),
        &ResourceContract::DEFAULT,
    )
    .map(|budget| {
        (
            budget.max_total_steps,
            budget.max_nesting_depth,
            budget.max_fanout,
            BoundednessPolicy::DEFAULT.validate(&budget),
        )
    });
    assert_eq!(validated, Ok((2, 0, 0, Ok(()))));
}
