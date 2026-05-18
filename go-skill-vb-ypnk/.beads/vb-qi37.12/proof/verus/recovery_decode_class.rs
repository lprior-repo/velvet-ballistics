// Obligations: PO-006, VERUS-DEC-005.
// Abstract proof kernel for recovery-critical decode classification.

use vstd::prelude::*;

verus! {

pub enum SpecDecodeClass {
    AbsentOptionalPayload,
    ValidPayload,
    CorruptPayload,
    TruncatedPayload,
}

pub enum SpecRecoveryOutcome {
    SuccessfulAbsent,
    SuccessfulValue,
    TypedCorruptError,
    TypedTruncatedError,
}

pub open spec fn spec_decode_classification(classification: SpecDecodeClass) -> SpecRecoveryOutcome {
    match classification {
        SpecDecodeClass::AbsentOptionalPayload => SpecRecoveryOutcome::SuccessfulAbsent,
        SpecDecodeClass::ValidPayload => SpecRecoveryOutcome::SuccessfulValue,
        SpecDecodeClass::CorruptPayload => SpecRecoveryOutcome::TypedCorruptError,
        SpecDecodeClass::TruncatedPayload => SpecRecoveryOutcome::TypedTruncatedError,
    }
}

pub proof fn proof_corrupt_decode_not_absent_success(classification: SpecDecodeClass)
    ensures
        classification == SpecDecodeClass::CorruptPayload
            ==> spec_decode_classification(classification) != SpecRecoveryOutcome::SuccessfulAbsent,
        classification == SpecDecodeClass::TruncatedPayload
            ==> spec_decode_classification(classification) != SpecRecoveryOutcome::SuccessfulAbsent,
        spec_decode_classification(SpecDecodeClass::CorruptPayload) == SpecRecoveryOutcome::TypedCorruptError,
        spec_decode_classification(SpecDecodeClass::TruncatedPayload) == SpecRecoveryOutcome::TypedTruncatedError,
{
    match classification {
        SpecDecodeClass::AbsentOptionalPayload => {},
        SpecDecodeClass::ValidPayload => {},
        SpecDecodeClass::CorruptPayload => {},
        SpecDecodeClass::TruncatedPayload => {},
    }
}

} // verus!

fn main() {}
