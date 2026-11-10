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
