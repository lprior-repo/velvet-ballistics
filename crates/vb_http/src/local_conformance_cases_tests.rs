use super::*;

#[test]
fn closed_public_runner_requires_all_cases_and_binds_source() -> Result<(), LocalConformanceFailure>
{
    let progress = crate::run_local_conformance()?;
    assert_eq!(
        progress.receipts,
        [
            "zero count",
            "positive count",
            "non-200",
            "malformed",
            "duplicate",
            "wrong type",
            "body limit",
            "header limit",
            "truncated body",
            "conflicting length",
            "length plus chunked",
            "digest separation",
            "profile digest separation",
            "body before deadline",
            "body timeout after headers",
            "pre-header failure marker false"
        ]
    );
    assert_eq!(progress.schema(), SCHEMA);
    assert_eq!(
        *progress.source_fingerprint(),
        independent_source_fingerprint().map_err(LocalConformanceFailure::from)?
    );
    assert_eq!(progress.case_receipts(), progress.receipts.as_slice());
    Ok(())
}

#[cfg(test)]
fn independent_source_fingerprint() -> Result<[u8; 32], CaseFailure> {
    // Separate one-shot framing oracle, not another call to the incremental helper.
    let mut frame = Vec::new();
    frame.extend_from_slice(
        &(u64::try_from(SCHEMA.len()).map_err(|_| CaseFailure::Check("oracle length"))?)
            .to_le_bytes(),
    );
    frame.extend_from_slice(SCHEMA.as_bytes());
    for (path, contents) in SOURCE_FILES {
        frame.extend_from_slice(
            &(u64::try_from(path.len()).map_err(|_| CaseFailure::Check("oracle length"))?)
                .to_le_bytes(),
        );
        frame.extend_from_slice(path.as_bytes());
        frame.extend_from_slice(
            &(u64::try_from(contents.len()).map_err(|_| CaseFailure::Check("oracle length"))?)
                .to_le_bytes(),
        );
        frame.extend_from_slice(contents);
    }
    Ok(blake3::hash(&frame).into())
}

#[test]
fn fingerprint_binds_schema_path_content_order_and_inventory() -> Result<(), CaseFailure> {
    let expected = [
        "actions.rs",
        "decode/error.rs",
        "decode/limits.rs",
        "decode/number.rs",
        "decode/parser.rs",
        "decode/string.rs",
        "decode.rs",
        "error.rs",
        "github.rs",
        "hurl.rs",
        "hurl_evaluation.rs",
        "hurl_headers.rs",
        "hurl_integer.rs",
        "hurl_predicate_transport.rs",
        "hurl_syntax.rs",
        "hurl_transport.rs",
        "lib.rs",
        "limits.rs",
        "observation.rs",
        "request.rs",
        "request_profile.rs",
        "transport.rs",
        "workflows.rs",
        "Cargo.toml",
        "local_conformance.rs",
        "local_conformance_cases.rs",
        "local_conformance_io.rs",
        "local_conformance_checks.rs",
        "local_conformance_suite.rs",
        "local_conformance_fingerprint.rs",
        "transport/body.rs",
        "local_conformance_cases_tests.rs",
    ];
    assert_eq!(
        SOURCE_FILES
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>(),
        expected
    );
    assert!(SOURCE_FILES.iter().all(|(_, content)| !content.is_empty()));
    assert_eq!(source_fingerprint()?, independent_source_fingerprint()?);
    let base = fingerprint("schema", &[("path", b"content")])?;
    assert_ne!(base, fingerprint("schema2", &[("path", b"content")])?);
    assert_ne!(base, fingerprint("schema", &[("path2", b"content")])?);
    assert_ne!(base, fingerprint("schema", &[("path", b"content2")])?);
    assert_ne!(base, fingerprint("schema", &[])?);
    assert_ne!(
        fingerprint("schema", &[("a", b"bc")])?,
        fingerprint("schema", &[("ab", b"c")])?
    );
    assert_ne!(
        fingerprint("schema", &[("a", b"x"), ("b", b"y")])?,
        fingerprint("schema", &[("b", b"y"), ("a", b"x")])?
    );
    Ok(())
}

#[cfg(test)]
fn valid_request() -> String {
    let mut request = String::from("GET /observed HTTP/1.1\r\n");
    for (name, value) in PROFILE_HEADERS {
        request.push_str(name);
        request.push_str(": ");
        request.push_str(value);
        request.push_str("\r\n");
    }
    request.push_str("\r\n");
    request
}

#[test]
fn profile_rejects_extra_duplicate_changed_value_and_bad_framing() {
    let valid = valid_request();
    let accepts = |value: &str| {
        profile(&Capture {
            request: value.as_bytes().to_vec(),
            body_held: false,
        })
        .is_ok()
    };
    assert!(accepts(&valid));
    // Field names are case-insensitive; field values are deliberately exact.
    assert!(accepts(&valid.replace("user-agent:", "UsEr-AgEnT:")));
    for injected in [
        "Authorization: Bearer secret",
        "Cookie: secret",
        "x-extra: value",
        "Host: fixture.invalid",
        "host: other.invalid",
    ] {
        let changed = valid.replacen("\r\n\r\n", &format!("\r\n{injected}\r\n\r\n"), 1);
        assert!(!accepts(&changed), "accepted {injected}");
    }
    for (before, after) in [
        ("GET /observed", "POST /observed"),
        ("/observed", "/other"),
        ("fixture.invalid", "FIXTURE.invalid"),
        ("application/vnd.github+json", "Application/vnd.github+json"),
        ("velvet-ballistics/0.1", "Velvet-ballistics/0.1"),
        ("identity", "Identity"),
        ("close", "Close"),
        ("2026-03-10", "2026-03-11"),
        ("host: ", "host:"),
        ("host: ", "host:  "),
    ] {
        assert!(
            !accepts(&valid.replace(before, after)),
            "accepted value change {before}"
        );
    }
    assert!(!accepts(&valid.replace("connection: close\r\n", "")));
    assert!(!accepts(&(valid.clone() + "trailing-body")));
    assert!(!accepts(&valid.replace("\r\n", "\n")));
}
