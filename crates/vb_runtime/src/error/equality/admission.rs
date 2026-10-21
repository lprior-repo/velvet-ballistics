use super::RuntimeError;

pub(super) fn runtime_error_admission_field_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    runtime_error_admission_digest_eq(left, right)
        || runtime_error_admission_capability_eq(left, right)
}

fn runtime_error_admission_digest_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    match (left, right) {
        (
            RuntimeError::AdmissionArtifactNotFound { digest: a },
            RuntimeError::AdmissionArtifactNotFound { digest: b },
        ) => a == b,
        (
            RuntimeError::AdmissionArtifactInvalid { digest: a },
            RuntimeError::AdmissionArtifactInvalid { digest: b },
        ) => a == b,
        (
            RuntimeError::AdmissionArtifactDigestMismatch {
                requested: a,
                found: c,
            },
            RuntimeError::AdmissionArtifactDigestMismatch {
                requested: b,
                found: d,
            },
        ) => a == b && c == d,
        (
            RuntimeError::AdmissionArtifactStale { digest: a },
            RuntimeError::AdmissionArtifactStale { digest: b },
        ) => a == b,
        (
            RuntimeError::AdmissionDigestMismatch {
                requested: a,
                record: c,
                envelope: e,
            },
            RuntimeError::AdmissionDigestMismatch {
                requested: b,
                record: d,
                envelope: f,
            },
        ) => a == b && c == d && e == f,
        _ => false,
    }
}

fn runtime_error_admission_capability_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    admission_capability_denied_eq(left, right)
        || admission_capability_count_mismatch_eq(left, right)
        || admission_action_registry_mismatch_eq(left, right)
        || admission_action_abi_digest_mismatch_eq(left, right)
}

fn admission_capability_denied_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    matches!(
        (left, right),
        (
            RuntimeError::AdmissionCapabilityDenied { action: la, required: lb, granted: lc },
            RuntimeError::AdmissionCapabilityDenied { action: ra, required: rb, granted: rc },
        ) if la == ra && lb == rb && lc == rc
    )
}

fn admission_capability_count_mismatch_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    matches!(
        (left, right),
        (
            RuntimeError::AdmissionCapabilityCountMismatch { required_count: lc, granted_count: lg },
            RuntimeError::AdmissionCapabilityCountMismatch { required_count: rc, granted_count: rg },
        ) if lc == rc && lg == rg
    )
}

fn admission_action_registry_mismatch_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    matches!(
        (left, right),
        (
            RuntimeError::AdmissionActionRegistryMismatch { digest: ld, expected_count: le, submitted_count: ls },
            RuntimeError::AdmissionActionRegistryMismatch { digest: rd, expected_count: re, submitted_count: rs },
        ) if ld == rd && le == re && ls == rs
    )
}

fn admission_action_abi_digest_mismatch_eq(left: &RuntimeError, right: &RuntimeError) -> bool {
    matches!(
        (left, right),
        (
            RuntimeError::AdmissionActionAbiDigestMismatch { action: la, expected: le, submitted: ls },
            RuntimeError::AdmissionActionAbiDigestMismatch { action: ra, expected: re, submitted: rs },
        ) if la == ra && le == re && ls == rs
    )
}
