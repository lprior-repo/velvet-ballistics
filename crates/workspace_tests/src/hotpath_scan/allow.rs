use std::collections::BTreeSet;
use std::io;
use std::path::Path;

use super::{ALLOW_PATH, Failure, Rejection, Token, legacy_whitespace, normalized_lines};

#[derive(Default)]
pub(super) struct Allowances(BTreeSet<(String, Token)>);

impl Allowances {
    pub(super) fn contains(&self, path: &str, token: Token) -> bool {
        self.0
            .iter()
            .any(|(allowed, item)| allowed == path && *item == token)
    }
}

pub(super) fn load(root: &Path) -> Result<Allowances, Failure> {
    match std::fs::read_to_string(root.join(ALLOW_PATH)) {
        Ok(text) => parse(&text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Allowances::default()),
        Err(error) => Err(error.into()),
    }
}

fn parse(text: &str) -> Result<Allowances, Failure> {
    normalized_lines(text)
        .split_terminator('\n')
        .enumerate()
        .try_fold(Allowances::default(), |mut allowed, (index, raw)| {
            let line = index.checked_add(1).ok_or(Failure::CountOverflow)?;
            let text = raw.trim_matches(legacy_whitespace);
            if !text.is_empty() && !text.starts_with('#') {
                allowed.0.extend(Some(parse_fields(
                    &text.split('|').collect::<Vec<_>>(),
                    line,
                )?));
            }
            Ok(allowed)
        })
}

fn parse_fields(parts: &[&str], line: usize) -> Result<(String, Token), Failure> {
    let [path, token, owner, reviewed, test, reason] = parts else {
        return Err(Failure::Allow {
            line,
            kind: Rejection::Shape,
        });
    };
    if path.contains('*') || !path.starts_with("crates/") || !path.ends_with(".rs") {
        return Err(Failure::Allow {
            line,
            kind: Rejection::Path,
        });
    }
    let token = Token::parse(token).ok_or(Failure::Allow {
        line,
        kind: Rejection::Token,
    })?;
    validate_metadata([owner, reviewed, test, reason], line)?;
    Ok(((*path).to_owned(), token))
}

fn validate_metadata(values: [&str; 4], line: usize) -> Result<(), Failure> {
    let valid = values
        .iter()
        .zip(["owner=", "reviewed_by=", "test=", "reason="])
        .all(|(value, prefix)| value.starts_with(prefix));
    if valid {
        Ok(())
    } else {
        Err(Failure::Allow {
            line,
            kind: Rejection::Metadata,
        })
    }
}
