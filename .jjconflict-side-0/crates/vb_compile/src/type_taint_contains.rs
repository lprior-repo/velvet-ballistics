use super::{CompileError, Facts, ValueType, matches_type, parsed_expression_fact, type_mismatch};
use crate::expression::ParsedExpression;
use vb_validate::type_taint::Taint;

pub(super) fn contains_taint(
    args: &[ParsedExpression],
    facts: &Facts<'_>,
    field: &'static str,
) -> Result<Taint, CompileError> {
    let (head, tail) = match args.split_first() {
        None => return Ok(Taint::Clean),
        Some(pair) => pair,
    };

    let receiver = parsed_expression_fact(head, facts, field)?;
    validate_receiver(receiver.value_type, field)?;

    tail.iter()
        .enumerate()
        .try_fold(receiver.taint, |taint, (index, arg)| {
            let needle = parsed_expression_fact(arg, facts, field)?;
            validate_needle(index, receiver.value_type, needle.value_type, field)?;
            Ok(taint.merge(needle.taint))
        })
}

fn validate_receiver(actual: ValueType, field: &'static str) -> Result<(), CompileError> {
    if actual == ValueType::Text || actual == ValueType::List || actual == ValueType::Any {
        return Ok(());
    }
    Err(CompileError::TypeMismatch {
        field,
        expected: "text or list",
        found: actual.as_str(),
    })
}

fn validate_needle(
    index: usize,
    receiver: ValueType,
    needle: ValueType,
    field: &'static str,
) -> Result<(), CompileError> {
    if index == 0 && receiver == ValueType::Text && !matches_type(needle, ValueType::Text) {
        return Err(type_mismatch(field, ValueType::Text, needle));
    }
    Ok(())
}
#[cfg(test)]
#[path = "type_taint_contains_tests.rs"]
mod tests;
