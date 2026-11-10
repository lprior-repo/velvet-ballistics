use crate::engine::repeat;
use crate::replay::{ReplayAction, ReplayError, engine_to_replay_err};
use crate::{CompiledNode, CompiledNodeKind as K, RunFrame};

pub(super) fn replay_repeat(
    node: &CompiledNode,
    run: &mut RunFrame,
) -> Result<ReplayAction, ReplayError> {
    let flow = match node.kind {
        K::RepeatStart {
            max_attempts, body, ..
        } => repeat::repeat_start(run, max_attempts, body, node.output.slot()),
        K::RepeatAttempt {
            attempt_slot, body, ..
        } => repeat::repeat_attempt(run, attempt_slot, body),
        K::RepeatCheck { attempt_slot, done } => {
            repeat::repeat_check(run, attempt_slot, done, node.routing.next.step(), node.id)
        }
        K::RepeatFinish { result } => repeat::repeat_finish(
            run,
            result,
            node.output.slot(),
            node.routing.next.step(),
            node.id,
        ),
        _ => {
            return Err(ReplayError::Internal {
                reason: "non-repeat node in Repeat replay",
            });
        }
    }
    .map_err(engine_to_replay_err)?;
    super::nodes::replay_jump(run, flow.target())
}
