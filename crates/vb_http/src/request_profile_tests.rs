use super::*;

#[test]
fn github_profile_has_exact_finite_wire_and_projection_constants() {
    let expected = RequestProfile {
        accept: "application/vnd.github+json",
        user_agent: "velvet-ballistics/0.1",
        api_version: Some("2026-03-10"),
        projection_schema: "github-workflow-count-i64-v1",
    };
    assert_eq!(GITHUB, expected);
    assert_eq!(for_media(crate::JsonMediaType::GitHub), expected);
}

#[test]
fn profile_serialization_has_a_literal_deterministic_wire_identity() -> crate::Result<()> {
    let expected: &[u8] = b"\x1bapplication/vnd.github+json\x15velvet-ballistics/0.1\x01\x0a2026-03-10\x1cgithub-workflow-count-i64-v1";
    for _ in 0..4 {
        assert_eq!(postcard::to_allocvec(&GITHUB)?, expected);
        assert_eq!(postcard::to_allocvec(&for_media(crate::JsonMediaType::GitHub))?, expected);
    }
    assert_eq!(postcard::to_allocvec(&crate::JsonMediaType::GitHub)?.as_slice(), &[0]);
    Ok(())
}

#[test]
fn every_wire_or_projection_dimension_separates_serialized_identity() -> crate::Result<()> {
    let profiles = [
        GITHUB,
        RequestProfile { accept: "application/json", ..GITHUB },
        RequestProfile { user_agent: "velvet-ballistics/0.2", ..GITHUB },
        RequestProfile { api_version: None, ..GITHUB },
        RequestProfile { api_version: Some("2026-03-11"), ..GITHUB },
        RequestProfile { projection_schema: "github-workflow-count-i64-v2", ..GITHUB },
    ];
    for (left_index, left) in profiles.iter().enumerate() {
        for (right_index, right) in profiles.iter().enumerate() {
            let left_bytes = postcard::to_allocvec(left)?;
            let right_bytes = postcard::to_allocvec(right)?;
            if left_index == right_index {
                assert_eq!(left_bytes, right_bytes);
            } else {
                assert_ne!(left_bytes, right_bytes, "profile dimensions {left_index}/{right_index}");
            }
        }
    }
    Ok(())
}
