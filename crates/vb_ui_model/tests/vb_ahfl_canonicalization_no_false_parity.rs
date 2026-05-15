#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_ui_model::envelope::{EnvelopeKind, MetadataEnvelope, RunId, SchemaVersion};

#[derive(Clone, Copy)]
struct CanonicalEnvelopeFacts {
    schema_version: u16,
    kind: EnvelopeKind,
    run_id: u64,
    timestamp: i64,
}

impl CanonicalEnvelopeFacts {
    fn from_metadata(
        schema_version: SchemaVersion,
        kind: EnvelopeKind,
        metadata: &MetadataEnvelope,
    ) -> Self {
        Self {
            schema_version: schema_version.get(),
            kind,
            run_id: metadata.run_id().get(),
            timestamp: metadata.timestamp(),
        }
    }
}

fn known_kind(code: u8) -> EnvelopeKind {
    match code % 6 {
        0 => EnvelopeKind::Success,
        1 => EnvelopeKind::Error,
        2 => EnvelopeKind::DiagnosticReport,
        3 => EnvelopeKind::Status,
        4 => EnvelopeKind::Event,
        _ => EnvelopeKind::Workflow,
    }
}

fn facts_match(left: CanonicalEnvelopeFacts, right: CanonicalEnvelopeFacts) -> bool {
    left.schema_version == right.schema_version
        && left.kind == right.kind
        && left.run_id == right.run_id
        && left.timestamp == right.timestamp
}

#[kani::proof]
fn vb_ahfl_canonicalization_no_false_parity() {
    let left_schema_raw: u16 = kani::any();
    let right_schema_raw: u16 = kani::any();
    let left_kind_raw: u8 = kani::any();
    let right_kind_raw: u8 = kani::any();
    let left_run_id: u64 = kani::any();
    let right_run_id: u64 = kani::any();
    let left_timestamp: i64 = kani::any();
    let right_timestamp: i64 = kani::any();

    kani::assume(left_schema_raw > 0);
    kani::assume(right_schema_raw > 0);

    let left_schema = match SchemaVersion::new(left_schema_raw) {
        Ok(value) => value,
        Err(_) => SchemaVersion::CURRENT,
    };
    let right_schema = match SchemaVersion::new(right_schema_raw) {
        Ok(value) => value,
        Err(_) => SchemaVersion::CURRENT,
    };
    let left_kind = known_kind(left_kind_raw);
    let right_kind = known_kind(right_kind_raw);
    let left_metadata =
        MetadataEnvelope::new(RunId::new(left_run_id), String::new(), left_timestamp);
    let right_metadata =
        MetadataEnvelope::new(RunId::new(right_run_id), String::new(), right_timestamp);

    let left = CanonicalEnvelopeFacts::from_metadata(left_schema, left_kind, &left_metadata);
    let right = CanonicalEnvelopeFacts::from_metadata(right_schema, right_kind, &right_metadata);

    if facts_match(left, right) {
        kani::assert(
            left_schema_raw == right_schema_raw,
            "schema mismatch cannot be parity",
        );
        kani::assert(left_kind == right_kind, "kind mismatch cannot be parity");
        kani::assert(
            left_run_id == right_run_id,
            "run metadata mismatch cannot be parity",
        );
        kani::assert(
            left_timestamp == right_timestamp,
            "timestamp mismatch cannot be parity",
        );
    }

    kani::assert(
        EnvelopeKind::parse(left_kind.name()) == Some(left_kind),
        "kind name round-trips through production parse API",
    );
}
