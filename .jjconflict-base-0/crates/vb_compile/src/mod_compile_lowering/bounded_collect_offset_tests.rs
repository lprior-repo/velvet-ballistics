#[test]
fn offset_overflow_u16_max_plus_one() {
    let result = super::checked_body_length_offset(65535, 1);
    assert_eq!(
        result,
        Err(crate::CompileErrors(vec![
            crate::CompileError::StepIndexOutOfRange { value: 65536 }
        ]))
    );
}

#[test]
fn offset_overflow_u16_max_plus_two() {
    let result = super::checked_body_length_offset(65535, 2);
    assert_eq!(
        result,
        Err(crate::CompileErrors(vec![
            crate::CompileError::StepIndexOutOfRange { value: 65537 }
        ]))
    );
}

#[test]
fn offset_overflow_usize_max() {
    let result = super::checked_body_length_offset(usize::MAX, 1);
    assert_eq!(
        result,
        Err(crate::CompileErrors(vec![
            crate::CompileError::StepIndexOutOfRange { value: usize::MAX }
        ]))
    );
}

#[test]
fn offset_valid_u16_max() {
    let result = super::checked_body_length_offset(65534, 1);
    assert_eq!(result, Ok(65535));
}

#[test]
fn offset_valid_min() {
    let result = super::checked_body_length_offset(0, 1);
    assert_eq!(result, Ok(1));
}
