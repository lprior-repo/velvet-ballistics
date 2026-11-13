use super::*;
fn parse(args: &[&str]) -> Result<(String, String)> {
    match parse_cli(args.iter().map(|v| v.to_string()))? {
        CliAction::Run(budget, evidence) => Ok((budget, evidence)),
        CliAction::Help => Err(PolicyError::Message(String::from("expected run action"))),
    }
}
#[test]
fn defaults() -> Result<()> {
    let (b, e) = parse(&[])?;
    assert_eq!(b, "contracts/perf-budget.yaml");
    assert_eq!(e, "evidence/benchmark-evidence.jsonl");
    Ok(())
}
#[test]
fn both_equals() -> Result<()> {
    let (b, e) = parse(&["--budget=b.yaml", "--evidence=e.jsonl"])?;
    assert_eq!(b, "b.yaml");
    assert_eq!(e, "e.jsonl");
    Ok(())
}
#[test]
fn equals_with_spaces() -> Result<()> {
    let (b, _) = parse(&["--budget= b.yaml "])?;
    assert_eq!(b, " b.yaml ");
    Ok(())
}
#[test]
fn equals_extra_equals() -> Result<()> {
    let (b, _) = parse(&["--budget=a=b=c"])?;
    assert_eq!(b, "a=b=c");
    Ok(())
}
#[test]
fn separate_positive() -> Result<()> {
    let (b, e) = parse(&["--budget", "b.yaml", "--evidence", "e.jsonl"])?;
    assert_eq!(b, "b.yaml");
    assert_eq!(e, "e.jsonl");
    Ok(())
}
#[test]
fn mixed_forms() -> Result<()> {
    let (b, e) = parse(&["--budget=b.yaml", "--evidence", "e.jsonl"])?;
    assert_eq!(b, "b.yaml");
    assert_eq!(e, "e.jsonl");
    Ok(())
}
#[test]
fn duplicate_budget_separate() -> Result<()> {
    fail(
        &["--budget", "a.yaml", "--budget", "b.yaml"],
        "error: the argument '--budget <BUDGET>' cannot be used multiple times\n\nUsage: xtask benchmark-regression-policy [OPTIONS]\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
#[test]
fn duplicate_budget_equals() -> Result<()> {
    fail(
        &["--budget=a.yaml", "--budget=b.yaml"],
        "error: the argument '--budget <BUDGET>' cannot be used multiple times\n\nUsage: xtask benchmark-regression-policy [OPTIONS]\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
#[test]
fn duplicate_budget_mixed() -> Result<()> {
    fail(
        &["--budget=a.yaml", "--budget", "b.yaml"],
        "error: the argument '--budget <BUDGET>' cannot be used multiple times\n\nUsage: xtask benchmark-regression-policy [OPTIONS]\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
#[test]
fn duplicate_evidence_separate() -> Result<()> {
    fail(
        &["--evidence", "a.jsonl", "--evidence", "b.jsonl"],
        "error: the argument '--evidence <EVIDENCE>' cannot be used multiple times\n\nUsage: xtask benchmark-regression-policy [OPTIONS]\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
#[test]
fn duplicate_evidence_equals() -> Result<()> {
    fail(
        &["--evidence=a.jsonl", "--evidence=b.jsonl"],
        "error: the argument '--evidence <EVIDENCE>' cannot be used multiple times\n\nUsage: xtask benchmark-regression-policy [OPTIONS]\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
#[test]
fn duplicate_evidence_mixed() -> Result<()> {
    fail(
        &["--evidence=a.jsonl", "--evidence", "b.jsonl"],
        "error: the argument '--evidence <EVIDENCE>' cannot be used multiple times\n\nUsage: xtask benchmark-regression-policy [OPTIONS]\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
#[test]
fn bare_budget_missing() -> Result<()> {
    fail(
        &["--budget"],
        "error: a value is required for '--budget <BUDGET>' but none was supplied\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
#[test]
fn bare_evidence_missing() -> Result<()> {
    fail(
        &["--evidence"],
        "error: a value is required for '--evidence <EVIDENCE>' but none was supplied\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
#[test]
fn unknown_separate() -> Result<()> {
    fail(
        &["--unknown"],
        "error: unexpected argument '--unknown' found\n\nUsage: xtask benchmark-regression-policy [OPTIONS]\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
#[test]
fn unknown_equals_reports_flag_name() -> Result<()> {
    fail(
        &["--unknown=value"],
        "error: unexpected argument '--unknown' found\n\nUsage: xtask benchmark-regression-policy [OPTIONS]\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
#[test]
fn empty_inline_budget() -> Result<()> {
    let (b, _) = parse(&["--budget="])?;
    assert_eq!(b, "");
    Ok(())
}
#[test]
fn empty_separate_budget() -> Result<()> {
    let (b, _) = parse(&["--budget", ""])?;
    assert_eq!(b, "");
    Ok(())
}
#[test]
fn inline_value_starting_with_dashes() -> Result<()> {
    let (b, _) = parse(&["--budget=--weird"])?;
    assert_eq!(b, "--weird");
    Ok(())
}
#[test]
fn duplicate_missing_value_precedes_conflict() -> Result<()> {
    fail(
        &["--budget=a.yaml", "--budget"],
        "error: a value is required for '--budget <BUDGET>' but none was supplied\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
#[test]
fn duplicate_evidence_missing_value_precedes_conflict() -> Result<()> {
    fail(
        &["--evidence=a.jsonl", "--evidence"],
        "error: a value is required for '--evidence <EVIDENCE>' but none was supplied\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
#[test]
fn separate_then_equals_duplicate() -> Result<()> {
    fail(
        &["--budget", "a.yaml", "--budget=b.yaml"],
        "error: the argument '--budget <BUDGET>' cannot be used multiple times\n\nUsage: xtask benchmark-regression-policy [OPTIONS]\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
#[test]
fn equals_then_separate_duplicate() -> Result<()> {
    fail(
        &["--budget=a.yaml", "--budget", "b.yaml"],
        "error: the argument '--budget <BUDGET>' cannot be used multiple times\n\nUsage: xtask benchmark-regression-policy [OPTIONS]\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
#[test]
fn evidence_separate_then_equals_duplicate() -> Result<()> {
    fail(
        &["--evidence", "a.jsonl", "--evidence=b.jsonl"],
        "error: the argument '--evidence <EVIDENCE>' cannot be used multiple times\n\nUsage: xtask benchmark-regression-policy [OPTIONS]\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
#[test]
fn evidence_equals_then_separate_duplicate() -> Result<()> {
    fail(
        &["--evidence=a.jsonl", "--evidence", "b.jsonl"],
        "error: the argument '--evidence <EVIDENCE>' cannot be used multiple times\n\nUsage: xtask benchmark-regression-policy [OPTIONS]\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
fn fail(args: &[&str], expected: &str) {
    let actual = parse(args).map_or_else(
        |error| error.to_string(),
        |_| String::from("unexpected success"),
    );
    assert_eq!(actual, expected);
}
