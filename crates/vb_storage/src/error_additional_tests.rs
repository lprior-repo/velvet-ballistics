#[test]
fn artifact_validation_variant_and_source() {
    let err = JournalError::ArtifactValidation(ValidationError::DuplicateKey);
    match err {
        JournalError::ArtifactValidation(inner) => {
            assert_eq!(inner, ValidationError::DuplicateKey);
        }
        other => panic!("expected ArtifactValidation, got {other:?}"),
    }
}

#[test]
fn artifact_validation_display_format() {
    let err = JournalError::ArtifactValidation(ValidationError::DuplicateKey);
    let display = format!("{err}");
    assert!(
        display.contains("compiled artifact validation failed"),
        "Display must mention artifact validation failure: {display}"
    );
    assert!(
        display.contains("DUPLICATE_KEY"),
        "Display must contain the validation error: {display}"
    );
}

#[test]
fn artifact_validation_error_code() {
    let err = JournalError::ArtifactValidation(ValidationError::DuplicateKey);
    assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    assert_eq!(
        JournalError::ARTIFACT_MALFORMED_CODE,
        DiagnosticCode::new(0x4017),
    );
}

// -----------------------------------------------------------------------
// ArtifactActionRegistry
// -----------------------------------------------------------------------

#[test]
fn artifact_action_registry_variant_and_source() {
    let err =
        JournalError::ArtifactActionRegistry(ActionRegistryError::IndexOverflow { index: 70000 });
    match err {
        JournalError::ArtifactActionRegistry(inner) => {
            assert_eq!(inner, ActionRegistryError::IndexOverflow { index: 70000 });
        }
        other => panic!("expected ArtifactActionRegistry, got {other:?}"),
    }
}

#[test]
fn artifact_action_registry_display_format() {
    let err =
        JournalError::ArtifactActionRegistry(ActionRegistryError::IndexOverflow { index: 70000 });
    let display = format!("{err}");
    assert!(
        display.contains("artifact action registry invalid"),
        "Display must mention action registry failure: {display}"
    );
    assert!(
        display.contains("70000"),
        "Display must contain the overflowing index: {display}"
    );
}

#[test]
fn artifact_action_registry_error_code() {
    let err = JournalError::ArtifactActionRegistry(ActionRegistryError::IndexOverflow { index: 1 });
    assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    assert_eq!(
        JournalError::ARTIFACT_MALFORMED_CODE,
        DiagnosticCode::new(0x4017),
    );
}

// -----------------------------------------------------------------------
// WorkflowReconstruction
// -----------------------------------------------------------------------

#[test]
fn workflow_reconstruction_variant_and_source() {
    let err = JournalError::WorkflowReconstruction(WorkflowError::EmptyNodes);
    match err {
        JournalError::WorkflowReconstruction(inner) => {
            assert_eq!(inner, WorkflowError::EmptyNodes);
        }
        other => panic!("expected WorkflowReconstruction, got {other:?}"),
    }
}

#[test]
fn workflow_reconstruction_display_format() {
    let err = JournalError::WorkflowReconstruction(WorkflowError::EmptyNodes);
    let display = format!("{err}");
    assert!(
        display.contains("compiled workflow reconstruction failed"),
        "Display must mention workflow reconstruction failure: {display}"
    );
    assert!(
        display.contains("compiled workflow must contain at least one node"),
        "Display must contain the workflow error: {display}"
    );
}

#[test]
fn workflow_reconstruction_error_code() {
    let err = JournalError::WorkflowReconstruction(WorkflowError::EmptyNodes);
    assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    assert_eq!(
        JournalError::ARTIFACT_MALFORMED_CODE,
        DiagnosticCode::new(0x4017),
    );
}

// -----------------------------------------------------------------------
// CompiledIrReadback
// -----------------------------------------------------------------------

#[test]
fn compiled_ir_readback_variant_and_source() {
    let err = JournalError::CompiledIrReadback(Box::new(JournalError::BatchAborted));
    match err {
        JournalError::CompiledIrReadback(inner) => {
            assert!(matches!(inner.as_ref(), JournalError::BatchAborted));
        }
        other => panic!("expected CompiledIrReadback, got {other:?}"),
    }
}

#[test]
fn compiled_ir_readback_display_format() {
    let err = JournalError::CompiledIrReadback(Box::new(JournalError::BatchAborted));
    let display = format!("{err}");
    assert!(
        display.contains("compiled_ir readback failed"),
        "Display must mention compiled_ir readback failure: {display}"
    );
    assert!(
        display.contains("journal write batch was aborted"),
        "Display must contain the wrapped error: {display}"
    );
}

#[test]
fn compiled_ir_readback_error_code() {
    let err = JournalError::CompiledIrReadback(Box::new(JournalError::BatchAborted));
    assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    assert_eq!(
        JournalError::ARTIFACT_MALFORMED_CODE,
        DiagnosticCode::new(0x4017),
    );
}

// -----------------------------------------------------------------------
// AdmissionAllocationFailed
// -----------------------------------------------------------------------

#[test]
fn admission_allocation_failed_variant_and_source() {
    let reserve = Vec::<u8>::new().try_reserve(usize::MAX);
    let source = match reserve {
        Ok(()) => panic!("reserving usize::MAX bytes must fail"),
        Err(err) => err,
    };
    assert_eq!(
        format!("{source}"),
        "memory allocation failed because the computed capacity exceeded the collection's maximum"
    );
    let err = JournalError::AdmissionAllocationFailed(source);
    match err {
        JournalError::AdmissionAllocationFailed(inner) => {
            assert_eq!(
                format!("{inner}"),
                "memory allocation failed because the computed capacity exceeded the collection's maximum"
            );
        }
        other => panic!("expected AdmissionAllocationFailed, got {other:?}"),
    }
}

#[test]
fn admission_allocation_failed_display_format() {
    let reserve = Vec::<u8>::new().try_reserve(usize::MAX);
    let source = match reserve {
        Ok(()) => panic!("reserving usize::MAX bytes must fail"),
        Err(err) => err,
    };
    let err = JournalError::AdmissionAllocationFailed(source);
    let display = format!("{err}");
    assert!(
        display.contains("admission pre-allocation failed"),
        "Display must mention pre-allocation failure: {display}"
    );
    assert!(
        display.contains("capacity exceeded the collection's maximum"),
        "Display must contain the reserve failure: {display}"
    );
}

#[test]
fn admission_allocation_failed_error_code() {
    let reserve = Vec::<u8>::new().try_reserve(usize::MAX);
    let source = match reserve {
        Ok(()) => panic!("reserving usize::MAX bytes must fail"),
        Err(err) => err,
    };
    let err = JournalError::AdmissionAllocationFailed(source);
    assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    assert_eq!(
        JournalError::ARTIFACT_MALFORMED_CODE,
        DiagnosticCode::new(0x4017),
    );
}

// -----------------------------------------------------------------------
// OwnedInputs
// -----------------------------------------------------------------------

#[test]
fn owned_inputs_variant_and_source() {
    let err = JournalError::OwnedInputs(OwnedInputError::IdentityMismatch);
    match err {
        JournalError::OwnedInputs(inner) => {
            assert!(matches!(inner, OwnedInputError::IdentityMismatch));
        }
        other => panic!("expected OwnedInputs, got {other:?}"),
    }
}

#[test]
fn owned_inputs_display_format() {
    let err = JournalError::OwnedInputs(OwnedInputError::IdentityMismatch);
    let display = format!("{err}");
    assert!(
        display.contains("owned input record rejected"),
        "Display must mention owned input rejection: {display}"
    );
    assert!(
        display.contains("owned input identity mismatch"),
        "Display must contain the owned input error: {display}"
    );
}

#[test]
fn owned_inputs_error_code() {
    let err = JournalError::OwnedInputs(OwnedInputError::IdentityMismatch);
    assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    assert_eq!(
        JournalError::ARTIFACT_MALFORMED_CODE,
        DiagnosticCode::new(0x4017),
    );
}

// -----------------------------------------------------------------------
// IndexStatusStateCollision
// -----------------------------------------------------------------------

#[test]
fn index_status_state_collision_variant_and_fields() {
    let err = JournalError::IndexStatusStateCollision {
        byte: 0x10,
        min: 0x20,
    };
    match err {
        JournalError::IndexStatusStateCollision { byte, min } => {
            assert_eq!(byte, 0x10);
            assert_eq!(min, 0x20);
        }
        other => panic!("expected IndexStatusStateCollision, got {other:?}"),
    }
}

#[test]
fn index_status_state_collision_display_format() {
    let err = JournalError::IndexStatusStateCollision {
        byte: 0x10,
        min: 0x20,
    };
    let display = format!("{err}");
    assert!(
        display.contains("IndexStatusState byte collision"),
        "Display must mention index status collision: {display}"
    );
    assert!(
        display.contains("0x10") && display.contains("0x20"),
        "Display must contain byte and minimum: {display}"
    );
}

#[test]
fn index_status_state_collision_error_code() {
    let err = JournalError::IndexStatusStateCollision { byte: 1, min: 2 };
    assert_eq!(err.diagnostic_code(), JournalError::ARTIFACT_MALFORMED_CODE,);
    assert_eq!(
        JournalError::ARTIFACT_MALFORMED_CODE,
        DiagnosticCode::new(0x4017),
    );
}

// -----------------------------------------------------------------------
// InvalidConfig
// -----------------------------------------------------------------------

#[test]
fn invalid_config_variant_and_fields() {
    let err = JournalError::InvalidConfig {
        field: "max_bytes",
        reason: "must be positive",
    };
    match err {
        JournalError::InvalidConfig { field, reason } => {
            assert_eq!(field, "max_bytes");
            assert_eq!(reason, "must be positive");
        }
        other => panic!("expected InvalidConfig, got {other:?}"),
    }
}

#[test]
fn invalid_config_display_format() {
    let err = JournalError::InvalidConfig {
        field: "max_bytes",
        reason: "must be positive",
    };
    let display = format!("{err}");
    assert!(
        display.contains("invalid configuration for field max_bytes"),
        "Display must mention invalid configuration: {display}"
    );
    assert!(
        display.contains("must be positive"),
        "Display must contain the reason: {display}"
    );
}

#[test]
fn invalid_config_error_code() {
    let err = JournalError::InvalidConfig {
        field: "f",
        reason: "r",
    };
    assert_eq!(err.diagnostic_code(), JournalError::INVALID_CONFIG_CODE,);
    assert_eq!(
        JournalError::INVALID_CONFIG_CODE,
        DiagnosticCode::new(0x4033),
    );
}

// -----------------------------------------------------------------------
// UnsupportedReadOnly
// -----------------------------------------------------------------------

#[test]
fn unsupported_read_only_variant_and_display() {
    let err = JournalError::UnsupportedReadOnly;
    assert!(
        matches!(err, JournalError::UnsupportedReadOnly),
        "variant must be UnsupportedReadOnly"
    );
    let display = format!("{err}");
    assert!(
        display.contains("read-only journal open is not supported"),
        "Display must mention unsupported read-only open: {display}"
    );
}

#[test]
fn unsupported_read_only_error_code() {
    let err = JournalError::UnsupportedReadOnly;
    assert_eq!(
        err.diagnostic_code(),
        JournalError::UNSUPPORTED_READ_ONLY_CODE,
    );
    assert_eq!(
        JournalError::UNSUPPORTED_READ_ONLY_CODE,
        DiagnosticCode::new(0x4034),
    );
}

// -----------------------------------------------------------------------
// EmergencyDenyMarkerUnsupportedVersion
// -----------------------------------------------------------------------

#[test]
fn emergency_deny_marker_unsupported_version_variant_and_fields() {
    let err = JournalError::EmergencyDenyMarkerUnsupportedVersion { version: 9 };
    match err {
        JournalError::EmergencyDenyMarkerUnsupportedVersion { version } => {
            assert_eq!(version, 9);
        }
        other => panic!("expected EmergencyDenyMarkerUnsupportedVersion, got {other:?}"),
    }
}

#[test]
fn emergency_deny_marker_unsupported_version_display_format() {
    let err = JournalError::EmergencyDenyMarkerUnsupportedVersion { version: 9 };
    let display = format!("{err}");
    assert!(
        display.contains("emergency deny marker has unsupported version"),
        "Display must mention unsupported marker version: {display}"
    );
    assert!(
        display.contains('9'),
        "Display must contain the version: {display}"
    );
}

#[test]
fn emergency_deny_marker_unsupported_version_error_code() {
    let err = JournalError::EmergencyDenyMarkerUnsupportedVersion { version: 1 };
    assert_eq!(
        err.diagnostic_code(),
        JournalError::EMERGENCY_DENY_UNSUPPORTED_VERSION_CODE,
    );
    assert_eq!(
        JournalError::EMERGENCY_DENY_UNSUPPORTED_VERSION_CODE,
        DiagnosticCode::new(0x4042),
    );
}

// -----------------------------------------------------------------------
// EmergencyDenyMarkerMalformed
// -----------------------------------------------------------------------

#[test]
fn emergency_deny_marker_malformed_variant_and_display() {
    let err = JournalError::EmergencyDenyMarkerMalformed;
    assert!(
        matches!(err, JournalError::EmergencyDenyMarkerMalformed),
        "variant must be EmergencyDenyMarkerMalformed"
    );
    let display = format!("{err}");
    assert!(
        display.contains("emergency deny marker is malformed"),
        "Display must mention malformed marker: {display}"
    );
}

#[test]
fn emergency_deny_marker_malformed_error_code() {
    let err = JournalError::EmergencyDenyMarkerMalformed;
    assert_eq!(
        err.diagnostic_code(),
        JournalError::EMERGENCY_DENY_MALFORMED_CODE,
    );
    assert_eq!(
        JournalError::EMERGENCY_DENY_MALFORMED_CODE,
        DiagnosticCode::new(0x4043),
    );
}
