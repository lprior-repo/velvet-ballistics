use super::*;

const ZERO: &[u8] = b"HTTP/1.1 200 Status\r\nContent-Type: application/json\r\nContent-Length: 17\r\nConnection: close\r\n\r\n{\"total_count\":0}";

const NON200: &[u8] = b"HTTP/1.1 503 Status\r\nContent-Type: application/json\r\nContent-Length: 17\r\nConnection: close\r\n\r\n{\"total_count\":0}";

const MALFORMED: &[u8] = b"HTTP/1.1 200 Status\r\nContent-Type: application/json\r\nContent-Length: 1\r\nConnection: close\r\n\r\n{";

const DUPLICATE: &[u8] = b"HTTP/1.1 200 Status\r\nContent-Type: application/json\r\nContent-Length: 33\r\nConnection: close\r\n\r\n{\"total_count\":0,\"total_count\":1}";

const WRONG_TYPE: &[u8] = b"HTTP/1.1 200 Status\r\nContent-Type: application/json\r\nContent-Length: 19\r\nConnection: close\r\n\r\n{\"total_count\":\"0\"}";

const POSITIVE: &[u8] = b"HTTP/1.1 200 Status\r\nContent-Type: application/json\r\nContent-Length: 17\r\nConnection: close\r\n\r\n{\"total_count\":3}";

// Transport rejects incomplete declared bodies and conflicting framing, never
// converting a partial JSON prefix or ambiguous length into a workflow count.
const TRUNCATED: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 18\r\nConnection: close\r\n\r\n{\"total_count\":3}";

const CONFLICTING_LENGTH: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 17\r\nContent-Length: 18\r\nConnection: close\r\n\r\n{\"total_count\":3}";

const LENGTH_AND_CHUNKED: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 17\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n11\r\n{\"total_count\":3}\r\n0\r\n\r\n";

fn workflow_case(
    owner: &mut CustodyOwner,
    response: &'static [u8],
) -> Result<Result<crate::WorkflowCount, WorkflowObservationFailure>, CaseFailure> {
    let http = limits(1024, 1024)?;
    let decode = decode()?;
    let (observed, capture) = with_owned_response(owner, response, |endpoint| {
        let action = WorkflowCountAction::fixture(endpoint, http, decode);
        crate::observe_workflow_count(&action)
    })?;
    profile(&capture)?;
    Ok(observed)
}

/// Explicit invocation only, closed cases, no arbitrary URL, callback or report input.
/// Private implementation: callers cannot supply cases, URLs, reports or callbacks.
/// Witness construction occurs only after every check and worker reaping succeeds.
pub(super) fn run_local_stage() -> Result<ExecutedLocalHttpConformance, CaseFailure> {
    with_custody(|owner| {
        if owner.worker.is_some() {
            owner.wait_reap()?;
        }
        let mut receipts = Vec::new();
        receipts
            .try_reserve_exact(16)
            .map_err(|_| CaseFailure::Check("receipt capacity"))?;
        require(
            workflow_case(owner, ZERO)?.map(|count| count.get()) == Ok(0),
            "zero count",
        )?;
        receipts.push("zero count");
        require(
            workflow_case(owner, POSITIVE)?.map(|count| count.get()) == Ok(3),
            "positive count",
        )?;
        receipts.push("positive count");
        require(
            matches!(workflow_case(owner, NON200)?, Err(WorkflowObservationFailure::Status(status)) if status.0 == 503),
            "non-200",
        )?;
        receipts.push("non-200");
        require(
            workflow_case(owner, MALFORMED)?
                == Err(WorkflowObservationFailure::Decode(DecodeError::InvalidJson)),
            "malformed",
        )?;
        receipts.push("malformed");
        require(
            workflow_case(owner, DUPLICATE)?
                == Err(WorkflowObservationFailure::Decode(
                    DecodeError::DuplicateWorkflowCount,
                )),
            "duplicate",
        )?;
        receipts.push("duplicate");
        require(
            workflow_case(owner, WRONG_TYPE)?
                == Err(WorkflowObservationFailure::Decode(
                    DecodeError::InvalidWorkflowCount,
                )),
            "wrong type",
        )?;
        receipts.push("wrong type");
        let http = limits(1, 1024)?;
        let (observed, capture) =
            with_owned_response(owner, POSITIVE, |endpoint| crate::get_once(&endpoint, http))?;
        profile(&capture)?;
        require(
            matches!(observed, Err(crate::AttemptFailure::BodyLimit)),
            "body limit",
        )?;
        receipts.push("body limit");
        let http = limits(1024, 16)?;
        let (observed, capture) =
            with_owned_response(owner, POSITIVE, |endpoint| crate::get_once(&endpoint, http))?;
        profile(&capture)?;
        require(
            matches!(observed, Err(crate::AttemptFailure::HeaderLimit)),
            "header limit",
        )?;
        receipts.push("header limit");
        for (name, response) in [
            ("truncated body", TRUNCATED),
            ("conflicting length", CONFLICTING_LENGTH),
            ("length plus chunked", LENGTH_AND_CHUNKED),
        ] {
            require(
                workflow_case(owner, response)?
                    == Err(WorkflowObservationFailure::Attempt(
                        crate::AttemptFailure::Transport,
                    )),
                name,
            )?;
            receipts.push(name);
        }
        digest_separation()?;
        receipts.push("digest separation");
        profile_digest_separation()?;
        receipts.push("profile digest separation");
        timed_body_case(owner, BodyTiming::Immediate)?;
        receipts.push("body before deadline");
        timed_body_case(owner, BodyTiming::HoldUntilOperationReturns)?;
        receipts.push("body timeout after headers");
        let http = limits(1024, 1024)?;
        let ((result, headers_received), capture) = with_owned_response(
            owner,
            b"HTTP/1.1 200 OK\r\nContent-Length: 17\r\n",
            |endpoint| crate::transport::local_observed_get_once(&endpoint, http),
        )?;
        profile(&capture)?;
        require(
            !headers_received,
            "pre-header failure has no stale parsed marker",
        )?;
        require(
            matches!(result, Err(crate::AttemptFailure::Transport)),
            "incomplete headers transport failure",
        )?;
        receipts.push("pre-header failure marker false");
        if owner.worker.is_some() {
            return Err(FixtureFailure::CleanupPending.into());
        }
        Ok(ExecutedLocalHttpConformance {
            receipts,
            schema: SCHEMA,
            source_fingerprint: source_fingerprint()?,
        })
    })
}
