#![forbid(unsafe_code)]

use vb_compile::compile_workflow;
use vb_core::workflow::{CompiledNode, CompiledNodeKind as K, ExprOp};
use vb_core::{
    CapabilitySet, CompiledWorkflow, InputSlotKind, ListId, RunFrame, RunId, SlotIdx, SlotValue,
    StepBudget, Taint, ValueStore, new_run_frame,
};
use vb_core::{ConstValue, StepIdx};
use vb_runtime::engine::{
    EvidenceCollector, RetryPolicy, RuntimeEngineError, RuntimeSignal, drive_deterministic_full,
};
use vb_runtime::primitives::collect::CollectStates;

pub(super) const BODY: &str = "        - id: copy_first\n          set:\n            output: copied\n            value: '$entry'\n        - id: copy_second\n          set:\n            output: echoed\n            value: '$copied'\n";

pub(super) const FINAL: &str = "  - id: done\n    finish:\n      result: anchor\n";

pub(super) fn source(
    schema: &str,
    at_once: Option<u32>,
    variable: &str,
    body: &str,
    tail: &str,
) -> String {
    let at_once_line = match at_once {
        Some(n) => format!("      at_once: {n}\n"),
        None => String::new(),
    };
    format!(
        "version: velvet-ballistics/v1\nname: f1_bounded_foreach\nwhen:\n  manual: {{}}\ninputs:\n  entries: {schema}\nsteps:\n  - id: seed\n    set:\n      output: anchor\n      value: '42'\n  - id: bounded\n    for_each:\n      input: '$input.entries'\n      variable: {variable}\n{at_once_line}      steps:\n{body}{tail}"
    )
}

pub(super) fn compile(source: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(source.as_bytes())
        .map_err(|error| format!("workflow compilation failed: {error:?}"))
}

pub(super) struct SourceRun {
    pub(super) workflow: CompiledWorkflow,
    pub(super) frame: RunFrame,
    pub(super) store: ValueStore,
    pub(super) evidence: EvidenceCollector,
    pub(super) collect_states: CollectStates,
    pub(super) input_slot: SlotIdx,
    pub(super) input_list: ListId,
}

impl SourceRun {
    pub(super) fn new(source: &str, values: &[SlotValue], taint: Taint) -> Result<Self, String> {
        let workflow = compile(source)?;
        let inputs = workflow.input_slots();
        if inputs.len() != 1 {
            return Err(format!("expected 1 input, found {}", inputs.len()));
        }
        let input = inputs.first().ok_or("missing declared input")?;
        if input.kind != InputSlotKind::List {
            return Err(format!("expected list input, found {:?}", input.kind));
        }
        let input_slot = input.slot;
        let mut frame = new_run_frame(RunId::new(1), &workflow)
            .map_err(|error| format!("frame creation failed: {error:?}"))?;
        let mut store = ValueStore::new();
        let input_list = store
            .insert_list(values.to_vec().into_boxed_slice())
            .map_err(|error| format!("list insertion failed: {error:?}"))?;
        frame
            .write_slot_with_taint(input_slot, SlotValue::List(input_list), taint)
            .map_err(|error| format!("input slot write failed: {error:?}"))?;
        Ok(Self {
            workflow,
            frame,
            store,
            evidence: EvidenceCollector::new(),
            collect_states: CollectStates::new(),
            input_slot,
            input_list,
        })
    }

    pub(super) fn drive(&mut self, steps: u64) -> Result<RuntimeSignal, RuntimeEngineError> {
        drive_deterministic_full(
            &self.workflow,
            &mut self.frame,
            &mut StepBudget::new(steps),
            &mut self.store,
            &[],
            RetryPolicy::NEVER,
            &mut self.evidence,
            &mut self.collect_states,
            &CapabilitySet::empty(),
            1,
        )
    }

    pub(super) fn assert_input(&self, values: &[SlotValue], taint: Taint) -> Result<(), String> {
        let slot_value = self
            .frame
            .read_slot(self.input_slot)
            .map_err(|error| format!("input slot read failed: {error:?}"))?;
        match slot_value {
            SlotValue::List(id) if *id == self.input_list => {}
            other => {
                return Err(format!(
                    "input slot changed: expected List({:?}), found {:?}",
                    self.input_list, other
                ));
            }
        }
        let slot_taint = self
            .frame
            .read_taint(self.input_slot)
            .map_err(|error| format!("input taint read failed: {error:?}"))?;
        if slot_taint != taint {
            return Err(format!(
                "input taint changed: expected {:?}, found {:?}",
                taint, slot_taint
            ));
        }
        let stored = self
            .store
            .list(self.input_list)
            .map_err(|error| format!("list read failed: {error:?}"))?;
        if stored.len() != values.len() {
            return Err(format!(
                "list length changed: expected {}, found {}",
                values.len(),
                stored.len()
            ));
        }
        for (i, (expected, actual)) in values.iter().zip(stored.iter()).enumerate() {
            if expected != actual {
                return Err(format!("list element {} changed", i));
            }
        }
        let entries = self
            .store
            .list_items_with_taint(self.input_list)
            .map_err(|error| format!("input metadata read failed: {error:?}"))?
            .collect::<Vec<_>>();
        let expected = values
            .iter()
            .map(|value| (*value, Taint::Clean))
            .collect::<Vec<_>>();
        assert_eq!(entries, expected, "source item metadata changed");
        Ok(())
    }

    pub(super) fn output(&self, step: StepIdx) -> Result<SlotIdx, String> {
        let node = self
            .workflow
            .node(step)
            .ok_or_else(|| format!("step {step:?} not found"))?;
        node.output
            .slot()
            .ok_or_else(|| format!("step {step:?} has no output slot"))
    }
}

pub(super) struct LoopLayout {
    pub(super) before_start: [StepIdx; 3],
    pub(super) start: StepIdx,
    pub(super) body: [StepIdx; 4],
    pub(super) after: [StepIdx; 2],
    pub(super) anchor: SlotIdx,
    pub(super) accumulator: SlotIdx,
    pub(super) item: SlotIdx,
    pub(super) cursor: SlotIdx,
    pub(super) first: SlotIdx,
    pub(super) second: SlotIdx,
    pub(super) output: SlotIdx,
}

impl SourceRun {
    fn node(&self, step: StepIdx) -> Result<&CompiledNode, String> {
        self.workflow
            .node(step)
            .ok_or_else(|| format!("missing node {step:?}"))
    }

    fn next(&self, node: &CompiledNode) -> Result<&CompiledNode, String> {
        self.node(
            node.routing
                .next
                .step()
                .ok_or_else(|| format!("missing next at {:?}", node.id))?,
        )
    }

    fn assert_load(&self, node: &CompiledNode, source: SlotIdx) -> Result<(), String> {
        let K::EvalExpr { expr } = &node.kind else {
            return Err("body set must evaluate its slot reference".into());
        };
        let program = self
            .workflow
            .expression(*expr)
            .ok_or("missing body expression")?;
        assert_eq!(program.ops.as_ref(), &[ExprOp::LoadSlot(source)]);
        Ok(())
    }

    pub(super) fn layout(&self, max: u32) -> Result<LoopLayout, String> {
        let seed = self.node(self.workflow.entry())?;
        let K::SetConst { value } = &seed.kind else {
            return Err("seed must be SetConst".into());
        };
        assert_eq!(self.workflow.constant(*value), Some(&ConstValue::I64(42)));
        let anchor = self.output(seed.id)?;
        let init = self.next(seed)?;
        assert_eq!(
            init.kind,
            K::BuildList {
                items: Box::from([])
            }
        );
        let accumulator = self.output(init.id)?;
        let bridge = self.next(init)?;
        let start = self.next(bridge)?;
        let K::ForEachStart {
            input,
            item_slot,
            limit,
            body,
            done,
        } = &start.kind
        else {
            return Err("missing ForEachStart after root bridge".into());
        };
        assert_eq!(*input, self.input_slot);
        assert_eq!(*limit, max.into());
        let item = *item_slot;
        let cursor = self.output(start.id)?;
        let K::EvalExpr { expr } = &bridge.kind else {
            return Err("missing source root bridge".into());
        };
        let bridge_ops = self
            .workflow
            .expression(*expr)
            .ok_or("missing bridge expression")?;
        let [
            ExprOp::LoadSlot(left),
            ExprOp::LoadSlot(source),
            ExprOp::LoadConst(flag),
            ExprOp::AppendIf,
        ] = bridge_ops.ops.as_ref()
        else {
            return Err("wrong source root bridge".into());
        };
        assert_eq!((*left, *source), (accumulator, self.input_slot));
        assert_eq!(
            self.workflow.constant(*flag),
            Some(&ConstValue::Bool(false))
        );
        assert_eq!(self.output(bridge.id)?, accumulator);
        let first = self.node(*body)?;
        self.assert_load(first, item)?;
        let first_slot = self.output(first.id)?;
        let second = self.next(first)?;
        self.assert_load(second, first_slot)?;
        let second_slot = self.output(second.id)?;
        let append = self.next(second)?;
        let K::EvalExpr { expr } = &append.kind else {
            return Err("missing body result append".into());
        };
        let append_ops = self
            .workflow
            .expression(*expr)
            .ok_or("missing append expression")?;
        assert_eq!(
            append_ops.ops.as_ref(),
            &[
                ExprOp::LoadSlot(accumulator),
                ExprOp::LoadSlot(second_slot),
                ExprOp::Append
            ]
        );
        assert_eq!(self.output(append.id)?, accumulator);
        let advance = self.next(append)?;
        assert_eq!(
            advance.kind,
            K::ForEachNext {
                iterator_slot: cursor,
                body: *body,
                done: *done
            }
        );
        assert_eq!(self.output(advance.id)?, item);
        let join = self.node(*done)?;
        assert_eq!(
            join.kind,
            K::ForEachJoin {
                output: accumulator
            }
        );
        let output = self.output(join.id)?;
        let finish = self.next(join)?;
        assert_eq!(finish.kind, K::Finish { result: anchor });
        assert_eq!(finish.routing.next.step(), None);
        let layout = LoopLayout {
            before_start: [seed.id, init.id, bridge.id],
            start: start.id,
            body: [first.id, second.id, append.id, advance.id],
            after: [join.id, finish.id],
            anchor,
            accumulator,
            item,
            cursor,
            first: first_slot,
            second: second_slot,
            output,
        };
        let steps = layout
            .before_start
            .into_iter()
            .chain([layout.start])
            .chain(layout.body)
            .chain(layout.after)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(steps.len(), 10);
        assert_eq!(usize::from(self.workflow.node_count()), steps.len());
        Ok(layout)
    }
}
