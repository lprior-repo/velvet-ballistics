//! Step metadata parsing - retry, error handler, result, examples.

use crate::{YamlError, YamlResult};

use super::types::{opt_str, require_str_in, require_u16, ErrorHandlerAst, ExampleAst, ResultMapping, RetryPolicy};

// ---------------------------------------------------------------------------
// Retry
// ---------------------------------------------------------------------------

pub fn parse_retry(node: &saphyr::Yaml<'_>) -> YamlResult<Option<RetryPolicy>> {
    let Some(sub) = super::types::lookup(node, "retry") else {
        return Ok(None);
    };
    if !sub.is_mapping() {
        return Ok(None);
    }

    let max_attempts = require_u16(sub, "max_attempts")?;
    let delay = opt_str(sub, "delay");

    Ok(Some(RetryPolicy {
        max_attempts,
        delay,
    }))
}

// ---------------------------------------------------------------------------
// Error handler
// ---------------------------------------------------------------------------

pub fn parse_error_handler(node: &saphyr::Yaml<'_>) -> YamlResult<Option<ErrorHandlerAst>> {
    let Some(sub) = super::types::lookup(node, "on_error") else {
        return Ok(None);
    };
    if !sub.is_mapping() {
        return Ok(None);
    }

    let handler = require_str_in(sub, "handler", "on_error.handler")?;
    Ok(Some(ErrorHandlerAst { handler }))
}

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

pub fn parse_result(node: &saphyr::Yaml<'_>) -> YamlResult<Option<ResultMapping>> {
    let Some(sub) = super::types::lookup(node, "result") else {
        return Ok(None);
    };
    if !sub.is_mapping() {
        return Ok(None);
    }

    let value = require_str_in(sub, "value", "result.value")?;
    Ok(Some(ResultMapping { value }))
}

// ---------------------------------------------------------------------------
// Examples
// ---------------------------------------------------------------------------

pub fn parse_examples(node: &saphyr::Yaml<'_>) -> YamlResult<Vec<ExampleAst>> {
    let Some(seq) = super::types::lookup(node, "examples").and_then(|v| v.as_vec()) else {
        return Ok(Vec::new());
    };

    let mut examples = Vec::new();
    for item in seq {
        if !item.is_mapping() {
            return Err(YamlError::FieldShape {
                field: "examples",
                expected: "mapping",
            });
        }
        let description = opt_str(item, "description");
        let input = opt_str(item, "input");
        let expected = opt_str(item, "expected");
        examples.push(ExampleAst {
            description,
            input,
            expected,
        });
    }
    Ok(examples)
}
