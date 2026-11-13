use crate::benchmark_policy::error::{Context, Result, bail};
use crate::benchmark_policy::unicode16_digits;

#[derive(Clone, Copy)]
enum ListBlock {
    Outside,
    Target,
}

pub(super) fn normalize_lists(text: &str) -> Result<String> {
    text.split_inclusive('\n')
        .enumerate()
        .try_fold(
            (String::new(), ListBlock::Outside),
            |(mut output, state), (index, line)| {
                let number = index
                    .checked_add(1)
                    .context("budget line number overflow")?;
                let (normalized, next) = normalize_line(line, number, state)?;
                output.push_str(&normalized);
                Ok((output, next))
            },
        )
        .map(|(output, _)| output)
}

fn normalize_line(line: &str, number: usize, state: ListBlock) -> Result<(String, ListBlock)> {
    if line.contains('\t') {
        bail!("performance budget line {number}: tab indentation is not allowed");
    }
    let (body, ending) = line_ending(line);
    if let Some((key, value)) = target_header(body) {
        return normalize_header(key, value, ending, number);
    }
    if body.trim().is_empty() || body.trim_start().starts_with('#') {
        return Ok((line.to_owned(), state));
    }
    if let (ListBlock::Target, Some(item)) = (state, body.strip_prefix("  - ")) {
        let value = normalize_scalar(item, number)?;
        return Ok((format!("  - {value}{ending}"), ListBlock::Target));
    }
    Ok((line.to_owned(), ListBlock::Outside))
}

fn line_ending(line: &str) -> (&str, &str) {
    if let Some(body) = line.strip_suffix("\r\n") {
        return (body, "\r\n");
    }
    match line.strip_suffix('\n') {
        Some(body) => (body, "\n"),
        None => (line, ""),
    }
}

fn target_header(line: &str) -> Option<(&str, &str)> {
    if line.starts_with(char::is_whitespace) {
        return None;
    }
    line.split_once(':')
        .filter(|(key, _)| matches!(key.trim(), "deferred_modes" | "required_evidence_fields"))
}

fn normalize_header(
    key: &str,
    raw: &str,
    ending: &str,
    number: usize,
) -> Result<(String, ListBlock)> {
    if raw.trim().is_empty() {
        return Ok((format!("{key}:{raw}{ending}"), ListBlock::Target));
    }
    let value = normalize_scalar(raw, number)?;
    Ok((format!("{key}: {value}{ending}"), ListBlock::Outside))
}

fn normalize_scalar(raw: &str, number: usize) -> Result<String> {
    let text =
        raw.trim_matches(|ch: char| ch.is_whitespace() || matches!(ch, '\u{001C}'..='\u{001F}'));
    if text.is_empty() {
        bail!("performance budget line {number}: empty scalar");
    }
    let value = if let Some(body) = unquoted_body(text, '"') {
        decode_double_quoted(body, number)?
    } else if let Some(body) = unquoted_body(text, '\'') {
        body.replace("''", "'")
    } else if matches!(text, "true" | "True" | "TRUE" | "false" | "False" | "FALSE") {
        return Ok(text.to_owned());
    } else if unicode_integer(text, number)? {
        return Ok(String::from("0"));
    } else {
        text.to_owned()
    };
    serde_json::to_string(&value).context("encode performance budget list scalar")
}

fn unquoted_body(text: &str, delimiter: char) -> Option<&str> {
    text.strip_prefix(delimiter)?.strip_suffix(delimiter)
}

fn unicode_integer(text: &str, number: usize) -> Result<bool> {
    let digits = text.trim_start_matches('-');
    if digits.is_empty() || !digits.chars().all(unicode16_digits::is_digit) {
        return Ok(false);
    }
    if text.starts_with("--") {
        bail!("performance budget line {number}: integer out of range: {text}");
    }
    if !digits.chars().all(unicode16_digits::is_decimal) {
        bail!("performance budget line {number}: integer out of range: {text}");
    }
    Ok(true)
}

fn decode_double_quoted(body: &str, number: usize) -> Result<String> {
    let (output, escaped) = body.chars().try_fold(
        (String::new(), false),
        |(mut output, escaped), ch| -> Result<_> {
            if escaped {
                output.push(decode_escape(ch, number)?);
            } else if ch == '\\' {
                return Ok((output, true));
            } else {
                output.push(ch);
            }
            Ok((output, false))
        },
    )?;
    if escaped {
        bail!("performance budget line {number}: trailing backslash in quoted scalar");
    }
    Ok(output)
}

fn decode_escape(ch: char, number: usize) -> Result<char> {
    match ch {
        'n' => Ok('\n'),
        't' => Ok('\t'),
        '"' => Ok('"'),
        '\\' => Ok('\\'),
        _ => bail!("performance budget line {number}: unsupported escape \\{ch}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar(text: &str) -> std::result::Result<String, String> {
        normalize_scalar(text, 12).map_err(|e| e.to_string())
    }

    #[test]
    fn ascii_positive_integer() {
        assert_eq!(scalar("123"), Ok("0".to_owned()));
    }

    #[test]
    fn arabic_indic_positive_integer() {
        assert_eq!(scalar("٢"), Ok("0".to_owned()));
    }

    #[test]
    fn devanagari_positive_integer() {
        assert_eq!(scalar("२"), Ok("0".to_owned()));
    }

    #[test]
    fn fullwidth_positive_integer() {
        assert_eq!(scalar("２"), Ok("0".to_owned()));
    }

    #[test]
    fn negative_arabic_indic_integer() {
        assert_eq!(scalar("-٢"), Ok("0".to_owned()));
    }

    #[test]
    fn mixed_nd_digit_integer() {
        assert_eq!(scalar("1٢3"), Ok("0".to_owned()));
    }

    #[test]
    fn superscript_digit_fails() {
        assert_eq!(
            scalar("²"),
            Err("performance budget line 12: integer out of range: ²".to_owned())
        );
    }

    #[test]
    fn circled_digit_fails() {
        assert_eq!(
            scalar("①"),
            Err("performance budget line 12: integer out of range: ①".to_owned())
        );
    }

    #[test]
    fn repeated_minus_fails() {
        assert_eq!(
            scalar("--٢"),
            Err("performance budget line 12: integer out of range: --٢".to_owned())
        );
    }

    #[test]
    fn vulgar_fraction_is_string() {
        assert_eq!(scalar("⅓"), Ok("\"⅓\"".to_owned()));
    }

    #[test]
    fn roman_numeral_is_string() {
        assert_eq!(scalar("Ⅻ"), Ok("\"Ⅻ\"".to_owned()));
    }

    #[test]
    fn mixed_digit_letter_is_string() {
        assert_eq!(scalar("2a"), Ok("\"2a\"".to_owned()));
    }

    #[test]
    fn plus_sign_is_string() {
        assert_eq!(scalar("+2"), Ok("\"+2\"".to_owned()));
    }

    #[test]
    fn quoted_digit_is_string() {
        assert_eq!(scalar("\"٢\""), Ok("\"٢\"".to_owned()));
    }

    #[test]
    fn boolean_true() {
        assert_eq!(scalar("true"), Ok("true".to_owned()));
    }

    #[test]
    fn boolean_false() {
        assert_eq!(scalar("false"), Ok("false".to_owned()));
    }
}
