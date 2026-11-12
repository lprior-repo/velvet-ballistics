use crate::frame::RunFrame;
use crate::ids::{SlotIdx, StepIdx};
use crate::value::SlotValue;
use crate::workflow::CompiledNode;

use super::super::{ReplayError, engine_to_replay_err};
use super::iter::{replay_next, replay_output};
use super::nodes::{advance_to_next, replay_jump};
use super::types::ReplayAction;

/// Bit shift used to pack `max_attempts` into the high 32 bits of the
/// repeat-state I64. Mirrors `vb_runtime::primitives::repeat::REPEAT_SHIFT`.
const REPEAT_SHIFT: u32 = 32;

/// Encodes `(max_attempts, current_attempt)` into one I64, mirroring the
/// executor layout: high 32 bits hold `max_attempts`, low 16 bits hold the
/// current attempt.
fn encode_repeat_state(max_attempts: u16, current_attempt: u16) -> Result<i64, ReplayError> {
    if max_attempts == 0 || current_attempt > max_attempts {
        return Err(invalid_repeat_state());
    }
    let high = i64::from(max_attempts)
        .checked_shl(REPEAT_SHIFT)
        .ok_or(ReplayError::Internal {
            reason: "repeat_state_encode_overflow during replay",
        })?;
    high.checked_add(i64::from(current_attempt))
        .ok_or(ReplayError::Internal {
            reason: "repeat_state_encode_overflow during replay",
        })
}

/// Decodes a packed repeat-state I64 into `(max_attempts, current_attempt)`.
fn decode_repeat_state(packed: i64) -> Result<(u16, u16), ReplayError> {
    let bits = u64::try_from(packed).map_err(|_| invalid_repeat_state())?;
    let max_bits = bits >> REPEAT_SHIFT;
    let low_bits = bits & 0xFFFF;
    let reserved_bits = bits & 0xFFFF_0000;
    if reserved_bits != 0 {
        return Err(invalid_repeat_state());
    }
    let max_attempts = u16::try_from(max_bits).map_err(|_| invalid_repeat_state())?;
    let current_attempt = u16::try_from(low_bits).map_err(|_| invalid_repeat_state())?;
    if max_attempts == 0 || current_attempt > max_attempts {
        return Err(invalid_repeat_state());
    }
    Ok((max_attempts, current_attempt))
}

fn expect_repeat_state(value: SlotValue) -> Result<i64, ReplayError> {
    match value {
        SlotValue::I64(packed) => Ok(packed),
        _ => Err(ReplayError::Internal {
            reason: "repeat attempt slot was not number during replay",
        }),
    }
}

fn invalid_repeat_state() -> ReplayError {
    ReplayError::Internal {
        reason: "invalid_repeat_state during replay",
    }
}

/// Replays `RepeatStart`: writes packed repeat state and jumps to the body.
/// Mirrors `vb_runtime::primitives::repeat::repeat_start`.
pub(super) fn replay_repeat_start(
    run: &mut RunFrame,
    node: &CompiledNode,
    max_attempts: u16,
    body: StepIdx,
) -> Result<ReplayAction, ReplayError> {
    let attempt_output = replay_output(node, "RepeatStart node missing output slot")?;
    let state = encode_repeat_state(max_attempts, 0)?;
    run.write_slot(attempt_output, SlotValue::I64(state))
        .map_err(engine_to_replay_err)?;
    replay_jump(run, body)
}

/// Replays `RepeatAttempt`: validates the packed state and jumps to the body.
/// Mirrors `vb_runtime::primitives::repeat::repeat_attempt`.
pub(super) fn replay_repeat_attempt(
    run: &mut RunFrame,
    attempt_slot: SlotIdx,
    body: StepIdx,
) -> Result<ReplayAction, ReplayError> {
    let packed = expect_repeat_state(*run.read_slot(attempt_slot).map_err(engine_to_replay_err)?)?;
    let _validated = decode_repeat_state(packed)?;
    replay_jump(run, body)
}

/// Replays `RepeatCheck`: increments the attempt counter and routes to `done`
/// once attempts are exhausted, else back to the body entry.
/// Mirrors `repeat_check_with_policy`.
pub(super) fn replay_repeat_check(
    run: &mut RunFrame,
    node: &CompiledNode,
    attempt_slot: SlotIdx,
    done: StepIdx,
) -> Result<ReplayAction, ReplayError> {
    let packed = expect_repeat_state(*run.read_slot(attempt_slot).map_err(engine_to_replay_err)?)?;
    let (max_attempts, current_attempt) = decode_repeat_state(packed)?;
    let next_attempt = current_attempt.saturating_add(1);
    let updated = encode_repeat_state(max_attempts, next_attempt)?;
    run.write_slot(attempt_slot, SlotValue::I64(updated))
        .map_err(engine_to_replay_err)?;
    if next_attempt >= max_attempts {
        return replay_jump(run, done);
    }
    let body_entry = replay_next(node, "RepeatCheck node missing next step")?;
    replay_jump(run, body_entry)
}

/// Replays `RepeatFinish`: writes the result value and advances.
/// Mirrors `vb_runtime::primitives::repeat::repeat_finish`.
pub(super) fn replay_repeat_finish(
    run: &mut RunFrame,
    node: &CompiledNode,
    result: SlotIdx,
) -> Result<ReplayAction, ReplayError> {
    let value = *run.read_slot(result).map_err(engine_to_replay_err)?;
    let taint = run.read_taint(result).map_err(engine_to_replay_err)?;
    let output = replay_output(node, "RepeatFinish node missing output slot")?;
    run.write_slot_with_taint(output, value, taint)
        .map_err(engine_to_replay_err)?;
    advance_to_next(run, node).map(ReplayAction::Continue)
}
