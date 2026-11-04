use vb_compile::{CompileError, CompileErrors};
use vb_core::budget::BudgetDimension;
use vb_core::workflow::ContractResource;
use vb_core::{ConstIdx, CoreError, HasSymbolicCode, SlotIdx, StepIdx, SymbolId, WorkflowError};

fn workflow_cases() -> [(WorkflowError, &'static str); 20] {
    [
        (WorkflowError::EmptyNodes, "INVALID_COMPILED_WORKFLOW"),
        (
            WorkflowError::ResourceContractExceeded {
                resource: ContractResource::Slots,
            },
            "LIMIT_EXCEEDED",
        ),
        (
            WorkflowError::EntryOutOfBounds {
                entry: StepIdx::new(4),
            },
            "INVALID_COMPILED_WORKFLOW",
        ),
        (
            WorkflowError::ResourceContractTooLarge {
                resource: ContractResource::Steps,
            },
            "LIMIT_EXCEEDED",
        ),
        (
            WorkflowError::NodeIdMismatch {
                expected: StepIdx::new(0),
                actual: StepIdx::new(1),
            },
            "INVALID_COMPILED_WORKFLOW",
        ),
        (
            WorkflowError::BudgetPolicyExceeded {
                detail: BudgetDimension::TotalSteps,
            },
            "LIMIT_EXCEEDED",
        ),
        (WorkflowError::EmptyBranchTable, "INVALID_COMPILED_WORKFLOW"),
        (
            WorkflowError::DepthOverflow { depth: u16::MAX },
            "LIMIT_EXCEEDED",
        ),
        (
            WorkflowError::UnreachableNode {
                step: StepIdx::new(3),
            },
            "INVALID_COMPILED_WORKFLOW",
        ),
        (
            WorkflowError::StepOutOfBounds {
                step: StepIdx::new(5),
            },
            "INVALID_THEN_TARGET",
        ),
        (
            WorkflowError::BackwardEdge {
                from: StepIdx::new(2),
                to: StepIdx::new(1),
            },
            "INVALID_COMPILED_WORKFLOW",
        ),
        (
            WorkflowError::SlotOutOfBounds {
                slot: SlotIdx::new(5),
            },
            "TYPE_MISMATCH",
        ),
        (
            WorkflowError::ImproperLoopNesting {
                inner: StepIdx::new(2),
                outer_done: StepIdx::new(1),
            },
            "INVALID_COMPILED_WORKFLOW",
        ),
        (
            WorkflowError::ConstOutOfBounds {
                constant: ConstIdx::new(5),
            },
            "CONST_OUT_OF_BOUNDS",
        ),
        (
            WorkflowError::SymbolOutOfBounds {
                symbol: SymbolId::new(2),
            },
            "INVALID_COMPILED_WORKFLOW",
        ),
        (
            WorkflowError::Expression(CoreError::DivisionByZero),
            "INVALID_EXPRESSION",
        ),
        (
            WorkflowError::AccessorPathTooDeep { depth: 9, max: 8 },
            "INVALID_COMPILED_WORKFLOW",
        ),
        (
            WorkflowError::StepCountOverflow { actual: u64::MAX },
            "INVALID_COMPILED_WORKFLOW",
        ),
        (
            WorkflowError::JumpCycle {
                step: StepIdx::new(2),
                target: StepIdx::new(1),
            },
            "INVALID_COMPILED_WORKFLOW",
        ),
        (
            WorkflowError::RepeatCounterForeignWrite {
                owner: StepIdx::new(0),
                writer: StepIdx::new(1),
                slot: SlotIdx::new(2),
            },
            "INVALID_COMPILED_WORKFLOW",
        ),
    ]
}

#[test]
fn workflow_diagnostics_preserve_all_public_code_surfaces() {
    for (workflow, expected) in workflow_cases() {
        let error = CompileError::Workflow(workflow);
        assert_eq!(error.code().as_str(), expected, "code for {error:?}");
        assert_eq!(
            error.diagnostic_code().as_str(),
            expected,
            "diagnostic_code for {error:?}"
        );
        assert_eq!(
            HasSymbolicCode::symbolic_code(&error).as_str(),
            expected,
            "symbolic_code for {error:?}"
        );
    }
}

#[test]
fn workflow_diagnostic_collection_preserves_complete_reporting_order() {
    let (errors, expected): (Vec<_>, Vec<_>) = workflow_cases()
        .into_iter()
        .map(|(workflow, expected)| (CompileError::Workflow(workflow), expected))
        .unzip();
    let errors = CompileErrors(errors);
    let actual: Vec<_> = errors
        .diagnostic_codes()
        .map(|code| code.as_str())
        .collect();
    assert_eq!(actual, expected);
}
