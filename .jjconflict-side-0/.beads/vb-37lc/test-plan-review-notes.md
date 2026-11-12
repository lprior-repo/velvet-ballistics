# Test Plan Repair Notes: vb-37lc

STATUS: REPAIRED

Changes made to `test-plan.md` only:
- Raised behavior inventory to 52 and unit-density register to 42 named unit tests for 7 public contract functions (6.0x, above 5x floor).
- Added explicit BDD scenario `validate_scan_config_returns_pattern_compilation_failed_when_scan_pattern_is_invalid` asserting `Err(NamingScanError::PatternCompilationFailed { pattern, source })`.
- Split collapsed config rejection coverage into deletion-resistant named tests: empty config, missing product, missing crate/module, missing language version, one-below kind count, duplicate kind, one-above kind count, contradictory token, wildcard allowlist, prefix-only allowlist, substring allowlist, and invalid pattern.
- Replaced vague “public equivalent” wording for occurrence classes and scan report success with exact expected enum/value shapes and exact report fields.
- Expanded mutation checkpoints and boundary matrices for argument swaps, `Ok(Default::default())`, wrong remediation, skipped/included path classes, wrong error variants, and dropped sorting.

No implementation code, test code, commits, pushes, bead status changes, or closure actions were performed.
