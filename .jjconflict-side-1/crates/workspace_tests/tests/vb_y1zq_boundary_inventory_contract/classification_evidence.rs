use super::support::*;

#[test]
fn classify_boundary_returns_c_abi_when_candidate_declares_extern_c_boundary() {
    let source = "crates/ffi/src/c_abi.rs";
    let result = classify_boundary(candidate(source, "extern-c-boundary"));

    assert_eq!(result, Ok(classified(BoundaryClass::CAbi, source)));
}

#[test]
fn classify_boundary_returns_ffi_when_candidate_declares_foreign_function_boundary() {
    let source = "crates/ffi/src/lib.rs";
    let result = classify_boundary(candidate(source, "foreign-function-boundary"));

    assert_eq!(result, Ok(classified(BoundaryClass::Ffi, source)));
}

#[test]
fn classify_boundary_returns_ipc_when_candidate_declares_ipc_frame_boundary() {
    let source = "crates/vb_ipc/src/frame.rs";
    let result = classify_boundary(candidate(source, "ipc-frame-boundary"));

    assert_eq!(result, Ok(classified(BoundaryClass::Ipc, source)));
}

#[test]
fn classify_boundary_returns_external_binary_when_candidate_invokes_process_boundary() {
    let source = "scripts/run-verifier.sh";
    let result = classify_boundary(candidate(source, "external-binary-boundary"));

    assert_eq!(
        result,
        Ok(classified(BoundaryClass::ExternalBinary, source))
    );
}

#[test]
fn classify_boundary_returns_decoder_when_candidate_ingests_external_bytes() {
    let source = "crates/vb_yaml/src/decode.rs";
    let result = classify_boundary(candidate(source, "decoder-byte-ingest-boundary"));

    assert_eq!(result, Ok(classified(BoundaryClass::Decoder, source)));
}

#[test]
fn classify_boundary_returns_generated_code_when_candidate_is_generated_interface() {
    let source = "crates/vb_runtime/src/generated/interface.rs";
    let result = classify_boundary(candidate(source, "generated-interface-boundary"));

    assert_eq!(result, Ok(classified(BoundaryClass::GeneratedCode, source)));
}

#[test]
fn classify_boundary_returns_unsafe_adjacent_dependency_when_candidate_is_dependency_boundary() {
    let source = "Cargo.toml";
    let result = classify_boundary(candidate(source, "unsafe-adjacent-dependency-boundary"));

    assert_eq!(
        result,
        Ok(classified(BoundaryClass::UnsafeAdjacentDependency, source))
    );
}

#[test]
fn classify_boundary_returns_unknown_boundary_class_when_candidate_has_no_allowed_marker() {
    let result = classify_boundary(candidate("crates/vb_core/src/lib.rs", "plain-rust-module"));

    assert_eq!(result, Err(BoundaryInventoryError::UnknownBoundaryClass));
}

#[test]
fn required_evidence_returns_fuzz_isolation_or_manual_qa_for_c_abi_crossing_boundary() {
    let result = required_evidence(classified(BoundaryClass::CAbi, "crates/ffi/src/c_abi.rs"));

    assert_eq!(result, Ok(EvidenceRequirement::FuzzOrIsolationOrManualQa));
}

#[test]
fn required_evidence_returns_fuzz_isolation_or_manual_qa_for_ffi_crossing_boundary() {
    let result = required_evidence(classified(BoundaryClass::Ffi, "crates/ffi/src/lib.rs"));

    assert_eq!(result, Ok(EvidenceRequirement::FuzzOrIsolationOrManualQa));
}

#[test]
fn required_evidence_returns_fuzz_isolation_or_manual_qa_for_ipc_byte_boundary() {
    let result = required_evidence(classified(BoundaryClass::Ipc, "crates/vb_ipc/src/frame.rs"));

    assert_eq!(result, Ok(EvidenceRequirement::FuzzOrIsolationOrManualQa));
}

#[test]
fn required_evidence_returns_fuzz_isolation_or_manual_qa_for_external_binary_process_boundary() {
    let result = required_evidence(classified(
        BoundaryClass::ExternalBinary,
        "scripts/run-verifier.sh",
    ));

    assert_eq!(result, Ok(EvidenceRequirement::FuzzOrIsolationOrManualQa));
}

#[test]
fn required_evidence_returns_fuzz_isolation_or_manual_qa_for_decoder_byte_boundary() {
    let result = required_evidence(classified(
        BoundaryClass::Decoder,
        "crates/vb_yaml/src/decode.rs",
    ));

    assert_eq!(result, Ok(EvidenceRequirement::FuzzOrIsolationOrManualQa));
}

#[test]
fn required_evidence_returns_unknown_boundary_class_when_boundary_class_is_unknown() {
    let boundary = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: String::from("vb-y1zq-unknown"),
        class: BoundaryClass::Unknown,
        source_path: PathBuf::from("crates/unknown/src/lib.rs"),
        exposure: BoundaryExposure::none(),
    });

    let result = required_evidence(boundary);

    assert_eq!(result, Err(BoundaryInventoryError::UnknownBoundaryClass));
}

#[test]
fn required_evidence_returns_missing_evidence_path_when_known_boundary_has_no_risk_flags() {
    let boundary = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: String::from("vb-y1zq-ipc-no-risk"),
        class: BoundaryClass::Ipc,
        source_path: PathBuf::from("crates/vb_ipc/src/frame.rs"),
        exposure: BoundaryExposure::none(),
    });

    let result = required_evidence(boundary);

    assert_eq!(result, Err(BoundaryInventoryError::MissingEvidencePath));
}

#[test]
fn required_evidence_returns_required_evidence_when_only_process_limit_crosses() {
    let boundary = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: String::from("vb-y1zq-ipc-process-risk"),
        class: BoundaryClass::Ipc,
        source_path: PathBuf::from("crates/vb_ipc/src/frame.rs"),
        exposure: BoundaryExposure::risky(BoundaryRisk::ProcessLimit),
    });

    let result = required_evidence(boundary);

    assert_eq!(result, Ok(EvidenceRequirement::FuzzOrIsolationOrManualQa));
}

#[test]
fn required_evidence_returns_required_evidence_when_only_language_limit_crosses() {
    let boundary = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: String::from("vb-y1zq-ipc-language-risk"),
        class: BoundaryClass::Ipc,
        source_path: PathBuf::from("crates/vb_ipc/src/frame.rs"),
        exposure: BoundaryExposure::risky(BoundaryRisk::LanguageLimit),
    });

    let result = required_evidence(boundary);

    assert_eq!(result, Ok(EvidenceRequirement::FuzzOrIsolationOrManualQa));
}

#[test]
fn required_evidence_returns_required_evidence_for_generated_code_even_without_risk_flags() {
    let boundary = ClassifiedBoundary::new(ClassifiedBoundaryInput {
        id: String::from("vb-y1zq-generated-policy-risk"),
        class: BoundaryClass::GeneratedCode,
        source_path: PathBuf::from("crates/vb_runtime/src/generated/interface.rs"),
        exposure: BoundaryExposure::none(),
    });

    let result = required_evidence(boundary);

    assert_eq!(result, Ok(EvidenceRequirement::FuzzOrIsolationOrManualQa));
}
