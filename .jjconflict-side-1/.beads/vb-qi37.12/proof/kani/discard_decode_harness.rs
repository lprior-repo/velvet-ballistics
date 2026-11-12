// Obligations: PO-007.
// Standalone bounded state-space harness sketch. This bead-local artifact is not
// wired into Cargo because the user explicitly forbids production, dependency,
// CI, and test edits in this state.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Criticality {
    ReleaseCritical,
    NonCritical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Classification {
    Unclassified,
    MustPropagate,
    MustAccumulate,
    TypedOptional,
    TypedBestEffortDiscard,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DecodeClass {
    AbsentOptionalPayload,
    ValidPayload,
    CorruptPayload,
    TruncatedPayload,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RecoveryOutcome {
    SuccessfulAbsent,
    SuccessfulValue,
    TypedCorruptError,
    TypedTruncatedError,
}

fn accepts(criticality: Criticality, classification: Classification) -> bool {
    match criticality {
        Criticality::ReleaseCritical => matches!(
            classification,
            Classification::MustPropagate | Classification::MustAccumulate
        ),
        Criticality::NonCritical => !matches!(classification, Classification::Unclassified),
    }
}

fn decode(classification: DecodeClass) -> RecoveryOutcome {
    match classification {
        DecodeClass::AbsentOptionalPayload => RecoveryOutcome::SuccessfulAbsent,
        DecodeClass::ValidPayload => RecoveryOutcome::SuccessfulValue,
        DecodeClass::CorruptPayload => RecoveryOutcome::TypedCorruptError,
        DecodeClass::TruncatedPayload => RecoveryOutcome::TypedTruncatedError,
    }
}

#[kani::proof]
fn vb_qi37_12_discard_decode_state_space() {
    let classification: Classification = kani::any();
    let decode_class: DecodeClass = kani::any();

    if accepts(Criticality::ReleaseCritical, classification) {
        assert_ne!(classification, Classification::Unclassified);
        assert_ne!(classification, Classification::TypedOptional);
        assert_ne!(classification, Classification::TypedBestEffortDiscard);
    }

    if matches!(decode_class, DecodeClass::CorruptPayload | DecodeClass::TruncatedPayload) {
        assert_ne!(decode(decode_class), RecoveryOutcome::SuccessfulAbsent);
    }
}
