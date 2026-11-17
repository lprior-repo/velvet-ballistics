use super::*;

fn outcome(accepted: bool, index: usize) -> Result<(), HurlEvalError> {
    if accepted {
        Ok(())
    } else {
        Err(HurlEvalError::AssertionFailed { index })
    }
}

fn legacy_error_is_exhaustive(error: HurlEvalError) -> &'static str {
    match error {
        HurlEvalError::BodyBound => "body",
        HurlEvalError::InvalidStatus => "status",
        HurlEvalError::RequiredStatus { .. } => "required",
        HurlEvalError::AssertionFailed { .. } => "assertion",
        HurlEvalError::HeaderObservationRequired => "observation",
        HurlEvalError::HeaderBound => "header-bound",
        HurlEvalError::InvalidHeader => "header",
    }
}

#[test]
fn layouts_preserve_borrowed_literal_target_and_plan() -> crate::Result<()> {
    for source in [
        "GET /a?x=1&y=%20\nHTTP 200",
        "GET /a?x=1&y=%20\nHTTP 200\n",
        "\r\n\tGET\t/a?x=1&y=%20 \t\r\n\r\n HTTP \t200\t\r\n[Asserts]\r\n",
    ] {
        let plan = parse_hurl(source)?;
        assert_eq!(plan.target(), "/a?x=1&y=%20");
        assert_eq!(plan.required_status(), 200);
        assert_eq!(plan.assertion_count(), 0);
        assert!(!plan.requires_headers());
        let offset = source.find("/a?").ok_or("fixture target absent")?;
        let borrowed = source.get(offset..).ok_or("fixture target range")?;
        assert_eq!(plan.target().as_ptr(), borrowed.as_ptr());
        assert_eq!(evaluate_hurl(&plan, 200, &[0xff, 0, 0x80]), Ok(()));
        assert_eq!(plan.target(), "/a?x=1&y=%20");
        assert_eq!(plan.assertion_count(), 0);
    }
    let plan = parse_hurl("GET /\nHTTP 200\n[Asserts]\nstatus\t>=\t200\nstatus \tnot \t!= \t200")?;
    assert_eq!(plan.assertion_count(), 2);
    assert_eq!(evaluate_hurl(&plan, 200, b""), Ok(()));
    Ok(())
}

#[test]
fn all_status_comparisons_and_negations_have_literal_truth_tables() -> crate::Result<()> {
    let tables = [
        ("==", [false, true, false], [false, false, false, false]),
        ("!=", [true, false, true], [true, true, true, true]),
        ("<", [false, false, true], [false, true, false, false]),
        ("<=", [false, true, true], [false, true, false, false]),
        (">", [true, false, false], [true, false, true, true]),
        (">=", [true, true, false], [true, false, true, true]),
    ];
    for (operator, neighboring, extrema) in tables {
        let rows = ["199", "200", "201"].into_iter().zip(neighboring).chain(
            ["-9223372036854775808", "9223372036854775807", "0", "-0"]
                .into_iter()
                .zip(extrema),
        );
        for (operand, accepted) in rows {
            for (qualifier, expected) in [("", accepted), ("not ", !accepted)] {
                let source =
                    format!("GET /\nHTTP 200\n[Asserts]\nstatus {qualifier}{operator} {operand}");
                let plan = parse_hurl(&source)?;
                assert_eq!(plan.assertion_count(), 1);
                assert_eq!(
                    evaluate_hurl(&plan, 200, b""),
                    outcome(expected, 1),
                    "{source}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn integer_spelling_and_token_consumption_reject_prefixes() {
    for operand in [
        "+0",
        "+1",
        "00",
        "01",
        "-00",
        "-01",
        "-",
        "--1",
        "- 1",
        "-9223372036854775809",
        "9223372036854775808",
        "1.0",
        "1e2",
        "\"200\"",
        "true",
        "null",
        "２００",
        "200 trailing",
        "200#comment",
    ] {
        let source = format!("GET /\nHTTP 200\n[Asserts]\nstatus == {operand}");
        assert_eq!(
            parse_hurl(&source).err(),
            Some(HurlParseError {
                line: 4,
                kind: HurlParseErrorKind::Syntax
            }),
            "{operand}"
        );
    }
    for assertion in [
        "status ==",
        "status ==200",
        "status== 200",
        "status not !=200",
        "status not not == 200",
        "status = 200",
        "status == 200 # x",
    ] {
        let source = format!("GET /\nHTTP 200\n[Asserts]\n{assertion}");
        assert_eq!(
            parse_hurl(&source).err(),
            Some(HurlParseError {
                line: 4,
                kind: HurlParseErrorKind::Syntax
            }),
            "{assertion}"
        );
    }
}

#[test]
fn header_shapes_preserve_missing_empty_scalar_duplicate_and_comma() -> crate::Result<()> {
    let observations: [&[ResponseHeader<'_>]; 5] = [
        &[],
        &[ResponseHeader {
            name: "X",
            value: "",
        }],
        &[ResponseHeader {
            name: "X",
            value: "alpha",
        }],
        &[
            ResponseHeader {
                name: "X",
                value: "alpha",
            },
            ResponseHeader {
                name: "x",
                value: "beta",
            },
        ],
        &[ResponseHeader {
            name: "x",
            value: "alpha,beta",
        }],
    ];
    let cases = [
        ("exists", [false, true, true, true, true]),
        ("isEmpty", [false, true, false, false, false]),
        ("== \"alpha\"", [false, false, true, false, false]),
        ("!= \"alpha\"", [false, true, false, true, true]),
        ("not exists", [true, false, false, false, false]),
        ("not == \"alpha\"", [true, true, false, true, true]),
        ("isString", [false, true, true, false, true]),
        ("isList", [false, false, false, true, false]),
        ("isCollection", [false, false, false, true, false]),
    ];
    for (predicate, expected) in cases {
        let source = format!("GET /\nHTTP 200\n[Asserts]\nheader \"x\" {predicate}");
        let plan = parse_hurl_with_header_predicates(&source)?;
        assert!(plan.requires_headers());
        for (headers, accepted) in observations.into_iter().zip(expected) {
            assert_eq!(
                evaluate_hurl_with_headers(&plan, 200, headers, b""),
                outcome(accepted, 1),
                "{predicate} {headers:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn text_queries_have_typed_failures_that_negation_cannot_hide() -> crate::Result<()> {
    let scalar = [ResponseHeader {
        name: "X",
        value: "alpha,beta",
    }];
    let list = [
        ResponseHeader {
            name: "X",
            value: "alpha",
        },
        ResponseHeader {
            name: "x",
            value: "beta",
        },
    ];
    for (predicate, headers, accepted) in [
        ("startsWith \"alpha\"", scalar.as_slice(), true),
        ("endsWith \"beta\"", scalar.as_slice(), true),
        ("contains \"ha,be\"", scalar.as_slice(), true),
        ("contains \"alp\"", list.as_slice(), false),
        ("contains \"alpha\"", list.as_slice(), true),
        ("includes \"beta\"", list.as_slice(), true),
        ("includes \"alp\"", list.as_slice(), false),
    ] {
        let source = format!("GET /\nHTTP 200\n[Asserts]\nheader \"x\" {predicate}");
        let plan = parse_hurl_with_header_predicates(&source)?;
        assert_eq!(
            evaluate_hurl_with_header_predicates(&plan, 200, headers, b""),
            outcome(accepted, 1).map_err(HurlPredicateEvalError::Evaluation),
            "{predicate}"
        );
    }
    for (predicate, headers) in [
        ("startsWith", list.as_slice()),
        ("endsWith", list.as_slice()),
        ("includes", scalar.as_slice()),
    ] {
        for qualifier in ["", "not "] {
            let source = format!(
                "GET /\nHTTP 200\n[Asserts]\nstatus == 200\nheader \"x\" {qualifier}{predicate} \"alpha\""
            );
            let plan = parse_hurl_with_header_predicates(&source)?;
            assert_eq!(
                evaluate_hurl_with_header_predicates(&plan, 200, headers, b""),
                Err(HurlPredicateEvalError::HeaderPredicateTypeMismatch { index: 2 })
            );
            assert_eq!(
                evaluate_hurl_with_headers(&plan, 200, headers, b""),
                Err(HurlEvalError::AssertionFailed { index: 2 })
            );
        }
    }
    for predicate in ["startsWith", "endsWith", "contains", "includes"] {
        for (qualifier, accepted) in [("", false), ("not ", true)] {
            let source = format!(
                "GET /\nHTTP 200\n[Asserts]\nheader \"missing\" {qualifier}{predicate} \"x\""
            );
            let plan = parse_hurl_with_header_predicates(&source)?;
            assert_eq!(
                evaluate_hurl_with_header_predicates(&plan, 200, &[], b""),
                outcome(accepted, 1).map_err(HurlPredicateEvalError::Evaluation)
            );
        }
    }
    Ok(())
}

#[test]
fn count_uses_occurrences_and_type_failures_survive_not() -> crate::Result<()> {
    let list = [
        ResponseHeader {
            name: "X",
            value: "alpha,beta",
        },
        ResponseHeader {
            name: "x",
            value: "beta",
        },
    ];
    let scalar = [ResponseHeader {
        name: "x",
        value: "alpha,beta",
    }];
    for (operator, neighboring, extrema) in [
        ("==", [false, true, false], [false, false, false, false]),
        ("!=", [true, false, true], [true, true, true, true]),
        ("<", [false, false, true], [false, true, false, false]),
        ("<=", [false, true, true], [false, true, false, false]),
        (">", [true, false, false], [true, false, true, true]),
        (">=", [true, true, false], [true, false, true, true]),
    ] {
        let rows = ["1", "2", "3"].into_iter().zip(neighboring).chain(
            ["-9223372036854775808", "9223372036854775807", "0", "-0"]
                .into_iter()
                .zip(extrema),
        );
        for (operand, accepted) in rows {
            for (qualifier, expected) in [("", accepted), ("not ", !accepted)] {
                let source = format!(
                    "GET /\nHTTP 200\n[Asserts]\nheader \"x\" count {qualifier}{operator} {operand}"
                );
                let plan = parse_hurl_with_headers(&source)?;
                assert_eq!(
                    evaluate_hurl_with_headers(&plan, 200, &list, b""),
                    outcome(expected, 1),
                    "{source}"
                );
            }
        }
    }
    let missing: &[ResponseHeader<'_>] = &[];
    for headers in [missing, scalar.as_slice()] {
        for qualifier in ["", "not "] {
            let source = format!(
                "GET /\nHTTP 200\n[Asserts]\nstatus == 200\nheader \"x\" count {qualifier}== 0"
            );
            let plan = parse_hurl_with_headers(&source)?;
            assert_eq!(
                evaluate_hurl_with_headers(&plan, 200, headers, b""),
                Err(HurlEvalError::AssertionFailed { index: 2 })
            );
            assert_eq!(
                evaluate_hurl_with_header_predicates(&plan, 200, headers, b""),
                Err(HurlPredicateEvalError::HeaderPredicateTypeMismatch { index: 2 })
            );
        }
    }
    Ok(())
}

#[test]
fn parser_opt_in_tiers_and_quoted_rows_consume_every_byte() -> crate::Result<()> {
    let base = "GET /\nHTTP 200\n[Asserts]\nheader \"x\" exists";
    assert!(parse_hurl(base).is_err());
    assert!(parse_hurl_with_headers(base)?.requires_headers());
    for predicate in [
        "isString",
        "isList",
        "isCollection",
        "startsWith \"a\"",
        "endsWith \"a\"",
        "contains \"a\"",
        "includes \"a\"",
    ] {
        let source = format!("GET /\nHTTP 200\n[Asserts]\nheader \"x\" {predicate}");
        assert_eq!(
            parse_hurl_with_headers(&source).err(),
            Some(HurlParseError {
                line: 4,
                kind: HurlParseErrorKind::Syntax
            })
        );
        assert_eq!(
            parse_hurl_with_header_predicates(&source)?.assertion_count(),
            1
        );
    }
    for row in [
        "header x exists",
        "header \"\" exists",
        "header \"x y\" exists",
        "header \"x\"exists",
        "header \"x\" exists trailing",
        "header \"x\" ==\"a\"",
        "header \"x\" == \"a\" trailing",
        "header \"x\" == \"a\\b\"",
        "header \"x\" == \"{{x}}\"",
        "header \"x\" == \"é\"",
        "header \"x\" == \"a\tb\"",
        "header \"x\" not not exists",
        "header \"x\" not count == 2",
        "header \"x\" count == 2 trailing",
        "header \"x\" count == +2",
        "header \"x\" count isString",
    ] {
        let source = format!("GET /\nHTTP 200\n[Asserts]\n{row}");
        assert_eq!(
            parse_hurl_with_header_predicates(&source).err(),
            Some(HurlParseError {
                line: 4,
                kind: HurlParseErrorKind::Syntax
            }),
            "{row}"
        );
    }
    Ok(())
}

#[test]
fn legacy_error_api_stays_exhaustive_and_typed_failures_wrap_it() -> crate::Result<()> {
    let plan = parse_hurl("GET /\nHTTP 200\n[Asserts]\nstatus == 201")?;
    let legacy = HurlEvalError::AssertionFailed { index: 1 };
    assert_eq!(legacy_error_is_exhaustive(legacy), "assertion");
    assert_eq!(evaluate_hurl(&plan, 200, b""), Err(legacy));
    assert_eq!(
        evaluate_hurl_with_header_predicates(&plan, 200, &[], b""),
        Err(HurlPredicateEvalError::Evaluation(legacy))
    );
    let headers = parse_hurl_with_headers("GET /\nHTTP 200\n[Asserts]\nheader \"x\" exists")?;
    assert_eq!(
        evaluate_hurl(&headers, 200, b""),
        Err(HurlEvalError::HeaderObservationRequired)
    );
    assert_eq!(
        evaluate_hurl_with_headers(&headers, 200, &[], b""),
        Err(legacy)
    );
    Ok(())
}
