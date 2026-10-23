#![forbid(unsafe_code)]

use super::common::{cn, fin, mkr, mkwfc, nop};
use crate::engine::drive::drive_deterministic_full;
use crate::engine::types::{EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeSignal};
use crate::primitives::collect::CollectStates;
use vb_core::capability::CapabilitySet;
use vb_core::engine::StepBudget;
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx};
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledNodeKind;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduce_start_distinct_targets_emits_all_evidence() -> Result<(), String> {
        let reduce = cn(
            0,
            Some(2),
            None,
            CompiledNodeKind::ReduceStart {
                input: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                initial: ConstIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        );
        let finish = fin(2, 1);
        let wf = mkwfc(
            vec![reduce, nop(1, 2), finish],
            3,
            vec![ConstValue::I64(42)],
        )?;

        let mut run = mkr(3, 3)?;
        let mut store = ValueStore::new();
        let items = Box::from([SlotValue::I64(7)]);
        let list_id = store.insert_list(items).map_err(|e| format!("{e}"))?;
        run.write_slot(SlotIdx::new(0), SlotValue::List(list_id))
            .map_err(|e| format!("{e}"))?;

        let mut budget = StepBudget::new(10);
        let mut evidence = EvidenceCollector::new();
        let mut collect_states = CollectStates::new();

        let sig = drive_deterministic_full(
            &wf,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect_states,
            &CapabilitySet::empty(),
            1,
        )
        .map_err(|e| format!("{e}"))?;

        match sig {
            RuntimeSignal::Finished(v) => {
                if v != SlotValue::I64(42) {
                    return Err(format!("expected Finished(42), got {v:?}"));
                }
            }
            other => return Err(format!("expected Finished, got {other:?}")),
        }

        let input = run.read_slot(SlotIdx::new(0)).map_err(|e| format!("{e}"))?;
        if !matches!(input, SlotValue::I64(7)) {
            return Err(format!("slot0 expected I64(7), got {input:?}"));
        }
        let input_taint = run
            .read_taint(SlotIdx::new(0))
            .map_err(|e| format!("{e}"))?;
        if input_taint != Taint::Clean {
            return Err(format!("slot0 taint expected Clean, got {input_taint:?}"));
        }

        let acc = run.read_slot(SlotIdx::new(1)).map_err(|e| format!("{e}"))?;
        if !matches!(acc, SlotValue::I64(42)) {
            return Err(format!("slot1 expected I64(42), got {acc:?}"));
        }
        let acc_taint = run
            .read_taint(SlotIdx::new(1))
            .map_err(|e| format!("{e}"))?;
        if acc_taint != Taint::Clean {
            return Err(format!("slot1 taint expected Clean, got {acc_taint:?}"));
        }

        let iter_slot = run.read_slot(SlotIdx::new(2)).map_err(|e| format!("{e}"))?;
        let iter_taint = run
            .read_taint(SlotIdx::new(2))
            .map_err(|e| format!("{e}"))?;
        if iter_taint != Taint::Clean {
            return Err(format!("slot2 taint expected Clean, got {iter_taint:?}"));
        }
        let SlotValue::List(iter_id) = *iter_slot else {
            return Err(format!("slot2 expected List, got {iter_slot:?}"));
        };
        let iter_list = store.list(iter_id).map_err(|e| format!("{e}"))?;
        let expected = [SlotValue::I64(i64::from(list_id.get())), SlotValue::I64(1)];
        if iter_list != &expected {
            return Err(format!(
                "slot2 list mismatch: got {iter_list:?}, expected {expected:?}"
            ));
        }

        let events = evidence.drain();
        let slot_writes: Vec<_> = events
            .iter()
            .filter_map(|e| match e {
                EvidenceEvent::SlotWritten {
                    slot, value, taint, ..
                } => Some((*slot, *value, *taint)),
                _ => None,
            })
            .collect();

        if slot_writes.len() != 3 {
            return Err(format!(
                "expected 3 SlotWritten events, got {}",
                slot_writes.len()
            ));
        }

        let mut iter = slot_writes.iter();

        match iter.next() {
            Some((s, v, t))
                if *s == SlotIdx::new(1) && *v == SlotValue::I64(42) && *t == Taint::Clean => {}
            other => return Err(format!("acc wrong: {other:?}")),
        }

        match iter.next() {
            Some((s, v, t))
                if *s == SlotIdx::new(0) && *v == SlotValue::I64(7) && *t == Taint::Clean => {}
            other => return Err(format!("input wrong: {other:?}")),
        }

        match iter.next() {
            Some((s, v, t)) if *s == SlotIdx::new(2) && *t == Taint::Clean => match v {
                SlotValue::List(_) => {}
                _ => return Err(format!("iter not List: {v:?}")),
            },
            other => return Err(format!("iter wrong: {other:?}")),
        }

        if iter.next().is_some() {
            return Err("extra SlotWritten events".into());
        }

        Ok(())
    }

    fn assert_reduce_alias(kind: CompiledNodeKind) -> Result<(), String> {
        let reduce = cn(
            0,
            Some(2),
            None,
            CompiledNodeKind::ReduceStart {
                input: SlotIdx::new(0),
                accumulator: SlotIdx::new(1),
                initial: ConstIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        );
        let baseline = mkwfc(
            vec![reduce, nop(1, 2), fin(2, 1)],
            3,
            vec![ConstValue::I64(42)],
        )
        .map_err(|e| format!("{e}"))?;

        let mut parts = baseline.to_parts();
        let node = parts
            .nodes
            .iter_mut()
            .find(|n| n.id == StepIdx::new(0))
            .ok_or("node 0 not found")?;
        node.kind = kind;

        let result = vb_core::workflow::CompiledWorkflow::try_from_parts(parts);
        match result {
            Err(vb_core::WorkflowError::ReduceBranchTargetAlias { step, target })
                if step == StepIdx::new(0) && target == StepIdx::new(1) =>
            {
                Ok(())
            }
            Err(e) => Err(format!("expected ReduceBranchTargetAlias, got {e}")),
            Ok(_) => Err("expected error, got Ok".into()),
        }
    }

    #[test]
    fn reduce_start_body_equals_done_rejected() -> Result<(), String> {
        assert_reduce_alias(CompiledNodeKind::ReduceStart {
            input: SlotIdx::new(0),
            accumulator: SlotIdx::new(1),
            initial: ConstIdx::new(0),
            body: StepIdx::new(1),
            done: StepIdx::new(1),
        })
    }

    #[test]
    fn reduce_next_body_equals_done_rejected() -> Result<(), String> {
        assert_reduce_alias(CompiledNodeKind::ReduceNext {
            iterator_slot: SlotIdx::new(0),
            accumulator: SlotIdx::new(1),
            body: StepIdx::new(1),
            done: StepIdx::new(1),
        })
    }
}
