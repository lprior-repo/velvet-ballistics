use crate::{
    AttemptFailure, DecodeError, DecodeLimits, HttpLimits, HttpObservation, JsonMediaType,
    PinnedGet, RequestRejection, ResponseStatus, WorkflowCount, decode_workflow_count_body,
    get_once,
};
use std::net::SocketAddr;

/// Validated path segments, not a URL, token or proof of repository access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryRef {
    owner: String,
    repository: String,
}

impl RepositoryRef {
    pub fn new(owner: &str, repository: &str) -> Result<Self, RequestRejection> {
        if !segment(owner) || !segment(repository) {
            return Err(RequestRejection::Endpoint);
        }
        Ok(Self {
            owner: owner.to_owned(),
            repository: repository.to_owned(),
        })
    }
}

fn segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

/// One read-only provider operation. No workflow dispatch, pagination or hidden retries.
/// An execution authority check and durable intent are required outside this cold adapter.
pub struct WorkflowCountAction {
    endpoint: PinnedGet,
    http: HttpLimits,
    decode: DecodeLimits,
}

impl WorkflowCountAction {
    pub(crate) const fn deadline(&self) -> std::time::Duration {
        self.http.deadline()
    }

    /// Digest binds this exact endpoint/peer, finite profiles and projection schema.
    pub fn request_digest(&self) -> Result<vb_core::action_pack::RequestDigest, RequestRejection> {
        self.digest_with_profile(crate::request_profile::for_media(self.endpoint.media))
    }

    fn digest_with_profile(
        &self,
        profile: crate::request_profile::RequestProfile,
    ) -> Result<vb_core::action_pack::RequestDigest, RequestRejection> {
        let bytes = postcard::to_allocvec(&(
            1_u16,
            "github.actions.workflow-count",
            profile,
            self.endpoint.uri.to_string(),
            self.endpoint.address.to_string(),
            self.endpoint.media,
            self.http,
            self.decode,
        ))
        .map_err(|_| RequestRejection::Limits)?;
        Ok(vb_core::action_pack::RequestDigest::from_digest(
            vb_core::WorkflowDigest::from_bytes(blake3::hash(&bytes).into()),
        ))
    }

    #[cfg(test)]
    pub(crate) fn fixture(endpoint: PinnedGet, http: HttpLimits, decode: DecodeLimits) -> Self {
        Self {
            endpoint,
            http,
            decode,
        }
    }
    pub fn new(
        repository: &RepositoryRef,
        peer: SocketAddr,
        http: HttpLimits,
        decode: DecodeLimits,
    ) -> Result<Self, RequestRejection> {
        let uri = format!(
            "https://api.github.com/repos/{}/{}/actions/workflows?per_page=1",
            repository.owner, repository.repository
        );
        let endpoint = PinnedGet::https(&uri, peer, JsonMediaType::GitHub)?;
        Ok(Self {
            endpoint,
            http,
            decode,
        })
    }

    /// Returns an observation only. It is neither persisted nor accepted by the runtime.
    pub(crate) fn observe(&self) -> Result<WorkflowCount, WorkflowObservationFailure> {
        let observed = get_once(&self.endpoint, self.http)?;
        project_workflow_count(&observed, self.decode)
    }
}

/// Failure is explicit; no variant grants automatic retry or changes runtime state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WorkflowObservationFailure {
    #[error(transparent)]
    Attempt(#[from] AttemptFailure),
    #[error("workflow request returned HTTP {0} instead of 200")]
    Status(ResponseStatus),
    #[error("workflow response is not UTF-8 JSON media")]
    MediaType,
    #[error(transparent)]
    Decode(#[from] DecodeError),
}

pub(crate) fn project_workflow_count(
    observed: &HttpObservation,
    limits: DecodeLimits,
) -> Result<WorkflowCount, WorkflowObservationFailure> {
    if observed.status() != 200 {
        return Err(WorkflowObservationFailure::Status(
            observed.response_status(),
        ));
    }
    if !observed.has_json_media_type() {
        return Err(WorkflowObservationFailure::MediaType);
    }
    Ok(decode_workflow_count_body(observed.body(), limits)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn constructs_exact_public_read_without_injectable_path() -> Result<(), RequestRejection> {
        for (owner, repo) in [
            ("", "repo"),
            ("owner", ".."),
            ("ow/ner", "repo"),
            ("owner", "a?b"),
            ("owner", "a%2fb"),
            ("ow\r\nner", "repo"),
            ("é", "repo"),
        ] {
            assert!(RepositoryRef::new(owner, repo).is_err());
        }
        assert!(RepositoryRef::new(&"a".repeat(101), "repo").is_err());
        let repository = RepositoryRef::new("lprior-repo", "velvet-ballistics")?;
        let action = WorkflowCountAction::new(
            &repository,
            SocketAddr::from(([1, 1, 1, 1], 443)),
            HttpLimits::new(1024, 1024, Duration::from_secs(1))?,
            DecodeLimits::new(1024, 8, 128, 128).map_err(|_| RequestRejection::Limits)?,
        )?;
        assert_eq!(
            action.endpoint.uri.to_string(),
            "https://api.github.com/repos/lprior-repo/velvet-ballistics/actions/workflows?per_page=1"
        );
        assert!(matches!(action.endpoint.media, JsonMediaType::GitHub));
        let original = action.request_digest()?;
        let changed_api = crate::request_profile::RequestProfile {
            api_version: Some("2099-01-01"),
            ..crate::request_profile::GITHUB
        };
        let changed_schema = crate::request_profile::RequestProfile {
            projection_schema: "github-workflow-count-i64-v2",
            ..crate::request_profile::GITHUB
        };
        assert_ne!(original, action.digest_with_profile(changed_api)?);
        assert_ne!(original, action.digest_with_profile(changed_schema)?);
        assert_eq!(
            original,
            action.digest_with_profile(crate::request_profile::GITHUB)?
        );
        Ok(())
    }
}

#[cfg(test)]
mod wire_tests;
