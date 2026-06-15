#![forbid(unsafe_code)]

#[derive(Clone, Copy)]
enum AdmissionError {
    AcceptedArtifactMissing,
    AcceptedArtifactInvalid,
    CompiledIrDigestMismatch,
    CapabilityMismatch,
}

enum AdmissionOutcome {
    Admitted,
    Rejected(AdmissionError),
}

#[derive(Clone, Copy)]
struct AcceptedArtifactModel {
    envelope_present: bool,
    gate_count: u8,
    proof_flags_ok: bool,
    artifact_digest_ok: bool,
    capability_grant_ok: bool,
}

const REQUIRED_GATE_COUNT: u8 = 15;

fn strict_admission(model: AcceptedArtifactModel) -> AdmissionOutcome {
    if !model.envelope_present {
        return AdmissionOutcome::Rejected(AdmissionError::AcceptedArtifactMissing);
    }
    if model.gate_count != REQUIRED_GATE_COUNT || !model.proof_flags_ok {
        return AdmissionOutcome::Rejected(AdmissionError::AcceptedArtifactInvalid);
    }
    if !model.artifact_digest_ok {
        return AdmissionOutcome::Rejected(AdmissionError::CompiledIrDigestMismatch);
    }
    if !model.capability_grant_ok {
        return AdmissionOutcome::Rejected(AdmissionError::CapabilityMismatch);
    }
    AdmissionOutcome::Admitted
}

fn strict_predicate(model: AcceptedArtifactModel) -> bool {
    model.envelope_present
        && model.gate_count == REQUIRED_GATE_COUNT
        && model.proof_flags_ok
        && model.artifact_digest_ok
        && model.capability_grant_ok
}

fn bounded_gate_count(selector: u8) -> u8 {
    match selector % 5 {
        0 => 0,
        1 => 2,
        2 => 14,
        3 => REQUIRED_GATE_COUNT,
        _ => 16,
    }
}

#[kani::proof]
fn yaml_e2e_admission_matrix() {
    let model = AcceptedArtifactModel {
        envelope_present: kani::any(),
        gate_count: bounded_gate_count(kani::any()),
        proof_flags_ok: kani::any(),
        artifact_digest_ok: kani::any(),
        capability_grant_ok: kani::any(),
    };

    let outcome = strict_admission(model);
    if strict_predicate(model) {
        kani::assert(matches!(outcome, AdmissionOutcome::Admitted));
    } else {
        kani::assert(matches!(outcome, AdmissionOutcome::Rejected(_)));
    }
}
