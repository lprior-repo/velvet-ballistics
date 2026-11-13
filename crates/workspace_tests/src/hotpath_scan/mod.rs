mod allow;
mod scan;
mod walk;

use std::fmt;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const ROOTS: &[&str] = &[
    "crates/vb_core/src",
    "crates/vb_runtime/src",
    "crates/vb_storage/src",
    "crates/vb_ipc/src",
];
const ALLOW_PATH: &str = "scripts/hotpath-scan.allow";

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Token {
    HashMap,
    IndexMap,
    IndexSet,
    BTreeMap,
    SyncMpsc,
    MpscChannel,
    Channel,
}

const TOKENS: &[Token] = &[
    Token::HashMap,
    Token::IndexMap,
    Token::IndexSet,
    Token::BTreeMap,
    Token::SyncMpsc,
    Token::MpscChannel,
    Token::Channel,
];

impl Token {
    fn text(self) -> &'static str {
        match self {
            Self::HashMap => "HashMap",
            Self::IndexMap => "IndexMap",
            Self::IndexSet => "IndexSet",
            Self::BTreeMap => "BTreeMap",
            Self::SyncMpsc => "std::sync::mpsc",
            Self::MpscChannel => "mpsc::channel",
            Self::Channel => "channel(",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        TOKENS.iter().copied().find(|token| token.text() == text)
    }
}

pub(crate) enum Rejection {
    Shape,
    Path,
    Token,
    Metadata,
}

impl Rejection {
    fn category(&self) -> &'static str {
        match self {
            Self::Path => "OverbroadException",
            Self::Token => "UnknownTokenException",
            Self::Shape | Self::Metadata => "MalformedException",
        }
    }

    fn detail(&self) -> String {
        match self {
            Self::Shape => {
                "expected path|token|owner=...|reviewed_by=...|test=...|reason=...".into()
            }
            Self::Path => "path must be exact crates/*/src/*.rs".into(),
            Self::Token => format!(
                "token must be one of {}",
                TOKENS
                    .iter()
                    .map(|t| t.text())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::Metadata => "missing owner/reviewed_by/test/reason".into(),
        }
    }
}

pub(crate) enum Failure {
    Io(io::Error),
    InvalidInvocation,
    Allow { line: usize, kind: Rejection },
    CountOverflow,
    InvalidPath(PathBuf),
}

impl Failure {
    pub(crate) fn exit_code(&self) -> u8 {
        match self {
            Self::InvalidInvocation => 64,
            Self::Allow { .. } => 3,
            Self::Io(_) | Self::CountOverflow | Self::InvalidPath(_) => 1,
        }
    }
}

impl From<io::Error> for Failure {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "HotpathScanIoError: {error}"),
            Self::InvalidInvocation => write!(f, "InvalidInvocation: run from repository root"),
            Self::Allow { line, kind } => write!(
                f,
                "{}: {ALLOW_PATH}:{line} {}",
                kind.category(),
                kind.detail()
            ),
            Self::CountOverflow => write!(f, "HotpathScanCountOverflow"),
            Self::InvalidPath(path) => write!(f, "HotpathScanInvalidPath: {}", path.display()),
        }
    }
}

pub(crate) fn run(root: &Path, out: &mut impl Write) -> Result<u8, Failure> {
    if !root.join("Cargo.toml").is_file() || !root.join("crates").is_dir() {
        return Err(Failure::InvalidInvocation);
    }
    scan::Scanner::new(allow::load(root)?, out).run(root)
}

fn increment(counter: &mut usize) -> Result<(), Failure> {
    *counter = counter.checked_add(1).ok_or(Failure::CountOverflow)?;
    Ok(())
}

fn normalized_lines(text: &str) -> String {
    text.replace("\r\n", "\n").replace(
        [
            '\r', '\u{b}', '\u{c}', '\u{1c}', '\u{1d}', '\u{1e}', '\u{85}', '\u{2028}', '\u{2029}',
        ],
        "\n",
    )
}

fn legacy_whitespace(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '\u{1c}' | '\u{1d}' | '\u{1e}' | '\u{1f}')
}
