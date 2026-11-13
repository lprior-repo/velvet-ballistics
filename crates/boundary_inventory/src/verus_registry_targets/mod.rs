use std::collections::HashSet;
use std::ffi::OsString;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use std::string::FromUtf8Error;

#[derive(Debug, Eq, PartialEq)]
enum SelectionError {
    NoTargets,
}

#[derive(Debug)]
enum Failure {
    Usage,
    Read(io::Error),
    InvalidUtf8(FromUtf8Error),
    NoTargets,
    Output(io::Error),
}

impl Display for Failure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage => write!(formatter, "usage: verus-registry-targets FILE"),
            Self::Read(error) => write!(formatter, "failed to read registry: {error}"),
            Self::InvalidUtf8(error) => write!(formatter, "registry is not valid UTF-8: {error}"),
            Self::NoTargets => write!(formatter, "no Verus targets found"),
            Self::Output(error) => write!(formatter, "failed to write targets: {error}"),
        }
    }
}

fn is_python_whitespace(character: char) -> bool {
    character.is_whitespace() || matches!(character, '\u{1c}'..='\u{1f}')
}

fn is_splitline_boundary(character: char) -> bool {
    matches!(
        character,
        '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{1c}'..='\u{1e}' | '\u{85}' | '\u{2028}' | '\u{2029}'
    )
}

fn extract_line(line: &str) -> Option<&str> {
    let line = line.trim_start_matches(is_python_whitespace);
    let mut rest = line
        .strip_prefix("verus:")?
        .trim_start_matches(is_python_whitespace);
    if rest.starts_with('"') || rest.starts_with('\'') {
        rest = rest.get(1..)?;
    }
    let (candidate, tail, closing_quote) = split_capture_tail(rest)?;
    if closing_quote && !tail.is_empty() && !tail.starts_with('#') {
        return None;
    }
    let target = candidate.trim_matches(is_python_whitespace);
    (!target.is_empty()).then_some(target)
}

fn split_capture_tail(rest: &str) -> Option<(&str, &str, bool)> {
    let boundary = rest
        .char_indices()
        .find(|(_, character)| matches!(*character, '\'' | '"' | '#'));
    Some(match boundary {
        Some((offset, '#')) => (
            rest.get(..offset)?,
            rest.get(offset..)?.trim_start_matches(is_python_whitespace),
            false,
        ),
        Some((offset, quote)) => {
            let candidate = rest.get(..offset)?;
            let tail = rest
                .get(offset..)?
                .get(quote.len_utf8()..)?
                .trim_start_matches(is_python_whitespace);
            (candidate, tail, true)
        }
        None => (rest, "", false),
    })
}

fn select_targets(input: &str) -> Result<Vec<&str>, SelectionError> {
    let mut seen = HashSet::new();
    let targets = input
        .split(is_splitline_boundary)
        .filter_map(extract_line)
        .filter(|target| seen.insert(*target))
        .collect::<Vec<_>>();
    if targets.is_empty() {
        return Err(SelectionError::NoTargets);
    }
    Ok(targets)
}

fn decode_registry(bytes: Vec<u8>) -> Result<String, Failure> {
    String::from_utf8(bytes).map_err(Failure::InvalidUtf8)
}

fn read_registry(path: &Path) -> Result<String, Failure> {
    let bytes = fs::read(path).map_err(Failure::Read)?;
    decode_registry(bytes)
}

fn read_targets(path: &Path) -> Result<Vec<String>, Failure> {
    let registry = read_registry(path)?;
    let selected =
        select_targets(&registry).map_err(|SelectionError::NoTargets| Failure::NoTargets)?;
    Ok(selected.into_iter().map(str::to_owned).collect())
}

fn write_targets<W: Write>(targets: &[String], output: &mut W) -> io::Result<()> {
    targets.iter().try_for_each(|target| {
        output.write_all(target.as_bytes())?;
        output.write_all(b"\n")
    })?;
    output.flush()
}

fn report<W: Write>(failure: Failure, diagnostics: &mut W) -> ExitCode {
    let status = match &failure {
        Failure::Usage => 2,
        _ => 1,
    };
    match writeln!(diagnostics, "{failure}") {
        Ok(()) => ExitCode::from(status),
        Err(_) => ExitCode::FAILURE,
    }
}

pub(super) fn run<I, W, E>(arguments: I, output: &mut W, diagnostics: &mut E) -> ExitCode
where
    I: IntoIterator<Item = OsString>,
    W: Write,
    E: Write,
{
    let mut arguments = arguments.into_iter();
    let Some(path) = arguments.next() else {
        return report(Failure::Usage, diagnostics);
    };
    if arguments.next().is_some() {
        return report(Failure::Usage, diagnostics);
    }
    match read_targets(Path::new(&path)) {
        Ok(targets) => match write_targets(&targets, output) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => report(Failure::Output(error), diagnostics),
        },
        Err(error) => report(error, diagnostics),
    }
}

#[cfg(test)]
mod tests;
