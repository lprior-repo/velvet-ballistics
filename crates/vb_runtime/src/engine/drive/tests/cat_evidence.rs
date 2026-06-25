#![forbid(unsafe_code)]

//! cat6 evidence emission tests + collect-pagination evidence test.

use super::common::{collect_start, dde, fin, mkwf, mkr, setc, ws};
use crate::engine::drive::drive_deterministic_full;
use crate::engine::types::{
    EvidenceCollector, EvidenceEvent, RetryPolicy,
};
use crate::primitives::collect::CollectStates;
use vb_core::capability::CapabilitySet;
use vb_core::engine::StepBudget;
use vb_core::ids::SlotIdx;
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;

#[cfg(test)]
mod tests {
        #[test]
    fn cat6_evidence_step_events() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), fin(1, 0)], 1)?;
        let mut r = mkr(2, 1)?;
        ws(&mut r, 0, SlotValue::I64(0))?;
        let mut b = StepBudget::new(10);
        let mut ev = EvidenceCollector::new();
        dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty())?;
        let events = ev.drain();
        let started = events
            .iter()
            .filter(|e| matches!(e, EvidenceEvent::StepStarted { .. }))
            .count();
        if started < 2 {
            return Err(format!("expected >= 2 StepStarted, got {started}"));
        }
        Ok(())
    }

    #[test]
        #[test]
    fn cat6_evidence_slot_written() -> Result<(), String> {
        let wf = mkwfc(
            vec![setc(0, 0, 0, 1), fin(1, 0)],
            1,
            vec![ConstValue::I64(7)],
        )?;
        let mut r = mkr(2, 1)?;
        let mut b = StepBudget::new(10);
        let mut ev = EvidenceCollector::new();
        dde(&wf, &mut r, &mut b, &mut ev, &CapabilitySet::empty())?;
        let events = ev.drain();
        let writes: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, EvidenceEvent::SlotWritten { .. }))
            .collect();
        if writes.is_empty() {
            return Err("expected at least one SlotWritten".into());
        }
        Ok(())
    }

    #[test]
        #[test]
    fn collect_pagination_extra_single_authoritative_evidence_write() -> Result<(), String> {
        let wf = mkwf(vec![collect_start(0, 0, 1, 1, 2), fin(1, 1), fin(2, 1)], 2)?;
        let mut run = mkr(3, 2)?;
        let mut store = ValueStore::new();
        let page = Box::from([SlotValue::I64(10), SlotValue::I64(20)]);
        let list_id = store.insert_list(page).map_err(|e| format!("{e}"))?;
        run.write_slot(SlotIdx::new(0), SlotValue::List(list_id))
            .map_err(|e| format!("{e}"))?;
        let mut budget = StepBudget::new(10);
        let mut evidence = EvidenceCollector::new();
        let mut collect_states = CollectStates::new();

        drive_deterministic_full(
            &wf,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut evidence,
            &mut collect_states,
            &CapabilitySet::empty(),
        )
        .map_err(|e| format!("{e}"))?;

        let events = evidence.drain();
        let matching_writes = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    EvidenceEvent::SlotWritten {
                        slot,
                        ..
                    } if *slot == SlotIdx::new(1)
                )
            })
            .count();
        let extra_bearing_writes = events
            .iter()
            .filter(|event| {
                matches!(
                    event,
                    EvidenceEvent::SlotWritten {
                        slot,
                        extra: Some(_),
                        ..
                    } if *slot == SlotIdx::new(1)
                )
            })
            .count();
        assert_eq!(matching_writes, 1);
        assert_eq!(extra_bearing_writes, 1);
        Ok(())
    }

    #[test]
        #[test]
    fn cat7_multi_step_chain() -> Result<(), String> {
        let wf = mkwf(vec![nop(0, 1), nop(1, 2), nop(2, 3), fin(3, 0)], 1)?;
        let mut r = mkr(4, 1)?;
        ws(&mut r, 0, SlotValue::Bool(false))?;
        let mut b = StepBudget::new(10);
        let sig = dd(&wf, &mut r, &mut b)?;
        match sig {
            RuntimeSignal::Finished(v) => assert_eq!(v, SlotValue::Bool(false)),
            _ => return Err("expected Finished".into()),
        }
        Ok(())
    }

    #[test]
    }
