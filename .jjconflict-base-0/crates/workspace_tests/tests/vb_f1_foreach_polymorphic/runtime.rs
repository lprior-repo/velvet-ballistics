use std::{num::NonZeroUsize, sync::Arc};
use vb_compile::compile_workflow;
use vb_core::action::{
    ActionContract, ActionName, ActionOutputReady, Idempotency, RetrySafety, SideEffect,
};
use vb_core::{
    ActionId, CapabilitySet, CompiledNodeKind as K, RunId, SlotIdx, SlotValue as V, Taint,
};
use vb_runtime::{
    journal::{RuntimeJournalEvent as E, VolatileRuntimeJournal},
    runtime::Runtime,
    shard::ShardConfig,
};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn source(expression: &str) -> String {
    format!(
        r#"version: velvet-ballistics/v1
name: polymorphic_foreach
when: {{manual: {{}}}}
vars: {{empty_list: []}}
steps:
  - id: one
    do: {{action: '0', input: '$vars.empty_list'}}
  - id: two
    do: {{action: '0', input: '$vars.empty_list'}}
  - id: nil
    do: {{action: '0', input: '$vars.empty_list'}}
  - id: assembled
    set: {{output: assembled, value: 'append(append(append($vars.empty_list, $steps.one), $steps.two), $steps.nil)'}}
  - id: fetch
    do: {{action: '0', input: '$vars.empty_list'}}
  - id: mapped
    for_each:
      variable: entry
      input: '$steps.fetch'
      steps:
        - id: compared
          set: {{output: compared, value: '{expression}'}}
  - id: observe
    do: {{action: '0', input: '$vars.empty_list'}}
  - id: done
    finish: {{result: '$steps.mapped'}}
"#
    )
}

struct Fixture {
    runtime: Runtime,
    journal: Arc<VolatileRuntimeJournal>,
    actions: Vec<(vb_core::StepIdx, SlotIdx, SlotIdx)>,
    result: SlotIdx,
    seed: SlotIdx,
}

impl Fixture {
    fn new(expression: &str) -> TestResult<Self> {
        let workflow = compile_workflow(source(expression).as_bytes())?;
        let parts = workflow.to_parts();
        let actions = parts
            .nodes
            .iter()
            .filter_map(|node| match node.kind {
                K::Do { input, .. } => Some(
                    node.output
                        .slot()
                        .map(|out| (node.id, input, out))
                        .ok_or("missing Do output"),
                ),
                _ => None,
            })
            .collect::<Result<Vec<_>, _>>()?;
        assert_eq!(actions.len(), 5);
        let result = parts
            .nodes
            .iter()
            .find_map(|node| match node.kind {
                K::Finish { result } => Some(result),
                _ => None,
            })
            .ok_or("missing Finish")?;
        let seed = parts
            .nodes
            .iter()
            .find_map(|node| match node.kind {
                K::EvalExpr { .. } => node.output.slot(),
                _ => None,
            })
            .ok_or("missing assembled list output")?;
        let journal = Arc::new(VolatileRuntimeJournal::new());
        let mut runtime = Runtime::new(
            NonZeroUsize::MIN,
            ShardConfig {
                policy: vb_core::RuntimePolicy::Relaxed,
                ..ShardConfig::default()
            },
            journal.clone(),
        );
        runtime.submit_direct_with_grants_and_contracts(
            RunId::new(1),
            workflow,
            CapabilitySet::empty(),
            Box::from([ActionContract {
                id: ActionId::new(0),
                name: ActionName::new("list-reply")?,
                input_slot_count: 1,
                output_slot_count: 1,
                max_input_bytes: 1024,
                max_output_bytes: 1024,
                timeout_ms: 1000,
                idempotency: Idempotency::DeterministicPure,
                side_effect: SideEffect::None,
                retry_safety: RetrySafety::Safe,
                required_capabilities: Box::new([]),
            }]),
        )?;
        runtime.tick_all()?;
        Ok(Self {
            runtime,
            journal,
            actions,
            result,
            seed,
        })
    }

    fn complete(&mut self, index: usize, value: V, taint: Taint) -> TestResult {
        self.enqueue(index, value, taint)?;
        self.runtime.tick_all()?;
        Ok(())
    }

    fn enqueue(&mut self, index: usize, value: V, taint: Taint) -> TestResult {
        let (step, _, output) = *self.actions.get(index).ok_or("missing action")?;
        let ticket = self
            .runtime
            .lookup_pending_action_ticket(RunId::new(1), u64::from(step.get()))
            .ok_or("missing issued ticket")?;
        assert_eq!(ticket.step, step);
        assert_eq!(ticket.action, ActionId::new(0));
        self.runtime.complete_action_with_output(
            ticket,
            ActionOutputReady {
                output_slot: output,
                value,
                taint,
                encoded_len: u32::try_from(postcard::to_allocvec(&value)?.len())?,
            },
        )?;
        Ok(())
    }

    fn seed(&mut self, taints: [Taint; 3]) -> TestResult<V> {
        for (index, (value, taint)) in [V::I64(1), V::I64(2), V::Null]
            .into_iter()
            .zip(taints)
            .enumerate()
        {
            self.complete(index, value, taint)?;
        }
        let state = self
            .runtime
            .inspect_run_state(RunId::new(1))
            .map_err(|error| format!("seed state: {error:?}"))?;
        let value = *state.frame.read_slot(self.seed)?;
        let V::List(list) = value else {
            return Err("assembled reply is not List".into());
        };
        assert_eq!(
            state.store.list_items_with_taint(list)?.collect::<Vec<_>>(),
            [V::I64(1), V::I64(2), V::Null]
                .into_iter()
                .zip(taints)
                .collect::<Vec<_>>()
        );
        Ok(value)
    }
}

pub(super) fn verify(
    expression: &str,
    expected: [V; 3],
    root: Taint,
    elements: [Taint; 3],
    output_taints: [Taint; 3],
) -> TestResult {
    let mut fixture = Fixture::new(expression)?;
    let reply = fixture.seed(elements)?;
    fixture.complete(3, reply, root)?;
    let (observe, _, _) = *fixture.actions.get(4).ok_or("missing observation action")?;
    assert_eq!(
        fixture
            .runtime
            .lookup_pending_action_ticket(RunId::new(1), u64::from(observe.get()))
            .ok_or("missing observation ticket")?
            .step,
        observe
    );
    let state = fixture
        .runtime
        .inspect_run_state(RunId::new(1))
        .map_err(|error| {
            format!(
                "mapped root={root:?} elements={elements:?}: {error:?}; journal: {:?}",
                fixture.journal.snapshot()
            )
        })?;
    let value = *state.frame.read_slot(fixture.result)?;
    let V::List(list) = value else {
        return Err("ForEach result is not List".into());
    };
    assert_eq!(
        state.store.list_items_with_taint(list)?.collect::<Vec<_>>(),
        expected.into_iter().zip(output_taints).collect::<Vec<_>>()
    );
    let expected_root = elements.into_iter().fold(root, vb_core::value::join_taint);
    assert_eq!(state.frame.read_taint(fixture.result)?, expected_root);
    fixture.complete(4, V::Null, Taint::Clean)?;
    let events = fixture.journal.snapshot()?;
    assert_eq!(
        events
            .iter()
            .filter_map(|event| match event {
                E::RunFinished { run, result, .. } => Some((*run, *result)),
                _ => None,
            })
            .collect::<Vec<_>>(),
        vec![(RunId::new(1), fixture.result)]
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::ActionCompletedEnvelope { .. }))
            .count(),
        5
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, E::RunFailed { .. }))
            .count(),
        0
    );
    Ok(())
}

#[path = "non_list.rs"]
mod non_list;
pub(super) use non_list::check as non_list;
