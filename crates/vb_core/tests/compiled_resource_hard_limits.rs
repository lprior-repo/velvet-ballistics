#![forbid(unsafe_code)]

use std::error::Error;
use vb_core::workflow::{ContractResource, node};
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstValue, ResourceContract, StepIdx,
    WorkflowDigest, WorkflowError, WorkflowParts,
};

type TestResult<T = ()> = Result<T, Box<dyn Error>>;

const STEP_LIMIT: usize = 1_000;
const CONSTANT_LIMIT: usize = 8_192;

fn nodes(node_count: usize) -> TestResult<Box<[CompiledNode]>> {
    let last = node_count
        .checked_sub(1)
        .ok_or("node count must be positive")?;
    let nodes = (0..node_count)
        .map(|index| {
            let id = StepIdx::new(u16::try_from(index)?);
            let next = if index < last {
                let next_index = index.checked_add(1).ok_or("step index overflow")?;
                Some(StepIdx::new(u16::try_from(next_index)?))
            } else {
                None
            };
            Ok(node(id, CompiledNodeKind::Nop, None, next))
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    Ok(nodes.into_boxed_slice())
}

fn parts(
    node_count: usize,
    constants: Vec<ConstValue>,
    contract: ResourceContract,
) -> TestResult<WorkflowParts> {
    Ok(WorkflowParts {
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        name: Box::<str>::from("hard_limit_test"),
        digest: WorkflowDigest::from_bytes([0xAB; 32]),
        nodes: nodes(node_count)?,
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: constants.into_boxed_slice(),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: contract,
        step_names: Box::default(),
    })
}

fn with_steps(max_steps: u16) -> ResourceContract {
    ResourceContract {
        max_steps,
        ..ResourceContract::DEFAULT
    }
}

fn with_constants(max_constants: u16) -> ResourceContract {
    ResourceContract {
        max_constants,
        ..ResourceContract::DEFAULT
    }
}

#[test]
fn checked_constructor_accepts_exact_step_limit() -> TestResult {
    let parts = parts(
        STEP_LIMIT,
        vec![ConstValue::Null],
        with_steps(u16::try_from(STEP_LIMIT)?),
    )?;
    let workflow = CompiledWorkflow::try_from_parts(parts)?;
    assert_eq!(usize::from(workflow.node_count()), STEP_LIMIT);
    Ok(())
}

#[test]
fn checked_constructor_rejects_one_over_declared_step_limit() -> TestResult {
    let parts = parts(
        STEP_LIMIT.checked_add(1).ok_or("step count overflow")?,
        vec![ConstValue::Null],
        with_steps(u16::try_from(STEP_LIMIT)?),
    )?;
    assert!(matches!(
        CompiledWorkflow::try_from_parts(parts),
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::Steps
        })
    ));
    Ok(())
}

#[test]
fn checked_constructor_rejects_declared_step_limit_above_hard_cap() -> TestResult {
    let parts = parts(
        1,
        vec![ConstValue::Null],
        with_steps(u16::try_from(
            STEP_LIMIT.checked_add(1).ok_or("step count overflow")?,
        )?),
    )?;
    assert!(matches!(
        CompiledWorkflow::try_from_parts(parts),
        Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Steps
        })
    ));
    Ok(())
}

#[test]
fn checked_constructor_accepts_exact_constant_limit() -> TestResult {
    let constants = vec![ConstValue::Null; CONSTANT_LIMIT];
    let parts = parts(1, constants, with_constants(u16::try_from(CONSTANT_LIMIT)?))?;
    CompiledWorkflow::try_from_parts(parts)?;
    Ok(())
}

#[test]
fn checked_constructor_rejects_actual_constants_over_declared_limit() -> TestResult {
    let constants = vec![
        ConstValue::Null;
        CONSTANT_LIMIT
            .checked_add(1)
            .ok_or("constant count overflow")?
    ];
    let parts = parts(1, constants, with_constants(u16::try_from(CONSTANT_LIMIT)?))?;
    assert!(matches!(
        CompiledWorkflow::try_from_parts(parts),
        Err(WorkflowError::ResourceContractExceeded {
            resource: ContractResource::Constants
        })
    ));
    Ok(())
}

#[test]
fn checked_constructor_rejects_declared_constant_limit_above_hard_cap() -> TestResult {
    let parts = parts(
        1,
        vec![ConstValue::Null],
        with_constants(u16::try_from(
            CONSTANT_LIMIT
                .checked_add(1)
                .ok_or("constant count overflow")?,
        )?),
    )?;
    assert!(matches!(
        CompiledWorkflow::try_from_parts(parts),
        Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Constants
        })
    ));
    Ok(())
}

#[test]
fn oversized_serialized_parts_decode_but_checked_admission_rejects() -> TestResult {
    let mut oversized = ResourceContract::DEFAULT;
    oversized.max_constants = u16::MAX;
    let original = parts(1, vec![ConstValue::Null], oversized)?;
    let encoded = postcard::to_allocvec(&original)?;
    let decoded: WorkflowParts = postcard::from_bytes(&encoded)?;
    assert_eq!(decoded, original);
    assert_eq!(postcard::to_allocvec(&decoded)?, encoded);
    assert!(matches!(
        CompiledWorkflow::try_from_parts(decoded),
        Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Constants
        })
    ));
    Ok(())
}

#[test]
fn old_main_serialized_bounds_decode_but_checked_admission_rejects_steps() -> TestResult {
    let legacy_contract = ResourceContract {
        max_steps: 10_000,
        max_constants: u16::MAX,
        ..ResourceContract::DEFAULT
    };
    let original = parts(1, vec![ConstValue::Null], legacy_contract)?;
    let encoded = postcard::to_allocvec(&original)?;
    let decoded: WorkflowParts = postcard::from_bytes(&encoded)?;
    assert_eq!(decoded, original);
    assert_eq!(postcard::to_allocvec(&decoded)?, encoded);
    assert!(matches!(
        CompiledWorkflow::try_from_parts(decoded),
        Err(WorkflowError::ResourceContractTooLarge {
            resource: ContractResource::Steps
        })
    ));
    Ok(())
}

#[test]
fn hard_limits_match_master_and_keep_execution_budget_independent() {
    assert_eq!(vb_core::limits::MAX_STEPS_PER_WORKFLOW, STEP_LIMIT);
    assert_eq!(vb_core::limits::MAX_CONSTANTS, CONSTANT_LIMIT);
    assert_eq!(usize::from(ResourceContract::DEFAULT.max_steps), STEP_LIMIT);
    assert_eq!(
        usize::from(ResourceContract::DEFAULT.max_constants),
        CONSTANT_LIMIT
    );
    assert_eq!(
        vb_core::budget::BoundednessPolicy::DEFAULT.max_total_steps,
        1_000_000
    );
}
