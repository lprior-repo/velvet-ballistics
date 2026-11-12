use std::sync::Arc;

use blake3::Hasher;
use vb_core::ActionTicket;
use vb_core::policy::{
    CurrentPolicyIdentity, DecisionCode, InfluenceState, PinnedAdmissionPolicy, PolicyDigest,
    PolicyLayer,
};
use vb_core::value::{SlotValue, Taint};
use vb_storage::FjallJournal;

use super::*;

// Oracle: compute the expected input digest independently of production code.
// Uses canonical I64 bytes, byte 0 for Clean taint, and all 8 ticket fields
// with domain separation. This matches the production encoding but is the
// test's own oracle, not a call to LivePolicy.evaluate or encode_taint_canonical.

fn serialize_input_bytes(value: i64) -> Result<Vec<u8>, String> {
    postcard::to_allocvec(&SlotValue::I64(value))
        .map_err(|e| format!("failed to serialize input: {e}"))
}

fn taint_byte_clean() -> u8 {
    // Canonical encoding: Clean = 0
    0
}

fn new_policy_hasher() -> Hasher {
    Hasher::new()
}

fn hash_input_domain(hasher: &mut Hasher, input_bytes: &[u8]) {
    hasher.update(b"velvet-ballistics:policy:input:");
    hasher.update(input_bytes);
}

fn hash_taint_domain(hasher: &mut Hasher, taint_byte: u8) {
    hasher.update(b"velvet-ballistics:policy:taint:");
    hasher.update(&[taint_byte]);
}

fn hash_ticket_domain_prefix(hasher: &mut Hasher) {
    hasher.update(b"velvet-ballistics:policy:ticket:");
}

fn hash_run_field(hasher: &mut Hasher, run: u64) {
    hasher.update(b"run:");
    hasher.update(&run.to_be_bytes());
}

fn hash_step_field(hasher: &mut Hasher, step: u16) {
    hasher.update(b"step:");
    hasher.update(&step.to_be_bytes());
}

fn hash_seq_field(hasher: &mut Hasher, seq: u64) {
    hasher.update(b"seq:");
    hasher.update(&seq.to_be_bytes());
}

fn hash_action_field(hasher: &mut Hasher, action: u16) {
    hasher.update(b"action:");
    hasher.update(&action.to_be_bytes());
}

fn hash_run_attempt_field(hasher: &mut Hasher, run_attempt: u16) {
    hasher.update(b"run_attempt:");
    hasher.update(&run_attempt.to_be_bytes());
}

fn hash_attempt_field(hasher: &mut Hasher, attempt: u16) {
    hasher.update(b"attempt:");
    hasher.update(&attempt.to_be_bytes());
}

fn hash_idempotency_key_field(hasher: &mut Hasher, key: u128) {
    hasher.update(b"key:");
    hasher.update(&key.to_be_bytes());
}

fn hash_capacity_field(hasher: &mut Hasher, capacity: u16) {
    hasher.update(b"capacity:");
    hasher.update(&capacity.to_be_bytes());
}

fn finalize_policy_hash(hasher: Hasher) -> PolicyDigest {
    let result = hasher.finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(result.as_bytes());
    PolicyDigest::from_bytes(output)
}

pub(super) fn compute_expected_input_digest(
    value: i64,
    taint: Taint,
    ticket: &ActionTicket,
) -> Result<PolicyDigest, String> {
    let input_bytes = serialize_input_bytes(value)?;
    let taint_byte = match taint {
        Taint::Clean => taint_byte_clean(),
        other => return Err(format!("unexpected taint variant for oracle: {other:?}")),
    };

    let mut hasher = new_policy_hasher();
    hash_input_domain(&mut hasher, &input_bytes);
    hash_taint_domain(&mut hasher, taint_byte);
    hash_ticket_domain_prefix(&mut hasher);
    hash_run_field(&mut hasher, ticket.run.get());
    hash_step_field(&mut hasher, ticket.step.get());
    hash_seq_field(&mut hasher, ticket.seq.get());
    hash_action_field(&mut hasher, ticket.action.get());
    hash_run_attempt_field(&mut hasher, ticket.run_attempt);
    hash_attempt_field(&mut hasher, ticket.attempt);
    hash_idempotency_key_field(&mut hasher, ticket.idempotency_key);
    hash_capacity_field(&mut hasher, ticket.capacity);

    Ok(finalize_policy_hash(hasher))
}

pub(super) fn make_expected_decision(
    ticket: ActionTicket,
    admission_pin: PinnedAdmissionPolicy,
    current_live: CurrentPolicyIdentity,
    input_digest: PolicyDigest,
    outcome: DecisionCode,
) -> vb_core::policy::DecisionRecord {
    vb_core::policy::DecisionRecord {
        ticket,
        admission: admission_pin,
        current_live,
        input_digest,
        taint: Taint::Clean,
        outcome,
        layer: PolicyLayer::Live,
        emergency_influence: InfluenceState::Inactive,
        compat_influence: InfluenceState::NotEvaluated,
    }
}

pub(super) fn admission_pin_from_artifact(policy_digest: [u8; 32]) -> PinnedAdmissionPolicy {
    PinnedAdmissionPolicy::Artifact(PolicyDigest::from_bytes(policy_digest))
}

pub(super) fn current_live_configured(digest: [u8; 32]) -> CurrentPolicyIdentity {
    CurrentPolicyIdentity::Configured {
        digest: PolicyDigest::from_bytes(digest),
    }
}

pub(super) fn read_journal_decision_events(
    journal: &Arc<FjallJournal>,
    run: RunId,
) -> Result<Vec<vb_core::policy::DecisionRecord>, Box<dyn std::error::Error>> {
    let events = vb_storage::read_run_events(journal, run)?;
    Ok(events
        .iter()
        .filter_map(|e| match e {
            vb_storage::JournalEvent::PolicyDecisionEvent { decision, .. } => {
                Some(decision.clone())
            }
            _ => None,
        })
        .collect())
}
