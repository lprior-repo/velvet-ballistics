/// Canonical spelling constants and shared utilities for the naming scan subsystem.
///
/// These are immutable domain constants and pure utility functions;
/// the core never mutates them.

use std::path::PathBuf;

/// Canonical product name token.
pub(crate) const CANONICAL_HYPHEN: &str = "velvet-ballastics";

/// Canonical crate-module / bead-database name token (underscore form).
pub(crate) const CANONICAL_UNDERSCORE: &str = "vb_cli";

/// Canonical language-version token.
pub(crate) const CANONICAL_LANGUAGE_VERSION: &str = "velvet-ballistics/v1";

/// Produce a config fingerprint based on whether a report destination
/// was supplied.
pub(crate) fn fingerprint_for_destination(destination: Option<&PathBuf>) -> String {
    if destination.is_some() {
        "vb-37lc-maximum-bounded-config".to_owned()
    } else {
        "vb-37lc-minimum-config".to_owned()
    }
}

/// Construct an [`NamingScanError::InvalidConfiguration`] error.
///
/// Generic over the ignored `Ok` type so callers can use it in any
/// `Result<T, NamingScanError>` position.
///
/// Shared by both the build and validate submodules; no domain state
/// dependency makes it a natural home in the types module.
pub(crate) fn invalid_config<T>(reason: &str) -> Result<T, crate::naming_scan::types::NamingScanError> {
    Err(crate::naming_scan::types::NamingScanError::InvalidConfiguration {
        reason: reason.to_owned(),
    })
}
