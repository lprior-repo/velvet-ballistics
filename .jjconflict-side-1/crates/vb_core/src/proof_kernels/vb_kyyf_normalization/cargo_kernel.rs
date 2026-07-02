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
