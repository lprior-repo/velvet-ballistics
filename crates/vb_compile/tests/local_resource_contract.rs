#![forbid(unsafe_code)]
use vb_core::workflow::ContractResource;
use vb_core::{CompiledNodeKind as K, CompiledWorkflow, WorkflowError, WorkflowParts};

type Checked = fn(WorkflowParts) -> Result<CompiledWorkflow, WorkflowError>;
fn constructors() -> [Checked; 2] {
    [
        CompiledWorkflow::try_from_parts,
        CompiledWorkflow::try_from_concurrent_parts,
    ]
}
const HEADER: &str = "version: velvet-ballistics/v1\nname: local_resources\nwhen: {manual: {}}\n";
const INPUTS: &str = "inputs:\n  items: {is: list, of: any, max: 4}\n";
fn ordinary_foreach() -> CompiledWorkflow {
    vb_compile::compile_workflow(format!("{HEADER}{INPUTS}steps:\n  - id: each\n    for_each:\n      variable: entry\n      input: '$input.items'\n      steps:\n        - id: body\n          set: {{output: value, value: '$entry'}}\n  - id: done\n    finish: {{result: '$input.items'}}\n").as_bytes()).expect("compiler-derived ordinary foreach")
}
fn collect() -> CompiledWorkflow {
    vb_compile::compile_workflow(format!("{HEADER}{INPUTS}steps:\n  - id: seed\n    set: {{output: value, value: '1'}}\n  - id: gather\n    collect:\n      variable: batch\n      source: '$input.items'\n      pages: 2\n      items: 3\n      steps:\n        - id: body\n          set: {{output: page, value: '$batch'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n").as_bytes()).expect("compiler-derived collect")
}
fn together() -> CompiledWorkflow {
    vb_compile::compile_workflow(format!("{HEADER}steps:\n  - id: fanout\n    together:\n      branches:\n        - label: a\n          steps:\n            - id: left\n              set: {{output: value, value: '1'}}\n        - label: b\n          steps:\n            - id: right\n              set: {{output: value, value: '2'}}\n  - id: done\n    finish: {{result: '$step.fanout'}}\n").as_bytes()).expect("compiler-derived together")
}

#[test]
fn ordinary_foreach_checked_profiles_enforce_declared_item_limit() {
    let workflow = ordinary_foreach();
    assert!(
        workflow
            .to_parts()
            .nodes
            .iter()
            .any(|node| matches!(node.kind, K::ForEachStart { limit, .. } if limit.get() == 4))
    );
    let outcomes: Vec<_> = constructors()
        .into_iter()
        .map(|checked| {
            assert!(checked(workflow.to_parts()).is_ok());
            let mut exact = workflow.to_parts();
            exact.resource_contract.max_collect_items = 4;
            assert!(checked(exact.clone()).is_ok());
            exact.resource_contract.max_collect_items = 3;
            checked(exact).map(|_| ())
        })
        .collect();
    assert_eq!(
        outcomes,
        vec![
            Err(WorkflowError::ResourceContractExceeded {
                resource: ContractResource::CollectItems
            }),
            Err(WorkflowError::ResourceContractExceeded {
                resource: ContractResource::CollectItems
            })
        ]
    );
}

#[test]
fn collect_checked_profiles_bound_total_items_not_source_page_count() {
    let workflow = collect();
    assert!(workflow.to_parts().nodes.iter().any(
        |node| matches!(node.kind, K::CollectStart { limit, page_size: 3, .. } if limit.get() == 6)
    ));
    let outcomes: Vec<_> = constructors()
        .into_iter()
        .map(|checked| {
            assert!(checked(workflow.to_parts()).is_ok());
            let mut exact = workflow.to_parts();
            exact.resource_contract.max_collect_items = 6;
            assert!(checked(exact.clone()).is_ok());
            exact.resource_contract.max_collect_items = 5;
            checked(exact).map(|_| ())
        })
        .collect();
    assert_eq!(
        outcomes,
        vec![
            Err(WorkflowError::ResourceContractExceeded {
                resource: ContractResource::CollectItems
            }),
            Err(WorkflowError::ResourceContractExceeded {
                resource: ContractResource::CollectItems
            })
        ]
    );
}

#[test]
fn ordinary_together_checked_profiles_enforce_declared_branch_limit() {
    let workflow = together();
    assert!(workflow.to_parts().nodes.iter().any(
        |node| matches!(&node.kind, K::TogetherStart { branches, .. } if branches.len() == 2)
    ));
    assert!(workflow.to_parts().nodes.iter().any(
        |node| matches!(node.kind, K::TogetherJoin { branch_count, .. } if branch_count.get() == 2)
    ));
    let outcomes: Vec<_> = constructors()
        .into_iter()
        .map(|checked| {
            assert!(checked(workflow.to_parts()).is_ok());
            let mut exact = workflow.to_parts();
            exact.resource_contract.max_fanout = 2;
            assert!(checked(exact.clone()).is_ok());
            exact.resource_contract.max_fanout = 1;
            checked(exact).map(|_| ())
        })
        .collect();
    assert_eq!(
        outcomes,
        vec![
            Err(WorkflowError::ResourceContractExceeded {
                resource: ContractResource::Fanout
            }),
            Err(WorkflowError::ResourceContractExceeded {
                resource: ContractResource::Fanout
            })
        ]
    );
}

fn concurrent_foreach() -> CompiledWorkflow {
    let source = format!(
        "{HEADER}{INPUTS}steps:\n  - id: each\n    for_each:\n      variable: entry\n      input: '$input.items'\n      at_once: 2\n      steps:\n        - id: body\n          set: {{output: value, value: '$entry'}}\n  - id: done\n    finish: {{result: '$input.items'}}\n"
    );
    vb_compile::compile_workflow(source.as_bytes()).expect("concurrent foreach")
}

#[test]
fn concurrent_foreach_checked_profile_enforces_same_iteration_item_bound() {
    let workflow = concurrent_foreach();
    assert!(workflow.to_parts().nodes.iter().any(|node| matches!(node.kind, K::ForEachConcurrentStart { limit, at_once, .. } if limit.get() == 4 && at_once.get() == 2)));
    let mut exact = workflow.to_parts();
    exact.resource_contract.max_collect_items = 4;
    exact.resource_contract.max_fanout = 2;
    assert!(CompiledWorkflow::try_from_concurrent_parts(exact.clone()).is_ok());
    exact.resource_contract.max_collect_items = 3;
    assert_eq!(
        CompiledWorkflow::try_from_concurrent_parts(exact).map(|_| ()),
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::CollectItems
        })
    );
}

#[test]
fn concurrent_foreach_keeps_independent_parallelism_bound_and_profile_barrier() {
    let workflow = concurrent_foreach();
    assert!(matches!(
        CompiledWorkflow::try_from_parts(workflow.to_parts()),
        Err(WorkflowError::ConcurrentProfileRequired { .. })
    ));
    for cap in [0, 1, 2] {
        let mut parts = workflow.to_parts();
        parts.resource_contract.max_fanout = cap;
        let result = CompiledWorkflow::try_from_concurrent_parts(parts).map(|_| ());
        let expected = if cap == 2 {
            Ok(())
        } else {
            Err(WorkflowError::ResourceContractExceeded {
                resource: ContractResource::Fanout,
            })
        };
        assert_eq!(result, expected);
    }
}

#[test]
fn unused_zero_caps_stay_valid_and_used_zero_caps_are_refused() {
    let plain = vb_compile::compile_workflow(format!("{HEADER}steps:\n  - id: seed\n    set: {{output: value, value: '1'}}\n  - id: done\n    finish: {{result: value}}\n").as_bytes()).expect("plain workflow");
    for checked in constructors() {
        let mut unused = plain.to_parts();
        unused.resource_contract.max_collect_items = 0;
        unused.resource_contract.max_fanout = 0;
        unused.resource_contract.max_retry_attempts = 0;
        assert!(checked(unused).is_ok());
        let mut serial = ordinary_foreach().to_parts();
        serial.resource_contract.max_fanout = 0;
        assert!(checked(serial.clone()).is_ok());
        serial.resource_contract.max_collect_items = 0;
        assert_eq!(
            checked(serial).map(|_| ()),
            Err(WorkflowError::ResourceContractExceeded {
                resource: ContractResource::CollectItems
            })
        );
        let mut gather = collect().to_parts();
        gather.resource_contract.max_collect_items = 0;
        assert_eq!(
            checked(gather).map(|_| ()),
            Err(WorkflowError::ResourceContractExceeded {
                resource: ContractResource::CollectItems
            })
        );
        let mut branches = together().to_parts();
        branches.resource_contract.max_fanout = 0;
        assert_eq!(
            checked(branches).map(|_| ()),
            Err(WorkflowError::ResourceContractExceeded {
                resource: ContractResource::Fanout
            })
        );
    }
}

#[test]
fn compiler_retains_unknown_source_bound_and_binds_truthful_contract_to_digest() {
    let source = format!(
        "{HEADER}{INPUTS}steps:\n  - id: each\n    for_each:\n      variable: entry\n      input: '$input.items'\n      steps:\n        - id: body\n          set: {{output: value, value: '$entry'}}\n  - id: again\n    for_each:\n      variable: another\n      input: '$step.each'\n      steps:\n        - id: copy\n          set: {{output: value, value: '$another'}}\n  - id: done\n    finish: {{result: '$input.items'}}\n"
    );
    let workflow = vb_compile::compile_workflow(source.as_bytes()).expect("bounded unknown source");
    let mut parts = workflow.to_parts();
    let fallback = u32::try_from(vb_core::limits::MAX_LIST_ITEMS_PER_VALUE).expect("u32 list cap");
    assert!(
        parts.nodes.iter().any(
            |node| matches!(node.kind, K::ForEachStart {limit, ..} if limit.get() == fallback)
        )
    );
    assert_eq!(parts.resource_contract.max_collect_items, fallback);
    let digest = parts.digest;
    parts.digest = vb_core::WorkflowDigest::from_bytes([0; 32]);
    let bytes = postcard::to_allocvec(&parts).expect("encode digest input");
    assert_eq!(
        digest,
        vb_core::WorkflowDigest::from_bytes(blake3::hash(&bytes).into())
    );
    parts.resource_contract.max_collect_items =
        vb_core::ResourceContract::DEFAULT.max_collect_items;
    let old_bytes = postcard::to_allocvec(&parts).expect("old declared cap bytes");
    assert_ne!(blake3::hash(&old_bytes), blake3::hash(&bytes));
    let old: WorkflowParts = postcard::from_bytes(&old_bytes).expect("unchanged parts format");
    for checked in constructors() {
        assert_eq!(
            checked(old.clone()).map(|_| ()),
            Err(WorkflowError::ResourceContractExceeded {
                resource: ContractResource::CollectItems
            })
        );
    }
    assert_eq!(
        ordinary_foreach().resource_contract().max_collect_items,
        vb_core::ResourceContract::DEFAULT.max_collect_items
    );
}

#[test]
fn compiler_inferred_local_caps_preserve_hard_and_whole_workflow_policy_refusals() {
    let collect = format!(
        "{HEADER}{INPUTS}steps:\n  - id: gather\n    collect:\n      variable: batch\n      source: '$input.items'\n      pages: 1048577\n      items: 1\n      steps:\n        - id: body\n          set: {{output: page, value: '$batch'}}\n  - id: done\n    finish: {{result: '$input.items'}}\n"
    );
    let supported = vb_compile::compile_workflow(collect.replace("1048577", "1025").as_bytes())
        .expect("supported item requirement above default");
    assert_eq!(supported.resource_contract().max_collect_items, 1025);
    assert!(supported.to_parts().nodes.iter().any(|node| matches!(node.kind, K::CollectStart { limit, page_size: 1, .. } if limit.get() == 1025)));
    let errors = vb_compile::compile_workflow(collect.as_bytes()).expect_err("hard item ceiling");
    assert!(matches!(
        errors.0.as_slice(),
        [vb_compile::CompileError::Workflow(
            WorkflowError::ResourceContractTooLarge {
                resource: ContractResource::CollectItems
            }
        )]
    ));
    let nested = format!(
        "{HEADER}inputs:\n  items: {{is: list, of: any, max: 1024}}\nsteps:\n  - id: seed\n    set: {{output: value, value: '1'}}\n  - id: outer\n    for_each:\n      variable: entry\n      input: '$input.items'\n      steps:\n        - id: inner\n          for_each:\n            variable: sub\n            input: '$input.items'\n            steps:\n              - id: body\n                set: {{output: value, value: '$sub'}}\n  - id: done\n    finish: {{result: '$step.seed'}}\n"
    );
    let errors =
        vb_compile::compile_workflow(nested.as_bytes()).expect_err("whole workflow policy");
    assert!(matches!(
        errors.0.as_slice(),
        [vb_compile::CompileError::Workflow(
            WorkflowError::BudgetPolicyExceeded {
                detail: vb_core::budget::BudgetDimension::TotalSteps
            }
        )]
    ));
}
