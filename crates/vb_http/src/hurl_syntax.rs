use super::headers::{HeaderTextPredicate, header_name_valid};
use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Profile {
    Status,
    Headers,
    Predicates,
}

pub fn parse_hurl(source: &str) -> Result<HurlPlan<'_>, HurlParseError> {
    parse(source, Profile::Status)
}
pub fn parse_hurl_with_headers(source: &str) -> Result<HurlPlan<'_>, HurlParseError> {
    parse(source, Profile::Headers)
}
pub fn parse_hurl_with_header_predicates(source: &str) -> Result<HurlPlan<'_>, HurlParseError> {
    parse(source, Profile::Predicates)
}
fn error(line: usize, kind: HurlParseErrorKind) -> HurlParseError {
    HurlParseError { line, kind }
}
fn physical(raw: &str) -> &str {
    match raw.strip_suffix('\n') {
        Some(line) => line.strip_suffix('\r').map_or(line, |line| line),
        None => raw,
    }
}
fn trim(value: &str) -> &str {
    value.trim_matches(|c| c == ' ' || c == '\t')
}
fn parse(source: &str, profile: Profile) -> Result<HurlPlan<'_>, HurlParseError> {
    if source.len() > MAX_SOURCE_BYTES {
        return Err(error(0, HurlParseErrorKind::SourceBound));
    }
    for (index, raw) in source.split_inclusive('\n').enumerate() {
        if physical(raw).len() > MAX_LINE_BYTES {
            return Err(error(
                index.saturating_add(1),
                HurlParseErrorKind::LineBound,
            ));
        }
    }
    let mut plan = HurlPlan {
        target: "",
        required_status: 0,
        assertions: [None; MAX_ASSERTIONS],
        count: 0,
    };
    let mut phase = 0_u8;
    for (index, raw) in source.split_inclusive('\n').enumerate() {
        let line_number = index.saturating_add(1);
        let syntax = || error(line_number, HurlParseErrorKind::Syntax);
        let line = trim(physical(raw));
        if line.contains('\r') {
            return Err(syntax());
        }
        if line.is_empty() {
            continue;
        }
        match phase {
            0 => {
                let (method, target) = word(line).ok_or_else(syntax)?;
                if method != "GET"
                    || target.len() > MAX_TARGET_BYTES
                    || target.is_empty()
                    || !target.bytes().all(|b| {
                        (b'!'..=b'~').contains(&b) && !matches!(b, b'#' | b'\\' | b'{' | b'}')
                    })
                {
                    return Err(syntax());
                }
                plan.target = target;
                phase = 1;
            }
            1 => {
                let (version, status) = word(line).ok_or_else(syntax)?;
                if version != "HTTP"
                    || status.len() != 3
                    || !status.bytes().all(|b| b.is_ascii_digit())
                {
                    return Err(syntax());
                }
                let status = status.parse::<u16>().map_err(|_| syntax())?;
                if !(100..=599).contains(&status) {
                    return Err(syntax());
                }
                plan.required_status = status;
                phase = 2;
            }
            2 => {
                if line != "[Asserts]" {
                    return Err(syntax());
                }
                phase = 3;
            }
            _ => {
                let assertion = assertion(line, profile).ok_or_else(syntax)?;
                let slot = plan
                    .assertions
                    .get_mut(plan.count)
                    .ok_or_else(|| error(line_number, HurlParseErrorKind::AssertionBound))?;
                *slot = Some(assertion);
                plan.count = plan.count.checked_add(1).ok_or_else(syntax)?;
            }
        }
    }
    if phase < 2 {
        return Err(error(1, HurlParseErrorKind::Syntax));
    }
    Ok(plan)
}
fn word(value: &str) -> Option<(&str, &str)> {
    let end = value.find([' ', '\t'])?;
    let token = value.get(..end)?;
    let rest = value.get(end..)?.trim_start_matches([' ', '\t']);
    if token.is_empty() || rest.is_empty() {
        return None;
    }
    Some((token, rest))
}
fn optional_not(value: &str) -> (bool, &str) {
    match word(value) {
        Some(("not", rest)) => (true, rest),
        _ => (false, value),
    }
}
fn numeric(value: &str) -> Option<(bool, Comparison, i64)> {
    let (negated, value) = optional_not(value);
    let (operator, operand) = word(value)?;
    Some((
        negated,
        Comparison::parse(operator)?,
        integer::parse(operand)?,
    ))
}
fn quoted(value: &str) -> Option<(&str, &str)> {
    let value = value.strip_prefix('"')?;
    let end = value.find('"')?;
    let literal = value.get(..end)?;
    if !literal
        .bytes()
        .all(|b| (b' '..=b'~').contains(&b) && !matches!(b, b'\\' | b'{' | b'}'))
    {
        return None;
    }
    let tail = value.get(end..)?.strip_prefix('"')?;
    Some((literal, tail))
}
fn literal(value: &str) -> Option<&str> {
    let (literal, tail) = quoted(value)?;
    if tail.is_empty() && literal.len() <= MAX_HEADER_VALUE_BYTES {
        Some(literal)
    } else {
        None
    }
}
fn assertion(line: &str, profile: Profile) -> Option<Assertion<'_>> {
    let (query, value) = word(line)?;
    if query == "status" {
        let (negated, comparison, expected) = numeric(value)?;
        return Some(Assertion {
            query: Query::Status {
                comparison,
                expected,
            },
            negated,
        });
    }
    if query != "header" || profile == Profile::Status {
        return None;
    }
    let (name, tail) = quoted(value)?;
    if name.len() > MAX_HEADER_NAME_BYTES
        || !header_name_valid(name)
        || !tail.starts_with([' ', '\t'])
    {
        return None;
    }
    let predicate = tail.trim_start_matches([' ', '\t']);
    if let Some(("count", rest)) = word(predicate) {
        let (negated, comparison, expected) = numeric(rest)?;
        return Some(Assertion {
            query: Query::Header(HeaderAssertion {
                name,
                predicate: HeaderPredicate::Count {
                    comparison,
                    expected,
                },
            }),
            negated,
        });
    }
    let (negated, predicate) = optional_not(predicate);
    let predicate = match predicate {
        "exists" => HeaderPredicate::Exists,
        "isEmpty" => HeaderPredicate::IsEmpty,
        "isString" if profile == Profile::Predicates => HeaderPredicate::IsString,
        "isList" if profile == Profile::Predicates => HeaderPredicate::IsList,
        "isCollection" if profile == Profile::Predicates => HeaderPredicate::IsCollection,
        value => {
            let (operator, operand) = word(value)?;
            let expected = literal(operand)?;
            match operator {
                "==" => HeaderPredicate::Equal(expected),
                "!=" => HeaderPredicate::NotEqual(expected),
                _ if profile == Profile::Predicates => {
                    let kind = match operator {
                        "startsWith" => HeaderTextPredicate::StartsWith,
                        "endsWith" => HeaderTextPredicate::EndsWith,
                        "contains" => HeaderTextPredicate::Contains,
                        "includes" => HeaderTextPredicate::Includes,
                        _ => return None,
                    };
                    HeaderPredicate::Text { kind, expected }
                }
                _ => return None,
            }
        }
    };
    Some(Assertion {
        query: Query::Header(HeaderAssertion { name, predicate }),
        negated,
    })
}
