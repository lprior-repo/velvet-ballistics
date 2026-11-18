//! Explicit typed predicates over one bounded, pinned HTTP exchange.
use crate::hurl::{HurlEvalError, HurlPlan, HurlPredicateEvalError, ResponseHeader};
use crate::observation::MAX_OBSERVED_HEADERS;
use crate::{AttemptFailure, HttpLimits, HttpObservationWithHeaders, PinnedGet};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HurlPredicateAttemptFailure {
    #[error("Hurl target differs from the pinned request")]
    Target,
    #[error(transparent)]
    Attempt(#[from] AttemptFailure),
    #[error("Hurl typed predicate evaluation failed: {0:?}")]
    Evaluation(HurlPredicateEvalError),
}

/// Required status precedes deferred header conversion after a bounded exchange.
/// Transport failures precede evaluation; no receipt or runtime grant is created.
pub fn execute_hurl_with_header_predicates(
    plan: &HurlPlan<'_>,
    endpoint: &PinnedGet,
    limits: HttpLimits,
) -> Result<HttpObservationWithHeaders, HurlPredicateAttemptFailure> {
    let started = std::time::Instant::now();
    let target = endpoint
        .uri
        .path_and_query()
        .map_or("/", |value| value.as_str());
    if plan.target() != target {
        return Err(HurlPredicateAttemptFailure::Target);
    }
    let (response, captured) = crate::transport::get_once_for_headers(endpoint, limits)?;
    let evaluated = (|| {
        if response.status() != plan.required_status() {
            return Err(HurlPredicateAttemptFailure::Evaluation(
                HurlPredicateEvalError::Evaluation(HurlEvalError::RequiredStatus {
                    expected: plan.required_status(),
                    actual: response.status(),
                }),
            ));
        }
        let observed = HttpObservationWithHeaders::new(response, captured?);
        evaluate_captured_predicates(plan, &observed)?;
        Ok(observed)
    })();
    if started.elapsed() > limits.deadline() {
        return Err(AttemptFailure::Timeout.into());
    }
    evaluated
}

fn evaluate_captured_predicates(
    plan: &HurlPlan<'_>,
    observed: &HttpObservationWithHeaders,
) -> Result<(), HurlPredicateAttemptFailure> {
    let bound = HurlPredicateAttemptFailure::Evaluation(HurlPredicateEvalError::Evaluation(
        HurlEvalError::HeaderBound,
    ));
    let mut fields = [ResponseHeader {
        name: "",
        value: "",
    }; MAX_OBSERVED_HEADERS];
    for (index, field) in observed.headers().take(MAX_OBSERVED_HEADERS).enumerate() {
        let slot = fields.get_mut(index).ok_or(bound)?;
        *slot = ResponseHeader {
            name: field.name,
            value: field.value,
        };
    }
    let headers = fields.get(..observed.header_count()).ok_or(bound)?;
    crate::hurl::evaluate_hurl_with_header_predicates(
        plan,
        observed.response().status(),
        headers,
        observed.response().body(),
    )
    .map_err(HurlPredicateAttemptFailure::Evaluation)
}

#[cfg(test)]
#[path = "hurl_predicate_transport_tests.rs"]
mod tests;
