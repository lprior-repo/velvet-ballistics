#![forbid(unsafe_code)]

//! MASTER §77.17 compatibility gates for the storage layer.
//!
//! The algebra itself lives in this module. This module owns the
//! storage-side pieces: the BLAKE3 deciding-rule digest, journaling of
//! decisions that affect admission or live-effect dispatch, and the two
//! gate seams — admission and live dispatch.
//!
//! Gate split, by design:
//! - Admission stores the artifact and journals the decision. It blocks
//!   [`CompatibilityClass::MigrationRequired`] and
//!   [`CompatibilityClass::Rejected`]; a
//!   [`CompatibilityClass::ReplayOnly`] artifact may be admitted (stored
//!   for replay) because admission alone mints no live-effect authority.
//! - Live dispatch re-checks the journaled decision and blocks
//!   [`CompatibilityClass::ReplayOnly`] too: a replay-only decision never
//!   authorizes a new live effect.
//! - Every decision is journaled before enforcement, including blocked
//!   ones, so a refusal is itself on the durable record.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Semver-shaped schema version. Single-number wire versions (for example
/// journal schema versions) map to `major` with minor and patch zero via
/// [`SchemaVersion::from_major`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SchemaVersion {
    /// Major line. A change here breaks wire reading without migration.
    pub major: u16,
    /// Minor line. Additive change within a major line.
    pub minor: u16,
    /// Patch line. No structural change.
    pub patch: u16,
}

/// Current compiled-IR schema version. Admission stamps it onto accepted
/// artifacts; the admission/dispatch gates classify against it.
pub const CURRENT_IR_VERSION: SchemaVersion = SchemaVersion::new(2, 0, 0);

impl std::fmt::Display for SchemaVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl SchemaVersion {
    /// Builds a version triple.
    #[must_use]
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    /// Builds a version from a single wire number (minor and patch zero).
    #[must_use]
    pub const fn from_major(major: u16) -> Self {
        Self {
            major,
            minor: 0,
            patch: 0,
        }
    }
}

/// MASTER §77.17 compatibility classes, in strictness order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum CompatibilityClass {
    /// Held and required versions are equal.
    Identical = 0,
    /// Consumer is newer than the data within one major line; the consumer
    /// reads the older shape.
    BackwardCompatible = 1,
    /// Consumer is older than the data within one major line; the data
    /// adds only fields the older reader ignores.
    ForwardCompatible = 2,
    /// Major lines differ and a migration path exists; use is blocked
    /// until the data is migrated.
    MigrationRequired = 3,
    /// Major lines differ with no migration path, but the data stays
    /// readable for replay. It never authorizes new live effects.
    ReplayOnly = 4,
    /// No reading is safe: major lines differ with neither a migration
    /// path nor a replay allowance.
    Rejected = 5,
}

impl CompatibilityClass {
    /// True only for classes that may authorize a new live effect.
    #[must_use]
    pub const fn authorizes_live_effect(self) -> bool {
        matches!(
            self,
            Self::Identical | Self::BackwardCompatible | Self::ForwardCompatible
        )
    }

    /// True for every class except [`CompatibilityClass::Rejected`].
    /// Replay reads historical data as-is; it mints no new authority.
    #[must_use]
    pub const fn authorizes_replay(self) -> bool {
        !matches!(self, Self::Rejected)
    }

    /// True for classes that must block dispatch until resolved.
    #[must_use]
    pub const fn blocks_dispatch(self) -> bool {
        matches!(self, Self::MigrationRequired | Self::Rejected)
    }
}

/// The twelve MASTER §77.17 boundaries. Input and output schemas share one
/// boundary, matching the spec's "input/output schemas" item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum CompatBoundary {
    /// Compiled workflow IR versions.
    IrVersion = 0,
    /// Action ABI versions.
    ActionAbi = 1,
    /// Input and output schema versions.
    InputOutputSchema = 2,
    /// Policy input schema versions.
    PolicyInputSchema = 3,
    /// Signal schema versions.
    SignalSchema = 4,
    /// Journal event versions.
    JournalEventVersion = 5,
    /// Snapshot versions.
    SnapshotVersion = 6,
    /// Debug map versions.
    DebugMapVersion = 7,
    /// Resource contract versions.
    ResourceContract = 8,
    /// Incident bundle versions.
    IncidentBundle = 9,
    /// SDK ABI versions.
    SdkAbi = 10,
    /// Artifact envelope versions.
    ArtifactEnvelope = 11,
}

impl CompatBoundary {
    /// All twelve boundaries, in spec order.
    pub const ALL: [CompatBoundary; 12] = [
        CompatBoundary::IrVersion,
        CompatBoundary::ActionAbi,
        CompatBoundary::InputOutputSchema,
        CompatBoundary::PolicyInputSchema,
        CompatBoundary::SignalSchema,
        CompatBoundary::JournalEventVersion,
        CompatBoundary::SnapshotVersion,
        CompatBoundary::DebugMapVersion,
        CompatBoundary::ResourceContract,
        CompatBoundary::IncidentBundle,
        CompatBoundary::SdkAbi,
        CompatBoundary::ArtifactEnvelope,
    ];

    /// Stable name used in rule-digest encoding and diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::IrVersion => "ir_version",
            Self::ActionAbi => "action_abi",
            Self::InputOutputSchema => "input_output_schema",
            Self::PolicyInputSchema => "policy_input_schema",
            Self::SignalSchema => "signal_schema",
            Self::JournalEventVersion => "journal_event_version",
            Self::SnapshotVersion => "snapshot_version",
            Self::DebugMapVersion => "debug_map_version",
            Self::ResourceContract => "resource_contract",
            Self::IncidentBundle => "incident_bundle",
            Self::SdkAbi => "sdk_abi",
            Self::ArtifactEnvelope => "artifact_envelope",
        }
    }
}

/// The rule that decided a classification. This is what the decision
/// record's rule digest covers, distinct from the decision code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum CompatRule {
    /// Held and required versions are equal.
    IdenticalVersions = 0,
    /// Same major line; the consumer is newer than the data.
    SameMajorNewerConsumer = 1,
    /// Same major line; the consumer is older than the data.
    SameMajorOlderConsumer = 2,
    /// Major lines differ and a migration path is declared.
    MajorChangeMigrationAvailable = 3,
    /// Major lines differ with no migration path but a replay allowance.
    MajorChangeReplayOnly = 4,
    /// Major lines differ with neither migration nor replay allowance.
    MajorChangeUnsupported = 5,
}

impl CompatRule {
    /// Stable name used in rule-digest encoding and diagnostics.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::IdenticalVersions => "identical_versions",
            Self::SameMajorNewerConsumer => "same_major_newer_consumer",
            Self::SameMajorOlderConsumer => "same_major_older_consumer",
            Self::MajorChangeMigrationAvailable => "major_change_migration_available",
            Self::MajorChangeReplayOnly => "major_change_replay_only",
            Self::MajorChangeUnsupported => "major_change_unsupported",
        }
    }
}

/// Stable numeric decision code carried on the decision record.
/// 1xx allows, 2xx blocks with a recovery path, 3xx blocks outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CompatDecisionCode {
    /// Identical versions; live effects allowed.
    AllowIdentical = 100,
    /// Backward compatible; live effects allowed.
    AllowBackward = 101,
    /// Forward compatible; live effects allowed.
    AllowForward = 102,
    /// Migration required; dispatch blocked until migrated.
    BlockMigrationRequired = 200,
    /// Replay only; live effects forbidden, replay allowed.
    ReplayOnlyNoLiveEffects = 201,
    /// Rejected; nothing allowed.
    BlockRejected = 300,
}

impl CompatDecisionCode {
    /// Numeric code for diagnostics and wire stability.
    #[must_use]
    pub const fn code(self) -> u16 {
        match self {
            Self::AllowIdentical => 100,
            Self::AllowBackward => 101,
            Self::AllowForward => 102,
            Self::BlockMigrationRequired => 200,
            Self::ReplayOnlyNoLiveEffects => 201,
            Self::BlockRejected => 300,
        }
    }
}

/// Inputs to one classification. Bundled so no call site grows past the
/// argument-count limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompatInputs {
    /// Boundary the decision applies to.
    pub boundary: CompatBoundary,
    /// Version of the data or artifact at hand.
    pub held: SchemaVersion,
    /// Version the consumer understands.
    pub required: SchemaVersion,
    /// A migration path from the held major line to the required major
    /// line is declared for this boundary.
    pub migration_available: bool,
    /// The held major line is declared replayable without migration.
    pub replay_only: bool,
}

/// Digests carried on a decision record. Callers supply real digests;
/// this module never fabricates one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompatDigests {
    /// Digest of the deciding rule (domain-separated encoding of boundary
    /// plus rule; storage computes it with BLAKE3).
    pub rule_digest: [u8; 32],
    /// Digest of the held schema.
    pub old_schema_digest: [u8; 32],
    /// Digest of the required schema.
    pub new_schema_digest: [u8; 32],
}

/// Outcome of [`classify`]: class, deciding rule, and decision code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Classification {
    /// Compatibility class.
    pub class: CompatibilityClass,
    /// Rule that decided the class.
    pub rule: CompatRule,
    /// Numeric decision code for the class.
    pub code: CompatDecisionCode,
}

/// Pure §77.17 classification. Migration availability wins over a replay
/// allowance: when both are declared, the data must still migrate before
/// live use rather than silently falling back to replay-only.
#[must_use]
pub const fn classify(inputs: CompatInputs) -> Classification {
    if inputs.held.major == inputs.required.major
        && inputs.held.minor == inputs.required.minor
        && inputs.held.patch == inputs.required.patch
    {
        return Classification {
            class: CompatibilityClass::Identical,
            rule: CompatRule::IdenticalVersions,
            code: CompatDecisionCode::AllowIdentical,
        };
    }
    if inputs.held.major == inputs.required.major {
        if inputs.required.minor > inputs.held.minor
            || (inputs.required.minor == inputs.held.minor
                && inputs.required.patch > inputs.held.patch)
        {
            return Classification {
                class: CompatibilityClass::BackwardCompatible,
                rule: CompatRule::SameMajorNewerConsumer,
                code: CompatDecisionCode::AllowBackward,
            };
        }
        return Classification {
            class: CompatibilityClass::ForwardCompatible,
            rule: CompatRule::SameMajorOlderConsumer,
            code: CompatDecisionCode::AllowForward,
        };
    }
    if inputs.migration_available {
        return Classification {
            class: CompatibilityClass::MigrationRequired,
            rule: CompatRule::MajorChangeMigrationAvailable,
            code: CompatDecisionCode::BlockMigrationRequired,
        };
    }
    if inputs.replay_only {
        return Classification {
            class: CompatibilityClass::ReplayOnly,
            rule: CompatRule::MajorChangeReplayOnly,
            code: CompatDecisionCode::ReplayOnlyNoLiveEffects,
        };
    }
    Classification {
        class: CompatibilityClass::Rejected,
        rule: CompatRule::MajorChangeUnsupported,
        code: CompatDecisionCode::BlockRejected,
    }
}

/// Typed incompatibility failures, one variant per blocking class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum CompatError {
    /// Dispatch is blocked until the data migrates to the required line.
    #[error("compatibility blocked at {boundary:?}: migration required")]
    MigrationRequired {
        /// Boundary the decision applies to.
        boundary: CompatBoundary,
    },
    /// A replay-only decision never authorizes a new live effect.
    #[error(
        "compatibility blocked at {boundary:?}: replay-only data cannot authorize live effects"
    )]
    ReplayOnlyLiveEffect {
        /// Boundary the decision applies to.
        boundary: CompatBoundary,
    },
    /// The versions are incompatible with no migration or replay path.
    #[error("compatibility blocked at {boundary:?}: versions rejected")]
    Rejected {
        /// Boundary the decision applies to.
        boundary: CompatBoundary,
    },
}

/// Journaled §77.17 decision record. Every decision that affects admission,
/// replay, or live-effect dispatch is persisted with class, deciding-rule
/// digest, old/new schema digests, and decision code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompatDecision {
    /// Boundary the decision applies to.
    pub boundary: CompatBoundary,
    /// Compatibility class.
    pub class: CompatibilityClass,
    /// Rule that decided the class.
    pub rule: CompatRule,
    /// Digest of the deciding rule.
    pub rule_digest: [u8; 32],
    /// Digest of the held schema.
    pub old_schema_digest: [u8; 32],
    /// Digest of the required schema.
    pub new_schema_digest: [u8; 32],
    /// Numeric decision code for the class.
    pub decision_code: CompatDecisionCode,
}

impl CompatDecision {
    /// Classifies and builds the decision record from supplied digests.
    #[must_use]
    pub const fn decide(inputs: CompatInputs, digests: CompatDigests) -> Self {
        let classification = classify(inputs);
        Self {
            boundary: inputs.boundary,
            class: classification.class,
            rule: classification.rule,
            rule_digest: digests.rule_digest,
            old_schema_digest: digests.old_schema_digest,
            new_schema_digest: digests.new_schema_digest,
            decision_code: classification.code,
        }
    }

    /// Authorizes a new live effect, or fails with the typed blocking error.
    /// Replay-only decisions always fail here, by construction.
    pub const fn authorize_live_effect(self) -> Result<(), CompatError> {
        if self.class.authorizes_live_effect() {
            return Ok(());
        }
        match self.class {
            CompatibilityClass::MigrationRequired => Err(CompatError::MigrationRequired {
                boundary: self.boundary,
            }),
            CompatibilityClass::ReplayOnly => Err(CompatError::ReplayOnlyLiveEffect {
                boundary: self.boundary,
            }),
            CompatibilityClass::Rejected => Err(CompatError::Rejected {
                boundary: self.boundary,
            }),
            CompatibilityClass::Identical
            | CompatibilityClass::BackwardCompatible
            | CompatibilityClass::ForwardCompatible => Ok(()),
        }
    }

    /// Authorizes replay of the data. Only rejected data is unreadable;
    /// migration-required data replays as-is, and replay mints no new
    /// live-effect authority (see [`CompatDecision::authorize_live_effect`]).
    pub const fn authorize_replay(self) -> Result<(), CompatError> {
        if self.class.authorizes_replay() {
            Ok(())
        } else {
            Err(CompatError::Rejected {
                boundary: self.boundary,
            })
        }
    }

    /// True when the decision must block dispatch until resolved.
    #[must_use]
    pub const fn blocks_dispatch(self) -> bool {
        self.class.blocks_dispatch()
    }
}

use crate::{
    EventSeq, constants::CURRENT_SCHEMA_VERSION, error::JournalError, events::JournalEvent,
    journal::FjallJournal,
};
use vb_core::RunId;

/// Domain separation for deciding-rule digests.
const RULE_DIGEST_DOMAIN: &[u8] = b"velvet-ballistics/compat-rule/v1";

/// BLAKE3 digest of the deciding rule: domain, boundary name, rule name.
/// Distinct (boundary, rule) pairs always digest distinctly, and the
/// digest is stable across runs.
#[must_use]
pub fn compat_rule_digest(boundary: CompatBoundary, rule: CompatRule) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(
        RULE_DIGEST_DOMAIN
            .len()
            .saturating_add(boundary.name().len())
            .saturating_add(rule.name().len())
            .saturating_add(2),
    );
    bytes.extend_from_slice(RULE_DIGEST_DOMAIN);
    bytes.push(b'/');
    bytes.extend_from_slice(boundary.name().as_bytes());
    bytes.push(b'/');
    bytes.extend_from_slice(rule.name().as_bytes());
    *blake3::hash(&bytes).as_bytes()
}

/// Domain separation for schema-version digests.
const SCHEMA_VERSION_DIGEST_DOMAIN: &[u8] = b"velvet-ballistics/compat-schema-version/v1";

/// BLAKE3 digest of one schema version: domain plus the major, minor, and
/// patch lines as little-endian u16. Distinct versions always digest
/// distinctly, and the digest is stable across runs, so it serves as the
/// old/new schema digest on a decision record.
#[must_use]
pub fn schema_version_digest(version: SchemaVersion) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(SCHEMA_VERSION_DIGEST_DOMAIN.len().saturating_add(6));
    bytes.extend_from_slice(SCHEMA_VERSION_DIGEST_DOMAIN);
    bytes.extend_from_slice(&version.major.to_le_bytes());
    bytes.extend_from_slice(&version.minor.to_le_bytes());
    bytes.extend_from_slice(&version.patch.to_le_bytes());
    *blake3::hash(&bytes).as_bytes()
}

/// Classifies one boundary and builds the decision record, computing the
/// deciding-rule digest from the decided rule.
#[must_use]
pub fn decide_boundary(
    inputs: CompatInputs,
    old_schema_digest: [u8; 32],
    new_schema_digest: [u8; 32],
) -> CompatDecision {
    let classification = classify(inputs);
    CompatDecision::decide(
        inputs,
        CompatDigests {
            rule_digest: compat_rule_digest(inputs.boundary, classification.rule),
            old_schema_digest,
            new_schema_digest,
        },
    )
}

/// Appends a compatibility decision to the run journal under strict
/// durability. Callers supply the next sequence for the run.
pub fn journal_compat_decision(
    journal: &FjallJournal,
    run: RunId,
    seq: EventSeq,
    decision: CompatDecision,
) -> Result<(), JournalError> {
    journal.append_strict(&JournalEvent::CompatibilityDecisionEvent { run, seq, decision })
}

/// Failure of a compatibility gate: either the decision itself blocks,
/// or journaling the decision failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CompatGateError {
    /// The decision blocks this gate.
    #[error("compatibility gate blocked: {0}")]
    Compat(#[from] CompatError),
    /// Journaling the decision failed.
    #[error("compatibility decision journaling failed: {0}")]
    Journal(#[from] JournalError),
}

/// Admission gate seam: classifies, journals the decision, then blocks
/// migration-required and rejected data. Replay-only data passes
/// admission (storage for replay) and is stopped later at live dispatch.
pub fn gate_admission(
    journal: &FjallJournal,
    run: RunId,
    seq: EventSeq,
    inputs: CompatInputs,
    old_schema_digest: [u8; 32],
    new_schema_digest: [u8; 32],
) -> Result<CompatDecision, CompatGateError> {
    let decision = decide_boundary(inputs, old_schema_digest, new_schema_digest);
    journal_compat_decision(journal, run, seq, decision)?;
    match decision.class {
        CompatibilityClass::MigrationRequired => {
            Err(CompatGateError::Compat(CompatError::MigrationRequired {
                boundary: decision.boundary,
            }))
        }
        CompatibilityClass::Rejected => Err(CompatGateError::Compat(CompatError::Rejected {
            boundary: decision.boundary,
        })),
        CompatibilityClass::Identical
        | CompatibilityClass::BackwardCompatible
        | CompatibilityClass::ForwardCompatible
        | CompatibilityClass::ReplayOnly => Ok(decision),
    }
}

/// Live-dispatch gate seam: a replay-only decision never authorizes a new
/// live effect. Callers pass the decision journaled at admission.
pub fn gate_live_dispatch(decision: CompatDecision) -> Result<(), CompatError> {
    decision.authorize_live_effect()
}

/// One shared seam mapping u16 wire versions onto the §77.17 algebra.
///
/// `held` is the version on the bytes at hand; `required` is the version
/// the consumer understands. Both map through
/// [`SchemaVersion::from_major`]: single-number wire versions are major
/// lines with minor and patch zero. Every version barrier in this crate
/// routes through here instead of re-implementing the comparison, so
/// unknown versions fail closed in exactly one place.
#[must_use]
pub const fn wire_version_inputs(
    boundary: CompatBoundary,
    held: u16,
    required: u16,
    migration_available: bool,
    replay_only: bool,
) -> CompatInputs {
    CompatInputs {
        boundary,
        held: SchemaVersion::from_major(held),
        required: SchemaVersion::from_major(required),
        migration_available,
        replay_only,
    }
}

/// Inputs for one accepted artifact's IR version at the
/// [`CompatBoundary::IrVersion`] boundary. No IR migration path and no IR
/// replay allowance is declared anywhere in this tree, so a major-line
/// difference classifies `Rejected` and fails closed.
#[must_use]
pub const fn artifact_ir_version_inputs(held: SchemaVersion) -> CompatInputs {
    CompatInputs {
        boundary: CompatBoundary::IrVersion,
        held,
        required: crate::compat::CURRENT_IR_VERSION,
        migration_available: false,
        replay_only: false,
    }
}

/// Inputs for absent or inactive SDK metadata at the
/// [`CompatBoundary::SdkAbi`] boundary.
///
/// V1-min declares no SDK version line and activates no SDK, codegen, or
/// UI, so there is no migration path and no replay allowance: absent
/// metadata (held 0.0.0, no declared version) classifies
/// [`CompatibilityClass::Rejected`] and fails closed. Future SDK metadata
/// (a major line no consumer understands, no declared path) fails closed
/// the same way; callers encode it with [`wire_version_inputs`] at the
/// `SdkAbi` boundary.
#[must_use]
pub const fn sdk_absent_inputs() -> CompatInputs {
    CompatInputs {
        boundary: CompatBoundary::SdkAbi,
        held: SchemaVersion::from_major(0),
        required: SchemaVersion::from_major(CURRENT_SCHEMA_VERSION),
        migration_available: false,
        replay_only: false,
    }
}

/// Reads back every journaled compatibility decision for a run, in full
/// journal order. Full history, not snapshot+tail: admission-time
/// decisions below a durable snapshot must stay visible to enforcement.
pub fn compat_decisions_for_run(
    journal: &FjallJournal,
    run: RunId,
) -> Result<Vec<CompatDecision>, JournalError> {
    let mut decisions = Vec::new();
    for event in journal.events_for_run_full(run)? {
        if let JournalEvent::CompatibilityDecisionEvent { decision, .. } = event {
            decisions.push(decision);
        }
    }
    Ok(decisions)
}

/// Run-start enforcement seam: re-checks the run's journaled decisions
/// and blocks starting (or resuming for new live effects) when any of
/// them forbids live effects.
///
/// A fresh run with no journaled decisions passes unchanged. A run whose
/// history holds a replay-only decision cannot start new live effects;
/// replay of that history stays governed by `authorize_replay`, not by
/// this gate. Migration-required and rejected decisions block outright
/// with their typed errors. Migration never silently substitutes
/// defaults: the caller must migrate explicitly and journal a new
/// decision before this gate passes.
pub fn gate_run_start(
    journal: &FjallJournal,
    run: RunId,
) -> Result<Vec<CompatDecision>, CompatGateError> {
    let decisions = compat_decisions_for_run(journal, run)?;
    for decision in decisions.iter().copied() {
        gate_live_dispatch(decision)?;
    }
    Ok(decisions)
}

#[cfg(test)]
// Only deny-level lints are allowed here: the forbid-level lints
// (expect_used, panic, unwrap_used) cannot be allowed back, so the tests
// below use `?` instead of `expect`/`unwrap`.
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::compat::SchemaVersion;
    use crate::{RecordKind, constants::CURRENT_SCHEMA_VERSION, types::RecordEnvelope};
    use vb_core::{SlotIdx, StepIdx};

    fn inputs(
        boundary: CompatBoundary,
        held: SchemaVersion,
        required: SchemaVersion,
        migration_available: bool,
        replay_only: bool,
    ) -> CompatInputs {
        CompatInputs {
            boundary,
            held,
            required,
            migration_available,
            replay_only,
        }
    }

    fn v(major: u16, minor: u16, patch: u16) -> SchemaVersion {
        SchemaVersion::new(major, minor, patch)
    }

    fn temp_journal() -> Result<(tempfile::TempDir, FjallJournal), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let journal = FjallJournal::open(temp.path(), None)?;
        Ok((temp, journal))
    }

    fn journaled_decisions(
        journal: &FjallJournal,
        run: RunId,
    ) -> Result<Vec<CompatDecision>, JournalError> {
        Ok(journal
            .events_for_run(run)?
            .into_iter()
            .filter_map(|event| match event {
                JournalEvent::CompatibilityDecisionEvent { decision, .. } => Some(decision),
                _ => None,
            })
            .collect())
    }

    #[test]
    fn artifact_ir_version_inputs_classify_against_current() {
        use crate::compat::CURRENT_IR_VERSION;
        assert_eq!(CURRENT_IR_VERSION, v(2, 0, 0));
        let current = super::artifact_ir_version_inputs(v(2, 0, 0));
        assert_eq!(current.boundary, CompatBoundary::IrVersion);
        assert_eq!(current.required, CURRENT_IR_VERSION);
        assert!(!current.migration_available);
        assert!(!current.replay_only);
        assert_eq!(classify(current).class, CompatibilityClass::Identical);
        assert!(
            classify(current).class.authorizes_live_effect(),
            "identical IR version authorizes live effects"
        );
        let newer_minor = super::artifact_ir_version_inputs(v(2, 2, 0));
        assert_eq!(
            classify(newer_minor).class,
            CompatibilityClass::ForwardCompatible
        );
        assert!(
            classify(newer_minor).class.authorizes_live_effect(),
            "same-major newer artifact is forward compatible"
        );
        let legacy_major = super::artifact_ir_version_inputs(v(1, 0, 0));
        assert_eq!(classify(legacy_major).class, CompatibilityClass::Rejected);
        assert!(
            !classify(legacy_major).class.authorizes_live_effect(),
            "legacy postcard layout cannot authorize live effects"
        );
        let future_major = super::artifact_ir_version_inputs(v(3, 0, 0));
        assert_eq!(classify(future_major).class, CompatibilityClass::Rejected);
        assert!(
            !classify(future_major).class.authorizes_live_effect(),
            "undeclared major-line change fails closed"
        );
    }

    #[test]
    fn rule_digest_is_deterministic_and_rule_specific() {
        let first = compat_rule_digest(CompatBoundary::IrVersion, CompatRule::IdenticalVersions);
        let again = compat_rule_digest(CompatBoundary::IrVersion, CompatRule::IdenticalVersions);
        assert_eq!(first, again);
        assert_ne!(
            first,
            compat_rule_digest(
                CompatBoundary::IrVersion,
                CompatRule::SameMajorNewerConsumer
            )
        );
        assert_ne!(
            first,
            compat_rule_digest(CompatBoundary::ActionAbi, CompatRule::IdenticalVersions)
        );
        assert_ne!(first, [0u8; 32]);
    }

    #[test]
    fn schema_version_digest_is_deterministic_and_version_specific() {
        let first = schema_version_digest(v(1, 0, 0));
        let again = schema_version_digest(v(1, 0, 0));
        assert_eq!(first, again);
        assert_ne!(first, schema_version_digest(v(1, 0, 1)));
        assert_ne!(first, schema_version_digest(v(1, 1, 0)));
        assert_ne!(first, schema_version_digest(v(2, 0, 0)));
        assert_ne!(first, [0u8; 32]);
    }

    #[test]
    fn boundary_matrix_covers_all_twelve_boundaries() {
        let cases: [(
            CompatBoundary,
            SchemaVersion,
            SchemaVersion,
            bool,
            bool,
            CompatibilityClass,
        ); 12] = [
            (
                CompatBoundary::IrVersion,
                v(1, 0, 0),
                v(1, 0, 0),
                false,
                false,
                CompatibilityClass::Identical,
            ),
            (
                CompatBoundary::ActionAbi,
                v(1, 0, 0),
                v(1, 1, 0),
                false,
                false,
                CompatibilityClass::BackwardCompatible,
            ),
            (
                CompatBoundary::InputOutputSchema,
                v(1, 1, 0),
                v(1, 0, 0),
                false,
                false,
                CompatibilityClass::ForwardCompatible,
            ),
            (
                CompatBoundary::PolicyInputSchema,
                v(1, 0, 0),
                v(2, 0, 0),
                true,
                false,
                CompatibilityClass::MigrationRequired,
            ),
            (
                CompatBoundary::SignalSchema,
                v(1, 0, 0),
                v(2, 0, 0),
                false,
                true,
                CompatibilityClass::ReplayOnly,
            ),
            (
                CompatBoundary::JournalEventVersion,
                v(1, 0, 0),
                v(2, 0, 0),
                false,
                false,
                CompatibilityClass::Rejected,
            ),
            (
                CompatBoundary::SnapshotVersion,
                v(3, 2, 1),
                v(3, 2, 1),
                false,
                false,
                CompatibilityClass::Identical,
            ),
            (
                CompatBoundary::DebugMapVersion,
                v(2, 0, 0),
                v(2, 3, 0),
                false,
                false,
                CompatibilityClass::BackwardCompatible,
            ),
            (
                CompatBoundary::ResourceContract,
                v(2, 3, 0),
                v(2, 0, 0),
                false,
                false,
                CompatibilityClass::ForwardCompatible,
            ),
            (
                CompatBoundary::IncidentBundle,
                v(1, 0, 0),
                v(2, 0, 0),
                true,
                false,
                CompatibilityClass::MigrationRequired,
            ),
            (
                CompatBoundary::SdkAbi,
                v(1, 0, 0),
                v(2, 0, 0),
                false,
                true,
                CompatibilityClass::ReplayOnly,
            ),
            (
                CompatBoundary::ArtifactEnvelope,
                v(1, 0, 0),
                v(2, 0, 0),
                false,
                false,
                CompatibilityClass::Rejected,
            ),
        ];
        assert_eq!(CompatBoundary::ALL.len(), 12);
        for (boundary, held, required, migration, replay, class) in cases {
            let decision = decide_boundary(
                inputs(boundary, held, required, migration, replay),
                [1u8; 32],
                [2u8; 32],
            );
            assert_eq!(decision.class, class, "boundary {boundary:?}");
            assert_eq!(decision.boundary, boundary);
            assert_eq!(decision.old_schema_digest, [1u8; 32]);
            assert_eq!(decision.new_schema_digest, [2u8; 32]);
            assert_eq!(
                decision.rule_digest,
                compat_rule_digest(boundary, decision.rule),
                "boundary {boundary:?}"
            );
        }
    }

    #[test]
    fn journal_round_trip_for_decision_event() -> Result<(), Box<dyn std::error::Error>> {
        let (_temp, journal) = temp_journal()?;
        let run = RunId::new(7);
        let decision = decide_boundary(
            inputs(
                CompatBoundary::ArtifactEnvelope,
                v(1, 0, 0),
                v(2, 0, 0),
                false,
                true,
            ),
            [1u8; 32],
            [2u8; 32],
        );
        journal_compat_decision(&journal, run, EventSeq::new(0), decision)?;
        let found = journaled_decisions(&journal, run)?;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0], decision);
        assert_eq!(found[0].class, CompatibilityClass::ReplayOnly);
        Ok(())
    }

    #[test]
    fn admission_gate_blocks_migration_required_but_journals_first()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_temp, journal) = temp_journal()?;
        let run = RunId::new(11);
        let result = gate_admission(
            &journal,
            run,
            EventSeq::new(0),
            inputs(
                CompatBoundary::IrVersion,
                v(1, 0, 0),
                v(2, 0, 0),
                true,
                false,
            ),
            [1u8; 32],
            [2u8; 32],
        );
        assert!(matches!(
            result,
            Err(CompatGateError::Compat(CompatError::MigrationRequired {
                boundary: CompatBoundary::IrVersion,
            }))
        ));
        let found = journaled_decisions(&journal, run)?;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].class, CompatibilityClass::MigrationRequired);
        Ok(())
    }

    #[test]
    fn admission_gate_blocks_rejected() -> Result<(), Box<dyn std::error::Error>> {
        let (_temp, journal) = temp_journal()?;
        let run = RunId::new(12);
        let result = gate_admission(
            &journal,
            run,
            EventSeq::new(0),
            inputs(
                CompatBoundary::JournalEventVersion,
                v(1, 0, 0),
                v(2, 0, 0),
                false,
                false,
            ),
            [1u8; 32],
            [2u8; 32],
        );
        assert!(matches!(
            result,
            Err(CompatGateError::Compat(CompatError::Rejected { .. }))
        ));
        assert_eq!(journaled_decisions(&journal, run)?.len(), 1);
        Ok(())
    }

    #[test]
    fn replay_only_passes_admission_but_never_live_dispatch()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_temp, journal) = temp_journal()?;
        let run = RunId::new(13);
        let decision = gate_admission(
            &journal,
            run,
            EventSeq::new(0),
            inputs(CompatBoundary::SdkAbi, v(1, 0, 0), v(2, 0, 0), false, true),
            [1u8; 32],
            [2u8; 32],
        )?;
        assert_eq!(decision.class, CompatibilityClass::ReplayOnly);
        assert_eq!(
            gate_live_dispatch(decision),
            Err(CompatError::ReplayOnlyLiveEffect {
                boundary: CompatBoundary::SdkAbi,
            })
        );
        assert!(decision.authorize_replay().is_ok());
        Ok(())
    }

    #[test]
    fn compatible_versions_pass_both_gates() -> Result<(), Box<dyn std::error::Error>> {
        let (_temp, journal) = temp_journal()?;
        let run = RunId::new(14);
        let decision = gate_admission(
            &journal,
            run,
            EventSeq::new(0),
            inputs(
                CompatBoundary::ActionAbi,
                v(1, 0, 0),
                v(1, 1, 0),
                false,
                false,
            ),
            [1u8; 32],
            [2u8; 32],
        )?;
        assert_eq!(decision.class, CompatibilityClass::BackwardCompatible);
        assert!(gate_live_dispatch(decision).is_ok());
        Ok(())
    }

    #[test]
    fn journal_envelope_classification() {
        use crate::events::classify_journal_envelope;
        let event = JournalEvent::StepSucceeded {
            run: RunId::new(21),
            seq: EventSeq::new(1),
            step: StepIdx::new(0),
            output: SlotIdx::new(0),
        };
        let matching = RecordEnvelope {
            magic: 0,
            schema_version: CURRENT_SCHEMA_VERSION,
            record_kind: RecordKind::StepSucceeded.id(),
            sequence: 1,
        };
        assert_eq!(
            classify_journal_envelope(&matching, &event),
            CompatibilityClass::Identical
        );
        let legacy = RecordEnvelope {
            magic: 0,
            schema_version: CURRENT_SCHEMA_VERSION,
            record_kind: RecordKind::SlotWritten.id(),
            sequence: 1,
        };
        assert_eq!(
            classify_journal_envelope(&legacy, &event),
            CompatibilityClass::BackwardCompatible
        );
        let unknown_schema = RecordEnvelope {
            magic: 0,
            schema_version: 99,
            record_kind: RecordKind::StepSucceeded.id(),
            sequence: 1,
        };
        assert_eq!(
            classify_journal_envelope(&unknown_schema, &event),
            CompatibilityClass::Rejected
        );
        let mismatched = RecordEnvelope {
            magic: 0,
            schema_version: CURRENT_SCHEMA_VERSION,
            record_kind: RecordKind::ActionFailed.id(),
            sequence: 1,
        };
        assert_eq!(
            classify_journal_envelope(&mismatched, &event),
            CompatibilityClass::Rejected
        );
    }

    #[test]
    fn wire_seam_preserves_journal_barrier_errors() {
        use crate::codec::validation::validate_schema_version;
        assert!(validate_schema_version(CURRENT_SCHEMA_VERSION).is_ok());
        assert!(matches!(
            validate_schema_version(0),
            Err(JournalError::MigrationRequired {
                from: 0,
                to: CURRENT_SCHEMA_VERSION
            })
        ));
        for unknown in [2, 3, 99, u16::MAX] {
            assert!(
                matches!(
                    validate_schema_version(unknown),
                    Err(JournalError::UnsupportedSchemaVersion { version })
                    if version == unknown
                ),
                "unknown wire version {unknown} must fail closed"
            );
        }
    }

    #[test]
    fn evidence_routing_maps_legacy_to_migration_and_unknown_to_reject() {
        use crate::admission::verification::{EVIDENCE_VERSION, classify_evidence_held};
        assert!(matches!(
            classify_evidence_held(1),
            JournalError::MigrationRequired {
                from: 1,
                to: EVIDENCE_VERSION
            }
        ));
        assert!(matches!(
            classify_evidence_held(2),
            JournalError::MigrationRequired {
                from: 2,
                to: EVIDENCE_VERSION
            }
        ));
        for unknown in [0, 4, 99, u16::MAX] {
            assert!(
                matches!(
                    classify_evidence_held(unknown),
                    JournalError::UnsupportedSchemaVersion { version }
                    if version == unknown
                ),
                "unknown evidence version {unknown} must fail closed"
            );
        }
    }

    #[test]
    fn sdk_absent_and_future_metadata_fail_closed() {
        use crate::compat::{CompatInputs, classify};
        let absent = CompatDecision::decide(
            super::sdk_absent_inputs(),
            CompatDigests {
                rule_digest: super::compat_rule_digest(
                    CompatBoundary::SdkAbi,
                    CompatRule::MajorChangeUnsupported,
                ),
                old_schema_digest: [0u8; 32],
                new_schema_digest: [0u8; 32],
            },
        );
        assert_eq!(absent.class, CompatibilityClass::Rejected);
        assert_eq!(absent.boundary, CompatBoundary::SdkAbi);
        assert!(absent.blocks_dispatch());
        assert!(absent.authorize_live_effect().is_err());
        assert!(absent.authorize_replay().is_err());
        // Future SDK metadata: a major line no consumer understands, with
        // no declared migration or replay path, fails closed identically.
        for future in [2, 99, u16::MAX] {
            let inputs = CompatInputs {
                boundary: CompatBoundary::SdkAbi,
                held: SchemaVersion::from_major(future),
                required: SchemaVersion::from_major(CURRENT_SCHEMA_VERSION),
                migration_available: false,
                replay_only: false,
            };
            assert_eq!(
                classify(inputs).class,
                CompatibilityClass::Rejected,
                "future SDK major {future} must fail closed"
            );
        }
    }

    #[test]
    fn gate_run_start_blocks_journaled_replay_only_and_migration()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_temp, journal) = temp_journal()?;
        let run = RunId::new(31);
        // Fresh runs with no journaled decisions pass unchanged.
        assert!(super::gate_run_start(&journal, run)?.is_empty());
        // A journaled replay-only decision passes admission (storage for
        // replay) but blocks run start for new live effects.
        let replay_only = super::gate_admission(
            &journal,
            run,
            EventSeq::new(0),
            inputs(
                CompatBoundary::ArtifactEnvelope,
                v(1, 0, 0),
                v(2, 0, 0),
                false,
                true,
            ),
            [1u8; 32],
            [2u8; 32],
        )?;
        assert_eq!(replay_only.class, CompatibilityClass::ReplayOnly);
        assert!(
            matches!(
                super::gate_run_start(&journal, run),
                Err(CompatGateError::Compat(
                    CompatError::ReplayOnlyLiveEffect { boundary }
                )) if boundary == CompatBoundary::ArtifactEnvelope
            ),
            "journaled replay-only decision must block run start"
        );
        // Replay of that history is still allowed: the start gate governs
        // new live effects only.
        assert!(replay_only.authorize_replay().is_ok());
        // A journaled migration-required decision blocks run start with
        // its typed error; nothing is silently defaulted.
        let migration_run = RunId::new(32);
        assert!(
            super::gate_admission(
                &journal,
                migration_run,
                EventSeq::new(0),
                inputs(
                    CompatBoundary::IrVersion,
                    v(1, 0, 0),
                    v(2, 0, 0),
                    true,
                    false,
                ),
                [1u8; 32],
                [2u8; 32],
            )
            .is_err()
        );
        assert!(
            matches!(
                super::gate_run_start(&journal, migration_run),
                Err(CompatGateError::Compat(CompatError::MigrationRequired {
                    boundary
                })) if boundary == CompatBoundary::IrVersion
            ),
            "journaled migration-required decision must block run start"
        );
        Ok(())
    }

    #[test]
    fn reopen_preserves_decision_rule_and_schema_digests() -> Result<(), Box<dyn std::error::Error>>
    {
        let temp = tempfile::tempdir()?;
        let run = RunId::new(41);
        let wanted: [(CompatBoundary, SchemaVersion, SchemaVersion, bool, bool); 4] = [
            (
                CompatBoundary::IrVersion,
                v(1, 0, 0),
                v(1, 0, 0),
                false,
                false,
            ),
            (
                CompatBoundary::ActionAbi,
                v(1, 0, 0),
                v(2, 0, 0),
                true,
                false,
            ),
            (CompatBoundary::SdkAbi, v(1, 0, 0), v(2, 0, 0), false, true),
            (
                CompatBoundary::ArtifactEnvelope,
                v(1, 0, 0),
                v(2, 0, 0),
                false,
                false,
            ),
        ];
        let mut seq_no = 0u64;
        {
            let journal = FjallJournal::open(temp.path(), None)?;
            for (boundary, held, required, migration, replay) in wanted {
                let seq = EventSeq::new(seq_no);
                seq_no = seq_no.saturating_add(1);
                // Blocked decisions are journaled before enforcement
                // refuses them, so even the refusal is on the record.
                let _ = super::gate_admission(
                    &journal,
                    run,
                    seq,
                    inputs(boundary, held, required, migration, replay),
                    [0xABu8; 32],
                    [0xCDu8; 32],
                );
            }
        }
        let reopened = FjallJournal::open(temp.path(), None)?;
        let found = super::compat_decisions_for_run(&reopened, run)?;
        assert_eq!(found.len(), wanted.len());
        for (decision, (boundary, held, required, migration, replay)) in found.iter().zip(wanted) {
            let expected = super::decide_boundary(
                inputs(boundary, held, required, migration, replay),
                [0xABu8; 32],
                [0xCDu8; 32],
            );
            assert_eq!(*decision, expected);
            assert_eq!(decision.boundary, boundary);
            assert_eq!(decision.old_schema_digest, [0xABu8; 32]);
            assert_eq!(decision.new_schema_digest, [0xCDu8; 32]);
            assert_eq!(
                decision.rule_digest,
                super::compat_rule_digest(boundary, decision.rule)
            );
        }
        Ok(())
    }
}
