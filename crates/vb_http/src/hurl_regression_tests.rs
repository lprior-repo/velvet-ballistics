use super::*;

fn parse_error(source: &str, line: usize, kind: HurlParseErrorKind) {
    assert_eq!(
        parse_hurl(source).err(),
        Some(HurlParseError { line, kind }),
        "{source:?}"
    );
}

#[test]
fn strict_document_grammar_rejects_unsupported_or_trailing_input() {
    for source in ["", "GET /"] {
        assert_eq!(
            parse_hurl(source).err().map(|error| error.kind),
            Some(HurlParseErrorKind::Syntax)
        );
    }
    for (source, line) in [
        ("HTTP 200", 1),
        ("POST /\nHTTP 200", 1),
        ("get /\nHTTP 200", 1),
        ("GET/\nHTTP 200", 1),
        ("GET /\nhttp 200", 2),
        ("GET /\nHTTP200", 2),
        ("GET /\nHTTP/1.1 200", 2),
        ("GET /\nHTTP *", 2),
        ("GET /\nHTTP 099", 2),
        ("GET /\nHTTP 600", 2),
        ("GET /\nHTTP 000", 2),
        ("GET /\nHTTP +200", 2),
        ("GET /\nHTTP 0200", 2),
        ("GET /\nHTTP 2000", 2),
        ("GET /\nHTTP 200 trailing", 2),
        ("GET /\nHTTP 200\nGET /two", 3),
        ("GET /\nHTTP 200\nHTTP 200", 3),
        ("GET /\nHTTP 200\nstatus == 200", 3),
        ("GET /\nHTTP 200\n[asserts]", 3),
        ("GET /\nHTTP 200\n[Asserts]\n[Asserts]", 4),
        ("GET /\nHTTP 200\n[Captures]", 3),
        ("GET /\nHTTP 200\n[Asserts]\nbody == \"x\"", 4),
        ("GET /\nHTTP 200\n[Asserts]\nStatus == 200", 4),
        ("GET /\nHTTP 200\n[Asserts]\nstatus Not == 200", 4),
        ("GET /\nHTTP 200\n# comment", 3),
        ("GET /\nHTTP 200 # comment", 2),
        ("GET /\nHTTP 200\nbody", 3),
        ("GET /\rHTTP 200", 1),
        ("GET /\nHTTP 200\r", 2),
        ("GET\u{a0}/\nHTTP 200", 1),
        ("GET /\nHTTP\u{a0}200", 2),
        ("GET /\nHTTP 200\n[Asserts]\nstatus == 200\nunknown", 5),
    ] {
        parse_error(source, line, HurlParseErrorKind::Syntax);
    }
}

#[test]
fn source_byte_cap_is_inclusive_and_precedes_all_other_rejections() -> crate::Result<()> {
    let prefix = "GET /\nHTTP 200\n";
    let padding = 16384_usize
        .checked_sub(prefix.len())
        .ok_or("source fixture length")?;
    let exact = format!("{prefix}{}", "\n".repeat(padding));
    assert_eq!(exact.len(), 16384);
    assert_eq!(parse_hurl(&exact)?.assertion_count(), 0);
    parse_error(&format!("{exact}\n"), 0, HurlParseErrorKind::SourceBound);
    let invalid = format!("POST /\n{}", "x".repeat(16384));
    parse_error(&invalid, 0, HurlParseErrorKind::SourceBound);
    let multibyte = "é".repeat(8193);
    assert_eq!(multibyte.len(), 16386);
    parse_error(&multibyte, 0, HurlParseErrorKind::SourceBound);
    Ok(())
}

#[test]
fn physical_line_cap_excludes_terminators_and_prepasses_later_lines() -> crate::Result<()> {
    let row = "HTTP 200";
    let padding = 4096_usize
        .checked_sub(row.len())
        .ok_or("line fixture length")?;
    for terminator in ["\n", "\r\n"] {
        let exact = format!("{row}{}", " ".repeat(padding));
        assert_eq!(exact.len(), 4096);
        let source = format!("{terminator}GET /{terminator}{exact}{terminator}");
        assert_eq!(parse_hurl(&source)?.required_status(), 200);
        let over = format!("{terminator}GET /{terminator}{exact} {terminator}");
        parse_error(&over, 3, HurlParseErrorKind::LineBound);
        let invalid_first =
            format!("POST /{terminator}{terminator}{terminator}{exact} {terminator}");
        parse_error(&invalid_first, 4, HurlParseErrorKind::LineBound);
    }
    let multibyte = format!("GET /\nHTTP 200\n\n{}", "é".repeat(2049));
    parse_error(&multibyte, 4, HurlParseErrorKind::LineBound);
    parse_error("POST /\nHTTP 200\ninvalid", 1, HurlParseErrorKind::Syntax);
    Ok(())
}

#[test]
fn target_limit_and_literal_policy_do_not_normalize_or_accept_controls() -> crate::Result<()> {
    for target in ["/", "%", "/a?x=%zz", "/a?x=%20&y=1"] {
        let source = format!("GET {target}\nHTTP 200");
        assert_eq!(parse_hurl(&source)?.target(), target);
    }
    let exact = format!("/{}", "a".repeat(2047));
    assert_eq!(exact.len(), 2048);
    let source = format!("GET {exact}\nHTTP 200");
    assert_eq!(parse_hurl(&source)?.target(), exact);
    parse_error(
        &format!("GET {exact}a\nHTTP 200"),
        1,
        HurlParseErrorKind::Syntax,
    );
    for target in [
        "",
        "/#fragment",
        "/a b",
        "/a\tb",
        "/a\\b",
        "/{{x}}",
        "/é",
        "/a\0b",
        "/a\u{7f}b",
    ] {
        parse_error(
            &format!("GET {target}\nHTTP 200"),
            1,
            HurlParseErrorKind::Syntax,
        );
    }
    Ok(())
}

#[test]
fn optional_assertion_limit_counts_rows_after_validating_their_grammar() -> crate::Result<()> {
    let exact = format!(
        "GET /\nHTTP 200\n[Asserts]\n{}",
        "status == 200\n".repeat(64)
    );
    let plan = parse_hurl(&exact)?;
    assert_eq!(plan.assertion_count(), 64);
    assert_eq!(evaluate_hurl(&plan, 200, b""), Ok(()));
    parse_error(
        &format!("{exact}status == 200"),
        68,
        HurlParseErrorKind::AssertionBound,
    );
    parse_error(
        &format!("{exact}status ==200"),
        68,
        HurlParseErrorKind::Syntax,
    );
    let empty = parse_hurl("GET /\nHTTP 200\n[Asserts]\n\n")?;
    assert_eq!(empty.assertion_count(), 0);
    assert_eq!(evaluate_hurl(&empty, 200, b""), Ok(()));
    Ok(())
}

#[test]
fn opaque_body_bound_precedes_status_domain_required_status_and_assertions() -> crate::Result<()> {
    let plan = parse_hurl("GET /\nHTTP 200")?;
    let exact = vec![0xff; 1048576];
    let over = vec![0xff; 1048577];
    assert_eq!(evaluate_hurl(&plan, 200, &exact), Ok(()));
    assert_eq!(evaluate_hurl(&plan, 200, &[0xff, 0, 0x80]), Ok(()));
    let failing = parse_hurl("GET /\nHTTP 200\n[Asserts]\nstatus == 201")?;
    for status in [0, 99, 200, 201, 600, u16::MAX] {
        assert_eq!(
            evaluate_hurl(&failing, status, &over),
            Err(HurlEvalError::BodyBound)
        );
    }
    for status in [0, 99, 600, u16::MAX] {
        assert_eq!(
            evaluate_hurl(&failing, status, b""),
            Err(HurlEvalError::InvalidStatus)
        );
    }
    for status in [100, 201, 599] {
        assert_eq!(
            evaluate_hurl(&failing, status, b""),
            Err(HurlEvalError::RequiredStatus {
                expected: 200,
                actual: status
            })
        );
    }
    for status in [100, 599] {
        let source = format!("GET /\nHTTP {status}");
        assert_eq!(evaluate_hurl(&parse_hurl(&source)?, status, b""), Ok(()));
    }
    Ok(())
}

#[test]
fn evaluation_order_retains_first_index_and_wraps_precedence_in_typed_api() -> crate::Result<()> {
    let source = "GET /\nHTTP 200\n[Asserts]\nstatus >= 200\nstatus != 200\nstatus == 201";
    let plan = parse_hurl(source)?;
    assert_eq!(
        evaluate_hurl(&plan, 200, b""),
        Err(HurlEvalError::AssertionFailed { index: 2 })
    );
    let invalid = [ResponseHeader {
        name: "x",
        value: "\r",
    }];
    assert_eq!(
        evaluate_hurl_with_headers(&plan, 200, &invalid, b""),
        Err(HurlEvalError::InvalidHeader)
    );
    assert_eq!(
        evaluate_hurl_with_headers(&plan, 201, &invalid, b""),
        Err(HurlEvalError::RequiredStatus {
            expected: 200,
            actual: 201
        })
    );
    assert_eq!(
        evaluate_hurl_with_headers(&plan, 99, &invalid, b""),
        Err(HurlEvalError::InvalidStatus)
    );
    let over = vec![0; 1048577];
    assert_eq!(
        evaluate_hurl_with_headers(&plan, 99, &invalid, &over),
        Err(HurlEvalError::BodyBound)
    );
    assert_eq!(
        evaluate_hurl_with_header_predicates(&plan, 200, &invalid, b""),
        Err(HurlPredicateEvalError::Evaluation(
            HurlEvalError::InvalidHeader
        ))
    );
    let typed = parse_hurl_with_header_predicates(
        "GET /\nHTTP 200\n[Asserts]\nstatus != 200\nheader \"x\" includes \"a\"",
    )?;
    let scalar = [ResponseHeader {
        name: "x",
        value: "a",
    }];
    assert_eq!(
        evaluate_hurl_with_header_predicates(&typed, 200, &scalar, b""),
        Err(HurlPredicateEvalError::Evaluation(
            HurlEvalError::AssertionFailed { index: 1 }
        ))
    );
    Ok(())
}

#[test]
fn header_occurrence_name_value_and_combined_bounds_are_inclusive() -> crate::Result<()> {
    let plan = parse_hurl("GET /\nHTTP 200")?;
    let field = ResponseHeader {
        name: "x",
        value: "",
    };
    assert_eq!(
        evaluate_hurl_with_headers(&plan, 200, &[field; 64], b""),
        Ok(())
    );
    assert_eq!(
        evaluate_hurl_with_headers(&plan, 200, &[field; 65], b""),
        Err(HurlEvalError::HeaderBound)
    );
    let name = "x".repeat(256);
    let over_name = "x".repeat(257);
    let value = "a".repeat(8192);
    let over_value = "a".repeat(8193);
    for (name, value, expected) in [
        (name.as_str(), "", Ok(())),
        (over_name.as_str(), "", Err(HurlEvalError::HeaderBound)),
        ("x", value.as_str(), Ok(())),
        ("x", over_value.as_str(), Err(HurlEvalError::HeaderBound)),
    ] {
        assert_eq!(
            evaluate_hurl_with_headers(&plan, 200, &[ResponseHeader { name, value }], b""),
            expected
        );
    }
    let value = "a".repeat(8191);
    let exact = [ResponseHeader {
        name: "x",
        value: &value,
    }; 8];
    let exact_bytes = exact.iter().try_fold(0_usize, |total, field| {
        total
            .checked_add(field.name.len())?
            .checked_add(field.value.len())
    });
    assert_eq!(exact_bytes, Some(65536));
    assert_eq!(evaluate_hurl_with_headers(&plan, 200, &exact, b""), Ok(()));
    let over_value = "a".repeat(8192);
    let mut over = vec![
        ResponseHeader {
            name: "x",
            value: &value
        };
        7
    ];
    over.push(ResponseHeader {
        name: "x",
        value: &over_value,
    });
    let over_bytes = over.iter().try_fold(0_usize, |total, field| {
        total
            .checked_add(field.name.len())?
            .checked_add(field.value.len())
    });
    assert_eq!(over_bytes, Some(65537));
    assert_eq!(
        evaluate_hurl_with_headers(&plan, 200, &over, b""),
        Err(HurlEvalError::HeaderBound)
    );
    Ok(())
}

#[test]
fn response_headers_are_validated_even_without_queries_and_bound_checks_win() -> crate::Result<()> {
    let plan = parse_hurl("GET /\nHTTP 200")?;
    for name in ["", "x y", "x:y", "x\ty", "x\r", "é"] {
        assert_eq!(
            evaluate_hurl_with_headers(&plan, 200, &[ResponseHeader { name, value: "a" }], b""),
            Err(HurlEvalError::InvalidHeader),
            "{name:?}"
        );
    }
    for value in ["\0", "\r", "\n", "\u{7f}", "é"] {
        assert_eq!(
            evaluate_hurl_with_headers(&plan, 200, &[ResponseHeader { name: "x", value }], b""),
            Err(HurlEvalError::InvalidHeader),
            "{value:?}"
        );
    }
    let token = "!#$%&'*+-.^_`|~012azAZ";
    assert_eq!(
        evaluate_hurl_with_headers(
            &plan,
            200,
            &[ResponseHeader {
                name: token,
                value: " \t~"
            }],
            b""
        ),
        Ok(())
    );
    let long = "a".repeat(8193);
    let fields = [
        ResponseHeader {
            name: "bad name",
            value: "a",
        },
        ResponseHeader {
            name: "x",
            value: &long,
        },
    ];
    assert_eq!(
        evaluate_hurl_with_headers(&plan, 200, &fields, b""),
        Err(HurlEvalError::HeaderBound)
    );
    Ok(())
}
