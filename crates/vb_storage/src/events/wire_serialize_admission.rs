#![forbid(unsafe_code)]

use super::super::super::JournalEvent;
use super::super::tags::{
    TAG_RUN_ADMISSION, TAG_RUN_ADMISSION_BOUND, TAG_RUN_ADMISSION_BOUND_PINNED,
    TAG_RUN_ADMISSION_PINNED,
};
use super::{serialize_record_kind_variant, serialize_routing_error};
use serde::Serializer;

type AdmissionFields<'a> = (
    &'a vb_core::RunId,
    &'a crate::EventSeq,
    &'a vb_core::WorkflowDigest,
    &'a vb_core::CapabilitySet,
    &'a vb_core::RuntimePolicy,
);

pub(super) fn serialize_admission<S>(event: &JournalEvent, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match event {
        JournalEvent::RunAdmission {
            run,
            seq,
            artifact_digest,
            input_digest,
            policy_pin,
            granted_capabilities: caps,
            policy,
        } => serialize_admission_payload(
            serializer,
            (run, seq, artifact_digest, caps, policy),
            input_digest,
            policy_pin,
        ),
        _ => serialize_routing_error::<S>(),
    }
}

fn serialize_admission_payload<S>(
    serializer: S,
    fields: AdmissionFields<'_>,
    binding: &crate::RunInputBinding,
    pin: &crate::RunPolicyPin,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match (binding, pin) {
        (crate::RunInputBinding::LegacyUnbound, crate::RunPolicyPin::Unverified) => {
            serialize_record_kind_variant(serializer, TAG_RUN_ADMISSION, "RunAdmission", &fields)
        }
        (crate::RunInputBinding::Digest(digest), crate::RunPolicyPin::Unverified) => {
            serialize_bound_admission_payload(serializer, fields, digest)
        }
        (_, crate::RunPolicyPin::Pinned(policy_digest)) => {
            serialize_pinned_admission_payload(serializer, fields, binding, policy_digest)
        }
    }
}

fn serialize_pinned_admission_payload<S>(
    serializer: S,
    fields: AdmissionFields<'_>,
    binding: &crate::RunInputBinding,
    policy_digest: &vb_core::WorkflowDigest,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match binding {
        crate::RunInputBinding::LegacyUnbound => {
            serialize_pinned_unbound_admission(serializer, fields, policy_digest)
        }
        crate::RunInputBinding::Digest(input_digest) => {
            serialize_pinned_bound_admission(serializer, fields, input_digest, policy_digest)
        }
    }
}

fn serialize_pinned_unbound_admission<S>(
    serializer: S,
    fields: AdmissionFields<'_>,
    policy_digest: &vb_core::WorkflowDigest,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let (run, seq, artifact, caps, policy) = fields;
    let payload = (run, seq, artifact, policy_digest, caps, policy);
    serialize_record_kind_variant(
        serializer,
        TAG_RUN_ADMISSION_PINNED,
        "RunAdmission",
        &payload,
    )
}

fn serialize_pinned_bound_admission<S>(
    serializer: S,
    fields: AdmissionFields<'_>,
    input_digest: &vb_core::WorkflowDigest,
    policy_digest: &vb_core::WorkflowDigest,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let (run, seq, artifact, caps, policy) = fields;
    let payload = (
        run,
        seq,
        artifact,
        input_digest,
        policy_digest,
        caps,
        policy,
    );
    serialize_record_kind_variant(
        serializer,
        TAG_RUN_ADMISSION_BOUND_PINNED,
        "RunAdmission",
        &payload,
    )
}

fn serialize_bound_admission_payload<S>(
    serializer: S,
    fields: AdmissionFields<'_>,
    input_digest: &vb_core::WorkflowDigest,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let (run, seq, artifact, caps, policy) = fields;
    let payload = (run, seq, artifact, input_digest, caps, policy);
    serialize_record_kind_variant(
        serializer,
        TAG_RUN_ADMISSION_BOUND,
        "RunAdmission",
        &payload,
    )
}
