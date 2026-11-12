# Error Variant Completeness Audit

The root integration test `tests/error_variant_completeness_test.rs` is the executable audit for public `*Error` enums.

Coverage policy:

- Public diagnostic errors must be constructed variant-by-variant and checked for exact, unique diagnostic codes.
- Public constructible non-diagnostic errors must be constructed and passed through exhaustive no-wildcard matches.
- External wrapper variants are still covered by exhaustive matches, but are not all synthetically constructed when their wrapped third-party error has no stable public constructor.

Documented external-wrapper exceptions:

- `CompileError::Parse`, wrapping `saphyr::ScanError`.
- `JournalError::Fjall`, wrapping `fjall::Error`.
- `JournalError::Encode`, wrapping `postcard::Error`.
- Wrapper variants over project-local errors are constructed with a stable local sample.
