use super::*;
use proptest::prelude::*;

#[test]
fn test_body_constant_index_basic() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = SlotCompiler::new();
    let idx1 = body_constant_index(&mut builder, "7", 1, true)?;
    assert_eq!(idx1, ConstIdx::new(0));
    let idx2 = body_constant_index(&mut builder, "99", 2, true)?;
    assert_eq!(idx2, ConstIdx::new(1));
    assert_eq!(
        builder.constants,
        vec![ConstValue::I64(7), ConstValue::I64(99)]
    );
    Ok(())
}

#[test]
fn test_body_constant_index_reuse() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = SlotCompiler::new();
    assert_eq!(
        body_constant_index(&mut builder, "7", 1, true)?,
        ConstIdx::new(0)
    );
    assert_eq!(
        body_constant_index(&mut builder, "99", 2, true)?,
        ConstIdx::new(1)
    );
    let idx = body_constant_index(&mut builder, "7", 3, true)?;
    assert_eq!(idx, ConstIdx::new(0));
    assert_eq!(builder.constants.len(), 2);
    Ok(())
}

#[test]
fn test_body_constant_index_no_reuse() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = SlotCompiler::new();
    assert_eq!(
        body_constant_index(&mut builder, "7", 1, true)?,
        ConstIdx::new(0)
    );
    assert_eq!(
        body_constant_index(&mut builder, "99", 2, true)?,
        ConstIdx::new(1)
    );
    let idx = body_constant_index(&mut builder, "7", 3, false)?;
    assert_eq!(idx, ConstIdx::new(2));
    assert_eq!(
        builder.constants,
        vec![ConstValue::I64(7), ConstValue::I64(99), ConstValue::I64(7)]
    );
    Ok(())
}

#[test]
fn test_body_constant_index_parse_errors() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = SlotCompiler::new();
    assert_eq!(
        body_constant_index(&mut builder, "1", 1, true)?,
        ConstIdx::new(0)
    );
    for bad in ["bad", "9223372036854775808", "-9223372036854775809"] {
        match body_constant_index(&mut builder, bad, 9, true) {
            Ok(_) => return Err("expected parse error".into()),
            Err(err) => assert_eq!(
                err,
                CompileErrors(vec![CompileError::StepFieldShape {
                    step: 9,
                    field: "set.value",
                    expected: "integer string"
                }])
            ),
        }
    }
    assert_eq!(builder.constants, vec![ConstValue::I64(1)]);
    Ok(())
}

#[test]
fn test_body_constant_index_extremes() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = SlotCompiler::new();
    let idx_min = body_constant_index(&mut builder, &i64::MIN.to_string(), 1, true)?;
    let idx_max = body_constant_index(&mut builder, &i64::MAX.to_string(), 2, true)?;
    let idx_zero = body_constant_index(&mut builder, "0", 3, true)?;
    assert_eq!(idx_min, ConstIdx::new(0));
    assert_eq!(idx_max, ConstIdx::new(1));
    assert_eq!(idx_zero, ConstIdx::new(2));
    assert_eq!(
        builder.constants,
        vec![
            ConstValue::I64(i64::MIN),
            ConstValue::I64(i64::MAX),
            ConstValue::I64(0)
        ]
    );
    Ok(())
}

#[test]
fn test_body_constant_index_overflow() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = SlotCompiler::new();
    builder.constants = vec![ConstValue::I64(0); 65536];
    match body_constant_index(&mut builder, "1", 1, true) {
        Ok(_) => return Err("expected overflow error".into()),
        Err(err) => assert_eq!(
            err,
            CompileErrors(vec![CompileError::Workflow(
                WorkflowError::ConstOutOfBounds {
                    constant: ConstIdx::new(u16::MAX)
                }
            )])
        ),
    }
    assert_eq!(builder.constants.len(), 65536);
    Ok(())
}

#[test]
fn test_body_constant_index_reuse_when_full() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder = SlotCompiler::new();
    builder.constants = vec![ConstValue::I64(0); 65536];
    let idx = body_constant_index(&mut builder, "0", 1, true)?;
    assert_eq!(idx, ConstIdx::new(0));
    Ok(())
}

proptest! {
    #[test]
    fn test_body_constant_index_proptest(a in i64::MIN..=i64::MAX, b in i64::MIN..=i64::MAX) {
        if a == b { return Ok(()); }
        let mut builder = SlotCompiler::new();
        let idx_a = body_constant_index(&mut builder, &a.to_string(), 1, true)
            .map_err(|e| TestCaseError::fail(format!("{:?}", e)))?;
        let idx_b = body_constant_index(&mut builder, &b.to_string(), 2, true)
            .map_err(|e| TestCaseError::fail(format!("{:?}", e)))?;
        prop_assert_eq!(idx_a, ConstIdx::new(0));
        prop_assert_eq!(idx_b, ConstIdx::new(1));
        prop_assert_eq!(builder.constants, vec![ConstValue::I64(a), ConstValue::I64(b)]);
    }
}
