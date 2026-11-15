use vb_core::validate_node_bounds;
use vb_core::workflow::{ErrorRouting, NodeContinuation};
use vb_core::{CompiledWorkflow, SlotIdx, StepIdx, WorkflowError, WorkflowParts};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const SOURCE: &str = "version: velvet-ballistics/v1\nname: node_bounds\nwhen: {manual: {}}\nsteps:\n  - id: seeded\n    set: {output: saved, value: '1'}\n  - id: done\n    finish: {result: saved}\n";

fn parts() -> Result<WorkflowParts, Box<dyn std::error::Error>> {
    let mut parts = vb_compile::compile_workflow(SOURCE.as_bytes())?.to_parts();
    assert_eq!(parts.nodes.len(), 2);
    parts.nodes.first_mut().ok_or("entry")?.routing.error = ErrorRouting::Handler {
        handler: StepIdx::new(1),
        error_slot: Some(SlotIdx::ZERO),
    };
    Ok(parts)
}

fn construct(parts: WorkflowParts, concurrent: bool) -> Result<CompiledWorkflow, WorkflowError> {
    if concurrent {
        CompiledWorkflow::try_from_concurrent_parts(parts)
    } else {
        CompiledWorkflow::try_from_parts(parts)
    }
}

#[test]
fn public_node_bounds_checks_error_targets_like_checked_constructors() -> TestResult {
    let valid = parts()?;
    assert_eq!(validate_node_bounds(&valid), Ok(()));
    for concurrent in [false, true] {
        assert_eq!(construct(valid.clone(), concurrent)?.to_parts(), valid);
    }
    let mut absent = valid.clone();
    absent.nodes.first_mut().ok_or("entry")?.routing.error = ErrorRouting::None;
    assert_eq!(validate_node_bounds(&absent), Ok(()));
    for handler in [StepIdx::new(2), StepIdx::new(u16::MAX)] {
        let mut invalid = valid.clone();
        invalid.nodes.first_mut().ok_or("entry")?.routing.error = ErrorRouting::Handler {
            handler,
            error_slot: Some(SlotIdx::ZERO),
        };
        let expected = WorkflowError::StepOutOfBounds { step: handler };
        assert_eq!(validate_node_bounds(&invalid), Err(expected.clone()));
        for concurrent in [false, true] {
            assert_eq!(
                construct(invalid.clone(), concurrent).err(),
                Some(expected.clone())
            );
        }
    }
    Ok(())
}

#[test]
fn public_node_bounds_preserves_node_then_next_error_precedence() -> TestResult {
    let mut parts = parts()?;
    let node = parts.nodes.first_mut().ok_or("entry")?;
    node.id = StepIdx::new(5);
    node.routing.next = NodeContinuation::Step(StepIdx::new(6));
    node.routing.error = ErrorRouting::Handler {
        handler: StepIdx::new(7),
        error_slot: Some(SlotIdx::ZERO),
    };
    assert_eq!(
        validate_node_bounds(&parts),
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(5)
        })
    );
    parts.nodes.first_mut().ok_or("entry")?.id = StepIdx::ZERO;
    assert_eq!(
        validate_node_bounds(&parts),
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(6)
        })
    );
    parts.nodes.first_mut().ok_or("entry")?.routing.next = NodeContinuation::Step(StepIdx::new(1));
    assert_eq!(
        validate_node_bounds(&parts),
        Err(WorkflowError::StepOutOfBounds {
            step: StepIdx::new(7)
        })
    );
    Ok(())
}

#[test]
fn public_node_bounds_checks_error_targets_after_the_first_node() -> TestResult {
    let mut valid = parts()?;
    valid.nodes.last_mut().ok_or("terminal")?.routing.error = ErrorRouting::Handler {
        handler: StepIdx::new(1),
        error_slot: Some(SlotIdx::ZERO),
    };
    assert_eq!(validate_node_bounds(&valid), Ok(()));
    for handler in [StepIdx::new(2), StepIdx::new(u16::MAX)] {
        let mut invalid = valid.clone();
        invalid.nodes.last_mut().ok_or("terminal")?.routing.error = ErrorRouting::Handler {
            handler,
            error_slot: Some(SlotIdx::ZERO),
        };
        let expected = WorkflowError::StepOutOfBounds { step: handler };
        assert_eq!(validate_node_bounds(&invalid), Err(expected.clone()));
        for concurrent in [false, true] {
            assert_eq!(
                construct(invalid.clone(), concurrent).err(),
                Some(expected.clone())
            );
        }
    }
    Ok(())
}
