use super::{
    HeaderAssertion, HeaderPredicate, HurlEvalError, MAX_HEADER_BYTES, MAX_HEADER_NAME_BYTES,
    MAX_HEADER_VALUE_BYTES, MAX_HEADERS, ResponseHeader,
};

#[derive(Debug, Clone, Copy)]
pub(super) enum HeaderFilterError {
    MissingInput,
    InvalidInputType,
    PredicateTypeMismatch,
    Bound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HeaderTextPredicate {
    StartsWith,
    EndsWith,
    Contains,
    Includes,
}

pub(super) fn header_name_valid(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'\x60'
                        | b'|'
                        | b'~'
                )
        })
}

pub(super) fn validate_headers(headers: &[ResponseHeader<'_>]) -> Result<(), HurlEvalError> {
    if headers.len() > MAX_HEADERS {
        return Err(HurlEvalError::HeaderBound);
    }
    let mut total = 0_usize;
    for header in headers {
        if header.name.len() > MAX_HEADER_NAME_BYTES || header.value.len() > MAX_HEADER_VALUE_BYTES
        {
            return Err(HurlEvalError::HeaderBound);
        }
        total = total
            .checked_add(header.name.len())
            .and_then(|size| size.checked_add(header.value.len()))
            .ok_or(HurlEvalError::HeaderBound)?;
        if total > MAX_HEADER_BYTES {
            return Err(HurlEvalError::HeaderBound);
        }
    }
    if headers.iter().any(|header| {
        !header_name_valid(header.name)
            || !header
                .value
                .bytes()
                .all(|byte| byte == b'\t' || (b' '..=b'~').contains(&byte))
    }) {
        return Err(HurlEvalError::InvalidHeader);
    }
    Ok(())
}

pub(super) fn header_matches(
    assertion: &HeaderAssertion<'_>,
    headers: &[ResponseHeader<'_>],
) -> Result<bool, HeaderFilterError> {
    let mut matching = headers
        .iter()
        .filter(|header| header.name.eq_ignore_ascii_case(assertion.name));
    let first = matching.next();
    let second = matching.next();
    let multiple = second.is_some();
    match assertion.predicate {
        HeaderPredicate::Exists => Ok(first.is_some()),
        HeaderPredicate::IsString => Ok(first.is_some() && !multiple),
        HeaderPredicate::IsList | HeaderPredicate::IsCollection => Ok(multiple),
        HeaderPredicate::Text { kind, expected } => text_matches(
            kind,
            expected,
            first.map(|header| header.value),
            second.map(|header| header.value),
            matching.map(|header| header.value),
        ),
        HeaderPredicate::IsEmpty => {
            Ok(!multiple && first.is_some_and(|header| header.value.is_empty()))
        }
        HeaderPredicate::Equal(expected) => {
            Ok(!multiple && first.is_some_and(|header| header.value == expected))
        }
        HeaderPredicate::NotEqual(expected) => {
            Ok(first.is_some_and(|header| multiple || header.value != expected))
        }
        HeaderPredicate::Count {
            comparison,
            expected,
        } => {
            if first.is_none() {
                return Err(HeaderFilterError::MissingInput);
            }
            if !multiple {
                return Err(HeaderFilterError::InvalidInputType);
            }
            let count = matching
                .count()
                .checked_add(2)
                .ok_or(HeaderFilterError::Bound)?;
            let count = i64::try_from(count).map_err(|_| HeaderFilterError::Bound)?;
            Ok(comparison.matches(count, expected))
        }
    }
}

fn text_matches<'response>(
    kind: HeaderTextPredicate,
    expected: &str,
    first: Option<&'response str>,
    second: Option<&'response str>,
    remaining: impl Iterator<Item = &'response str>,
) -> Result<bool, HeaderFilterError> {
    match (kind, first, second) {
        (_, None, _) => Ok(false),
        (HeaderTextPredicate::StartsWith, Some(value), None) => Ok(value.starts_with(expected)),
        (HeaderTextPredicate::EndsWith, Some(value), None) => Ok(value.ends_with(expected)),
        (HeaderTextPredicate::Contains, Some(value), None) => Ok(value.contains(expected)),
        (
            HeaderTextPredicate::Contains | HeaderTextPredicate::Includes,
            Some(first),
            Some(second),
        ) => Ok([first, second]
            .into_iter()
            .chain(remaining)
            .any(|value| value == expected)),
        _ => Err(HeaderFilterError::PredicateTypeMismatch),
    }
}
