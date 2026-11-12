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
