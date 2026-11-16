use crate::hurl::{HurlEvalError, HurlPlan};
use crate::{AttemptFailure, HttpLimits, HttpObservation, PinnedGet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum HurlAttemptFailure {
    #[error("Hurl target differs from the pinned request")]
    Target,
    #[error(transparent)]
    Attempt(#[from] AttemptFailure),
    #[error("Hurl evaluation failed: {0:?}")]
    Evaluation(HurlEvalError),
}

/// Read one pinned target and evaluate its bounded observation. No receipt is committed.
pub fn execute_hurl(
    plan: &HurlPlan<'_>,
    endpoint: &PinnedGet,
    limits: HttpLimits,
) -> Result<HttpObservation, HurlAttemptFailure> {
    let started = std::time::Instant::now();
    let target = endpoint
        .uri
        .path_and_query()
        .map_or("/", |path| path.as_str());
    if plan.target() != target {
        return Err(HurlAttemptFailure::Target);
    }
    if plan.requires_headers() {
        return Err(HurlAttemptFailure::Evaluation(
            HurlEvalError::HeaderObservationRequired,
        ));
    }
    let observed = crate::get_once(endpoint, limits)?;
    let evaluated = crate::hurl::evaluate_hurl(plan, observed.status(), observed.body());
    if started.elapsed() > limits.deadline() {
        return Err(AttemptFailure::Timeout.into());
    }
    evaluated.map_err(HurlAttemptFailure::Evaluation)?;
    Ok(observed)
}

#[cfg(test)]
mod tests;

/// Explicit opt-in to the supported parsed header profile; one exchange, no receipt commit.
/// Wrong required status precedes deferred header conversion/policy errors after a
/// successfully bounded exchange. Existing transport framing/body/deadline errors still reject.
pub fn execute_hurl_with_headers(
    plan: &HurlPlan<'_>,
    endpoint: &PinnedGet,
    limits: HttpLimits,
) -> Result<crate::HttpObservationWithHeaders, HurlAttemptFailure> {
    let started = std::time::Instant::now();
    let target = endpoint
        .uri
        .path_and_query()
        .map_or("/", |path| path.as_str());
    if plan.target() != target {
        return Err(HurlAttemptFailure::Target);
    }
    let (response, headers) = crate::transport::get_once_for_headers(endpoint, limits)?;
    let result = (|| {
        if response.status() != plan.required_status() {
            return Err(HurlAttemptFailure::Evaluation(
                HurlEvalError::RequiredStatus {
                    expected: plan.required_status(),
                    actual: response.status(),
                },
            ));
        }
        let observed = crate::HttpObservationWithHeaders::new(response, headers?);
        evaluate_captured_headers(plan, &observed)?;
        Ok(observed)
    })();
    if started.elapsed() > limits.deadline() {
        return Err(AttemptFailure::Timeout.into());
    }
    result
}
fn evaluate_captured_headers(
    plan: &HurlPlan<'_>,
    observed: &crate::HttpObservationWithHeaders,
) -> Result<(), HurlAttemptFailure> {
    use crate::hurl::ResponseHeader;
    let mut fields = [ResponseHeader {
        name: "",
        value: "",
    }; crate::observation::MAX_OBSERVED_HEADERS];
    for (index, field) in observed.headers().enumerate() {
        let slot = fields
            .get_mut(index)
            .ok_or(HurlAttemptFailure::Evaluation(HurlEvalError::HeaderBound))?;
        *slot = ResponseHeader {
            name: field.name,
            value: field.value,
        };
    }
    let headers = fields
        .get(..observed.header_count())
        .ok_or(HurlAttemptFailure::Evaluation(HurlEvalError::HeaderBound))?;
    crate::hurl::evaluate_hurl_with_headers(
        plan,
        observed.response().status(),
        headers,
        observed.response().body(),
    )
    .map_err(HurlAttemptFailure::Evaluation)
}
#[cfg(test)]
mod header_tests;

#[cfg(test)]
mod count_tests;
