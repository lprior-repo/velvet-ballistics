fn stale_rejection(
    fixture: &GitFixture,
    measured: &str,
    head: &str,
) -> crate::benchmark_policy::error::Result<String> {
    match validate_evidence_freshness(fixture.path(), measured, head, 1) {
        Ok(()) => crate::benchmark_policy::error::bail!("expected a staleness rejection"),
        Err(error) => Ok(error.to_string()),
    }
}
