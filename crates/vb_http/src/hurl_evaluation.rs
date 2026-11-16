use super::*;
use super::headers::{HeaderFilterError, header_matches, validate_headers};

fn response(plan: &HurlPlan<'_>, status: u16, body: &[u8]) -> Result<(), HurlEvalError> {
    if body.len() > MAX_BODY_BYTES {
        return Err(HurlEvalError::BodyBound);
    }
    if !(100..=599).contains(&status) {
        return Err(HurlEvalError::InvalidStatus);
    }
    if status != plan.required_status {
        return Err(HurlEvalError::RequiredStatus { expected: plan.required_status, actual: status });
    }
    Ok(())
}

pub fn evaluate_hurl(
    plan: &HurlPlan<'_>, status: u16, body: &[u8],
) -> Result<(), HurlEvalError> {
    response(plan, status, body)?;
    if plan.requires_headers() {
        return Err(HurlEvalError::HeaderObservationRequired);
    }
    assertions(plan, status, &[]).map_err(legacy)
}

pub fn evaluate_hurl_with_headers(
    plan: &HurlPlan<'_>, status: u16, headers: &[ResponseHeader<'_>], body: &[u8],
) -> Result<(), HurlEvalError> {
    evaluate_hurl_with_header_predicates(plan, status, headers, body).map_err(legacy)
}

pub fn evaluate_hurl_with_header_predicates(
    plan: &HurlPlan<'_>, status: u16, headers: &[ResponseHeader<'_>], body: &[u8],
) -> Result<(), HurlPredicateEvalError> {
    response(plan, status, body).map_err(HurlPredicateEvalError::Evaluation)?;
    validate_headers(headers).map_err(HurlPredicateEvalError::Evaluation)?;
    assertions(plan, status, headers)
}

fn legacy(error: HurlPredicateEvalError) -> HurlEvalError {
    match error {
        HurlPredicateEvalError::Evaluation(error) => error,
        HurlPredicateEvalError::HeaderPredicateTypeMismatch { index } => {
            HurlEvalError::AssertionFailed { index }
        }
    }
}

fn assertions(
    plan: &HurlPlan<'_>, status: u16, headers: &[ResponseHeader<'_>],
) -> Result<(), HurlPredicateEvalError> {
    for (position, assertion) in plan.assertions.iter().flatten().enumerate() {
        let index = position.checked_add(1)
            .ok_or(HurlPredicateEvalError::Evaluation(HurlEvalError::HeaderBound))?;
        let fail = HurlPredicateEvalError::Evaluation(HurlEvalError::AssertionFailed { index });
        let success = match assertion.query {
            Query::Status { comparison, expected } => comparison.matches(i64::from(status), expected),
            Query::Header(header) => header_matches(&header, headers).map_err(|error| match error {
                HeaderFilterError::PredicateTypeMismatch
                | HeaderFilterError::MissingInput | HeaderFilterError::InvalidInputType => {
                    HurlPredicateEvalError::HeaderPredicateTypeMismatch { index }
                }
                HeaderFilterError::Bound => HurlPredicateEvalError::Evaluation(HurlEvalError::HeaderBound),
            })?,
        };
        if success == assertion.negated {
            return Err(fail);
        }
    }
    Ok(())
}
