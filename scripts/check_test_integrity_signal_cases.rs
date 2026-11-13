use super::check_test_integrity_fixture::{BASE_TEXT, Fixture, expect_findings, report_failure};

pub(crate) fn run_signal_cases() -> bool {
    let header = "fn signal_test()";
    let adjacent = "std::iter::empty::<u8>().skip(1);";
    let usize_argument = "std::iter::empty::<u8>().skip(1usize);";
    let raw_method = "std::iter::empty::<u8>().r#skip(1);";
    let spaced_method = "std::iter::empty::<u8>(). skip(1);";
    let data_cases = [
        ("r5-method-adjacent", header, adjacent),
        ("r5-method-usize-argument", header, usize_argument),
        ("r5-method-before-dot-space", header, "items .skip(1);"),
        ("r5-field", header, "let _field = items.skip;"),
        ("r5-raw-field", header, "let _field = items.r#skip;"),
        ("r5-raw-method", header, raw_method),
        ("r5-field-chain", header, "items.r#field.skip(1);"),
        ("signal-ignored-name", "fn ignored_test()", ""),
        ("signal-gitignored-name", "fn gitignored_test()", ""),
        ("signal-skipped-name", "fn skipped_test()", ""),
        ("signal-ignored-variable", header, "let ignored = 4;"),
        ("signal-skip-variable", header, "let skip = 4;"),
        ("signal-skipped-variable", header, "let skipped = 4;"),
        (
            "signal-doc-comment",
            "/// ignored skip skipped\nfn signal_test()",
            "",
        ),
        (
            "signal-quoted-words",
            header,
            r##"let text = "ignored skip skipped";"##,
        ),
        ("signal-iterator", header, "let values = items.skip(1);"),
        ("signal-skip-declaration", "fn skip()", ""),
        ("signal-return-value", header, r##"return "ignored";"##),
        ("signal-typed-skip", header, "let skip: u8 = 4;"),
        ("signal-split-let", header, "let skip\n= 4;"),
        ("signal-generic-method", header, "items.skip::<()>();"),
    ];
    let marker_cases = [
        ("r5-exclusive-spaced", header, "let _value = 0.. skip!();"),
        ("r5-exclusive-adjacent", header, "let _value = 0..skip!();"),
        ("r5-unbounded-spaced", header, "let _value = .. skipped!();"),
        (
            "r5-unbounded-adjacent",
            header,
            "let _value = ..skipped!();",
        ),
        ("r5-inclusive", header, "let _value = 0..= skipped!();"),
        ("r5-split-spaced", header, "let _value = 0.. skip\n!();"),
        ("r5-split-adjacent", header, "let _value = 0..skip\n!();"),
        ("r5-skipped-split", header, "let _value = ..skipped\n!();"),
        ("r5-function-spaced", header, "let _value = 0.. skip();"),
        ("r5-function-adjacent", header, "let _value = 0..skip();"),
        ("r5-skipped-function", header, "let _value = .. skipped();"),
        (
            "r5-generic-spaced",
            header,
            "let _value = 0.. skip::<u8>();",
        ),
        (
            "r5-generic-adjacent",
            header,
            "let _value = 0..skip::<u8>();",
        ),
        (
            "r5-generic-prefix",
            header,
            "let _value = ..skip::<\nu8>();",
        ),
        ("r5-method-spaced", header, spaced_method),
        ("r5-method-tab", header, "items.\tskip(1);"),
        ("r5-dot-macro", header, "items.skip!();"),
        ("r5-raw-return", header, "let _field = items.r#return;"),
        (
            "signal-line-comment",
            header,
            "// ignored skip skipped return; #[ignore]",
        ),
        (
            "signal-block-comment",
            header,
            "/* ignored skip skipped return; #[ignore] */",
        ),
        (
            "signal-quoted-markers",
            header,
            r##"let text = "#[ignore] return; skip();";"##,
        ),
        (
            "signal-raw-markers",
            header,
            r##"let text = r#"#[ignore] #[ skip ] return; skip();"#;"##,
        ),
        (
            "signal-escaped-quote",
            header,
            r##"let text = "\" #[ignore] return; skip();";"##,
        ),
        (
            "signal-raw-identifier",
            header,
            "let r#return = 4; r#return;",
        ),
        (
            "signal-cfg-predicate",
            "#[cfg_attr(all(ignored, skip), inline)]\nfn signal_test()",
            "",
        ),
        (
            "signal-generic-reference",
            header,
            "let handler = skip::<()>;",
        ),
        (
            "signal-nested-predicate",
            "#[cfg_attr(test, cfg_attr(all(ignore, skip), inline))]\nfn signal_test()",
            "",
        ),
        ("signal-ignore", "#[ignore]\nfn signal_test()", ""),
        ("signal-raw-ignore", "#[r#ignore]\nfn signal_test()", ""),
        ("signal-raw-skip", "#[r#skip]\nfn signal_test()", ""),
        (
            "signal-raw-cfg-ignore",
            "#[cfg_attr(test, r#ignore)]\nfn signal_test()",
            "",
        ),
        (
            "signal-raw-cfg-root",
            "#[r#cfg_attr(test, ignore)]\nfn signal_test()",
            "",
        ),
        (
            "signal-raw-cfg-root-ignore",
            "#[r#cfg_attr(test, r#ignore)]\nfn signal_test()",
            "",
        ),
        (
            "signal-raw-nested-ignore",
            "#[cfg_attr(test, r#cfg_attr(test, r#ignore))]\nfn signal_test()",
            "",
        ),
        (
            "signal-raw-nested-skip",
            "#[r#cfg_attr(test, cfg_attr(test, r#skip))]\nfn signal_test()",
            "",
        ),
        (
            "signal-continued-string-return",
            header,
            "let _text = \"opening\n\"; if std::hint::black_box(true) { return; }\n    assert_eq!(1, 2);",
        ),
        (
            "signal-incomplete-string",
            header,
            "let _text = \"return;\nend\";",
        ),
        (
            "signal-incomplete-raw-string",
            header,
            "let _text = r#\"return;\nend\"#;",
        ),
        ("signal-incomplete-block", header, "/* return;\ncomment */"),
        (
            "signal-ignore-reason",
            "#[ignore = \"reason\"]\nfn signal_test()",
            "",
        ),
        (
            "signal-cfg-ignore",
            "#[cfg_attr(test, ignore)]\nfn signal_test()",
            "",
        ),
        (
            "signal-cfg-ignore-reason",
            "#[cfg_attr(test, ignore = \"reason\")]\nfn signal_test()",
            "",
        ),
        (
            "signal-nested-ignore",
            "#[cfg_attr(test, cfg_attr(test, ignore))]\nfn signal_test()",
            "",
        ),
        (
            "signal-nested-skip",
            "#[cfg_attr(test, cfg_attr(test, skip))]\nfn signal_test()",
            "",
        ),
        ("signal-spaced-skip", "#[ skip ]\nfn signal_test()", ""),
        ("signal-return", header, "return;"),
        ("signal-spaced-return", header, "return ;"),
        (
            "signal-comment-return",
            header,
            "return; // ignored skip skipped",
        ),
        (
            "signal-comment-ignore",
            "#[ignore] // ignored skip skipped\nfn signal_test()",
            "",
        ),
        (
            "signal-block-return",
            header,
            "/* ignored skip skipped */ return;",
        ),
        (
            "signal-block-ignore",
            "/* ignored skip skipped */ #[ignore]\nfn signal_test()",
            "",
        ),
        ("signal-call", header, "skip();"),
        ("signal-skipped-call", header, "skipped();"),
        ("signal-skipped-macro", header, "skipped!(\"reason\");"),
        ("signal-split-macro", header, "skip\n!();"),
        ("signal-qualified-split-macro", header, "crate:: skip\n!();"),
        ("signal-skipped-empty-macro", header, "skipped!();"),
        (
            "signal-block-prefix-split-macro",
            header,
            "/* opening\n// */ skip\n!();",
        ),
        (
            "signal-string-prefix-split-macro",
            header,
            "let _text = \"opening\n// \"; skip\n!();",
        ),
        ("signal-generic-prefix", header, "skip::<\n()>();"),
        (
            "signal-skipped-generic-prefix",
            header,
            "skipped::<\n()>();",
        ),
        (
            "signal-continued-string-comment",
            header,
            "let _text = \"opening\n\"; if std::hint::black_box(true) { return; } // \"\n    assert_eq!(1, 2);",
        ),
        ("signal-generic-call", header, "skip::<()>();"),
        ("signal-macro", header, "skip!(\"reason\");"),
        (
            "signal-data-and-return",
            header,
            r##"let text = "ignored skip"; return;"##,
        ),
        (
            "signal-data-and-ignore",
            "#[doc = \"ignored skip\"] #[ignore]\nfn signal_test()",
            "",
        ),
    ];
    let data_ok = data_cases
        .into_iter()
        .fold(true, |all, (label, header, body)| {
            run_signal_case(label, header, body, false) && all
        });
    marker_cases
        .into_iter()
        .fold(data_ok, |all, (label, header, body)| {
            run_signal_case(label, header, body, true) && all
        })
}

fn run_signal_case(label: &str, header: &str, body: &str, marker: bool) -> bool {
    let path = "crates/workspace_tests/tests/behavior.rs";
    let fixture = match Fixture::new(label) {
        Ok(fixture) => fixture,
        Err(error) => return report_failure(label, &format!("fixture setup: {error}")),
    };
    let added =
        format!("{BASE_TEXT}\n#[test]\n{header} {{\n    {body}\n    assert_eq!(2, 2);\n}}\n");
    let setup = fixture
        .write(path, BASE_TEXT)
        .and_then(|()| fixture.commit_base())
        .and_then(|()| fixture.write(path, &added));
    if let Err(error) = setup {
        return report_failure(label, &format!("setup: {error}"));
    }
    let findings = match fixture.collect() {
        Ok(findings) => findings,
        Err(error) => return report_failure(label, &format!("collect: {error}")),
    };
    let exit = match fixture.check() {
        Ok(code) => code,
        Err(error) => return report_failure(label, &format!("check: {error}")),
    };
    let expected = if marker {
        vec![("IgnoredOrSkippedTest", path)]
    } else {
        Vec::new()
    };
    let findings_ok = expect_findings(label, &findings, &expected);
    let expected_exit = if marker { 1 } else { 0 };
    println!("SelfTest:{label}: exit expected={expected_exit} actual={exit}");
    findings_ok && exit == expected_exit
}
