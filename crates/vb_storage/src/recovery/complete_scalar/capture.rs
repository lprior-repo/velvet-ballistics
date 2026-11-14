use super::driver::Driver;
use super::driver::OwnedCollectReplaySnapshot;
use super::{TraceReplayMode, invalid};
use crate::{
    recovery::RecoveryResult,
    slot_extra::{DecodedSlotWrittenExtra, decode_slot_written_extra},
};
use vb_core::{SlotIdx, SlotValue};

fn collect_frame_extra_slot(
    kind: &vb_core::CompiledNodeKind,
    output: Option<SlotIdx>,
) -> Option<SlotIdx> {
    match kind {
        vb_core::CompiledNodeKind::CollectStart { source, .. } => Some(output.unwrap_or(*source)),
        vb_core::CompiledNodeKind::CollectNext { collector_slot, .. } => Some(*collector_slot),
        _ => None,
    }
}

impl Driver<'_> {
    pub(super) fn capture(
        &mut self,
        run: vb_core::RunId,
        seq: crate::EventSeq,
        slot: SlotIdx,
        bytes: Option<&[u8]>,
        extra: Option<&[u8]>,
    ) -> RecoveryResult<()> {
        let (step, action) = self
            .active
            .as_ref()
            .ok_or_else(|| invalid(self.frame.pc(), "capture without active step"))?;
        let step = *step;
        if self.frame.step_state(step).ok() == Some(vb_core::StepState::Failed) {
            return Err(invalid(step, "failed step cannot emit output capture"));
        }
        let value =
            postcard::from_bytes(bytes.ok_or_else(|| invalid(step, "missing captured value"))?)
                .map_err(|_| invalid(step, "invalid captured value"))?;
        let DecodedSlotWrittenExtra::Envelope(envelope) = decode_slot_written_extra(
            extra.ok_or_else(|| invalid(step, "missing captured taint"))?,
        )
        .map_err(|_| invalid(step, "invalid captured taint"))?
        else {
            return Err(invalid(step, "unbound legacy taint capture"));
        };
        let node = self
            .plan
            .node(step)
            .ok_or_else(|| invalid(step, "captured node missing"))?;
        let collect_slot = collect_frame_extra_slot(&node.kind, node.output.slot());
        if envelope.frame_extra.is_some() {
            // Runtime recovery separately validates the typed Collect state before resume.
            let authorized = self.mode == TraceReplayMode::OwnedArena && collect_slot == Some(slot);
            if !authorized {
                return Err(invalid(step, "unproven scalar frame component"));
            }
        }
        if self.mode == TraceReplayMode::OwnedArena && collect_slot == Some(slot) {
            self.collect_snapshots
                .try_reserve(1)
                .map_err(|_| invalid(step, "collect replay snapshot allocation failed"))?;
            self.collect_snapshots.push(OwnedCollectReplaySnapshot {
                seq,
                run,
                step,
                slot,
                state: self.collect.snapshot(slot),
            });
        }
        if let Some((_, _, answer)) = self.ask {
            if slot != answer || self.answer.is_some() || self.answered {
                return Err(invalid(step, "unauthorized Ask capture"));
            }
            if let SlotValue::Symbol(id) = value
                && self.store.symbol(id).is_err()
            {
                return Err(invalid(step, "Ask symbol outside replay source prefix"));
            }
            self.answer = Some((value, envelope.taint));
        } else if !self.required(step, action)?.contains(&slot)
            || self.frame.read_slot(slot).ok() != Some(&value)
            || self.frame.read_taint(slot).ok() != Some(envelope.taint)
        {
            return Err(invalid(step, "capture differs from artifact replay"));
        }
        if !self.captures.insert(slot) {
            return Err(invalid(step, "duplicate step capture"));
        }
        Ok(())
    }
}
