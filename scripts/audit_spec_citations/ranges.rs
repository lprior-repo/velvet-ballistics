use std::cmp::Ordering;

#[derive(Clone, Debug)]
pub struct CitationNumber {
    negative: bool,
    magnitude: String,
    original: String,
}

impl CitationNumber {
    fn parse(value: &str, original_spec: &str) -> Result<Self, String> {
        let (negative, digits) = match value.strip_prefix('-') {
            Some(digits) => (true, digits),
            None => (false, value),
        };
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(format!("invalid range number '{value}' in {original_spec}"));
        }
        let magnitude = digits.trim_start_matches('0');
        let magnitude = if magnitude.is_empty() { "0" } else { magnitude };
        Ok(Self {
            negative: negative && magnitude != "0",
            magnitude: magnitude.to_string(),
            original: value.to_string(),
        })
    }

    fn is_less_than_one(&self) -> bool {
        self.negative || self.magnitude == "0"
    }

    fn is_greater_than(&self, limit: usize) -> bool {
        !self.negative
            && compare_magnitudes(&self.magnitude, &limit.to_string()) == Ordering::Greater
    }

    fn to_u64(&self) -> Result<u64, String> {
        if self.negative {
            return Err("validated range number is negative".to_string());
        }
        self.magnitude
            .parse::<u64>()
            .map_err(|_| "validated range number cannot convert to u64".to_string())
    }

    pub fn original(&self) -> &str {
        &self.original
    }
}

impl PartialEq for CitationNumber {
    fn eq(&self, other: &Self) -> bool {
        self.negative == other.negative && self.magnitude == other.magnitude
    }
}

impl Eq for CitationNumber {}

impl PartialOrd for CitationNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CitationNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.negative, other.negative) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (false, false) => compare_magnitudes(&self.magnitude, &other.magnitude),
            (true, true) => compare_magnitudes(&other.magnitude, &self.magnitude),
        }
    }
}

fn compare_magnitudes(left: &str, right: &str) -> Ordering {
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

pub fn parse_ranges(spec: &str) -> Result<Vec<(CitationNumber, CitationNumber)>, String> {
    let normalized: String = spec.chars().filter(|character| *character != ' ').collect();
    let mut ranges = Vec::new();
    for part in normalized.split(',').filter(|part| !part.is_empty()) {
        if matches!(part.split_once('-'), Some((start, end)) if start.is_empty() || end.is_empty())
        {
            continue;
        }
        ranges.push(parse_range(part, spec)?);
    }
    Ok(ranges)
}

fn parse_range(
    part: &str,
    original_spec: &str,
) -> Result<(CitationNumber, CitationNumber), String> {
    match part.split_once('-') {
        Some((start, end)) => Ok((
            CitationNumber::parse(start, original_spec)?,
            CitationNumber::parse(end, original_spec)?,
        )),
        None => {
            let number = CitationNumber::parse(part, original_spec)?;
            Ok((number.clone(), number))
        }
    }
}

pub fn validate_range(
    start: &CitationNumber,
    end: &CitationNumber,
    line_count: usize,
) -> Option<String> {
    (start.is_less_than_one() || end.is_greater_than(line_count) || start > end)
        .then(|| format!("out of range (file has {line_count} lines)"))
}

pub fn bounded_range(start: &CitationNumber, end: &CitationNumber) -> Result<(u64, u64), String> {
    Ok((start.to_u64()?, end.to_u64()?))
}

pub fn range_failure(
    line_number: usize,
    path: &str,
    start: &CitationNumber,
    end: &CitationNumber,
    reason: &str,
) -> String {
    format!(
        "line {line_number}: {path}:{}-{} {reason}",
        start.original(),
        end.original()
    )
}

#[cfg(test)]
mod tests {
    use super::{bounded_range, parse_ranges, validate_range};

    #[test]
    fn parses_space_elision_empty_parts_and_negative_endpoint() -> Result<(), String> {
        assert_eq!(parse_ranges(" , 1, 2 - 4, , 7 ")?.len(), 3);
        assert!(parse_ranges("1-, -2")?.is_empty());
        let negative = parse_ranges("1--2")?;
        let pair = negative.first().ok_or("missing negative range")?;
        assert_eq!(pair.1.original(), "-2");
        assert_eq!(
            validate_range(&pair.0, &pair.1, 3),
            Some("out of range (file has 3 lines)".to_string())
        );
        Ok(())
    }

    #[test]
    fn canonical_identity_normalizes_leading_zeroes_and_negative_zero() -> Result<(), String> {
        let numbers = parse_ranges("1,01,0001,1--0,1-0")?;
        assert_eq!(
            numbers
                .iter()
                .cloned()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            2
        );
        assert_eq!(numbers.get(3).map(|range| range.1.original()), Some("-0"));
        Ok(())
    }

    #[test]
    fn arbitrary_magnitude_rejects_by_bounds_before_u64_conversion() -> Result<(), String> {
        let ranges = parse_ranges("184467440737095516160000000000000000000")?;
        let (start, end) = ranges.first().ok_or("missing test range")?;
        assert_eq!(
            validate_range(start, end, 3),
            Some("out of range (file has 3 lines)".to_string())
        );
        Ok(())
    }

    #[test]
    fn bounded_conversion_succeeds_after_validation() -> Result<(), String> {
        let ranges = parse_ranges("1-3")?;
        let (start, end) = ranges.first().ok_or("missing test range")?;
        assert_eq!(validate_range(start, end, 3), None);
        assert_eq!(bounded_range(start, end)?, (1, 3));
        Ok(())
    }

    #[test]
    fn rejects_malformed_decimal_integer() {
        assert!(parse_ranges("1,,x").is_err());
        assert!(parse_ranges("1---2").is_err());
    }
}
