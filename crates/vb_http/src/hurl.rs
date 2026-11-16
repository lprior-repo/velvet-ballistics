//! Pure, bounded Hurl subset. Bodies remain opaque; this module performs no I/O.

#[path = "hurl_syntax.rs"]
mod syntax;
#[path = "hurl_integer.rs"]
mod integer;
#[path = "hurl_headers.rs"]
mod headers;
#[path = "hurl_evaluation.rs"]
mod evaluation;

pub use evaluation::{evaluate_hurl, evaluate_hurl_with_headers, evaluate_hurl_with_header_predicates};
pub use syntax::{parse_hurl, parse_hurl_with_headers, parse_hurl_with_header_predicates};

const MAX_SOURCE_BYTES: usize = 16384;
const MAX_LINE_BYTES: usize = 4096;
const MAX_TARGET_BYTES: usize = 2048;
const MAX_ASSERTIONS: usize = 64;
const MAX_BODY_BYTES: usize = 1048576;
const MAX_HEADERS: usize = 64;
const MAX_HEADER_NAME_BYTES: usize = 256;
const MAX_HEADER_VALUE_BYTES: usize = 8192;
const MAX_HEADER_BYTES: usize = 65536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HurlParseErrorKind {
    SourceBound,
    LineBound,
    Syntax,
    AssertionBound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HurlParseError {
    pub line: usize,
    pub kind: HurlParseErrorKind,
}

impl std::fmt::Display for HurlParseError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "Hurl {:?} at line {}", self.kind, self.line)
    }
}
impl std::error::Error for HurlParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HurlEvalError {
    BodyBound,
    InvalidStatus,
    RequiredStatus { expected: u16, actual: u16 },
    AssertionFailed { index: usize },
    HeaderObservationRequired,
    HeaderBound,
    InvalidHeader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HurlPredicateEvalError {
    Evaluation(HurlEvalError),
    HeaderPredicateTypeMismatch { index: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponseHeader<'a> {
    pub name: &'a str,
    pub value: &'a str,
}

#[derive(Debug, Clone)]
pub struct HurlPlan<'source> {
    target: &'source str,
    required_status: u16,
    assertions: [Option<Assertion<'source>>; MAX_ASSERTIONS],
    count: usize,
}

impl HurlPlan<'_> {
    #[must_use]
    pub const fn target(&self) -> &str {
        self.target
    }
    #[must_use]
    pub const fn required_status(&self) -> u16 {
        self.required_status
    }
    #[must_use]
    pub const fn assertion_count(&self) -> usize {
        self.count
    }
    #[must_use]
    pub fn requires_headers(&self) -> bool {
        self.assertions
            .iter()
            .flatten()
            .any(|assertion| matches!(assertion.query, Query::Header(_)))
    }
}

#[derive(Debug, Clone, Copy)]
struct Assertion<'a> {
    query: Query<'a>,
    negated: bool,
}
#[derive(Debug, Clone, Copy)]
enum Query<'a> {
    Status { comparison: Comparison, expected: i64 },
    Header(HeaderAssertion<'a>),
}
#[derive(Debug, Clone, Copy)]
struct HeaderAssertion<'a> {
    name: &'a str,
    predicate: HeaderPredicate<'a>,
}
#[derive(Debug, Clone, Copy)]
enum HeaderPredicate<'a> {
    Exists,
    IsString,
    IsList,
    IsCollection,
    IsEmpty,
    Equal(&'a str),
    NotEqual(&'a str),
    Text {
        kind: headers::HeaderTextPredicate,
        expected: &'a str,
    },
    Count {
        comparison: Comparison,
        expected: i64,
    },
}
#[derive(Debug, Clone, Copy)]
enum Comparison {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}
impl Comparison {
    fn parse(token: &str) -> Option<Self> {
        match token {
            "==" => Some(Self::Equal),
            "!=" => Some(Self::NotEqual),
            "<" => Some(Self::Less),
            "<=" => Some(Self::LessEqual),
            ">" => Some(Self::Greater),
            ">=" => Some(Self::GreaterEqual),
            _ => None,
        }
    }
    fn matches(self, actual: i64, expected: i64) -> bool {
        match self {
            Self::Equal => actual == expected,
            Self::NotEqual => actual != expected,
            Self::Less => actual < expected,
            Self::LessEqual => actual <= expected,
            Self::Greater => actual > expected,
            Self::GreaterEqual => actual >= expected,
        }
    }
}

#[cfg(test)]
#[path = "hurl_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "hurl_regression_tests.rs"]
mod regression_tests;

impl std::fmt::Display for HurlEvalError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "Hurl evaluation failed: {self:?}")
    }
}
impl std::error::Error for HurlEvalError {}
impl std::fmt::Display for HurlPredicateEvalError {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(out, "Hurl predicate evaluation failed: {self:?}")
    }
}
impl std::error::Error for HurlPredicateEvalError {}
