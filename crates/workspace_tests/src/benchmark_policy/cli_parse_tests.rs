use super::*;

const MISSING_BUDGET: &str = "error: a value is required for '--budget <BUDGET>' but none was supplied\n\nFor more information, try '--help'.\n";
const MISSING_EVIDENCE: &str = "error: a value is required for '--evidence <EVIDENCE>' but none was supplied\n\nFor more information, try '--help'.\n";

fn parse_paths(args: &[&str]) -> Result<(String, String, String)> {
    match parse_cli(args.iter().map(|v| v.to_string()))? {
        CliAction::Run(root, budget, evidence) => Ok((root, budget, evidence)),
        CliAction::Help => Err(PolicyError::Message(String::from("expected run action"))),
    }
}
fn parse(args: &[&str]) -> Result<(String, String)> {
    parse_paths(args).map(|(_, budget, evidence)| (budget, evidence))
}
#[test]
fn defaults() -> Result<()> {
    let (root, b, e) = parse_paths(&[])?;
    assert_eq!(root, ".");
    assert_eq!(b, "contracts/perf-budget.yaml");
    assert_eq!(e, "evidence/benchmark-evidence.jsonl");
    Ok(())
}
#[test]
fn both_equals() -> Result<()> {
    let (b, e) = parse(&["--budget=b.yaml", "--evidence=e.jsonl"])?;
    assert_eq!((b.as_str(), e.as_str()), ("b.yaml", "e.jsonl"));
    Ok(())
}
#[test]
fn equals_with_spaces() -> Result<()> {
    assert_eq!(parse(&["--budget= b.yaml "])?.0, " b.yaml ");
    Ok(())
}
#[test]
fn equals_extra_equals() -> Result<()> {
    assert_eq!(parse(&["--budget=a=b=c"])?.0, "a=b=c");
    Ok(())
}
#[test]
fn separate_positive() -> Result<()> {
    let (b, e) = parse(&["--budget", "b.yaml", "--evidence", "e.jsonl"])?;
    assert_eq!((b.as_str(), e.as_str()), ("b.yaml", "e.jsonl"));
    Ok(())
}
#[test]
fn mixed_forms() -> Result<()> {
    let (b, e) = parse(&["--budget=b.yaml", "--evidence", "e.jsonl"])?;
    assert_eq!((b.as_str(), e.as_str()), ("b.yaml", "e.jsonl"));
    Ok(())
}

// The original add_argument("--budget"/"--evidence") declarations use argparse's
// default store action, so later occurrences replace earlier path values.
fn last_budget(args: &[&str]) -> Result<()> {
    let (b, e) = parse(args)?;
    assert_eq!(
        (b.as_str(), e.as_str()),
        ("b.yaml", "evidence/benchmark-evidence.jsonl")
    );
    Ok(())
}
fn last_evidence(args: &[&str]) -> Result<()> {
    let (b, e) = parse(args)?;
    assert_eq!(
        (b.as_str(), e.as_str()),
        ("contracts/perf-budget.yaml", "b.jsonl")
    );
    Ok(())
}
#[test]
fn duplicate_budget_separate() -> Result<()> {
    last_budget(&["--budget", "a.yaml", "--budget", "b.yaml"])
}
#[test]
fn duplicate_budget_equals() -> Result<()> {
    last_budget(&["--budget=a.yaml", "--budget=b.yaml"])
}
#[test]
fn duplicate_budget_mixed() -> Result<()> {
    last_budget(&["--budget=a.yaml", "--budget", "b.yaml"])
}
#[test]
fn duplicate_evidence_separate() -> Result<()> {
    last_evidence(&["--evidence", "a.jsonl", "--evidence", "b.jsonl"])
}
#[test]
fn duplicate_evidence_equals() -> Result<()> {
    last_evidence(&["--evidence=a.jsonl", "--evidence=b.jsonl"])
}
#[test]
fn duplicate_evidence_mixed() -> Result<()> {
    last_evidence(&["--evidence=a.jsonl", "--evidence", "b.jsonl"])
}
#[test]
fn bare_budget_missing() -> Result<()> {
    fail(&["--budget"], MISSING_BUDGET);
    Ok(())
}
#[test]
fn bare_evidence_missing() -> Result<()> {
    fail(&["--evidence"], MISSING_EVIDENCE);
    Ok(())
}
#[test]
fn unknown_separate() -> Result<()> {
    fail(
        &["--unknown"],
        "error: unexpected argument '--unknown' found\n\nUsage: benchmark-policy [OPTIONS] [ROOT]\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
#[test]
fn unknown_equals_reports_flag_name() -> Result<()> {
    fail(
        &["--unknown=value"],
        "error: unexpected argument '--unknown' found\n\nUsage: benchmark-policy [OPTIONS] [ROOT]\n\nFor more information, try '--help'.\n",
    );
    Ok(())
}
#[test]
fn empty_inline_budget() -> Result<()> {
    assert_eq!(parse(&["--budget="])?.0, "");
    Ok(())
}
#[test]
fn empty_separate_budget() -> Result<()> {
    assert_eq!(parse(&["--budget", ""])?.0, "");
    Ok(())
}
#[test]
fn inline_value_starting_with_dashes() -> Result<()> {
    assert_eq!(parse(&["--budget=--weird"])?.0, "--weird");
    Ok(())
}
#[test]
fn duplicate_missing_value_precedes_conflict() -> Result<()> {
    fail(&["--budget=a.yaml", "--budget"], MISSING_BUDGET);
    Ok(())
}
#[test]
fn duplicate_evidence_missing_value_precedes_conflict() -> Result<()> {
    fail(&["--evidence=a.jsonl", "--evidence"], MISSING_EVIDENCE);
    Ok(())
}
#[test]
fn separate_then_equals_duplicate() -> Result<()> {
    last_budget(&["--budget", "a.yaml", "--budget=b.yaml"])
}
#[test]
fn equals_then_separate_duplicate() -> Result<()> {
    last_budget(&["--budget=a.yaml", "--budget", "b.yaml"])
}
#[test]
fn evidence_separate_then_equals_duplicate() -> Result<()> {
    last_evidence(&["--evidence", "a.jsonl", "--evidence=b.jsonl"])
}
#[test]
fn evidence_equals_then_separate_duplicate() -> Result<()> {
    last_evidence(&["--evidence=a.jsonl", "--evidence", "b.jsonl"])
}
fn fail(args: &[&str], expected: &str) {
    match parse_paths(args) {
        Err(PolicyError::Usage(actual)) => assert_eq!(actual, expected),
        actual => assert_eq!(format!("{actual:?}"), "expected usage error"),
    }
}
#[test]
fn root_before_between_and_after_options() -> Result<()> {
    for args in [
        vec!["repo path", "--budget=b.yaml", "--evidence", "e.jsonl"],
        vec!["--budget=b.yaml", "repo path", "--evidence", "e.jsonl"],
        vec!["--budget=b.yaml", "--evidence", "e.jsonl", "repo path"],
    ] {
        let (root, b, e) = parse_paths(&args)?;
        assert_eq!(
            (root.as_str(), b.as_str(), e.as_str()),
            ("repo path", "b.yaml", "e.jsonl")
        );
    }
    Ok(())
}
#[test]
fn selected_root_retains_relative_absolute_and_equals_paths() -> Result<()> {
    for root in ["relative/repo", "/absolute/repo", "repo=name", "-", ""] {
        assert_eq!(parse_paths(&[root])?.0, root);
    }
    Ok(())
}
#[test]
fn repeated_paths_keep_last_values_with_root() -> Result<()> {
    let (root, b, e) = parse_paths(&[
        "--budget=old",
        "repo",
        "--evidence=old",
        "--budget",
        "new",
        "--evidence=new",
    ])?;
    assert_eq!(
        (root.as_str(), b.as_str(), e.as_str()),
        ("repo", "new", "new")
    );
    Ok(())
}
#[test]
fn terminator_keeps_option_shaped_root_literal() -> Result<()> {
    for root in ["--help", "--budget=literal", "--", "-repo"] {
        assert_eq!(parse_paths(&["--", root])?.0, root);
    }
    assert_eq!(parse_paths(&["--"])?.0, ".");
    Ok(())
}
#[test]
fn terminator_after_options_preserves_selected_values() -> Result<()> {
    let (root, b, e) = parse_paths(&["--budget=b.yaml", "--evidence=e.jsonl", "--", "-repo"])?;
    assert_eq!(
        (root.as_str(), b.as_str(), e.as_str()),
        ("-repo", "b.yaml", "e.jsonl")
    );
    Ok(())
}
#[test]
fn single_dash_is_a_root_and_path_option_value() -> Result<()> {
    let (root, b, e) = parse_paths(&["-", "--budget", "-", "--evidence=-"])?;
    assert_eq!((root.as_str(), b.as_str(), e.as_str()), ("-", "-", "-"));
    Ok(())
}
#[test]
fn extra_root_is_usage_error() {
    fail(
        &["one", "two"],
        "error: unexpected argument 'two' found\n\nUsage: benchmark-policy [OPTIONS] [ROOT]\n\nFor more information, try '--help'.\n",
    );
    fail(
        &["one", "--", "--budget=x"],
        "error: unexpected argument '--budget=x' found\n\nUsage: benchmark-policy [OPTIONS] [ROOT]\n\nFor more information, try '--help'.\n",
    );
}
#[test]
fn option_tokens_cannot_supply_missing_path_values() {
    for token in ["--", "--evidence=e.jsonl", "--unknown", "--help", "-h"] {
        fail(&["--budget", token], MISSING_BUDGET);
    }
    fail(&["--evidence", "--budget=b.yaml"], MISSING_EVIDENCE);
}
#[test]
fn malformed_option_spellings_are_usage_errors() {
    for args in [
        vec!["--budget:x"],
        vec!["--evidence:x"],
        vec!["--help=value"],
    ] {
        match parse_paths(&args) {
            Err(PolicyError::Usage(message)) => {
                assert!(message.starts_with("error:"));
                assert!(message.contains("benchmark-policy"));
            }
            actual => assert_eq!(format!("{actual:?}"), "expected usage error"),
        }
    }
}
#[test]
fn help_precedes_filesystem_and_later_missing_values() {
    for args in [
        vec!["--help", "--budget"],
        vec!["/absent/repo", "-h"],
        vec![
            "--budget=/absent/budget",
            "--evidence=/absent/evidence",
            "--help",
        ],
    ] {
        assert!(matches!(
            parse_cli(args.into_iter().map(str::to_owned)),
            Ok(CliAction::Help)
        ));
    }
    assert!(HELP.contains("benchmark-policy [OPTIONS] [ROOT]"));
    assert!(HELP.contains("--budget"));
    assert!(HELP.contains("--evidence"));
    assert!(!HELP.contains("xtask"));
}
#[test]
fn missing_value_precedes_help_action() {
    fail(&["--budget", "--help"], MISSING_BUDGET);
    fail(&["--evidence", "-h"], MISSING_EVIDENCE);
}

#[cfg(unix)]
#[test]
fn invalid_utf8_is_usage_error_before_parsing_paths() {
    use std::os::unix::ffi::OsStringExt;
    let arguments = [std::ffi::OsString::from_vec(vec![0xff])];
    match parse_os_cli(arguments.into_iter()) {
        Err(PolicyError::Usage(message)) => assert_eq!(
            message,
            "error: arguments must be valid UTF-8\n\nUsage: benchmark-policy [OPTIONS] [ROOT]\n\nFor more information, try '--help'.\n"
        ),
        _ => assert_eq!("unexpected action", "expected usage error"),
    }
}

#[test]
fn surplus_roots_allow_later_help_actions() {
    for help in ["--help", "-h"] {
        let args = ["one", "two", help, "--budget"];
        assert!(matches!(
            parse_cli(args.into_iter().map(str::to_owned)),
            Ok(CliAction::Help)
        ));
    }
}

#[test]
fn surplus_roots_do_not_mask_missing_option_values() {
    fail(&["one", "two", "--budget"], MISSING_BUDGET);
    fail(&["one", "two", "--budget", "--help"], MISSING_BUDGET);
    fail(&["one", "two", "--evidence"], MISSING_EVIDENCE);
    fail(&["one", "two", "--evidence", "-h"], MISSING_EVIDENCE);
    fail(&["--budget", "--help", "one", "two"], MISSING_BUDGET);
    fail(&["--evidence", "-h", "one", "two"], MISSING_EVIDENCE);
}

#[test]
fn negative_number_prefixes_are_root_operands() -> Result<()> {
    for root in ["-1", "-.5", "-1.5", "-0", "-1e3", "-1name", "-.5suffix"] {
        assert_eq!(parse_paths(&[root])?.0, root);
    }
    Ok(())
}

#[test]
fn negative_number_prefixes_supply_separate_path_values() -> Result<()> {
    for value in ["-1", "-.5", "-1.5", "-0", "-1e3", "-1name"] {
        let (root, budget, evidence) =
            parse_paths(&["--budget", value, "root", "--evidence", value])?;
        assert_eq!(root, "root");
        assert_eq!(budget, value);
        assert_eq!(evidence, value);
    }
    Ok(())
}

#[test]
fn invalid_negative_prefixes_remain_options() -> Result<()> {
    for value in ["-x1", "-.", "-.x", "--1"] {
        assert!(is_option(value)?);
        fail(&["--budget", value], MISSING_BUDGET);
        fail(&["--evidence", value], MISSING_EVIDENCE);
        match parse_paths(&[value]) {
            Err(PolicyError::Usage(message)) => {
                assert!(message.starts_with("error: unexpected argument"));
                assert!(message.contains(value));
            }
            actual => assert_eq!(format!("{actual:?}"), "expected usage error"),
        }
    }
    Ok(())
}

#[cfg(unix)]
fn invalid_os_argument() -> std::ffi::OsString {
    use std::os::unix::ffi::OsStringExt;
    std::ffi::OsString::from_vec(vec![0xff])
}

#[cfg(unix)]
fn os_usage_error(arguments: Vec<std::ffi::OsString>, expected: &str) {
    match parse_os_cli(arguments.into_iter()) {
        Err(PolicyError::Usage(message)) => assert_eq!(message, expected),
        _ => assert_eq!("unexpected action", "expected usage error"),
    }
}

#[cfg(unix)]
#[test]
fn os_help_does_not_decode_unused_invalid_argument() {
    for help in ["--help", "-h"] {
        let arguments = [std::ffi::OsString::from(help), invalid_os_argument()];
        assert!(matches!(
            parse_os_cli(arguments.into_iter()),
            Ok(CliAction::Help)
        ));
    }
}

#[cfg(unix)]
#[test]
fn os_help_after_surplus_roots_does_not_decode_later_argument() {
    for help in ["--help", "-h"] {
        let arguments = [
            std::ffi::OsString::from("one"),
            std::ffi::OsString::from("two"),
            std::ffi::OsString::from(help),
            invalid_os_argument(),
        ];
        assert!(matches!(
            parse_os_cli(arguments.into_iter()),
            Ok(CliAction::Help)
        ));
    }
}

#[cfg(unix)]
#[test]
fn os_invalid_separate_value_remains_encoding_usage_error() {
    for flag in ["--budget", "--evidence"] {
        os_usage_error(
            vec![std::ffi::OsString::from(flag), invalid_os_argument()],
            "error: arguments must be valid UTF-8\n\nUsage: benchmark-policy [OPTIONS] [ROOT]\n\nFor more information, try '--help'.\n",
        );
    }
}

#[cfg(unix)]
#[test]
fn os_missing_value_precedes_help_and_unused_invalid_argument() {
    for (flag, help, expected) in [
        ("--budget", "--help", MISSING_BUDGET),
        ("--evidence", "-h", MISSING_EVIDENCE),
    ] {
        os_usage_error(
            vec![
                std::ffi::OsString::from("one"),
                std::ffi::OsString::from("two"),
                std::ffi::OsString::from(flag),
                std::ffi::OsString::from(help),
                invalid_os_argument(),
            ],
            expected,
        );
    }
}

#[cfg(unix)]
#[test]
fn os_terminator_keeps_invalid_operand_as_encoding_error() {
    os_usage_error(
        vec![std::ffi::OsString::from("--"), invalid_os_argument()],
        "error: arguments must be valid UTF-8\n\nUsage: benchmark-policy [OPTIONS] [ROOT]\n\nFor more information, try '--help'.\n",
    );
}

const ND_PREFIX_PATHS: &[&str] = &[
    "-١",
    "-.١",
    "-١name",
    "-१",
    "-.१tail",
    "-１",
    "-.１name",
    "-𝟙",
    "-.𝟙tail",
];

#[test]
fn unicode_nd_prefixes_are_root_operands() -> Result<()> {
    for &root in ND_PREFIX_PATHS {
        assert_eq!(parse_paths(&[root])?.0, root);
    }
    Ok(())
}

#[test]
fn unicode_nd_prefixes_supply_separate_path_values() -> Result<()> {
    for &value in ND_PREFIX_PATHS {
        let (root, budget, evidence) =
            parse_paths(&["--budget", value, "repo", "--evidence", value])?;
        assert_eq!(root, "repo");
        assert_eq!(budget, value);
        assert_eq!(evidence, value);
    }
    Ok(())
}

#[test]
fn unicode_nl_and_no_numbers_remain_unknown_options() -> Result<()> {
    for value in ["-Ⅳ", "-.Ⅳ", "-²", "-.²", "-½", "-.½"] {
        assert!(is_option(value)?);
        let expected = format!(
            "error: unexpected argument '{value}' found\n\nUsage: benchmark-policy [OPTIONS] [ROOT]\n\nFor more information, try '--help'.\n"
        );
        fail(&[value], &expected);
        fail(&["--budget", value], MISSING_BUDGET);
        fail(&["--evidence", value], MISSING_EVIDENCE);
    }
    Ok(())
}

#[test]
fn unicode_nd_match_is_anchored_before_optional_dot() -> Result<()> {
    for value in ["-x١", "-..१", "--１", "-x𝟙"] {
        assert!(is_option(value)?);
        let expected = format!(
            "error: unexpected argument '{value}' found\n\nUsage: benchmark-policy [OPTIONS] [ROOT]\n\nFor more information, try '--help'.\n"
        );
        fail(&[value], &expected);
        fail(&["--budget", value], MISSING_BUDGET);
        fail(&["--evidence", value], MISSING_EVIDENCE);
    }
    Ok(())
}
