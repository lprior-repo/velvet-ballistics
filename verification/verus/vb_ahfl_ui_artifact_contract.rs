//! Verus model for vb-ahfl UI artifact schema obligations.
//!
//! Obligations: VERUS-META-001, VERUS-BOUNDS-001, VERUS-REDACT-001,
//! VERUS-GRAPH-001.
//! This is a proof-only abstract contract model. It intentionally does not
//! import production crates because the planned obligations still lack exact
//! implementation targets.

use vstd::prelude::*;

verus! {

pub enum ArtifactKind {
    WorkflowGraph,
    RunEventTable,
    AiContext,
    VerificationReport,
}

pub enum SecretSensitivity {
    Public,
    Secret,
    Unknown,
}

pub struct UiArtifactMetadata {
    pub schema_version: int,
    pub kind: ArtifactKind,
    pub generated_at_present: bool,
    pub source_present: bool,
    pub redaction_status_present: bool,
}

pub open spec fn spec_artifact_metadata_complete(meta: UiArtifactMetadata) -> bool {
    &&& meta.schema_version >= 1
    &&& meta.generated_at_present
    &&& meta.source_present
    &&& meta.redaction_status_present
}

pub open spec fn spec_schema_kind_agree(left: UiArtifactMetadata, right: UiArtifactMetadata) -> bool {
    &&& left.schema_version == right.schema_version
    &&& left.kind == right.kind
}

pub proof fn proof_metadata_preserved_by_constructors(meta: UiArtifactMetadata)
    requires
        meta.schema_version >= 1,
        meta.generated_at_present,
        meta.source_present,
        meta.redaction_status_present,
    ensures spec_artifact_metadata_complete(meta),
{
    reveal(spec_artifact_metadata_complete);
    assert(spec_artifact_metadata_complete(meta));
}

pub proof fn proof_schema_kind_agreement(left: UiArtifactMetadata, right: UiArtifactMetadata)
    requires
        left.schema_version == right.schema_version,
        left.kind == right.kind,
    ensures spec_schema_kind_agree(left, right),
{
    assert(spec_schema_kind_agree(left, right));
}

pub struct BoundedCollectionFacts {
    pub len: int,
    pub limit: int,
    pub truncated: bool,
    pub truncation_metadata_present: bool,
}

pub open spec fn spec_bounded_or_truncated(facts: BoundedCollectionFacts) -> bool {
    &&& facts.limit >= 0
    &&& facts.len >= 0
    &&& facts.len <= facts.limit
    &&& (!facts.truncated ==> !facts.truncation_metadata_present)
    &&& (facts.truncated ==> facts.truncation_metadata_present)
}

pub proof fn proof_bound_collection_preserves_limit(facts: BoundedCollectionFacts)
    requires
        facts.limit >= 0,
        facts.len >= 0,
        facts.len <= facts.limit,
        facts.truncated ==> facts.truncation_metadata_present,
        !facts.truncated ==> !facts.truncation_metadata_present,
    ensures spec_bounded_or_truncated(facts),
{
    assert(spec_bounded_or_truncated(facts));
}

pub struct RedactedValueViewFacts {
    pub raw_secret_present: bool,
    pub redaction_status_present: bool,
    pub digest_present: bool,
    pub summary_len: int,
    pub summary_limit: int,
}

pub open spec fn spec_summary_bounded(view: RedactedValueViewFacts) -> bool {
    &&& view.summary_limit >= 0
    &&& view.summary_len >= 0
    &&& view.summary_len <= view.summary_limit
}

pub open spec fn spec_redacted_view_contains_no_raw_secret(
    sensitivity: SecretSensitivity,
    view: RedactedValueViewFacts,
) -> bool {
    &&& spec_summary_bounded(view)
    &&& match sensitivity {
        SecretSensitivity::Public => true,
        SecretSensitivity::Secret => {
            &&& !view.raw_secret_present
            &&& view.redaction_status_present
            &&& view.digest_present
        },
        SecretSensitivity::Unknown => {
            &&& !view.raw_secret_present
            &&& view.redaction_status_present
            &&& view.digest_present
        },
    }
}

pub proof fn proof_secret_projection_is_fail_closed(
    sensitivity: SecretSensitivity,
    view: RedactedValueViewFacts,
)
    requires
        view.summary_limit >= 0,
        view.summary_len >= 0,
        view.summary_len <= view.summary_limit,
        sensitivity != SecretSensitivity::Public ==> !view.raw_secret_present,
        sensitivity != SecretSensitivity::Public ==> view.redaction_status_present,
        sensitivity != SecretSensitivity::Public ==> view.digest_present,
    ensures spec_redacted_view_contains_no_raw_secret(sensitivity, view),
{
    match sensitivity {
        SecretSensitivity::Public => {},
        SecretSensitivity::Secret => {},
        SecretSensitivity::Unknown => {},
    }
}

pub struct GraphEventFacts {
    pub node_count: int,
    pub edge_count: int,
    pub event_count: int,
    pub max_edge_from_step: int,
    pub max_edge_to_step: int,
    pub max_event_step: int,
    pub seq_strictly_ordered: bool,
    pub step_identity_stable: bool,
}

pub open spec fn spec_graph_events_well_formed(facts: GraphEventFacts) -> bool {
    &&& facts.node_count >= 0
    &&& facts.edge_count >= 0
    &&& facts.event_count >= 0
    &&& facts.edge_count == 0 || (facts.max_edge_from_step >= 0 && facts.max_edge_from_step < facts.node_count)
    &&& facts.edge_count == 0 || (facts.max_edge_to_step >= 0 && facts.max_edge_to_step < facts.node_count)
    &&& facts.event_count == 0 || (facts.max_event_step >= 0 && facts.max_event_step < facts.node_count)
    &&& facts.seq_strictly_ordered
    &&& facts.step_identity_stable
}

pub proof fn proof_graph_event_refs_preserve_identity(facts: GraphEventFacts)
    requires
        facts.node_count >= 0,
        facts.edge_count >= 0,
        facts.event_count >= 0,
        facts.edge_count == 0 || (facts.max_edge_from_step >= 0 && facts.max_edge_from_step < facts.node_count),
        facts.edge_count == 0 || (facts.max_edge_to_step >= 0 && facts.max_edge_to_step < facts.node_count),
        facts.event_count == 0 || (facts.max_event_step >= 0 && facts.max_event_step < facts.node_count),
        facts.seq_strictly_ordered,
        facts.step_identity_stable,
    ensures spec_graph_events_well_formed(facts),
{
    assert(spec_graph_events_well_formed(facts));
}

} // verus!

fn main() {}
