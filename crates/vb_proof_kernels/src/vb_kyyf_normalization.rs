//! vb-kyyf replay normalization and comparison proof seam.
//!
//! Local-only normalization sanity kernel. Runtime, storage, CLI, and codegen
//! adapters do not currently call this crate module or carry Verus contracts
//! tying their observations to these scalar fields. Verus checks over this file
//! are therefore local model evidence only, not production evidence for vb-kyyf
//! replay or generated-IR determinism.

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
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {

#[derive(Clone, Copy)]
pub enum TerminalResult {
    Ok,
    Blocked,
    Failed,
    None,
}

#[derive(Clone, Copy)]
pub enum TaintStatus {
    Clean,
    Tainted,
    Unknown,
}

#[derive(Clone, Copy)]
pub enum DeterminismError {
    NondeterministicObservation,
    ReplayDigestMismatch,
    ReplaySequenceViolation,
    ReplayPolicyBlocked,
    GeneratedIrDivergence,
    UnsupportedGeneratedSubset,
}

#[derive(Clone, Copy)]
pub struct DigestStatus {
    pub workflow_source_matches: bool,
    pub compiled_ir_matches: bool,
    pub action_abi_matches: bool,
    pub policy_matches: bool,
}

pub open spec fn spec_digest_all_match(status: DigestStatus) -> bool
{
    status.workflow_source_matches
        && status.compiled_ir_matches
        && status.action_abi_matches
        && status.policy_matches
}

impl DigestStatus {
    #[must_use]
    pub fn all_match(&self) -> (all_match: bool)
        ensures
            all_match == spec_digest_all_match(*self),
    {
        digest_all_match_body!(self)
    }
}

#[derive(Clone, Copy)]
pub struct PublicObservation {
    pub result: TerminalResult,
    pub taint: TaintStatus,
    pub event_signature: u64,
    pub event_payload_signature: u64,
    pub digest_status: DigestStatus,
    pub replay_policy_blocked: bool,
    pub unsupported_generated_subset: bool,
    pub semantic_slot_signature: u64,
    pub semantic_action_signature: u64,
    pub semantic_suspension: bool,
    pub semantic_taint_signature: u64,
    pub temp_path_signature: u64,
    pub process_id_signature: u64,
    pub wall_clock_signature: u64,
    pub generated_run_signature: u64,
}

#[derive(Clone, Copy)]
pub struct NormalizedObservation {
    pub result: TerminalResult,
    pub taint: TaintStatus,
    pub event_signature: u64,
    pub event_payload_signature: u64,
    pub digest_status: DigestStatus,
    pub replay_policy_blocked: bool,
    pub unsupported_generated_subset: bool,
    pub semantic_slot_signature: u64,
    pub semantic_action_signature: u64,
    pub semantic_suspension: bool,
    pub semantic_taint_signature: u64,
}

pub open spec fn spec_normalize_observation(raw: PublicObservation) -> NormalizedObservation
{
    normalize_observation_body!(raw)
}

pub open spec fn spec_normalized_observations_equal(
    left: NormalizedObservation,
    right: NormalizedObservation,
) -> bool
{
    left.result == right.result
        && left.taint == right.taint
        && left.event_signature == right.event_signature
        && left.event_payload_signature == right.event_payload_signature
        && left.digest_status.workflow_source_matches == right.digest_status.workflow_source_matches
        && left.digest_status.compiled_ir_matches == right.digest_status.compiled_ir_matches
        && left.digest_status.action_abi_matches == right.digest_status.action_abi_matches
        && left.digest_status.policy_matches == right.digest_status.policy_matches
        && left.replay_policy_blocked == right.replay_policy_blocked
        && left.unsupported_generated_subset == right.unsupported_generated_subset
        && left.semantic_slot_signature == right.semantic_slot_signature
        && left.semantic_action_signature == right.semantic_action_signature
        && left.semantic_suspension == right.semantic_suspension
        && left.semantic_taint_signature == right.semantic_taint_signature
}

pub open spec fn spec_compare_cross_run_result(
    left: PublicObservation,
    right: PublicObservation,
) -> Result<(), DeterminismError>
{
    if spec_normalized_observations_equal(
        spec_normalize_observation(left),
        spec_normalize_observation(right),
    ) {
        Ok(())
    } else {
        Err(DeterminismError::NondeterministicObservation)
    }
}

pub open spec fn spec_compare_replay_result(
    first: PublicObservation,
    second: PublicObservation,
) -> Result<(), DeterminismError>
{
    let first_norm = spec_normalize_observation(first);
    let second_norm = spec_normalize_observation(second);
    if !spec_digest_all_match(first_norm.digest_status)
        || !spec_digest_all_match(second_norm.digest_status) {
        Err(DeterminismError::ReplayDigestMismatch)
    } else if first_norm.replay_policy_blocked || second_norm.replay_policy_blocked {
        Err(DeterminismError::ReplayPolicyBlocked)
    } else if first_norm.event_signature != second_norm.event_signature {
        Err(DeterminismError::ReplaySequenceViolation)
    } else {
        spec_compare_cross_run_result(first, second)
    }
}

pub open spec fn spec_compare_generated_ir_result(
    ir: PublicObservation,
    generated: PublicObservation,
) -> Result<(), DeterminismError>
{
    let ir_norm = spec_normalize_observation(ir);
    let generated_norm = spec_normalize_observation(generated);
    if ir_norm.unsupported_generated_subset || generated_norm.unsupported_generated_subset {
        Err(DeterminismError::UnsupportedGeneratedSubset)
    } else if spec_generated_ir_observations_equal(ir_norm, generated_norm) {
        Ok(())
    } else {
        Err(DeterminismError::GeneratedIrDivergence)
    }
}

pub open spec fn spec_generated_ir_observations_equal(
    left: NormalizedObservation,
    right: NormalizedObservation,
) -> bool {
    left.result == right.result
        && left.taint == right.taint
        && left.event_signature == right.event_signature
        && left.event_payload_signature == right.event_payload_signature
        && left.digest_status.workflow_source_matches == right.digest_status.workflow_source_matches
        && left.digest_status.compiled_ir_matches == right.digest_status.compiled_ir_matches
        && left.digest_status.action_abi_matches == right.digest_status.action_abi_matches
        && left.digest_status.policy_matches == right.digest_status.policy_matches
        && left.replay_policy_blocked == right.replay_policy_blocked
        && left.unsupported_generated_subset == right.unsupported_generated_subset
        && left.semantic_slot_signature == right.semantic_slot_signature
        && left.semantic_action_signature == right.semantic_action_signature
        && left.semantic_suspension == right.semantic_suspension
}

#[must_use]
pub fn normalize_observation(raw: PublicObservation) -> (norm: NormalizedObservation)
    ensures
        norm == spec_normalize_observation(raw),
{
    NormalizedObservation {
        result: raw.result,
        taint: raw.taint,
        event_signature: raw.event_signature,
        event_payload_signature: raw.event_payload_signature,
        digest_status: raw.digest_status,
        replay_policy_blocked: raw.replay_policy_blocked,
        unsupported_generated_subset: raw.unsupported_generated_subset,
        semantic_slot_signature: raw.semantic_slot_signature,
        semantic_action_signature: raw.semantic_action_signature,
        semantic_suspension: raw.semantic_suspension,
        semantic_taint_signature: raw.semantic_taint_signature,
    }
}

pub fn compare_cross_run(
    left: PublicObservation,
    right: PublicObservation,
) -> (result: Result<(), DeterminismError>)
    ensures
        result == spec_compare_cross_run_result(left, right),
{
    compare_normalized_observations(normalize_observation(left), normalize_observation(right))
}

pub fn compare_replay(
    first: PublicObservation,
    second: PublicObservation,
) -> (result: Result<(), DeterminismError>)
    ensures
        result == spec_compare_replay_result(first, second),
{
    compare_replay_body!(first, second)
}

pub fn compare_generated_ir(
    ir: PublicObservation,
    generated: PublicObservation,
) -> (result: Result<(), DeterminismError>)
    ensures
        result == spec_compare_generated_ir_result(ir, generated),
{
    compare_generated_ir_body!(ir, generated)
}

fn compare_normalized_observations(
    left: NormalizedObservation,
    right: NormalizedObservation,
) -> (result: Result<(), DeterminismError>)
    ensures
        result == if spec_normalized_observations_equal(left, right) {
            Ok(())
        } else {
            Err(DeterminismError::NondeterministicObservation)
        },
{
    compare_normalized_observations_body!(left, right)
}

fn normalized_observations_equal(
    left: NormalizedObservation,
    right: NormalizedObservation,
) -> (equal: bool)
    ensures
        equal == spec_normalized_observations_equal(left, right),
{
    normalized_observations_equal_body!(left, right)
}

fn generated_ir_observations_equal(
    left: NormalizedObservation,
    right: NormalizedObservation,
) -> (equal: bool)
    ensures
        equal == spec_generated_ir_observations_equal(left, right),
{
    generated_ir_observations_equal_body!(left, right)
}

fn terminal_results_equal(left: TerminalResult, right: TerminalResult) -> (equal: bool)
    ensures
        equal == (left == right),
{
    terminal_results_equal_body!(left, right)
}

fn taint_statuses_equal(left: TaintStatus, right: TaintStatus) -> (equal: bool)
    ensures
        equal == (left == right),
{
    taint_statuses_equal_body!(left, right)
}

} // verus!

#[cfg(not(verus_keep_ghost))]
mod cargo_kernel {
    /// Result of a terminal step evaluation.
    #[derive(Clone, Copy)]
    pub enum TerminalResult {
        Ok,
        Blocked,
        Failed,
        None,
    }

    /// Taint classification for a value.
    #[derive(Clone, Copy)]
    pub enum TaintStatus {
        Clean,
        Tainted,
        Unknown,
    }

    /// Errors that can occur during determinism verification.
    #[derive(Clone, Copy)]
    pub enum DeterminismError {
        NondeterministicObservation,
        ReplayDigestMismatch,
        ReplaySequenceViolation,
        ReplayPolicyBlocked,
        GeneratedIrDivergence,
        UnsupportedGeneratedSubset,
    }

    /// Status of a workflow digest comparison.
    #[derive(Clone, Copy)]
    pub struct DigestStatus {
        pub workflow_source_matches: bool,
        pub compiled_ir_matches: bool,
        pub action_abi_matches: bool,
        pub policy_matches: bool,
    }

    impl DigestStatus {
        #[must_use]
        pub const fn all_match(&self) -> bool {
            digest_all_match_body!(self)
        }
    }

    /// Observable properties of a workflow execution.
    #[derive(Clone, Copy)]
    pub struct PublicObservation {
        pub result: TerminalResult,
        pub taint: TaintStatus,
        pub event_signature: u64,
        pub event_payload_signature: u64,
        pub digest_status: DigestStatus,
        pub replay_policy_blocked: bool,
        pub unsupported_generated_subset: bool,
        pub semantic_slot_signature: u64,
        pub semantic_action_signature: u64,
        pub semantic_suspension: bool,
        pub semantic_taint_signature: u64,
        pub temp_path_signature: u64,
        pub process_id_signature: u64,
        pub wall_clock_signature: u64,
        pub generated_run_signature: u64,
    }

    /// Normalized form of an observation for comparison.
    #[derive(Clone, Copy)]
    pub struct NormalizedObservation {
        pub result: TerminalResult,
        pub taint: TaintStatus,
        pub event_signature: u64,
        pub event_payload_signature: u64,
        pub digest_status: DigestStatus,
        pub replay_policy_blocked: bool,
        pub unsupported_generated_subset: bool,
        pub semantic_slot_signature: u64,
        pub semantic_action_signature: u64,
        pub semantic_suspension: bool,
        pub semantic_taint_signature: u64,
    }

    /// Normalizes a public observation by extracting comparison-relevant fields.
    #[must_use]
    pub const fn normalize_observation(raw: PublicObservation) -> NormalizedObservation {
        normalize_observation_body!(raw)
    }

    /// Compares observations across two different workflow runs for determinism.
    pub fn compare_cross_run(
        left: PublicObservation,
        right: PublicObservation,
    ) -> Result<(), DeterminismError> {
        compare_normalized_observations(normalize_observation(left), normalize_observation(right))
    }

    /// Compares observations from two replays of the same run for determinism.
    pub fn compare_replay(
        first: PublicObservation,
        second: PublicObservation,
    ) -> Result<(), DeterminismError> {
        compare_replay_body!(first, second)
    }

    /// Compares IR-generated observations against reference for consistency.
    pub fn compare_generated_ir(
        ir: PublicObservation,
        generated: PublicObservation,
    ) -> Result<(), DeterminismError> {
        compare_generated_ir_body!(ir, generated)
    }

    fn compare_normalized_observations(
        left: NormalizedObservation,
        right: NormalizedObservation,
    ) -> Result<(), DeterminismError> {
        compare_normalized_observations_body!(left, right)
    }

    fn normalized_observations_equal(
        left: NormalizedObservation,
        right: NormalizedObservation,
    ) -> bool {
        normalized_observations_equal_body!(left, right)
    }

    fn generated_ir_observations_equal(
        left: NormalizedObservation,
        right: NormalizedObservation,
    ) -> bool {
        generated_ir_observations_equal_body!(left, right)
    }

    fn terminal_results_equal(left: TerminalResult, right: TerminalResult) -> bool {
        terminal_results_equal_body!(left, right)
    }

    fn taint_statuses_equal(left: TaintStatus, right: TaintStatus) -> bool {
        taint_statuses_equal_body!(left, right)
    }
}

#[cfg(not(verus_keep_ghost))]
pub use cargo_kernel::*;

#[cfg(test)]
mod tests {
    use super::*;

    const CLEAN_DIGESTS: DigestStatus = DigestStatus {
        workflow_source_matches: true,
        compiled_ir_matches: true,
        action_abi_matches: true,
        policy_matches: true,
    };

    const fn observation() -> PublicObservation {
        PublicObservation {
            result: TerminalResult::Ok,
            taint: TaintStatus::Clean,
            event_signature: 1,
            event_payload_signature: 2,
            digest_status: CLEAN_DIGESTS,
            replay_policy_blocked: false,
            unsupported_generated_subset: false,
            semantic_slot_signature: 3,
            semantic_action_signature: 4,
            semantic_suspension: false,
            semantic_taint_signature: 5,
            temp_path_signature: 10,
            process_id_signature: 11,
            wall_clock_signature: 12,
            generated_run_signature: 13,
        }
    }

    #[test]
    fn cold_metadata_is_normalized_away() {
        let left = observation();
        let right = PublicObservation {
            temp_path_signature: 20,
            process_id_signature: 21,
            wall_clock_signature: 22,
            generated_run_signature: 23,
            ..observation()
        };

        assert!(matches!(compare_cross_run(left, right), Ok(())));
    }

    #[test]
    fn semantic_delta_is_rejected() {
        let left = observation();
        let right = PublicObservation {
            semantic_slot_signature: 99,
            ..observation()
        };

        assert!(matches!(
            compare_cross_run(left, right),
            Err(DeterminismError::NondeterministicObservation)
        ));
    }

    #[test]
    fn replay_digest_mismatch_keeps_exact_taxonomy() {
        let left = PublicObservation {
            digest_status: DigestStatus {
                workflow_source_matches: false,
                ..CLEAN_DIGESTS
            },
            ..observation()
        };

        assert!(matches!(
            compare_replay(left, observation()),
            Err(DeterminismError::ReplayDigestMismatch)
        ));
    }

    // ── DigestStatus::all_match — field coverage ───────────────────────────

    #[test]
    fn digest_status_all_match_when_all_true() {
        assert!(CLEAN_DIGESTS.all_match());
    }

    #[test]
    fn digest_status_all_match_false_on_workflow_source() {
        let s = DigestStatus {
            workflow_source_matches: false,
            ..CLEAN_DIGESTS
        };
        assert!(!s.all_match());
    }

    #[test]
    fn digest_status_all_match_false_on_compiled_ir() {
        let s = DigestStatus {
            compiled_ir_matches: false,
            ..CLEAN_DIGESTS
        };
        assert!(!s.all_match());
    }

    #[test]
    fn digest_status_all_match_false_on_action_abi() {
        let s = DigestStatus {
            action_abi_matches: false,
            ..CLEAN_DIGESTS
        };
        assert!(!s.all_match());
    }

    #[test]
    fn digest_status_all_match_false_on_policy() {
        let s = DigestStatus {
            policy_matches: false,
            ..CLEAN_DIGESTS
        };
        assert!(!s.all_match());
    }

    #[test]
    fn digest_status_all_match_false_when_all_false() {
        let s = DigestStatus {
            workflow_source_matches: false,
            compiled_ir_matches: false,
            action_abi_matches: false,
            policy_matches: false,
        };
        assert!(!s.all_match());
    }

    // ── normalize_observation — field projection ───────────────────────────

    #[test]
    fn normalize_observation_drops_temp_path_signature() {
        let raw = PublicObservation {
            temp_path_signature: 1234,
            ..observation()
        };
        let norm = normalize_observation(raw);
        let other = normalize_observation(observation());
        // All preserved fields should match between norm and other
        assert!(norm.event_signature == other.event_signature);
        assert!(norm.event_payload_signature == other.event_payload_signature);
    }

    #[test]
    fn normalize_observation_drops_process_id_signature() {
        let raw = PublicObservation {
            process_id_signature: 5678,
            ..observation()
        };
        let norm = normalize_observation(raw);
        let other = normalize_observation(observation());
        assert!(norm.event_signature == other.event_signature);
    }

    #[test]
    fn normalize_observation_drops_wall_clock_signature() {
        let raw = PublicObservation {
            wall_clock_signature: 9012,
            ..observation()
        };
        let norm = normalize_observation(raw);
        let other = normalize_observation(observation());
        assert!(norm.event_signature == other.event_signature);
    }

    #[test]
    fn normalize_observation_drops_generated_run_signature() {
        let raw = PublicObservation {
            generated_run_signature: 3456,
            ..observation()
        };
        let norm = normalize_observation(raw);
        let other = normalize_observation(observation());
        assert!(norm.event_signature == other.event_signature);
    }

    #[test]
    fn normalize_observation_preserves_all_nondropped_fields() {
        let raw = PublicObservation {
            result: TerminalResult::Failed,
            taint: TaintStatus::Tainted,
            event_signature: 99,
            event_payload_signature: 100,
            digest_status: CLEAN_DIGESTS,
            replay_policy_blocked: true,
            unsupported_generated_subset: true,
            semantic_slot_signature: 11,
            semantic_action_signature: 22,
            semantic_suspension: true,
            semantic_taint_signature: 33,
            temp_path_signature: 1,
            process_id_signature: 2,
            wall_clock_signature: 3,
            generated_run_signature: 4,
        };
        let norm = normalize_observation(raw);
        assert!(matches!(norm.result, TerminalResult::Failed));
        assert!(matches!(norm.taint, TaintStatus::Tainted));
        assert!(norm.event_signature == 99u64);
        assert!(norm.event_payload_signature == 100u64);
        assert!(norm.replay_policy_blocked);
        assert!(norm.unsupported_generated_subset);
        assert!(norm.semantic_slot_signature == 11u64);
        assert!(norm.semantic_action_signature == 22u64);
        assert!(norm.semantic_suspension);
        assert!(norm.semantic_taint_signature == 33u64);
    }

    // ── compare_cross_run — exhaustiveness ─────────────────────────────────

    #[test]
    fn compare_cross_run_equal_observations_ok() {
        let a = observation();
        let b = observation();
        assert!(matches!(compare_cross_run(a, b), Ok(())));
    }

    #[test]
    fn compare_cross_run_different_result_rejected() {
        let left = observation();
        let right = PublicObservation {
            result: TerminalResult::Failed,
            ..observation()
        };
        assert!(matches!(
            compare_cross_run(left, right),
            Err(DeterminismError::NondeterministicObservation)
        ));
    }

    #[test]
    fn compare_cross_run_different_taint_rejected() {
        let left = observation();
        let right = PublicObservation {
            taint: TaintStatus::Tainted,
            ..observation()
        };
        assert!(matches!(
            compare_cross_run(left, right),
            Err(DeterminismError::NondeterministicObservation)
        ));
    }

    #[test]
    fn compare_cross_run_different_event_signature_rejected() {
        let left = observation();
        let right = PublicObservation {
            event_signature: 7777,
            ..observation()
        };
        assert!(matches!(
            compare_cross_run(left, right),
            Err(DeterminismError::NondeterministicObservation)
        ));
    }

    #[test]
    fn compare_cross_run_different_payload_signature_rejected() {
        let left = observation();
        let right = PublicObservation {
            event_payload_signature: 7777,
            ..observation()
        };
        assert!(matches!(
            compare_cross_run(left, right),
            Err(DeterminismError::NondeterministicObservation)
        ));
    }

    #[test]
    fn compare_cross_run_different_semantic_action_rejected() {
        let left = observation();
        let right = PublicObservation {
            semantic_action_signature: 7777,
            ..observation()
        };
        assert!(matches!(
            compare_cross_run(left, right),
            Err(DeterminismError::NondeterministicObservation)
        ));
    }

    #[test]
    fn compare_cross_run_different_semantic_suspension_rejected() {
        let left = observation();
        let right = PublicObservation {
            semantic_suspension: true,
            ..observation()
        };
        assert!(matches!(
            compare_cross_run(left, right),
            Err(DeterminismError::NondeterministicObservation)
        ));
    }

    #[test]
    fn compare_cross_run_different_semantic_taint_rejected() {
        let left = observation();
        let right = PublicObservation {
            semantic_taint_signature: 7777,
            ..observation()
        };
        assert!(matches!(
            compare_cross_run(left, right),
            Err(DeterminismError::NondeterministicObservation)
        ));
    }

    // ── compare_replay — replay-specific paths ──────────────────────────────

    #[test]
    fn compare_replay_digest_mismatch_on_first_only() {
        let left = PublicObservation {
            digest_status: DigestStatus {
                workflow_source_matches: false,
                ..CLEAN_DIGESTS
            },
            ..observation()
        };
        assert!(matches!(
            compare_replay(left, observation()),
            Err(DeterminismError::ReplayDigestMismatch)
        ));
    }

    #[test]
    fn compare_replay_digest_mismatch_on_second_only() {
        let right = PublicObservation {
            digest_status: DigestStatus {
                action_abi_matches: false,
                ..CLEAN_DIGESTS
            },
            ..observation()
        };
        assert!(matches!(
            compare_replay(observation(), right),
            Err(DeterminismError::ReplayDigestMismatch)
        ));
    }

    #[test]
    fn compare_replay_policy_blocked_on_first() {
        let left = PublicObservation {
            replay_policy_blocked: true,
            ..observation()
        };
        assert!(matches!(
            compare_replay(left, observation()),
            Err(DeterminismError::ReplayPolicyBlocked)
        ));
    }

    #[test]
    fn compare_replay_policy_blocked_on_second() {
        let right = PublicObservation {
            replay_policy_blocked: true,
            ..observation()
        };
        assert!(matches!(
            compare_replay(observation(), right),
            Err(DeterminismError::ReplayPolicyBlocked)
        ));
    }

    #[test]
    fn compare_replay_sequence_violation_on_event_signature() {
        let right = PublicObservation {
            event_signature: 9999,
            ..observation()
        };
        assert!(matches!(
            compare_replay(observation(), right),
            Err(DeterminismError::ReplaySequenceViolation)
        ));
    }

    #[test]
    fn compare_replay_observation_specific_rejection_after_sequence_check() {
        let right = PublicObservation {
            semantic_slot_signature: 5555,
            ..observation()
        };
        assert!(matches!(
            compare_replay(observation(), right),
            Err(DeterminismError::NondeterministicObservation)
        ));
    }

    #[test]
    fn compare_replay_happy_path() {
        assert!(compare_replay(observation(), observation()).is_ok());
    }

    // ── compare_generated_ir — exhaustiveness ──────────────────────────────

    #[test]
    fn compare_generated_ir_equal_ok() {
        assert!(compare_generated_ir(observation(), observation()).is_ok());
    }

    #[test]
    fn compare_generated_ir_unsupported_on_ir() {
        let ir = PublicObservation {
            unsupported_generated_subset: true,
            ..observation()
        };
        assert!(matches!(
            compare_generated_ir(ir, observation()),
            Err(DeterminismError::UnsupportedGeneratedSubset)
        ));
    }

    #[test]
    fn compare_generated_ir_unsupported_on_generated() {
        let generated = PublicObservation {
            unsupported_generated_subset: true,
            ..observation()
        };
        assert!(matches!(
            compare_generated_ir(observation(), generated),
            Err(DeterminismError::UnsupportedGeneratedSubset)
        ));
    }

    #[test]
    fn compare_generated_ir_divergence_on_result() {
        let generated = PublicObservation {
            result: TerminalResult::Failed,
            ..observation()
        };
        assert!(matches!(
            compare_generated_ir(observation(), generated),
            Err(DeterminismError::GeneratedIrDivergence)
        ));
    }

    #[test]
    fn compare_generated_ir_divergence_on_taint() {
        let generated = PublicObservation {
            taint: TaintStatus::Tainted,
            ..observation()
        };
        assert!(matches!(
            compare_generated_ir(observation(), generated),
            Err(DeterminismError::GeneratedIrDivergence)
        ));
    }

    #[test]
    fn compare_generated_ir_divergence_on_event_signature() {
        let generated = PublicObservation {
            event_signature: 99,
            ..observation()
        };
        assert!(matches!(
            compare_generated_ir(observation(), generated),
            Err(DeterminismError::GeneratedIrDivergence)
        ));
    }

    #[test]
    fn compare_generated_ir_divergence_on_payload_signature() {
        let generated = PublicObservation {
            event_payload_signature: 99,
            ..observation()
        };
        assert!(matches!(
            compare_generated_ir(observation(), generated),
            Err(DeterminismError::GeneratedIrDivergence)
        ));
    }

    #[test]
    fn compare_generated_ir_divergence_on_digest_status() {
        let generated = PublicObservation {
            digest_status: DigestStatus {
                workflow_source_matches: false,
                ..CLEAN_DIGESTS
            },
            ..observation()
        };
        assert!(matches!(
            compare_generated_ir(observation(), generated),
            Err(DeterminismError::GeneratedIrDivergence)
        ));
    }

    #[test]
    fn compare_generated_ir_divergence_on_replay_policy() {
        let generated = PublicObservation {
            replay_policy_blocked: true,
            ..observation()
        };
        assert!(matches!(
            compare_generated_ir(observation(), generated),
            Err(DeterminismError::GeneratedIrDivergence)
        ));
    }

    #[test]
    fn compare_generated_ir_divergence_on_semantic_slot() {
        let generated = PublicObservation {
            semantic_slot_signature: 9999,
            ..observation()
        };
        assert!(matches!(
            compare_generated_ir(observation(), generated),
            Err(DeterminismError::GeneratedIrDivergence)
        ));
    }

    #[test]
    fn compare_generated_ir_divergence_on_semantic_action() {
        let generated = PublicObservation {
            semantic_action_signature: 9999,
            ..observation()
        };
        assert!(matches!(
            compare_generated_ir(observation(), generated),
            Err(DeterminismError::GeneratedIrDivergence)
        ));
    }

    #[test]
    fn compare_generated_ir_divergence_on_semantic_suspension() {
        let generated = PublicObservation {
            semantic_suspension: true,
            ..observation()
        };
        assert!(matches!(
            compare_generated_ir(observation(), generated),
            Err(DeterminismError::GeneratedIrDivergence)
        ));
    }

    // ── Copy semantics for Copy types ──────────────────────────────────────

    #[test]
    fn terminal_result_is_copy() {
        let a = TerminalResult::Ok;
        let b = a;
        assert!(matches!(a, TerminalResult::Ok));
        assert!(matches!(b, TerminalResult::Ok));
    }

    #[test]
    fn taint_status_is_copy() {
        let a = TaintStatus::Clean;
        let b = a;
        assert!(matches!(a, TaintStatus::Clean));
        assert!(matches!(b, TaintStatus::Clean));
    }

    #[test]
    fn determinism_error_is_copy() {
        let a = DeterminismError::NondeterministicObservation;
        let b = a;
        assert!(matches!(a, DeterminismError::NondeterministicObservation));
        assert!(matches!(b, DeterminismError::NondeterministicObservation));
    }

    #[test]
    fn digest_status_is_copy() {
        let a = CLEAN_DIGESTS;
        let b = a;
        assert!(a.workflow_source_matches);
        assert!(b.workflow_source_matches);
    }

    #[test]
    fn public_observation_is_copy() {
        let a = observation();
        let b = a;
        assert!(matches!(a.result, TerminalResult::Ok));
        assert!(matches!(b.result, TerminalResult::Ok));
    }
}
