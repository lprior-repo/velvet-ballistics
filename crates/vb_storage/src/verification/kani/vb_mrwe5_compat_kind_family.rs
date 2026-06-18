#![forbid(unsafe_code)]
//! Kani harness for `obl-vb-mrwe-5-compat-kind-family-verus-016`.
//!
//! Production binding: compares the public storage kind-family classifier with
//! the shared MRWE5 kernel, then proves that cross-kind StepSucceeded /
//! SlotWritten envelope-payload pairs are rejected by the fail-closed semantic
//! decode policy.

use crate::RecordKind;
use crate::codec::{RecordKindFamilyDecision, classify_record_kind_family, is_known_record_kind};
use crate::constants::MAGIC_JOURNAL_EVENT;
use crate::mrwe5_contract::{
    MRWE5_JOURNAL_MAX_KIND_ID, MRWE5_JOURNAL_MIN_KIND_ID, MRWE5_MAGIC_JOURNAL_EVENT,
    MRWE5_SLOT_WRITTEN_KIND_ID, MRWE5_STEP_SUCCEEDED_KIND_ID, Mrwe5KindCompatibility,
    Mrwe5RecordKindFamilyDecision, Mrwe5SemanticDecodeDecision, mrwe5_classify_kind_compatibility,
    mrwe5_classify_record_kind_family, mrwe5_classify_semantic_decode,
};

fn raw_kind_within_kani_bound() -> u16 {
    let raw = kani::any::<u16>();
    kani::assume(raw < 64);
    raw
}

fn production_journal_family_accepts(kind: u16) -> bool {
    matches!(
        classify_record_kind_family(MAGIC_JOURNAL_EVENT, kind),
        RecordKindFamilyDecision::Accepted
    )
}

fn kernel_journal_family_accepts(kind: u16) -> bool {
    matches!(
        mrwe5_classify_record_kind_family(MRWE5_MAGIC_JOURNAL_EVENT, kind),
        Mrwe5RecordKindFamilyDecision::Accepted
    )
}

fn kind_in_contract_journal_range(kind: u16) -> bool {
    MRWE5_JOURNAL_MIN_KIND_ID <= kind && kind <= MRWE5_JOURNAL_MAX_KIND_ID
}

#[kani::proof]
pub fn kind_ids_and_legacy_policy_are_narrow() {
    let raw_kind = raw_kind_within_kani_bound();
    let production_family = production_journal_family_accepts(raw_kind);
    let kernel_family = kernel_journal_family_accepts(raw_kind);
    let contract_range = kind_in_contract_journal_range(raw_kind);

    kani::assert(
        MAGIC_JOURNAL_EVENT == MRWE5_MAGIC_JOURNAL_EVENT,
        "production journal magic matches MRWE5 kernel",
    );
    kani::assert(
        RecordKind::SlotWritten.id() == MRWE5_SLOT_WRITTEN_KIND_ID,
        "SlotWritten kind id matches MRWE5 kernel",
    );
    kani::assert(
        RecordKind::StepSucceeded.id() == MRWE5_STEP_SUCCEEDED_KIND_ID,
        "StepSucceeded kind id matches MRWE5 kernel",
    );
    kani::assert(
        production_family == kernel_family,
        "public classifier agrees with MRWE5 kernel",
    );
    kani::assert(
        production_family == contract_range,
        "journal family is exactly the MRWE5 contract range",
    );
    if production_family {
        kani::assert(
            is_known_record_kind(raw_kind),
            "accepted journal kind must be known",
        );
    }

    let step_payload = kani::any::<bool>();
    let payload_kind = if step_payload {
        MRWE5_STEP_SUCCEEDED_KIND_ID
    } else {
        MRWE5_SLOT_WRITTEN_KIND_ID
    };
    let envelope_kind = if step_payload {
        MRWE5_SLOT_WRITTEN_KIND_ID
    } else {
        MRWE5_STEP_SUCCEEDED_KIND_ID
    };
    let event_valid = kani::any::<bool>();

    kani::assert(
        envelope_kind != payload_kind,
        "legacy-like cross-kind pair is a real mismatch",
    );
    kani::assert(
        matches!(
            mrwe5_classify_kind_compatibility(envelope_kind, payload_kind),
            Mrwe5KindCompatibility::RejectedMismatch
        ),
        "cross-kind compatibility is fail-closed",
    );
    kani::assert(
        matches!(
            mrwe5_classify_semantic_decode(envelope_kind, payload_kind, event_valid),
            Mrwe5SemanticDecodeDecision::KindPayloadMismatch
        ),
        "cross-kind semantic decode never succeeds",
    );
}
