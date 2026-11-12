//! vb-kyyf replay normalization and comparison proof seam.
//!
//! This module is the production-owned pure kernel for cross-run and replay
//! comparison. Runtime, storage, CLI, and codegen adapters project public
//! observations into these scalar fields; this kernel deliberately performs no
//! I/O, allocation, hashing, formatting, or clock/path/process inspection.

#[allow(unused_macros)]
macro_rules! digest_all_match_body {
    ($status:expr) => {
        $status.workflow_source_matches
            && $status.compiled_ir_matches
            && $status.action_abi_matches
            && $status.policy_matches
    };
}

#[allow(unused_macros)]
macro_rules! normalize_observation_body {
    ($raw:expr) => {
        NormalizedObservation {
            result: $raw.result,
            taint: $raw.taint,
            event_signature: $raw.event_signature,
            event_payload_signature: $raw.event_payload_signature,
            digest_status: $raw.digest_status,
            replay_policy_blocked: $raw.replay_policy_blocked,
            unsupported_generated_subset: $raw.unsupported_generated_subset,
            semantic_slot_signature: $raw.semantic_slot_signature,
            semantic_action_signature: $raw.semantic_action_signature,
            semantic_suspension: $raw.semantic_suspension,
            semantic_taint_signature: $raw.semantic_taint_signature,
        }
    };
}

#[allow(unused_macros)]
macro_rules! normalized_observations_equal_body {
    ($left:expr, $right:expr) => {
        terminal_results_equal($left.result, $right.result)
            && taint_statuses_equal($left.taint, $right.taint)
            && $left.event_signature == $right.event_signature
            && $left.event_payload_signature == $right.event_payload_signature
            && $left.digest_status.workflow_source_matches
                == $right.digest_status.workflow_source_matches
            && $left.digest_status.compiled_ir_matches == $right.digest_status.compiled_ir_matches
            && $left.digest_status.action_abi_matches == $right.digest_status.action_abi_matches
            && $left.digest_status.policy_matches == $right.digest_status.policy_matches
            && $left.replay_policy_blocked == $right.replay_policy_blocked
            && $left.unsupported_generated_subset == $right.unsupported_generated_subset
            && $left.semantic_slot_signature == $right.semantic_slot_signature
            && $left.semantic_action_signature == $right.semantic_action_signature
            && $left.semantic_suspension == $right.semantic_suspension
            && $left.semantic_taint_signature == $right.semantic_taint_signature
    };
}

#[allow(unused_macros)]
macro_rules! terminal_results_equal_body {
    ($left:expr, $right:expr) => {
        match ($left, $right) {
            (TerminalResult::Ok, TerminalResult::Ok) => true,
            (TerminalResult::Blocked, TerminalResult::Blocked) => true,
            (TerminalResult::Failed, TerminalResult::Failed) => true,
            (TerminalResult::None, TerminalResult::None) => true,
            _ => false,
        }
    };
}

#[allow(unused_macros)]
macro_rules! taint_statuses_equal_body {
    ($left:expr, $right:expr) => {
        match ($left, $right) {
            (TaintStatus::Clean, TaintStatus::Clean) => true,
            (TaintStatus::Tainted, TaintStatus::Tainted) => true,
            (TaintStatus::Unknown, TaintStatus::Unknown) => true,
            _ => false,
        }
    };
}

#[allow(unused_macros)]
macro_rules! compare_replay_body {
    ($first:expr, $second:expr) => {{
        let first_norm = normalize_observation($first);
        let second_norm = normalize_observation($second);
        if !first_norm.digest_status.all_match() || !second_norm.digest_status.all_match() {
            return Err(DeterminismError::ReplayDigestMismatch);
        }
        if first_norm.replay_policy_blocked || second_norm.replay_policy_blocked {
            return Err(DeterminismError::ReplayPolicyBlocked);
        }
        if first_norm.event_signature != second_norm.event_signature {
            return Err(DeterminismError::ReplaySequenceViolation);
        }
        compare_normalized_observations(first_norm, second_norm)
    }};
}

#[allow(unused_macros)]
macro_rules! compare_generated_ir_body {
    ($ir:expr, $generated:expr) => {{
        let ir_norm = normalize_observation($ir);
        let generated_norm = normalize_observation($generated);
        if ir_norm.unsupported_generated_subset || generated_norm.unsupported_generated_subset {
            return Err(DeterminismError::UnsupportedGeneratedSubset);
        }
        if generated_ir_observations_equal(ir_norm, generated_norm) {
            Ok(())
        } else {
            Err(DeterminismError::GeneratedIrDivergence)
        }
    }};
}

#[allow(unused_macros)]
macro_rules! generated_ir_observations_equal_body {
    ($left:expr, $right:expr) => {
        terminal_results_equal($left.result, $right.result)
            && taint_statuses_equal($left.taint, $right.taint)
            && $left.event_signature == $right.event_signature
            && $left.event_payload_signature == $right.event_payload_signature
            && $left.digest_status.workflow_source_matches
                == $right.digest_status.workflow_source_matches
            && $left.digest_status.compiled_ir_matches == $right.digest_status.compiled_ir_matches
            && $left.digest_status.action_abi_matches == $right.digest_status.action_abi_matches
            && $left.digest_status.policy_matches == $right.digest_status.policy_matches
            && $left.replay_policy_blocked == $right.replay_policy_blocked
            && $left.unsupported_generated_subset == $right.unsupported_generated_subset
            && $left.semantic_slot_signature == $right.semantic_slot_signature
            && $left.semantic_action_signature == $right.semantic_action_signature
            && $left.semantic_suspension == $right.semantic_suspension
    };
}

#[allow(unused_macros)]
macro_rules! compare_normalized_observations_body {
    ($left:expr, $right:expr) => {{
        if normalized_observations_equal($left, $right) {
            Ok(())
        } else {
            Err(DeterminismError::NondeterministicObservation)
        }
    }};
}

#[cfg(verus_keep_ghost)]
mod verus_kernel;

#[cfg(verus_keep_ghost)]
pub use verus_kernel::*;

#[cfg(not(verus_keep_ghost))]
mod cargo_kernel;

#[cfg(not(verus_keep_ghost))]
pub use cargo_kernel::*;

#[cfg(test)]
mod tests;
