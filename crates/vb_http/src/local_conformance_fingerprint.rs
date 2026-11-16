use super::{CaseFailure, SCHEMA};

pub(super) fn source_fingerprint() -> Result<[u8; 32], CaseFailure> {
    fingerprint(SCHEMA, SOURCE_FILES)
}

pub(super) fn fingerprint(
    schema: &str,
    sources: &[(&str, &[u8])],
) -> Result<[u8; 32], CaseFailure> {
    let mut hash = blake3::Hasher::new();
    hash_part(&mut hash, schema.as_bytes())?;
    for (path, bytes) in sources {
        hash_part(&mut hash, path.as_bytes())?;
        hash_part(&mut hash, bytes)?;
    }
    Ok(hash.finalize().into())
}

fn hash_part(hash: &mut blake3::Hasher, bytes: &[u8]) -> Result<(), CaseFailure> {
    let length =
        u64::try_from(bytes.len()).map_err(|_| CaseFailure::Check("source fingerprint length"))?;
    hash.update(&length.to_le_bytes());
    hash.update(bytes);
    Ok(())
}

pub(super) const SOURCE_FILES: &[(&str, &[u8])] = &[
    ("actions.rs", include_bytes!("actions.rs")),
    ("decode/error.rs", include_bytes!("decode/error.rs")),
    ("decode/limits.rs", include_bytes!("decode/limits.rs")),
    ("decode/number.rs", include_bytes!("decode/number.rs")),
    ("decode/parser.rs", include_bytes!("decode/parser.rs")),
    ("decode/string.rs", include_bytes!("decode/string.rs")),
    ("decode.rs", include_bytes!("decode.rs")),
    ("error.rs", include_bytes!("error.rs")),
    ("github.rs", include_bytes!("github.rs")),
    ("hurl.rs", include_bytes!("hurl.rs")),
    ("hurl_evaluation.rs", include_bytes!("hurl_evaluation.rs")),
    ("hurl_headers.rs", include_bytes!("hurl_headers.rs")),
    ("hurl_integer.rs", include_bytes!("hurl_integer.rs")),
    (
        "hurl_predicate_transport.rs",
        include_bytes!("hurl_predicate_transport.rs"),
    ),
    ("hurl_syntax.rs", include_bytes!("hurl_syntax.rs")),
    ("hurl_transport.rs", include_bytes!("hurl_transport.rs")),
    ("lib.rs", include_bytes!("lib.rs")),
    ("limits.rs", include_bytes!("limits.rs")),
    ("observation.rs", include_bytes!("observation.rs")),
    ("request.rs", include_bytes!("request.rs")),
    ("request_profile.rs", include_bytes!("request_profile.rs")),
    ("transport.rs", include_bytes!("transport.rs")),
    ("workflows.rs", include_bytes!("workflows.rs")),
    ("Cargo.toml", include_bytes!("../Cargo.toml")),
    (
        "local_conformance.rs",
        include_bytes!("local_conformance.rs"),
    ),
    (
        "local_conformance_cases.rs",
        include_bytes!("local_conformance_cases.rs"),
    ),
    (
        "local_conformance_io.rs",
        include_bytes!("local_conformance_io.rs"),
    ),
    (
        "local_conformance_checks.rs",
        include_bytes!("local_conformance_checks.rs"),
    ),
    (
        "local_conformance_suite.rs",
        include_bytes!("local_conformance_suite.rs"),
    ),
    (
        "local_conformance_fingerprint.rs",
        include_bytes!("local_conformance_fingerprint.rs"),
    ),
    ("transport/body.rs", include_bytes!("transport/body.rs")),
    (
        "local_conformance_cases_tests.rs",
        include_bytes!("local_conformance_cases_tests.rs"),
    ),
];
