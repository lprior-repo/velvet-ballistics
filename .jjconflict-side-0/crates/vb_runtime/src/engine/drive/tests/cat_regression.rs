#![forbid(unsafe_code)]

//! RE-004 / RE-011 regression tests for evidence gaps and capacity
//! overflow in the drive loop.

use super::common::{collect_start, fin, mkr, mkwf, mkwfc, setc, ws};
use crate::engine::drive::{DriveState, drive_deterministic_full, emit_slot_evidence};
use crate::engine::types::{EvidenceCollector, RetryPolicy, RuntimeEngineError};
use crate::primitives::collect::{CollectPaginationState, CollectStates};
use vb_core::capability::CapabilitySet;
use vb_core::engine::StepBudget;
use vb_core::errors::EngineError;
use vb_core::frame::StepState;
use vb_core::ids::{SlotIdx, StepIdx};
use vb_core::value::{ConstValue, SlotValue};
use vb_core::value_store::ValueStore;
use vb_core::workflow::{CompiledNode, CompiledNodeKind};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn re_011_evidence_capacity_overflow_does_not_mark_step_succeeded() -> Result<(), String> {
        // CollectStart at step 0 forces emit_slot_evidence down the
        // collect branch (push_slot_written_with_extra) instead of the
        // silent-drop push_slot_written_with_taint branch that the
        // previous SetConst+Finish workflow used.
        let wf = mkwf(vec![collect_start(0, 0, 1, 1, 2), fin(1, 1), fin(2, 1)], 2)?;
        let mut run = mkr(3, 2)?;
        let mut store = ValueStore::new();
        let source_page: Box<[SlotValue]> = Box::from([SlotValue::I64(10), SlotValue::I64(20)]);
        let source_list_id = store.insert_list(source_page).map_err(|e| format!("{e}"))?;
        run.write_slot(SlotIdx::new(0), SlotValue::List(source_list_id))
            .map_err(|e| format!("{e}"))?;
        let mut budget = StepBudget::new(10);
        // Capacity 0: push_slot_written_with_extra must surface
        // CollectEvidenceCapacityExceeded instead of silently dropping.
        let mut ev = EvidenceCollector::with_capacity(1);
        let mut cs = CollectStates::new();
        // Pre-populate CollectStates so capture_state returns Some(_)
        // for (run_id, slot 1). Even though collect_start will upsert a
        // new state during execution, the resulting state still satisfies
        // capture_state, so emit_slot_evidence takes the extra path.
        cs.upsert(CollectPaginationState {
            run_id: run.run_id(),
            collector_slot: SlotIdx::new(1),
            source: source_list_id,
            current_page: source_list_id,
            cursor: 0,
            page_size: 1,
            item_count: 2,
            limit: 100,
            time_limit_ms: None,
            start_millis: 0,
        })
        .map_err(|e| format!("{e}"))?;

        let result = drive_deterministic_full(
            &wf,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut ev,
            &mut cs,
            &CapabilitySet::empty(),
        );

        // RE-011 assertion 1: drive must surface the typed
        // CollectEvidenceCapacityExceeded error rather than completing
        // successfully and silently dropping the evidence event.
        let err = match result {
            Ok(sig) => {
                return Err(format!(
                    "RE-011: drive returned Ok({sig:?}); expected Err(CollectEvidenceCapacityExceeded)"
                ));
            }
            Err(e) => e,
        };
        match err {
            RuntimeEngineError::Core(EngineError::CollectEvidenceCapacityExceeded {
                run_id,
                slot,
                capacity,
                ..
            }) => {
                if run_id != run.run_id() {
                    return Err(format!(
                        "RE-011: capacity error carries wrong run_id: {run_id:?}"
                    ));
                }
                if slot != SlotIdx::new(1) {
                    return Err(format!(
                        "RE-011: capacity error carries wrong slot: {slot:?}"
                    ));
                }
                if capacity != 1 {
                    return Err(format!(
                        "RE-011: capacity error carries wrong capacity: {capacity}"
                    ));
                }
            }
            other => {
                return Err(format!("RE-011: unexpected drive error variant: {other:?}"));
            }
        }

        // RE-011 assertion 2: step 0 must NOT be Succeeded. The fix
        // swaps emit_slot_evidence ahead of mark_step_after_signal so
        // the capacity error short-circuits before the state
        // transition is committed. Step 0 stays in its pre-success
        // Running state (set by begin_drive_step).
        let step_state = run
            .step_state(StepIdx::new(0))
            .map_err(|e| format!("{e}"))?;
        match step_state {
            StepState::Succeeded => Err(format!(
                "RE-011: step 0 marked Succeeded despite capacity overflow (half-committed state)"
            )),
            StepState::Running => Ok(()),
            other => Err(format!(
                "RE-011: step 0 in unexpected state {other:?}; expected Running (pre-success, fail-closed)"
            )),
        }
    }

    /// RE-004 regression: when `emit_slot_evidence` cannot read the
    /// slot it would emit evidence for, the function must surface the
    /// failure via `record_evidence_gap` on the drive state and must
    /// NOT push any evidence event. The previous `if let Ok(value) =
    /// run.read_slot(slot)` pattern silently swallowed both the
    /// evidence and the underlying read error.
    #[test]
    fn re_004_emit_slot_evidence_records_gap_on_read_slot_error() -> Result<(), String> {
        // A RunFrame with slot_count=2 but no writes leaves both slots
        // uninitialized, so read_slot(SlotIdx::new(0)) returns
        // SlotUninitialized. That is the "read_slot error" the fix
        // surfaces.
        let run = mkr(1, 2)?;
        let mut evidence = EvidenceCollector::new();
        let collect_states = CollectStates::new();
        let mut drive_state = DriveState::new();

        // CollectStart with output = Some(slot 0) routes through the
        // `collect_written_slot` branch, which is the first place the
        // pre-fix code silently dropped a SlotWritten event.
        let node = collect_start(0, 0, 0, 0, 0);

        let result = emit_slot_evidence(
            &run,
            &mut evidence,
            &collect_states,
            &node,
            &mut drive_state,
        );
        if let Err(e) = result {
            return Err(format!(
                "RE-004: emit_slot_evidence must NOT propagate read_slot errors as gaps; got {e:?}"
            ));
        }

        if drive_state.evidence_gaps() != 1 {
            return Err(format!(
                "RE-004: record_evidence_gap must increment to 1; got {}",
                drive_state.evidence_gaps()
            ));
        }
        if !evidence.is_empty() {
            let events = evidence.drain();
            return Err(format!(
                "RE-004: no evidence must be emitted on a gap; got {events:?}"
            ));
        }
        Ok(())
    }

    /// RE-004 regression (output branch): nodes that fall through to
    /// the `node.output` branch must also surface read_slot errors via
    /// `record_evidence_gap` and must not emit evidence.
    #[test]
    fn re_004_emit_slot_evidence_records_gap_on_output_read_slot_error() -> Result<(), String> {
        let run = mkr(1, 2)?;
        let mut evidence = EvidenceCollector::new();
        let collect_states = CollectStates::new();
        let mut drive_state = DriveState::new();

        // setc has output = Some(slot 0) and a non-collect kind, so
        // `collect_written_slot` returns None and the function takes
        // the `node.output` branch.
        let node = setc(0, 0, 0, 0);

        let result = emit_slot_evidence(
            &run,
            &mut evidence,
            &collect_states,
            &node,
            &mut drive_state,
        );
        if let Err(e) = result {
            return Err(format!(
                "RE-004: emit_slot_evidence must NOT propagate read_slot errors as gaps; got {e:?}"
            ));
        }
        if drive_state.evidence_gaps() != 1 {
            return Err(format!(
                "RE-004: record_evidence_gap must increment to 1 on output-branch read failure; got {}",
                drive_state.evidence_gaps()
            ));
        }
        if !evidence.is_empty() {
            let events = evidence.drain();
            return Err(format!(
                "RE-004: no evidence must be emitted on a gap (output branch); got {events:?}"
            ));
        }
        Ok(())
    }

    /// RE-004 positive control: when the slot IS initialized,
    /// emit_slot_evidence pushes the SlotWritten event and does NOT
    /// record a gap. Guards against an over-eager gap counter that
    /// would fire on the happy path.
    #[test]
    fn re_004_emit_slot_evidence_does_not_record_gap_on_success() -> Result<(), String> {
        let mut run = mkr(1, 1)?;
        ws(&mut run, 0, SlotValue::I64(42))?;
        let mut evidence = EvidenceCollector::new();
        let collect_states = CollectStates::new();
        let mut drive_state = DriveState::new();

        let node = setc(0, 0, 0, 0);
        emit_slot_evidence(
            &run,
            &mut evidence,
            &collect_states,
            &node,
            &mut drive_state,
        )
        .map_err(|e| format!("RE-004: happy path must not error: {e:?}"))?;

        if drive_state.evidence_gaps() != 0 {
            return Err(format!(
                "RE-004: happy path must not record a gap; got {}",
                drive_state.evidence_gaps()
            ));
        }
        if evidence.is_empty() {
            return Err("RE-004: happy path must emit SlotWritten evidence".into());
        }
        Ok(())
    }

    /// RE-004 observability follow-up (black-hat FINDING-002): when
    /// `drive_deterministic_full` completes with zero recorded evidence
    /// gaps, the loop returns the natural `Ok(signal)` — the new gap
    /// surfacing path must NOT poison the happy path.
    #[test]
    fn drive_returns_ok_signal_when_no_evidence_gaps_recorded() -> Result<(), String> {
        // The SetConst → Finish workflow writes slot 0 from const 0
        // and then finishes with slot 0 as the result. emit_slot_evidence
        // reads slot 0 (the SetConst output), which is initialized by
        // the executor, so no gap is recorded.
        let wf = mkwfc(
            vec![setc(0, 0, 0, 1), fin(1, 0)],
            1,
            vec![ConstValue::I64(7)],
        )?;
        let mut run = mkr(2, 1)?;
        let mut store = ValueStore::new();
        let mut budget = StepBudget::new(10);
        let mut ev = EvidenceCollector::new();
        let mut cs = CollectStates::new();

        let result = drive_deterministic_full(
            &wf,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut ev,
            &mut cs,
            &CapabilitySet::empty(),
        );

        // Zero gaps → drive returns Ok(Finished(_)). The signal value
        // is preserved (not coerced into a gap error).
        match result {
            Ok(crate::engine::types::RuntimeSignal::Finished(slot_value)) => {
                if slot_value != SlotValue::I64(7) {
                    return Err(format!(
                        "RE-004-obs: zero-gap drive must return Finished with the constant value; got {slot_value:?}"
                    ));
                }
                Ok(())
            }
            Ok(other) => Err(format!(
                "RE-004-obs: zero-gap drive must return Ok(Finished); got Ok({other:?})"
            )),
            Err(e) => Err(format!(
                "RE-004-obs: zero-gap drive must NOT return Err; got {e:?}"
            )),
        }
    }

    /// RE-004 observability follow-up (black-hat FINDING-002 +
    /// FINDING-003): when `drive_deterministic_full` records at least
    /// one evidence gap during a drive that naturally ends with
    /// `Finished`, the loop surfaces the typed
    /// `RuntimeEngineError::EvidenceGapsRecorded { count, last_slot }`
    /// variant so operators can act on the gap without losing the
    /// slot index that failed.
    #[test]
    fn drive_returns_evidence_gaps_recorded_when_at_least_one_gap() -> Result<(), String> {
        // A three-step workflow: a Nop with `output = Some(slot 1)`
        // where slot 1 is intentionally uninitialized, followed by a
        // SetConst → Finish that exercises the normal evidence-emit
        // path on slot 0 and terminates the drive. The Nop step
        // doesn't write to its declared output, so emit_slot_evidence
        // reads slot 1, fails with SlotUninitialized, and records a
        // gap. Because the drive reaches a terminal Finished signal,
        // the gap-surfacing path converts it into the typed error.
        let nop_node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(1)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let wf = mkwfc(
            vec![nop_node, setc(1, 0, 0, 2), fin(2, 0)],
            2,
            vec![ConstValue::I64(7)],
        )?;
        let mut run = mkr(3, 2)?;
        let mut store = ValueStore::new();
        let mut budget = StepBudget::new(10);
        let mut ev = EvidenceCollector::new();
        let mut cs = CollectStates::new();

        let result = drive_deterministic_full(
            &wf,
            &mut run,
            &mut budget,
            &mut store,
            &[],
            RetryPolicy::NEVER,
            &mut ev,
            &mut cs,
            &CapabilitySet::empty(),
        );

        // >=1 gap AND natural Finished signal → drive returns the
        // typed EvidenceGapsRecorded variant carrying count and
        // last_slot. The natural Finished signal is consumed by the
        // gap surfacing path; the operator must investigate the gap
        // before treating the run as complete.
        match result {
            Err(RuntimeEngineError::EvidenceGapsRecorded { count, last_slot }) => {
                if count != 1 {
                    return Err(format!("RE-004-obs: gap count must be 1; got {count}"));
                }
                if last_slot != Some(SlotIdx::new(1)) {
                    return Err(format!(
                        "RE-004-obs: last_slot must be Some(SlotIdx(1)); got {last_slot:?}"
                    ));
                }
                Ok(())
            }
            Ok(other) => Err(format!(
                "RE-004-obs: gap drive must NOT return Ok; got Ok({other:?})"
            )),
            Err(other) => Err(format!(
                "RE-004-obs: gap drive must return Err(EvidenceGapsRecorded); got {other:?}"
            )),
        }
    }

    /// DriveState unit test for the new `take_evidence_gaps_report`
    /// API. Records multiple gaps across two slots and asserts the
    /// report carries the total count and the LAST failing slot,
    /// then resets the state so the same DriveState can be reused.
    #[test]
    fn drive_state_take_evidence_gaps_report_returns_count_and_last_slot() {
        let mut state = DriveState::new();

        // First gap on slot 0.
        state.record_evidence_gap(SlotIdx::new(0));
        let report = state.take_evidence_gaps_report();
        if report.count != 1 {
            panic!("expected count=1 after first gap; got {}", report.count);
        }
        if report.last_slot != Some(SlotIdx::new(0)) {
            panic!(
                "expected last_slot=Some(0) after first gap; got {:?}",
                report.last_slot
            );
        }

        // take_evidence_gaps_report resets the counter so the same
        // DriveState can serve a fresh drive. A second gap on slot 2
        // must report count=1, last_slot=Some(2) (not the stale 0).
        state.record_evidence_gap(SlotIdx::new(2));
        state.record_evidence_gap(SlotIdx::new(2));
        let report = state.take_evidence_gaps_report();
        if report.count != 2 {
            panic!("expected count=2 after two more gaps; got {}", report.count);
        }
        if report.last_slot != Some(SlotIdx::new(2)) {
            panic!(
                "expected last_slot=Some(2) after two more gaps; got {:?}",
                report.last_slot
            );
        }

        // After take, the state is back to default — a third report
        // must observe count=0 / last_slot=None.
        let report = state.take_evidence_gaps_report();
        assert_eq!(report.count, 0);
        assert_eq!(report.last_slot, None);
    }

    /// EvidenceGapReport has the stable runtime code
    /// `EVIDENCE_GAPS_RECORDED` so operators can dispatch on it.
    #[test]
    fn evidence_gaps_recorded_runtime_code_is_stable() {
        let error = RuntimeEngineError::EvidenceGapsRecorded {
            count: 3,
            last_slot: Some(SlotIdx::new(5)),
        };
        assert_eq!(
            error.runtime_code(),
            Some(RuntimeEngineError::EVIDENCE_GAPS_RECORDED_RUNTIME_CODE)
        );
        assert_eq!(
            RuntimeEngineError::EVIDENCE_GAPS_RECORDED_RUNTIME_CODE,
            "EVIDENCE_GAPS_RECORDED"
        );
    }
}
