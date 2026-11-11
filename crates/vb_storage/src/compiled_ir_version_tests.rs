use super::*;

#[test]
fn version_matrix_matching_v1() -> TestResult {
    assert_v1(0, Some(SchemaVersion::new(1, 0, 0)))
}

#[test]
fn version_matrix_matching_v2() -> TestResult {
    assert_v2(&[], Some(SchemaVersion::new(2, 0, 0)))
}

#[test]
fn version_matrix_future_3_0_0_rejected() -> TestResult {
    let held = SchemaVersion::new(3, 0, 0);
    match decode_compiled_ir(&[0xff], Some(held)) {
        Err(JournalError::ArtifactVersionIncompatible {
            held: actual,
            required,
            class,
        }) => {
            assert_eq!(actual, held);
            assert_eq!(required, SchemaVersion::new(2, 0, 0));
            assert_eq!(class, CompatibilityClass::Rejected);
            Ok(())
        }
        other => Err(unexpected(other)),
    }
}

#[test]
fn version_matrix_same_major_1_2_0_accepted() -> TestResult {
    assert_v1(0, Some(SchemaVersion::new(1, 2, 0)))
}

#[test]
fn version_matrix_v1_patch_accepted() -> TestResult {
    assert_v1(1, Some(SchemaVersion::new(1, 0, 1)))
}

#[test]
fn version_matrix_same_major_2_1_0_accepted() -> TestResult {
    assert_v2(&["key"], Some(SchemaVersion::new(2, 1, 0)))
}

#[test]
fn version_matrix_same_major_2_0_1_accepted() -> TestResult {
    assert_v2(&[], Some(SchemaVersion::new(2, 0, 1)))
}

#[test]
fn outer_inner_disagreement_v1_with_v2_prefix() -> TestResult {
    assert_malformed(decode_compiled_ir(
        &[0x01, 0xff, 0x56, 0x42],
        Some(SchemaVersion::new(1, 0, 0)),
    ))
}

#[test]
fn outer_inner_disagreement_v2_with_v1_bytes() -> TestResult {
    let bytes = postcard::to_allocvec(&legacy(0)?.to_parts())?;
    assert_malformed(decode_compiled_ir(
        &bytes,
        Some(SchemaVersion::new(2, 0, 0)),
    ))
}

#[test]
fn selected_v2_rejects_wrong_prefix() -> TestResult {
    assert_malformed(decode_compiled_ir(
        &[0x01, 0xff, 0x56, 0x43],
        Some(SchemaVersion::new(2, 0, 0)),
    ))
}

#[test]
fn explicit_classifier_keeps_legacy_wrapper() {
    assert_eq!(CURRENT_IR_VERSION, SchemaVersion::new(1, 0, 0));
    let held = SchemaVersion::new(1, 2, 0);
    assert_eq!(
        crate::compat::artifact_ir_version_inputs(held),
        crate::compat::artifact_ir_version_inputs_for(held, CURRENT_IR_VERSION)
    );
}
