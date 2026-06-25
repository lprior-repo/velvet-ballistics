#![forbid(unsafe_code)]

//! RE-004 / RE-011 regression tests for evidence gaps and capacity
//! overflow in the drive loop.

use super::common::{collect_start, dde, fin, mkwf, mkr, setc, ws};
use crate::engine::drive::{drive_deterministic_full, emit_slot_evidence, DriveState};
use crate::engine::types::{
    EvidenceCollector, EvidenceEvent, RetryPolicy, RuntimeEngineError, RuntimeSignal,
};
use crate::primitives::collect::{CollectPaginationState, CollectStates};
use vb_core::capability::CapabilitySet;
use vb_core::engine::StepBudget;
use vb_core::frame::StepState;
use vb_core::ids::SlotIdx;
use vb_core::value::SlotValue;
use vb_core::value_store::ValueStore;
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, WorkflowParts,
};

#[cfg(test)]
mod tests {
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
            vb_core::frame::StepState::Succeeded => Err(format!(
                "RE-011: step 0 marked Succeeded despite capacity overflow (half-committed state)"
            )),
            vb_core::frame::StepState::Running => Ok(()),
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
        #[test]
    fn re_004_emit_slot_evidence_records_gap_on_read_slot_error() -> Result<(), String> {
        use crate::engine::drive::{DriveState, emit_slot_evidence};

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
        #[test]
    fn re_004_emit_slot_evidence_records_gap_on_output_read_slot_error() -> Result<(), String> {
        use crate::engine::drive::{DriveState, emit_slot_evidence};

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
        #[test]
    fn re_004_emit_slot_evidence_does_not_record_gap_on_success() -> Result<(), String> {
        use crate::engine::drive::{DriveState, emit_slot_evidence};

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
    }
