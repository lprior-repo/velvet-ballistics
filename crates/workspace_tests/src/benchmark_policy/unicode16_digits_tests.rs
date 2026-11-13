use super::{is_decimal, is_digit};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Property {
    Decimal,
    Digit,
    Numeric,
    Unlisted,
}

#[derive(Debug)]
struct Record {
    start: u32,
    end: u32,
    property: Property,
}

fn parse_property(text: &str) -> Result<Property, String> {
    match text.trim() {
        "Decimal" => Ok(Property::Decimal),
        "Digit" => Ok(Property::Digit),
        "Numeric" => Ok(Property::Numeric),
        other => Err(format!("unknown Numeric_Type: {other}")),
    }
}

fn parse_hex(text: &str) -> Result<u32, String> {
    u32::from_str_radix(text.trim(), 16).map_err(|error| format!("invalid code point: {error}"))
}

fn parse_range(text: &str) -> Result<(u32, u32), String> {
    let (start, end) = match text.split_once("..") {
        Some((start, end)) => (parse_hex(start)?, parse_hex(end)?),
        None => {
            let point = parse_hex(text)?;
            (point, point)
        }
    };
    if start > end
        || end > u32::from(char::MAX)
        || (start..=end).any(|cp| char::from_u32(cp).is_none())
    {
        return Err(String::from("invalid scalar range"));
    }
    Ok((start, end))
}

fn parse_line(line: &str) -> Result<Option<Record>, String> {
    let body = match line.split_once('#') {
        Some((body, _)) => body.trim(),
        None => line.trim(),
    };
    if body.is_empty() {
        return Ok(None);
    }
    let (range, property) = body.split_once(';').ok_or("missing property separator")?;
    let (start, end) = parse_range(range.trim())?;
    Ok(Some(Record {
        start,
        end,
        property: parse_property(property)?,
    }))
}

fn parse_fixture() -> Result<Vec<Record>, String> {
    let mut records = include_str!("fixtures/DerivedNumericType-16.0.0.txt")
        .lines()
        .map(parse_line)
        .filter_map(Result::transpose)
        .collect::<Result<Vec<_>, _>>()?;
    records.sort_unstable_by_key(|record| record.start);
    records
        .windows(2)
        .try_for_each(|pair| match (pair.first(), pair.get(1)) {
            (Some(first), Some(second)) if first.end < second.start => Ok(()),
            _ => Err(String::from("overlapping fixture records")),
        })?;
    Ok(records)
}

fn scalar_count(records: &[Record], property: Property) -> usize {
    records
        .iter()
        .filter(|record| record.property == property)
        .flat_map(|record| record.start..=record.end)
        .count()
}

fn record_count(records: &[Record], property: Property) -> usize {
    records
        .iter()
        .filter(|record| record.property == property)
        .count()
}

#[test]
fn test_record_counts() -> Result<(), String> {
    let records = parse_fixture()?;
    assert_eq!(record_count(&records, Property::Decimal), 71);
    assert_eq!(record_count(&records, Property::Digit), 20);
    assert_eq!(record_count(&records, Property::Numeric), 165);
    Ok(())
}

#[test]
fn test_scalar_totals() -> Result<(), String> {
    let records = parse_fixture()?;
    assert_eq!(scalar_count(&records, Property::Decimal), 760);
    assert_eq!(scalar_count(&records, Property::Digit), 128);
    assert_eq!(scalar_count(&records, Property::Numeric), 1114);
    assert_eq!(
        records
            .iter()
            .filter(|r| matches!(r.property, Property::Decimal | Property::Digit))
            .flat_map(|r| r.start..=r.end)
            .count(),
        888
    );
    Ok(())
}

#[test]
fn test_all_codepoints_against_fixture() -> Result<(), String> {
    let records = parse_fixture()?;
    let expected: BTreeMap<_, _> = records
        .iter()
        .flat_map(|record| (record.start..=record.end).map(move |cp| (cp, record.property)))
        .collect();
    let scalars = || (0..=u32::from(char::MAX)).filter_map(char::from_u32);
    assert_eq!(scalars().count(), 1_112_064);
    scalars().for_each(|ch| assert_scalar(ch, &expected));
    Ok(())
}

fn assert_scalar(ch: char, expected: &BTreeMap<u32, Property>) {
    let property = match expected.get(&u32::from(ch)) {
        Some(property) => *property,
        None => Property::Unlisted,
    };
    assert_eq!(
        is_digit(ch),
        matches!(property, Property::Decimal | Property::Digit),
        "{ch:?}"
    );
    assert_eq!(is_decimal(ch), property == Property::Decimal, "{ch:?}");
}

#[test]
fn test_unassigned_and_scalar_boundaries() {
    ['\u{0378}', '\0', '\u{D7FF}', '\u{E000}', char::MAX]
        .iter()
        .for_each(|ch| {
            assert_eq!(is_digit(*ch), false);
            assert_eq!(is_decimal(*ch), false);
        });
}

#[test]
fn fixture_parser_rejects_unknown_or_missing_properties() {
    assert_eq!(
        parse_property("Decimal extra"),
        Err("unknown Numeric_Type: Decimal extra".into())
    );
    assert_eq!(
        parse_line("0030").err(),
        Some("missing property separator".into())
    );
    assert_eq!(
        parse_line("0030; Unknown").err(),
        Some("unknown Numeric_Type: Unknown".into())
    );
}

#[test]
fn fixture_parser_rejects_invalid_hex_and_scalar_ranges() {
    assert_eq!(
        parse_range("GG"),
        Err("invalid code point: invalid digit found in string".into())
    );
    ["0039..0030", "D800", "D7FF..E000", "110000"]
        .iter()
        .for_each(|text| {
            assert_eq!(parse_range(text), Err("invalid scalar range".into()));
        });
}

#[test]
fn test_ascii_digit() {
    assert_eq!(is_digit('0'), true);
    assert_eq!(is_digit('9'), true);
    assert_eq!(is_decimal('0'), true);
    assert_eq!(is_decimal('9'), true);
}

#[test]
fn test_arabic_digit() {
    assert_eq!(is_digit('\u{0662}'), true);
    assert_eq!(is_decimal('\u{0662}'), true);
}

#[test]
fn test_devanagari_digit() {
    assert_eq!(is_digit('\u{0968}'), true);
    assert_eq!(is_decimal('\u{0968}'), true);
}

#[test]
fn test_fullwidth_digit() {
    assert_eq!(is_digit('\u{FF12}'), true);
    assert_eq!(is_decimal('\u{FF12}'), true);
}

#[test]
fn test_garay_digit() {
    assert_eq!(is_digit('\u{10D42}'), true);
    assert_eq!(is_decimal('\u{10D42}'), true);
}

#[test]
fn test_superscript_two() {
    assert_eq!(is_digit('\u{00B2}'), true);
    assert_eq!(is_decimal('\u{00B2}'), false);
}

#[test]
fn test_circled_one() {
    assert_eq!(is_digit('\u{2460}'), true);
    assert_eq!(is_decimal('\u{2460}'), false);
}

#[test]
fn test_roman_twelve() {
    assert_eq!(is_digit('\u{216B}'), false);
    assert_eq!(is_decimal('\u{216B}'), false);
}

#[test]
fn test_fraction_one_third() {
    assert_eq!(is_digit('\u{2153}'), false);
    assert_eq!(is_decimal('\u{2153}'), false);
}

#[test]
fn test_alphabetic() {
    assert_eq!(is_digit('A'), false);
    assert_eq!(is_decimal('A'), false);
}

#[test]
fn test_minus() {
    assert_eq!(is_digit('-'), false);
    assert_eq!(is_decimal('-'), false);
}
