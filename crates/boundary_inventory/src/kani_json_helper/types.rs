use std::fmt;
use std::io;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct RequestedPackage(pub(super) String);

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ManifestSelection(pub(super) String);

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ValidatedJson;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct ReportedHarnessTotal(pub(super) String);

#[derive(Debug)]
pub(super) enum Operation {
    ManifestPath(String, RequestedPackage),
    ValidateJson(String),
    StandardHarnesses(String),
}

#[derive(Debug)]
pub(super) enum HelperError {
    Usage,
    InvalidArgumentEncoding,
    Read { path: String, source: io::Error },
    Json(serde_json::Error),
    MetadataShape(&'static str),
    PackageMatches { requested: String, count: usize },
    InvalidManifestPath(String),
    ListingRoot,
    TotalsShape,
    HarnessCountShape,
    CountOverflow,
    Output(io::Error),
    Diagnostic(io::Error),
}

impl fmt::Display for HelperError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => write!(
                formatter,
                "usage: manifest-path FILE PACKAGE | validate-json FILE | standard-harnesses FILE"
            ),
            Self::InvalidArgumentEncoding => write!(formatter, "arguments must be valid UTF-8"),
            Self::Read { path, source } => write!(formatter, "cannot read {path}: {source}"),
            Self::Json(source) => write!(formatter, "invalid JSON: {source}"),
            Self::MetadataShape(field) => write!(formatter, "metadata has invalid {field}"),
            Self::PackageMatches { requested, count } => write!(
                formatter,
                "expected exactly one package named {requested:?}, found {count}"
            ),
            Self::InvalidManifestPath(name) => {
                write!(formatter, "package {name:?} has no nonempty manifest_path")
            }
            Self::ListingRoot => write!(formatter, "listing root must be an object"),
            Self::TotalsShape => write!(formatter, "listing totals must be an object"),
            Self::HarnessCountShape => {
                write!(formatter, "standard-harnesses must be a number or string")
            }
            Self::CountOverflow => write!(formatter, "package match count overflow"),
            Self::Output(source) => write!(formatter, "cannot write helper output: {source}"),
            Self::Diagnostic(source) => {
                write!(formatter, "cannot write helper diagnostic: {source}")
            }
        }
    }
}
