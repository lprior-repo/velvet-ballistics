use super::expr::{parse_u16_field, integer_error_value};
use crate::CompileError;
use saphyr::{LoadableYamlNode, Yaml};

fn parse_yaml(s: &str) -> Yaml<'static> {
    Yaml::load_from_str(s)
        .expect("yaml parse")
        .into_iter()
        .next()
        .expect("single document")
}

#[test]
fn parse_u16_field_accepts_boundary_values() {
    let zero_node = parse_yaml("max_attempts: 0");
    match parse_u16_field(&zero_node, 0, "max_attempts") {
        Ok(0) => {}
        v => panic!("expected Ok(0), got {v:?}"),
    }

    let max_node = parse_yaml("max_attempts: 65535");
    match parse_u16_field(&max_node, 0, "max_attempts") {
        Ok(u16::MAX) => {}
        v => panic!("expected Ok(u16::MAX), got {v:?}"),
    }
}

#[test]
fn parse_u16_field_rejects_over_max() {
    let over_node = parse_yaml("max_attempts: 65536");
    match parse_u16_field(&over_node, 0, "max_attempts") {
        Err(CompileError::PrimitiveLoweringLimitExceeded { .. }) => {}
        v => panic!("expected PrimitiveLoweringLimitExceeded, got {v:?}"),
    }
}

#[test]
fn parse_u16_field_rejects_negative_integer() {
    let neg_node = parse_yaml("max_attempts: -1");
    match parse_u16_field(&neg_node, 0, "max_attempts") {
        Err(CompileError::PrimitiveLoweringLimitExceeded { .. }) => {}
        v => panic!("expected PrimitiveLoweringLimitExceeded, got {v:?}"),
    }
}

#[test]
fn parse_u16_field_rejects_non_integer() {
    let str_node = parse_yaml("max_attempts: hello");
    match parse_u16_field(&str_node, 0, "max_attempts") {
        Err(CompileError::StepFieldShape { .. }) => {}
        v => panic!("expected StepFieldShape, got {v:?}"),
    }
}

#[test]
fn integer_error_value_returns_value_when_in_range() {
    assert_eq!(integer_error_value(42), 42);
    assert_eq!(integer_error_value(0), 0);
    // i64::MAX fits in usize on 64-bit, returns i64::MAX
    assert_eq!(integer_error_value(i64::MAX), i64::MAX as usize);
}

#[test]
fn integer_error_value_returns_max_for_out_of_range() {
    assert_eq!(integer_error_value(-1), usize::MAX);
    assert_eq!(integer_error_value(i64::MIN), usize::MAX);
}
