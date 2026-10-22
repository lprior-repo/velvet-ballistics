// Independent oracle for computing expected input digests and policy
// identity values. Not a call to production LivePolicy.evaluate or
// encode_taint_canonical; matches production encoding for verification.

use blake3::Hasher;
use vb_core::ActionTicket;
use vb_core::policy::{
    CurrentPolicyIdentity, PinnedAdmissionPolicy, PolicyDigest,
};
use vb_core::value::{SlotValue, Taint};

fn serialize_input_bytes(value: i64) -> Result<Vec<u8>, String> {
    postcard::to_allocvec(&SlotValue::I64(value))
        .map_err(|e| format!("failed to serialize input: {e}"))
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
        Taint::Clean => 0,
        other => {
            return Err(format!("unexpected taint variant for oracle: {other:?}"));
        }
    };

    let mut hasher = new_policy_hasher();
    hash_input_domain(&mut hasher, &input_bytes);
    hash_taint_domain(&mut hasher, taint_byte);
    hash_ticket_domain_prefix(&mut hasher);
    hash_run_field(&mut hasher, u64::from(ticket.run.get()));
    hash_step_field(&mut hasher, u16::from(ticket.step.get()));
    hash_seq_field(&mut hasher, ticket.seq.get());
    hash_action_field(&mut hasher, u16::from(ticket.action.get()));
    hash_run_attempt_field(&mut hasher, ticket.run_attempt);
    hash_attempt_field(&mut hasher, ticket.attempt);
    hash_idempotency_key_field(&mut hasher, ticket.idempotency_key);
    hash_capacity_field(&mut hasher, ticket.capacity);

    Ok(finalize_policy_hash(hasher))
}

pub(super) fn admission_pin_from_artifact(policy_digest: [u8; 32]) -> PinnedAdmissionPolicy {
    PinnedAdmissionPolicy::Artifact(PolicyDigest::from_bytes(policy_digest))
}

pub(super) fn current_live_configured(digest: [u8; 32]) -> CurrentPolicyIdentity {
    CurrentPolicyIdentity::Configured {
        digest: PolicyDigest::from_bytes(digest),
    }
}