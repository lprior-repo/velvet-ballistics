use vb_core::budget::{BudgetDimension, WholeWorkflowBudget};
use vb_core::workflow::node;
use vb_core::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstIdx, ConstValue, ResourceContract,
    SlotIdx, StepIdx, WorkflowDigest, WorkflowError, WorkflowParts,
};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

enum Body {
    Leaf,
    Together(Vec<Body>),
}

fn nested(counts: &[usize]) -> Body {
    counts.iter().rev().fold(Body::Leaf, |inner, count| {
        let mut children = vec![inner];
        children.extend((1..*count).map(|_| Body::Leaf));
        Body::Together(children)
    })
}

#[derive(Default)]
struct Fixture {
    nodes: Vec<CompiledNode>,
    slots: u16,
}

impl Fixture {
    fn slot(&mut self) -> TestResult<SlotIdx> {
        let slot = SlotIdx::new(self.slots);
        self.slots = self.slots.checked_add(1).ok_or("slot overflow")?;
        Ok(slot)
    }

    fn push(&mut self, kind: CompiledNodeKind, output: Option<SlotIdx>) -> TestResult<StepIdx> {
        let id = StepIdx::new(u16::try_from(self.nodes.len())?);
        self.nodes.push(node(id, kind, output, None));
        Ok(id)
    }

    fn edit(&mut self, step: StepIdx) -> TestResult<&mut CompiledNode> {
        self.nodes
            .get_mut(step.as_usize())
            .ok_or("missing node".into())
    }

    fn emit(&mut self, body: Body) -> TestResult<(StepIdx, SlotIdx)> {
        let output = self.slot()?;
        let Body::Together(children) = body else {
            let end = self.push(
                CompiledNodeKind::SetConst {
                    value: ConstIdx::new(0),
                },
                Some(output),
            )?;
            return Ok((end, output));
        };
        let count = u16::try_from(children.len())?;
        let start = self.push(CompiledNodeKind::Nop, Some(output))?;
        let mut branches = Vec::new();
        let mut previous = None;
        let mut last_output = None;
        for (index, child) in children.into_iter().enumerate() {
            let branch = self.push(CompiledNodeKind::Nop, last_output)?;
            if let Some(end) = previous {
                self.edit(end)?.routing.next = Some(branch).into();
            }
            let entry = StepIdx::new(u16::try_from(self.nodes.len())?);
            self.edit(branch)?.kind = CompiledNodeKind::TogetherBranch {
                branch: u16::try_from(index)?,
                entry,
                join: start,
                accumulator: output,
            };
            branches.push(branch);
            let (end, result) = self.emit(child)?;
            previous = Some(end);
            last_output = Some(result);
        }
        let result = last_output.ok_or("empty Together fixture")?;
        let join = self.push(
            CompiledNodeKind::TogetherJoin {
                branch_count: count,
                accumulator: output,
            },
            Some(result),
        )?;
        self.edit(previous.ok_or("missing branch end")?)?
            .routing
            .next = Some(join).into();
        for branch in &branches {
            if let CompiledNodeKind::TogetherBranch { join: target, .. } =
                &mut self.edit(*branch)?.kind
            {
                *target = join;
            }
        }
        self.edit(start)?.kind = CompiledNodeKind::TogetherStart {
            branches: branches.into_boxed_slice(),
            join,
        };
        Ok((join, result))
    }
}

fn parts(scopes: Vec<Body>) -> TestResult<WorkflowParts> {
    let mut fixture = Fixture::default();
    let mut previous = None;
    let mut result = None;
    for scope in scopes {
        let entry = StepIdx::new(u16::try_from(fixture.nodes.len())?);
        if let Some(end) = previous {
            fixture.edit(end)?.routing.next = Some(entry).into();
        }
        let (end, output) = fixture.emit(scope)?;
        previous = Some(end);
        result = Some(output);
    }
    let end = fixture.push(
        CompiledNodeKind::Finish {
            result: result.ok_or("empty fixture")?,
        },
        None,
    )?;
    fixture.edit(previous.ok_or("missing scope")?)?.routing.next = Some(end).into();
    Ok(WorkflowParts {
        name: "together-active-budget".into(),
        digest: WorkflowDigest::from_bytes([0; 32]),
        semantic_source_identity: None,
        source_metadata: None,
        input_slots: Box::new([]),
        nodes: fixture.nodes.into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: Box::new([ConstValue::I64(7)]),
        slot_count: fixture.slots,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::new([]),
    })
}

fn check(scopes: Vec<Body>, parallel: u16, fanout: u16) -> TestResult {
    let parts = parts(scopes)?;
    let budget = WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)?;
    assert_eq!(budget.max_parallel_in_flight, parallel);
    assert_eq!(budget.max_fanout, fanout);
    assert_eq!(budget.max_together_branches, fanout);
    let workflow = CompiledWorkflow::try_from_parts(parts.clone())?;
    assert_eq!(workflow.resource_contract(), ResourceContract::DEFAULT);
    assert_eq!(
        vb_core::budget::compute_max_parallel_in_flight(
            (0..workflow.node_count()).filter_map(|index| workflow.node(StepIdx::new(index))),
        )?,
        parallel,
    );
    assert_eq!(workflow.to_parts(), parts);
    Ok(())
}

#[test]
fn nested_together_reserves_outer_and_inner_branches() -> TestResult {
    check(vec![nested(&[2, 2])], 4, 2)?;
    check(vec![nested(&[2, 3, 4])], 9, 4)
}

#[test]
fn sequential_together_scopes_reuse_parallel_capacity() -> TestResult {
    check(vec![nested(&[2, 3]), nested(&[4, 1])], 5, 4)
}

#[test]
fn sibling_nested_scopes_share_only_the_outer_reservation() -> TestResult {
    check(vec![Body::Together(vec![nested(&[3]), nested(&[4])])], 6, 4)
}

#[test]
fn nested_parallel_policy_boundary_stays_enforced() -> TestResult {
    check(vec![nested(&[64, 64, 64, 64])], 256, 64)?;
    let parts = parts(vec![nested(&[64, 64, 64, 64, 64])])?;
    let budget = WholeWorkflowBudget::compute(&parts.nodes, parts.entry, &parts.resource_contract)?;
    assert_eq!(budget.max_parallel_in_flight, 320);
    assert_eq!(
        CompiledWorkflow::try_from_parts(parts),
        Err(WorkflowError::BudgetPolicyExceeded {
            detail: BudgetDimension::Parallel,
        })
    );
    Ok(())
}
