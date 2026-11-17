use super::*;
use ureq::http::{HeaderMap, HeaderName, HeaderValue};

fn response(status: u16, body: &[u8], json_media: bool) -> HttpObservation {
    HttpObservation {
        status: ResponseStatus(status),
        body: body.to_vec(),
        json_media,
    }
}

fn append(headers: &mut HeaderMap, name: &str, value: &[u8]) -> crate::Result<()> {
    let name = HeaderName::from_bytes(name.as_bytes())?;
    let value = HeaderValue::from_bytes(value)?;
    let _same_name = headers.append(name, value);
    Ok(())
}

fn observed(headers: &HeaderMap) -> Result<HttpObservationWithHeaders, AttemptFailure> {
    Ok(HttpObservationWithHeaders::new(
        response(200, &[0xff, 0, 0x80], false),
        ObservedResponseHeaders::capture(headers)?,
    ))
}

#[test]
fn response_views_preserve_status_opaque_bytes_and_media_flag() {
    let binary = response(201, &[0xff, 0, 0x80], false);
    assert_eq!(binary.status(), 201);
    assert_eq!(binary.response_status(), ResponseStatus(201));
    assert_eq!(binary.response_status().to_string(), "201");
    assert_eq!(binary.body(), &[0xff, 0, 0x80]);
    assert!(!binary.has_json_media_type());
    assert!(response(200, b"{}", true).has_json_media_type());
}

#[test]
fn empty_capture_retains_the_response_without_fabricated_headers() -> crate::Result<()> {
    let observation = observed(&HeaderMap::new())?;
    assert_eq!(observation.header_count(), 0);
    assert_eq!(observation.headers().len(), 0);
    assert!(observation.headers().next().is_none());
    assert_eq!(observation.response().body(), &[0xff, 0, 0x80]);
    Ok(())
}

#[test]
fn captured_fields_own_empty_duplicate_and_comma_values() -> crate::Result<()> {
    let (captured, expected) = {
        let mut headers = HeaderMap::new();
        for value in [b"a".as_slice(), b"", b"b", b"a", b"a,b"] {
            append(&mut headers, "X-CaSe", value)?;
        }
        let expected = headers
            .iter()
            .map(|(name, value)| {
                std::str::from_utf8(value.as_bytes())
                    .map(|text| (name.as_str().to_owned(), text.to_owned()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        (ObservedResponseHeaders::capture(&headers)?, expected)
    };
    let observation = HttpObservationWithHeaders::new(response(200, b"opaque", false), captured);
    assert_eq!(observation.header_count(), 5);
    assert_eq!(observation.headers().len(), 5);
    for _ in 0..2 {
        let fields = observation
            .headers()
            .map(|field| (field.name, field.value))
            .collect::<Vec<_>>();
        let expected_fields = expected
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect::<Vec<_>>();
        assert_eq!(fields, expected_fields);
        assert!(observation.headers().all(|field| field.name == "x-case"));
        let mut values = observation
            .headers()
            .map(|field| field.value)
            .collect::<Vec<_>>();
        values.sort_unstable();
        assert_eq!(values, ["", "a", "a", "a,b", "b"]);
    }
    assert_eq!(observation.response().body(), b"opaque");
    Ok(())
}

#[test]
fn occurrence_limit_counts_duplicate_values_at_64_and_65() -> crate::Result<()> {
    for count in [64_usize, 65] {
        let mut headers = HeaderMap::new();
        for _ in 0..count {
            append(&mut headers, "x", b"a")?;
        }
        let result = observed(&headers);
        if count == 64 {
            let observation = result?;
            assert_eq!(observation.header_count(), 64);
            assert!(observation.headers().all(|field| field.value == "a"));
        } else {
            assert!(matches!(result, Err(AttemptFailure::HeaderLimit)));
        }
    }
    Ok(())
}

#[test]
fn single_field_budget_includes_its_name_at_65536_bytes() -> crate::Result<()> {
    for (length, accepted) in [(65_535_usize, true), (65_536, false)] {
        let mut headers = HeaderMap::new();
        append(&mut headers, "x", &vec![b'a'; length])?;
        let result = observed(&headers);
        if accepted {
            let observation = result?;
            assert_eq!(observation.header_count(), 1);
            assert_eq!(
                observation.headers().next().map(|field| field.value.len()),
                Some(length)
            );
        } else {
            assert!(matches!(result, Err(AttemptFailure::HeaderLimit)));
        }
    }
    Ok(())
}

#[test]
fn aggregate_budget_includes_names_and_every_duplicate_value() -> crate::Result<()> {
    for (second_length, accepted) in [(32_767_usize, true), (32_768, false)] {
        let mut headers = HeaderMap::new();
        append(&mut headers, "x", &vec![b'a'; 32_767])?;
        append(&mut headers, "x", &vec![b'b'; second_length])?;
        let result = observed(&headers);
        if accepted {
            let observation = result?;
            assert_eq!(observation.header_count(), 2);
            assert_eq!(
                observation
                    .headers()
                    .map(|field| field.value.len())
                    .sum::<usize>(),
                65_534
            );
        } else {
            assert!(matches!(result, Err(AttemptFailure::HeaderLimit)));
        }
    }
    Ok(())
}

#[test]
fn supported_whitespace_is_preserved_and_invalid_text_rejects_whole_capture() -> crate::Result<()> {
    let mut headers = HeaderMap::new();
    append(&mut headers, "x", b"\t a,b \t")?;
    let observation = observed(&headers)?;
    assert_eq!(
        observation.headers().next().map(|field| field.value),
        Some("\t a,b \t")
    );
    append(&mut headers, "x", &[0x80])?;
    assert!(matches!(observed(&headers), Err(AttemptFailure::Transport)));
    Ok(())
}

#[test]
fn workflow_projection_uses_observed_status_media_and_body_in_order() -> crate::Result<()> {
    use crate::actions::project_workflow_count;
    use crate::{DecodeLimits, WorkflowObservationFailure};
    let limits = DecodeLimits::new(1024, 8, 128, 128)?;
    assert!(matches!(
        project_workflow_count(&response(201, b"not-json", false), limits),
        Err(WorkflowObservationFailure::Status(ResponseStatus(201)))
    ));
    assert!(matches!(
        project_workflow_count(&response(200, b"not-json", false), limits),
        Err(WorkflowObservationFailure::MediaType)
    ));
    assert_eq!(
        project_workflow_count(
            &response(200, br#"{"total_count":3,"workflows":[]}"#, true),
            limits,
        )?
        .get(),
        3
    );
    Ok(())
}
