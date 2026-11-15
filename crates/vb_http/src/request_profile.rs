//! Finite wire profile and projection identity hashed by the GitHub provider.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct RequestProfile {
    pub(crate) accept: &'static str,
    pub(crate) user_agent: &'static str,
    pub(crate) api_version: Option<&'static str>,
    pub(crate) projection_schema: &'static str,
}

/// The transport must send these same header values for this request profile.
pub const GITHUB: RequestProfile = RequestProfile {
    accept: "application/vnd.github+json",
    user_agent: "velvet-ballistics/0.1",
    api_version: Some("2026-03-10"),
    projection_schema: "github-workflow-count-i64-v1",
};

/// Select the complete finite wire and digest profile for the requested media.
pub const fn for_media(media: crate::JsonMediaType) -> RequestProfile {
    match media {
        crate::JsonMediaType::GitHub => GITHUB,
    }
}

#[cfg(test)]
#[path = "request_profile_tests.rs"]
mod tests;
