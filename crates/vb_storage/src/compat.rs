#![forbid(unsafe_code)]

//! MASTER §77.17 compatibility gates for the storage layer.
//!
//! The algebra itself lives in [`vb_core::compat`]. This module owns the
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

use crate::{EventSeq, error::JournalError, events::JournalEvent, journal::FjallJournal};
use vb_core::RunId;
use vb_core::compat::{
    CompatBoundary, CompatDecision, CompatDigests, CompatError, CompatInputs, CompatRule,
    CompatibilityClass,
};

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

/// Classifies one boundary and builds the decision record, computing the
/// deciding-rule digest from the decided rule.
#[must_use]
pub fn decide_boundary(
    inputs: CompatInputs,
    old_schema_digest: [u8; 32],
    new_schema_digest: [u8; 32],
) -> CompatDecision {
    let classification = vb_core::compat::classify(inputs);
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
        // CompatibilityClass is non_exhaustive: an unknown future class
        // fails closed rather than passing admission.
        _ => Err(CompatGateError::Compat(CompatError::Rejected {
            boundary: decision.boundary,
        })),
    }
}

/// Live-dispatch gate seam: a replay-only decision never authorizes a new
/// live effect. Callers pass the decision journaled at admission.
pub fn gate_live_dispatch(decision: CompatDecision) -> Result<(), CompatError> {
    decision.authorize_live_effect()
}

#[cfg(test)]
// Only deny-level lints are allowed here: the forbid-level lints
// (expect_used, panic, unwrap_used) cannot be allowed back, so the tests
// below use `?` instead of `expect`/`unwrap`.
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::{RecordKind, constants::CURRENT_SCHEMA_VERSION, types::RecordEnvelope};
    use vb_core::compat::SchemaVersion;
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
}
