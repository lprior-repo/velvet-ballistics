// Obligations: PO-004, VERUS-CLS-003.
// Abstract proof kernel for release-critical discard classification.

use vstd::prelude::*;

verus! {

pub enum SpecCriticality {
    ReleaseCritical,
    NonCritical,
}

pub enum SpecClassification {
    Unclassified,
    MustPropagate,
    MustAccumulate,
    TypedOptional,
    TypedBestEffortDiscard,
}

pub open spec fn spec_classified(classification: SpecClassification) -> bool {
    classification != SpecClassification::Unclassified
}

pub open spec fn spec_release_critical_accepts(
    criticality: SpecCriticality,
    classification: SpecClassification,
) -> bool {
    match criticality {
        SpecCriticality::ReleaseCritical =>
            classification == SpecClassification::MustPropagate
                || classification == SpecClassification::MustAccumulate,
        SpecCriticality::NonCritical => spec_classified(classification),
    }
}

pub proof fn proof_no_implicit_discard_acceptance(
    criticality: SpecCriticality,
    classification: SpecClassification,
)
    ensures
        spec_release_critical_accepts(criticality, classification) ==> spec_classified(classification),
        spec_release_critical_accepts(SpecCriticality::ReleaseCritical, classification)
            ==> classification != SpecClassification::TypedBestEffortDiscard,
        spec_release_critical_accepts(SpecCriticality::ReleaseCritical, classification)
            ==> classification != SpecClassification::TypedOptional,
        spec_release_critical_accepts(SpecCriticality::ReleaseCritical, classification)
            ==> classification != SpecClassification::Unclassified,
{
    match criticality {
        SpecCriticality::ReleaseCritical => {
            match classification {
                SpecClassification::Unclassified => {},
                SpecClassification::MustPropagate => {},
                SpecClassification::MustAccumulate => {},
                SpecClassification::TypedOptional => {},
                SpecClassification::TypedBestEffortDiscard => {},
            }
        },
        SpecCriticality::NonCritical => {
            match classification {
                SpecClassification::Unclassified => {},
                SpecClassification::MustPropagate => {},
                SpecClassification::MustAccumulate => {},
                SpecClassification::TypedOptional => {},
                SpecClassification::TypedBestEffortDiscard => {},
            }
        },
    }
}

} // verus!

fn main() {}
