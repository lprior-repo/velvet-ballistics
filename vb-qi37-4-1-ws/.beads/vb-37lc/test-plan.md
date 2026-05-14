# Test Plan: vb-37lc Canonical Spelling Scan

## Summary
- Behaviors identified: 52
- Public contract functions: 7; required unit-density floor: 35 named unit tests; planned unit-density count: 42 named unit tests (6.0x public function count)
- Trophy allocation: 42 unit / 20 integration / 3 E2E-acceptance / 3 static
- Proptest invariants: 7
- Fuzz targets: 3
- Kani harnesses: 4
- Required mutation threshold: >=90% killed mutants, with zero surviving mutants for canonical table entries, allowlist branches, error mapping, finding ordering, and gate failure behavior.
- Red phase expectation: every named scenario below must be implemented as a failing test or failing verification harness before production implementation is added. Failures must be specific value mismatches or exact `NamingScanError` variants, never bare `is_ok()` or `is_err()`.

## 1. Behavior Inventory

1. Canonical spelling table returns exact product, binary, package, and bead rig token when loaded.
2. Canonical spelling table returns exact crate/module and bead database token when loaded.
3. Canonical spelling table returns exact language version token when loaded.
4. Scan config accepts complete canonical spelling table and exact documented allowlist when validation runs.
5. Scan config rejects missing canonical entries when validation runs.
6. Scan config rejects duplicated, contradictory, or broad wildcard entries when validation runs.
7. Scan root rejects missing, non-directory, symlink-escaped, or external paths when scan starts.
8. File discovery includes source, docs, manifests, scripts, and configured bead references when deterministic discovery runs.
9. File discovery excludes VCS internals, build outputs, binary blobs, embedded database state, and generated lock/runtime artifacts when deterministic discovery runs.
10. File discovery reports `NamingScanError::FileDiscoveryFailed` when traversal cannot complete.
11. Occurrence classifier accepts canonical `velvet-ballastics` spelling in product, binary, package, and bead rig contexts when text is scanned.
12. Occurrence classifier accepts canonical `velvet_ballastics` spelling in crate/module and bead database contexts when text is scanned.
13. Occurrence classifier accepts canonical `velvet-ballastics/v1` spelling in language-version contexts when text is scanned.
14. Occurrence classifier accepts legacy repository path exception only when the occurrence is the current external repository path.
15. Occurrence classifier accepts legacy master filename exception only when the occurrence is the current master filename.
16. Occurrence classifier accepts legacy migration reference only when explicitly labeled as a migration reference to a pre-existing external artifact.
17. Occurrence classifier rejects neighboring paths, substrings, generated names, and unrelated legacy text when they resemble allowed exceptions.
18. File scanner returns exact findings with path, one-based line, one-based column, spelling class, and remediation when invalid occurrences exist.
19. File scanner reports every invalid occurrence on a line when multiple invalid spellings appear on the same line.
20. File scanner reports column `1` when an invalid occurrence begins at the first text column.
21. File scanner returns `NamingScanError::InputReadFailed` when a selected input is unreadable or undecodable as supported text.
22. Repository scanner returns `Ok(ScanReport)` with zero findings when all selected inputs contain only canonical names or documented legacy exceptions.
23. Repository scanner returns `NamingScanError::InvalidCanonicalSpelling { findings }` with nonempty exact findings when any disallowed spelling exists.
24. Repository scanner produces identical report data for identical contents and configuration regardless of filesystem traversal order.
25. Repository scanner never modifies repository files, bead records, manifests, scripts, generated artifacts, or reports unless an explicit shell-layer report destination is configured.
26. Report renderer emits findings in stable path, line, column, spelling-class order when rendering a report.
27. Report renderer returns `NamingScanError::ReportWriteFailed` when a configured report destination cannot be written.
28. Quality gate blocks canonical quality flow when invalid naming fixture is present.
29. Static governance rejects scan implementation changes that introduce runtime-core YAML/JSON/HTTP dependencies, unsafe, unwrap, expect, panic, todo, unimplemented, dbg, unchecked indexing/slicing/casts/arithmetic, or ignored `Result`.
30. Config validation rejects an empty config when validation runs.
31. Config validation accepts the minimum valid config when exactly required canonical entries and exact exceptions are present.
32. Config validation accepts maximum bounded config when optional documented scan rules are present without broadening allowlist semantics.
33. Config validation rejects one-below required canonical kind count when any required kind is absent.
34. Config validation rejects one-above required canonical kind count when duplicate kind entries are present.
35. Config validation rejects contradictory canonical token for a required kind when validation runs.
36. Config validation rejects wildcard allowlist when validation runs.
37. Config validation rejects prefix-only allowlist when validation runs.
38. Config validation rejects substring allowlist when validation runs.
39. Config validation returns `NamingScanError::PatternCompilationFailed` when a configured scan pattern cannot compile.
40. Input discovery returns an empty eligible input set when a repository has no eligible text surfaces.
41. Input discovery returns exactly one input when repository has one minimum eligible file.
42. Input discovery rejects symlink loop or symlink escape when discovery runs.
43. Input discovery preserves deterministic order for maximum bounded tree when discovery runs.
44. Occurrence classification rejects empty text as no occurrence when classification is requested.
45. Occurrence classification handles token at start and token at end with exact one-based columns when text is scanned.
46. Occurrence classification rejects case variants and Unicode confusables when they resemble canonical spellings.
47. File scanner returns zero findings for empty supported file when scanned.
48. File scanner preserves exact columns for CRLF and final line without newline when invalid occurrence is present.
49. File scanner handles maximum bounded line length and occurrence count without truncating findings.
50. Repository scanner returns zero-finding report with exact selected input count when selected input set is empty.
51. Repository scanner reports all-invalid and mixed valid/invalid repositories with exact nonempty finding sets.
52. Report renderer handles empty report, single finding, duplicate sort keys, maximum bounded findings, non-ASCII paths, missing parent destination, and permission-denied destination with exact output or exact error.

## 2. Trophy Allocation

| # | Behavior | Layer | Tool | Rationale |
|---|----------|-------|------|-----------|
| 1 | Product/binary/package/rig exact token | Unit + proof | `#[test]`, Lean | Pure table value; assert exact `velvet-ballastics`. |
| 2 | Crate/module/database exact token | Unit + proof | `#[test]`, Lean | Pure table value; assert exact `velvet_ballastics`. |
| 3 | Language-version exact token | Unit + proof | `#[test]`, Lean | Pure table value; assert exact `velvet-ballastics/v1`. |
| 4 | Complete config accepted | Unit | `#[test]`, proptest | Pure validation with exact accepted `ScanConfig` contents. |
| 5 | Missing entries rejected | Unit | `#[test]`, proptest | Exhaustive malformed config behavior. |
| 6 | Duplicate/contradictory/wildcard rejected | Unit | `#[test]`, proptest, cargo-mutants | Pure validation; mutation must kill relaxed predicates. |
| 7 | Invalid root rejected | Integration | `/tests/`, manual QA | Uses real filesystem path resolution and workspace boundary. |
| 8 | Source/doc/manifest/script/bead inputs included | Integration | `/tests/` with temp repo | Discovery is boundary behavior with real filesystem. |
| 9 | Non-source surfaces excluded | Integration | `/tests/` with temp repo | Discovery path rules must be tested through real directory tree. |
| 10 | Discovery failure typed | Integration | `/tests/`, manual QA | OS behavior; use real permissions or broken traversal fixture. |
| 11 | Product spelling accepted | Unit | `#[test]` | Pure classifier exact-value behavior. |
| 12 | Crate spelling accepted | Unit | `#[test]` | Pure classifier exact-value behavior. |
| 13 | Language version accepted | Unit | `#[test]` | Pure classifier exact-value behavior. |
| 14 | Repository path exception accepted | Unit + proof | `#[test]`, Lean | Pure allowlist predicate; exact exception only. |
| 15 | Master filename exception accepted | Unit + proof | `#[test]`, Lean | Pure allowlist predicate; exact exception only. |
| 16 | Migration reference accepted | Unit | `#[test]` | Pure allowlist predicate with explicit context. |
| 17 | Exception-like text rejected | Unit + proptest | `#[test]`, proptest, Lean | Boundary predicate must not generalize. |
| 18 | Exact finding fields emitted | Integration | `/tests/` | Public file scan behavior with real text input. |
| 19 | Multiple occurrences emitted | Integration | `/tests/` | Behavior crosses tokenizer/classifier/report finding construction. |
| 20 | First-column location emitted | Unit + integration | `#[test]`, `/tests/` | Arithmetic boundary plus file-level confirmation. |
| 21 | Unreadable/undecodable input typed | Integration + fuzz | `/tests/`, cargo-fuzz | Real file/byte boundary and hostile bytes. |
| 22 | Canonical-only repository passes | Integration + proptest | `/tests/`, proptest | Full scan behavior over real repo-like fixture. |
| 23 | Invalid spelling fails closed | Integration + mutation | `/tests/`, cargo-mutants | Primary quality behavior; must test through public scan. |
| 24 | Traversal-order reproducible | Integration + proptest + proof | `/tests/`, proptest, Lean | Determinism guarantee over component composition. |
| 25 | Scan is read-only | E2E/manual | `git diff`, manual QA | Observable repository side-effect behavior. |
| 26 | Rendered order stable | Integration + proof | `/tests/`, Lean | Public rendered report semantics, not internal sort implementation. |
| 27 | Report write failure typed | Integration/manual | `/tests/`, manual QA | Shell-layer filesystem write failure. |
| 28 | Quality gate blocks invalid naming | E2E | `moon run :verify-fast`, `moon run :verify-all` | User-facing quality-lane behavior. |
| 29 | Static governance rejects forbidden constructs/deps | Static | `moon run :verify-fast`, clippy/static scan | Compile/lint/static policy behavior. |

Allocation target: integration remains widest for end-to-end behavioral confidence, but unit density is deliberately raised above 5x public function count to make pure config/table/classifier/sort/report-data branches deletion-resistant.

### Unit Density Register — 42 Named Unit Tests Required

The test writer must implement these as unit tests against public Calc/kernel APIs or approved public constructors. These do not replace integration tests.

#### `canonical_spelling_table()` — 6 unit tests
1. `canonical_table_returns_ballastics_for_product_when_loaded`
2. `canonical_table_returns_ballastics_for_binary_when_loaded`
3. `canonical_table_returns_ballastics_for_package_when_loaded`
4. `canonical_table_returns_ballastics_for_bead_rig_when_loaded`
5. `canonical_table_returns_underscore_ballastics_for_crate_module_and_database_when_loaded`
6. `canonical_table_returns_ballastics_v1_when_language_version_is_loaded`

#### `validate_scan_config(config)` — 14 unit tests
7. `validate_scan_config_returns_scan_config_when_minimum_valid_config_is_supplied`
8. `validate_scan_config_returns_scan_config_when_maximum_bounded_valid_config_is_supplied`
9. `validate_scan_config_returns_invalid_configuration_when_config_is_empty`
10. `validate_scan_config_returns_invalid_configuration_when_product_kind_is_missing`
11. `validate_scan_config_returns_invalid_configuration_when_crate_module_kind_is_missing`
12. `validate_scan_config_returns_invalid_configuration_when_language_version_kind_is_missing`
13. `validate_scan_config_returns_invalid_configuration_when_kind_count_is_one_below_required`
14. `validate_scan_config_returns_invalid_configuration_when_kind_is_duplicated`
15. `validate_scan_config_returns_invalid_configuration_when_kind_count_is_one_above_required`
16. `validate_scan_config_returns_invalid_configuration_when_canonical_token_contradicts_kind`
17. `validate_scan_config_returns_invalid_configuration_when_allowlist_contains_wildcard`
18. `validate_scan_config_returns_invalid_configuration_when_allowlist_contains_prefix_only_rule`
19. `validate_scan_config_returns_invalid_configuration_when_allowlist_contains_substring_rule`
20. `validate_scan_config_returns_pattern_compilation_failed_when_scan_pattern_is_invalid`

#### `classify_occurrence(path, line, column, text, config)` — 12 unit tests
21. `classify_occurrence_returns_canonical_product_when_ballastics_product_token_is_seen`
22. `classify_occurrence_returns_canonical_crate_module_when_underscore_ballastics_token_is_seen`
23. `classify_occurrence_returns_canonical_language_version_when_v1_token_is_seen`
24. `classify_occurrence_returns_allowed_repository_path_with_exact_payload_when_current_external_repository_path_is_seen`
25. `classify_occurrence_returns_allowed_master_filename_with_exact_payload_when_current_master_filename_is_seen`
26. `classify_occurrence_returns_allowed_migration_reference_with_exact_payload_when_explicit_migration_reference_is_seen`
27. `classify_occurrence_returns_invalid_legacy_when_repository_path_is_only_a_substring`
28. `classify_occurrence_returns_invalid_legacy_when_master_filename_is_embedded_in_unrelated_path`
29. `classify_occurrence_returns_invalid_legacy_when_migration_label_is_absent`
30. `classify_occurrence_returns_no_occurrence_when_text_is_empty`
31. `classify_occurrence_returns_invalid_legacy_when_case_variant_is_seen`
32. `classify_occurrence_returns_invalid_legacy_when_unicode_confusable_is_seen`

#### `scan_file(input, config)` pure data/location kernel — 4 unit tests
33. `scan_file_location_kernel_returns_column_one_when_token_starts_at_first_column`
34. `scan_file_location_kernel_preserves_crlf_columns_when_invalid_token_is_seen`
35. `scan_file_location_kernel_preserves_final_line_without_newline_when_invalid_token_is_seen`
36. `scan_file_finding_kernel_returns_all_occurrences_when_many_tokens_share_one_line`

#### `scan_repository(root, config)` report assembly kernel — 3 unit tests
37. `scan_repository_report_kernel_returns_zero_findings_with_exact_input_count_when_inputs_are_empty`
38. `scan_repository_report_kernel_preserves_config_identity_when_report_is_successful`
39. `scan_repository_report_kernel_returns_all_findings_when_valid_and_invalid_inputs_are_mixed`

#### `render_scan_report(report)` sort/render kernel — 3 unit tests
40. `render_scan_report_returns_empty_body_when_report_has_zero_findings`
41. `render_scan_report_preserves_single_finding_fields_when_report_has_one_finding`
42. `render_scan_report_orders_duplicate_sort_keys_deterministically_when_findings_have_equal_path_line_column`

## 3. BDD Scenarios

### Behavior 1: Canonical table exposes product, binary, package, and bead rig spelling
Test function: `fn canonical_table_returns_ballastics_for_product_binary_package_and_rig_when_loaded()`

Given: the default canonical spelling table.
When: product, binary, package, and bead rig entries are read through the public table API.
Then: each entry equals exactly `velvet-ballastics`.
And: the table contains no `velvet-ballistics` value for these kinds.

### Behavior 2: Canonical table exposes crate/module and database spelling
Test function: `fn canonical_table_returns_underscore_ballastics_for_crate_module_and_database_when_loaded()`

Given: the default canonical spelling table.
When: crate/module and bead database entries are read through the public table API.
Then: each entry equals exactly `velvet_ballastics`.
And: no hyphenated token is returned for these kinds.

### Behavior 3: Canonical table exposes language-version spelling
Test function: `fn canonical_table_returns_ballastics_v1_when_language_version_is_loaded()`

Given: the default canonical spelling table.
When: the language-version entry is read through the public table API.
Then: the entry equals exactly `velvet-ballastics/v1`.

### Behavior 4: Complete scan config validates
Test functions:
- `fn validate_scan_config_returns_scan_config_when_minimum_valid_config_is_supplied()`
- `fn validate_scan_config_returns_scan_config_when_maximum_bounded_valid_config_is_supplied()`

Given: `RawScanConfig` contains every required canonical name kind and only the three documented legacy exception classes.
When: `validate_scan_config` is called.
Then: it returns `Ok(ScanConfig)` whose canonical table entries equal the exact tokens above and whose allowlist contains exactly repository path, master filename, and explicit migration reference predicates.
And: the returned config preserves exact pattern set, excluded path rules, and allowlist payload values; `Ok(Default::default())` is not an acceptable result.

### Behavior 5: Empty and missing canonical entries are rejected
Test functions:
- `fn validate_scan_config_returns_invalid_configuration_when_config_is_empty()`
- `fn validate_scan_config_returns_invalid_configuration_when_product_kind_is_missing()`
- `fn validate_scan_config_returns_invalid_configuration_when_crate_module_kind_is_missing()`
- `fn validate_scan_config_returns_invalid_configuration_when_language_version_kind_is_missing()`
- `fn validate_scan_config_returns_invalid_configuration_when_kind_count_is_one_below_required()`

Given: `RawScanConfig` is empty or omits exactly one required canonical name kind.
When: `validate_scan_config` is called.
Then: it returns `Err(NamingScanError::InvalidConfiguration { reason })` where `reason` identifies the exact missing kind or empty config defect.

### Behavior 6: Duplicate canonical kind is rejected
Test function: `fn validate_scan_config_returns_invalid_configuration_when_kind_is_duplicated()`

Given: `RawScanConfig` contains two entries for the same required canonical kind with the same token.
When: `validate_scan_config` is called.
Then: it returns `Err(NamingScanError::InvalidConfiguration { reason })` where `reason` identifies the duplicated kind.

### Behavior 7: One-above required kind count is rejected
Test function: `fn validate_scan_config_returns_invalid_configuration_when_kind_count_is_one_above_required()`

Given: `RawScanConfig` contains all required canonical kinds plus one duplicate extra kind.
When: `validate_scan_config` is called.
Then: it returns `Err(NamingScanError::InvalidConfiguration { reason })` where `reason` identifies the one-above/duplicate kind count.

### Behavior 8: Contradictory canonical kind is rejected
Test function: `fn validate_scan_config_returns_invalid_configuration_when_canonical_token_contradicts_kind()`

Given: `RawScanConfig` maps a required canonical kind to a token that belongs to a different kind or to legacy spelling.
When: `validate_scan_config` is called.
Then: it returns `Err(NamingScanError::InvalidConfiguration { reason })` where `reason` identifies the contradictory kind and token.

### Behavior 9: Wildcard allowlist is rejected
Test function: `fn validate_scan_config_returns_invalid_configuration_when_allowlist_contains_wildcard()`

Given: `RawScanConfig` contains `*` or equivalent match-all allowlist entry.
When: `validate_scan_config` is called.
Then: it returns `Err(NamingScanError::InvalidConfiguration { reason })` where `reason` identifies the broad wildcard.

### Behavior 10: Prefix-only allowlist is rejected
Test function: `fn validate_scan_config_returns_invalid_configuration_when_allowlist_contains_prefix_only_rule()`

Given: `RawScanConfig` contains an allowlist entry that allows any path or text by prefix.
When: `validate_scan_config` is called.
Then: it returns `Err(NamingScanError::InvalidConfiguration { reason })` where `reason` identifies the prefix-only allowlist rule.

### Behavior 11: Substring allowlist is rejected
Test function: `fn validate_scan_config_returns_invalid_configuration_when_allowlist_contains_substring_rule()`

Given: `RawScanConfig` contains an allowlist entry that allows legacy spelling by substring containment.
When: `validate_scan_config` is called.
Then: it returns `Err(NamingScanError::InvalidConfiguration { reason })` where `reason` identifies the substring allowlist rule.

### Behavior 12: Invalid scan pattern maps to PatternCompilationFailed
Test function: `fn validate_scan_config_returns_pattern_compilation_failed_when_scan_pattern_is_invalid()`

Given: `RawScanConfig` contains the complete canonical table and exact allowlist, but one configured scan pattern is syntactically invalid.
When: `validate_scan_config` compiles scan patterns.
Then: it returns `Err(NamingScanError::PatternCompilationFailed { pattern, source })` where `pattern` equals the invalid pattern string and `source` is the pattern compiler error.

### Behavior 13: Broad or contradictory config branch coverage remains deletion-resistant
Test functions:
- `fn validate_scan_config_rejects_each_malformed_config_branch_independently()`

Given: a table-driven list of exactly one malformed branch per case: empty config, missing kind, duplicate kind, contradictory token, wildcard, prefix-only allowlist, substring allowlist, and invalid pattern.
When: `validate_scan_config` is called.
Then: each case returns its exact expected `NamingScanError` variant and reason/pattern; deleting any single branch causes at least one named case to fail.

### Behavior 14: Invalid root fails closed
Test function: `fn scan_repository_returns_invalid_root_when_root_is_missing_or_outside_workspace()`

Given: a root path that does not exist, is not a directory, or resolves outside the active workspace.
When: repository scanning or input discovery starts.
Then: it returns `Err(NamingScanError::InvalidRoot { root })` with the rejected root path.

### Behavior 15: Deterministic discovery includes source surfaces
Test function: `fn discover_scan_inputs_returns_source_docs_manifests_scripts_and_configured_beads_when_present()`

Given: a temp repository contains files under `crates/`, `docs/`, manifests such as `Cargo.toml`/Moon config, scripts, and configured bead reference files.
When: `discover_scan_inputs` runs with the default config.
Then: it returns a path list containing exactly those eligible paths, normalized relative to the repository root.

### Behavior 16: Deterministic discovery excludes non-source surfaces
Test function: `fn discover_scan_inputs_excludes_vcs_build_binary_embedded_database_and_runtime_artifacts_when_present()`

Given: a temp repository contains `.git/`, `target/`, binary blobs, `.beads/dolt`, `.beads/backup`, `.beads/embeddeddolt`, locks, runtime database state, and generated artifacts matching documented exclusions.
When: `discover_scan_inputs` runs.
Then: none of those excluded paths appear in the returned `Vec<ScanInput>`.
And: eligible sibling source files still appear.

### Behavior 17: Discovery failure maps to typed error
Test function: `fn discover_scan_inputs_returns_file_discovery_failed_when_traversal_cannot_complete()`

Given: a selected directory tree cannot be traversed due to permission or filesystem error.
When: `discover_scan_inputs` runs.
Then: it returns `Err(NamingScanError::FileDiscoveryFailed { path, source })` with the failing path.

### Behavior 18: Product-class canonical spelling is accepted
Test function: `fn classify_occurrence_returns_canonical_product_when_ballastics_product_token_is_seen()`

Given: text contains `velvet-ballastics` in a product, binary, package, or bead rig context.
When: `classify_occurrence` evaluates the occurrence.
Then: it returns exactly `Ok(OccurrenceClass::CanonicalProduct { canonical: "velvet-ballastics", kind: CanonicalNameKind::ProductBinaryPackageRig })`.

### Behavior 19: Crate-class canonical spelling is accepted
Test function: `fn classify_occurrence_returns_canonical_crate_when_underscore_ballastics_token_is_seen()`

Given: text contains `velvet_ballastics` in a crate/module or bead database context.
When: `classify_occurrence` evaluates the occurrence.
Then: it returns exactly `Ok(OccurrenceClass::CanonicalCrateModule { canonical: "velvet_ballastics", kind: CanonicalNameKind::CrateModuleDatabase })`.

### Behavior 20: Language-version canonical spelling is accepted
Test function: `fn classify_occurrence_returns_canonical_language_version_when_v1_token_is_seen()`

Given: text contains `velvet-ballastics/v1` in a language-version context.
When: `classify_occurrence` evaluates the occurrence.
Then: it returns exactly `Ok(OccurrenceClass::CanonicalLanguageVersion { canonical: "velvet-ballastics/v1", kind: CanonicalNameKind::LanguageVersion })`.

### Behavior 21: Repository path exception is exact
Test function: `fn classify_occurrence_returns_allowed_legacy_when_current_external_repository_path_is_seen()`

Given: text contains the configured current external repository path as the complete occurrence.
When: `classify_occurrence` evaluates it.
Then: it returns exactly `Ok(OccurrenceClass::AllowedLegacy { exception: LegacyException::RepositoryPath { path: configured_current_external_repository_path } })`.
And: the payload path equals the configured current external repository path byte-for-byte.

### Behavior 22: Master filename exception is exact
Test function: `fn classify_occurrence_returns_allowed_legacy_when_current_master_filename_is_seen()`

Given: text contains the current master filename as the complete occurrence.
When: `classify_occurrence` evaluates it.
Then: it returns exactly `Ok(OccurrenceClass::AllowedLegacy { exception: LegacyException::MasterFilename { filename: configured_current_master_filename } })`.
And: the payload filename equals the configured master filename byte-for-byte.

### Behavior 23: Explicit migration reference exception is exact
Test function: `fn classify_occurrence_returns_allowed_legacy_when_explicit_migration_reference_is_seen()`

Given: text contains legacy spelling only inside an explicitly labeled migration reference to a pre-existing external artifact.
When: `classify_occurrence` evaluates it.
Then: it returns exactly `Ok(OccurrenceClass::AllowedLegacy { exception: LegacyException::MigrationReference { artifact, label, legacy_text } })`.
And: `artifact`, `label`, and `legacy_text` equal the fixture's explicit migration artifact id, migration label, and legacy occurrence text byte-for-byte.

### Behavior 24: Exception-like legacy spelling is rejected
Test function: `fn classify_occurrence_returns_invalid_legacy_when_exception_substring_appears_in_unrelated_text()`

Given: text contains a neighboring path, substring, generated name, or unrelated legacy spelling that merely contains an exception token.
When: `classify_occurrence` evaluates it.
Then: it returns exactly `Ok(OccurrenceClass::InvalidLegacy { spelling_class: SpellingClass::LegacyProjectSpelling, remediation: "velvet-ballastics" })` for product/package contexts, or the context-specific canonical remediation for crate/database/language-version contexts.

### Behavior 25: Findings contain exact fields
Test function: `fn scan_file_returns_exact_finding_fields_when_invalid_occurrence_is_present()`

Given: a selected text file `docs/naming.md` contains `velvet-ballistics` at line 3, column 7 in a non-exception context.
When: `scan_file` runs.
Then: it returns a `Vec<NamingFinding>` equal to one finding with path `docs/naming.md`, line `3`, column `7`, spelling class `SpellingClass::LegacyProjectSpelling`, and remediation equal to exact canonical replacement `velvet-ballastics` for product/package contexts or the exact context-specific token (`velvet_ballastics` / `velvet-ballastics/v1`) for crate/database/language-version contexts.

### Behavior 26: Multiple invalid occurrences are all reported
Test function: `fn scan_file_returns_all_findings_when_multiple_invalid_occurrences_share_one_line()`

Given: one line contains two disallowed legacy spellings at distinct columns.
When: `scan_file` runs.
Then: it returns exactly two findings with the same path and line and the exact two one-based columns.

### Behavior 27: First-column occurrence reports column one
Test function: `fn scan_file_returns_column_one_when_invalid_occurrence_starts_at_first_column()`

Given: a selected file starts a line with a disallowed spelling.
When: `scan_file` runs.
Then: the finding for that occurrence has `column == 1`.

### Behavior 28: Unreadable or undecodable input fails closed
Test function: `fn scan_file_returns_input_read_failed_when_selected_input_is_unreadable_or_undecodable()`

Given: `ScanInput` points to a selected file that cannot be read or decoded as supported text.
When: `scan_file` runs.
Then: it returns `Err(NamingScanError::InputReadFailed { path, source })` with the selected path.

### Behavior 29: Canonical-only repository passes
Test function: `fn scan_repository_returns_zero_finding_report_when_inputs_are_canonical_or_allowed_exceptions()`

Given: a repository fixture contains only `velvet-ballastics`, `velvet_ballastics`, `velvet-ballastics/v1`, and documented legacy exceptions.
When: `scan_repository` runs with complete config.
Then: it returns exactly `Ok(ScanReport { root, config_fingerprint, selected_input_count, scanned_text_input_count, findings: [] })` where `root` equals the fixture root, `config_fingerprint` equals the complete config fingerprint, `selected_input_count` equals the fixture's eligible input count, and `scanned_text_input_count` equals the fixture's supported text input count.
And: `Ok(Default::default())` fails because root, fingerprint, and counts would not match.

### Behavior 30: Invalid spelling fails closed
Test function: `fn scan_repository_returns_invalid_canonical_spelling_when_legacy_spelling_is_outside_allowlist()`

Given: an eligible source/doc/manifest/script/bead file contains legacy spelling outside migration context.
When: `scan_repository` runs.
Then: it returns `Err(NamingScanError::InvalidCanonicalSpelling { findings })` where `findings` is nonempty and contains the exact path, line, column, class, and remediation.

### Behavior 31: Reports are reproducible regardless of traversal order
Test function: `fn scan_repository_returns_identical_report_when_same_inputs_are_discovered_in_different_orders()`

Given: the same set of selected inputs is supplied in two different traversal orders.
When: repository scanning or report rendering completes for each order.
Then: both `ScanReport` values are exactly equal and rendered report bytes are exactly equal.

### Behavior 32: Scan is read-only
Test function: `fn scan_repository_leaves_git_diff_empty_when_scan_runs_on_repository()`

Given: a git repository fixture with committed contents.
When: the scan runs without an explicit report destination.
Then: `git diff --exit-code` returns exit code `0`.
And: file hashes before and after scan are identical for source, docs, manifests, scripts, bead records, and generated artifacts.

### Behavior 33: Rendered report order is stable
Test function: `fn render_scan_report_orders_findings_by_path_line_column_and_class_when_findings_are_unsorted()`

Given: `ScanReport` contains unsorted findings.
When: `render_scan_report` runs.
Then: rendered findings appear sorted by path, then one-based line, then one-based column, then spelling class.

### Behavior 34: Report write failure maps to typed error
Test function: `fn render_scan_report_returns_report_write_failed_when_destination_is_unwritable()`

Given: a configured report destination cannot be written.
When: the shell layer writes the rendered report.
Then: it returns `Err(NamingScanError::ReportWriteFailed { path, source })` with the destination path.

### Behavior 35: Quality gate blocks invalid naming
Test function: `fn moon_verify_fast_fails_when_invalid_naming_fixture_is_present()`

Given: the canonical spelling scan is wired into the fast quality lane and a fixture introduces invalid legacy spelling outside the allowlist.
When: `moon run :verify-fast` runs.
Then: the command exits nonzero and output includes the exact invalid path, line, column, spelling class, and remediation.
And: replacing the occurrence with the canonical spelling makes the same command exit `0`.

### Behavior 36: Static governance rejects forbidden constructs and dependencies
Test functions:
- `fn static_scan_reports_forbidden_construct_when_scan_source_contains_unwrap_or_panic_family()`
- `fn static_dependency_check_reports_runtime_core_yaml_json_or_http_when_added()`

Given: scan implementation source or dependency metadata includes a forbidden construct or runtime-core YAML/JSON/HTTP dependency.
When: `moon run :verify-fast` runs the static gates.
Then: it exits nonzero and reports the exact forbidden token or dependency name.

### Behavior 37: Empty repository surface returns exact zero-input report
Test function: `fn scan_repository_returns_zero_input_report_when_no_eligible_inputs_exist()`

Given: a repository root exists and discovery returns no eligible source/doc/manifest/script/bead files.
When: `scan_repository` runs with complete config.
Then: it returns exactly `Ok(ScanReport { root, config_fingerprint, selected_input_count: 0, scanned_text_input_count: 0, findings: [] })`.

### Behavior 38: Minimum discovery surface returns exactly one input
Test function: `fn discover_scan_inputs_returns_one_input_when_one_eligible_file_exists()`

Given: a repository contains exactly one eligible source file and no excluded surfaces.
When: `discover_scan_inputs` runs.
Then: it returns exactly one `ScanInput` whose path equals the eligible file path relative to root.

### Behavior 39: Maximum bounded discovery surface remains deterministic
Test function: `fn discover_scan_inputs_returns_sorted_inputs_when_maximum_bounded_tree_is_present()`

Given: a generated repository tree at the configured maximum test bound contains eligible and excluded files in randomized creation order.
When: `discover_scan_inputs` runs twice.
Then: both returned `Vec<ScanInput>` values equal the same path-sorted expected eligible list.

### Behavior 40: Symlink escape root is invalid
Test function: `fn scan_repository_returns_invalid_root_when_root_symlink_escapes_workspace()`

Given: the supplied root path is a symlink that resolves outside the active workspace.
When: repository scanning starts.
Then: it returns `Err(NamingScanError::InvalidRoot { root })` with the original supplied root path.

### Behavior 41: Symlink loop discovery fails closed
Test function: `fn discover_scan_inputs_returns_file_discovery_failed_when_symlink_loop_is_reached()`

Given: a repository tree contains a symlink loop in a selected discovery path.
When: `discover_scan_inputs` runs.
Then: it returns `Err(NamingScanError::FileDiscoveryFailed { path, source })` with the loop path, unless symlinks are documented as excluded; in that case the returned input set must exactly exclude the loop path.

### Behavior 42: Empty classifier input returns no occurrence
Test function: `fn classify_occurrence_returns_no_occurrence_when_text_is_empty()`

Given: `text == ""` with valid path, line, column, and complete config.
When: `classify_occurrence` runs.
Then: it returns exactly `Ok(OccurrenceClass::NoOccurrence)`.

### Behavior 43: Case variants are not canonical
Test function: `fn classify_occurrence_returns_invalid_legacy_when_case_variant_is_seen()`

Given: text contains `Velvet-Ballastics`, `VELVET-BALLASTICS`, or mixed-case variants in an eligible context.
When: `classify_occurrence` runs.
Then: it returns exactly `Ok(OccurrenceClass::InvalidLegacy { spelling_class: SpellingClass::LegacyProjectSpelling, remediation: "velvet-ballastics" })` or the exact context-specific canonical remediation.

### Behavior 44: Unicode confusables are not canonical
Test function: `fn classify_occurrence_returns_invalid_legacy_when_unicode_confusable_is_seen()`

Given: text contains Unicode lookalike characters around a spelling-like token.
When: `classify_occurrence` runs.
Then: it returns an invalid/confusable occurrence class with exact remediation or `NamingScanError::InputReadFailed` if the final supported-text policy rejects that text; it must not return any canonical class.

### Behavior 45: CRLF columns remain exact
Test function: `fn scan_file_returns_exact_columns_when_file_uses_crlf_newlines()`

Given: a selected CRLF text file contains invalid legacy spelling at a known line and column.
When: `scan_file` runs.
Then: the finding line and column equal the fixture's one-based logical text location and are not shifted by `\r` handling.

### Behavior 46: Final line without newline remains scannable
Test function: `fn scan_file_returns_final_line_finding_when_file_does_not_end_with_newline()`

Given: the last line of a selected file has no trailing newline and contains invalid legacy spelling at a known column.
When: `scan_file` runs.
Then: it returns a finding for the final line with exact line and column.

### Behavior 47: Maximum occurrence count does not truncate findings
Test function: `fn scan_file_returns_all_findings_when_maximum_bounded_occurrence_count_is_present()`

Given: a selected file contains the maximum bounded number of invalid occurrences generated for tests.
When: `scan_file` runs.
Then: the returned findings length equals the generated occurrence count and the last finding equals the expected final path/line/column/class/remediation.

### Behavior 48: Mixed repository reports only invalid inputs
Test function: `fn scan_repository_returns_exact_invalid_findings_when_valid_and_invalid_inputs_are_mixed()`

Given: a repository fixture contains canonical-only files, allowed exception files, and invalid legacy files.
When: `scan_repository` runs.
Then: it returns `Err(NamingScanError::InvalidCanonicalSpelling { findings })` where `findings` equals the invalid-only expected list and contains no finding for canonical or allowed-exception files.

### Behavior 49: All-invalid repository reports every invalid file
Test function: `fn scan_repository_returns_all_findings_when_every_selected_input_is_invalid()`

Given: every selected eligible file contains at least one invalid legacy occurrence.
When: `scan_repository` runs.
Then: it returns `Err(NamingScanError::InvalidCanonicalSpelling { findings })` where `findings` equals the full expected list across all files in stable order.

### Behavior 50: Empty report rendering is exact
Test function: `fn render_scan_report_returns_empty_finding_output_when_report_has_zero_findings()`

Given: `ScanReport` has root, config fingerprint, selected/scanned counts, and `findings: []`.
When: `render_scan_report` runs.
Then: it returns exactly the documented zero-finding rendered report body including counts and no finding rows.

### Behavior 51: Duplicate sort keys render deterministically
Test function: `fn render_scan_report_orders_duplicate_sort_keys_by_spelling_class_when_path_line_and_column_match()`

Given: a report contains findings with equal path, line, and column but different spelling classes.
When: `render_scan_report` runs.
Then: rendered findings are ordered by spelling-class order after the equal path/line/column keys.

### Behavior 52: Report destination failures are distinguished
Test functions:
- `fn render_scan_report_returns_report_write_failed_when_destination_parent_is_missing()`
- `fn render_scan_report_returns_report_write_failed_when_destination_permission_is_denied()`

Given: a report destination has either a missing parent directory or permission-denied parent.
When: the shell layer writes the rendered report.
Then: it returns `Err(NamingScanError::ReportWriteFailed { path, source })` with the exact destination path and the OS/source error.

## 4. Proptest Invariants

### Proptest: `validate_scan_config`
Invariant: any config with all required canonical kinds mapped to exact canonical tokens and only exact documented exception predicates validates to a config whose normalized table equals the input canonical table.
Strategy: generate `RawScanConfig` from required name-kind enum, exact token enum, and exception enum; permute entry order.
Anti-invariant: configs missing at least one kind, duplicating a kind with a different token, using a broad wildcard, or mapping a kind to any legacy spelling always return `NamingScanError::InvalidConfiguration` with the offending kind/reason.

### Proptest: `classify_occurrence`
Invariant: every generated canonical token in a valid context is classified as its exact canonical class and never emits a finding.
Strategy: generate contexts for product/binary/package/rig, crate/module/database, and language-version with exact canonical tokens embedded at arbitrary line/column offsets.
Anti-invariant: generated legacy spellings outside exact exception contexts classify as invalid legacy or produce `InvalidCanonicalSpelling` findings.

### Proptest: allowlist predicate
Invariant: a legacy occurrence is allowed if and only if it exactly matches one of repository path, master filename, or explicit migration-reference predicates.
Strategy: generate exact exception occurrences plus near misses: prefix/suffix additions, parent/child paths, case variations, generated filenames, and unlabeled migration-like text.
Anti-invariant: every near miss must be rejected as invalid legacy; no substring match may authorize a near miss.

### Proptest: `discover_scan_inputs`
Invariant: for any generated repo tree, returned inputs equal the deterministic set of included surfaces minus documented excluded path classes, sorted by normalized path.
Strategy: generate bounded temp-directory trees with file classes: source, doc, manifest, script, configured bead reference, VCS internal, build output, binary blob, embedded database state, lock/runtime artifact.
Anti-invariant: undocumented path exclusions must not silently drop eligible text files.

### Proptest: `scan_file`
Invariant: the number and locations of findings equal the generated disallowed occurrences in supported text.
Strategy: generate UTF-8 files with canonical tokens, allowed exceptions, and invalid spellings at arbitrary line/column offsets including line start, line end, and multiple per line.
Anti-invariant: invalid spellings in eligible text never produce zero findings.

### Proptest: `scan_repository`
Invariant: scanning identical generated contents with permuted input order returns exactly identical `ScanReport` values.
Strategy: generate nonempty vectors of `ScanInput` fixtures with varied paths, line/column positions, and spelling classes; shuffle order.
Anti-invariant: if any generated disallowed occurrence exists, result is `NamingScanError::InvalidCanonicalSpelling { findings }` with nonempty findings.

### Proptest: `render_scan_report`
Invariant: rendered findings are ordered by path, line, column, spelling class and rendering the same report twice is byte-for-byte identical.
Strategy: generate arbitrary bounded `NamingFinding` lists with valid one-based line/column numbers and spelling classes.
Anti-invariant: no rendered output may preserve unsorted input order when it conflicts with the required sort key.

## 5. Fuzz Targets

### Fuzz Target: text decoder and `scan_file` boundary
Input type: raw bytes plus path metadata.
Risk: panic, OOM, invalid UTF-8 misclassification, silent skip of selected inputs, incorrect `InputReadFailed` mapping.
Corpus seeds:
- empty file
- ASCII canonical-only file
- invalid legacy spelling at byte 0
- multiple invalid spellings on one line
- invalid UTF-8 bytes around spelling-like ASCII
- very long line containing canonical and invalid tokens
- mixed newline styles `\n`, `\r\n`, `\r`
Expected outcome: either exact findings for supported text or `NamingScanError::InputReadFailed`; never panic and never silently pass invalid supported text.

### Fuzz Target: `classify_occurrence` path-like and migration-like input
Input type: arbitrary UTF-8 string plus generated path/context tags.
Risk: broad substring allowlist, path traversal confusion, case/Unicode confusable bypass, exception predicate overreach.
Corpus seeds:
- exact repository path exception
- repository path with prefix/suffix
- exact master filename exception
- master filename embedded in unrelated path
- explicit migration reference
- unlabeled migration-like sentence
- Unicode lookalikes around `velvet-ballistics`
Expected outcome: exact exceptions classify as allowed legacy; near misses classify as invalid legacy with no panic.

### Fuzz Target: config validation parser/boundary if config is loaded from external data
Input type: raw bytes or arbitrary config struct, depending on final interface.
Risk: malformed config accepted, wildcard accepted, pattern compilation crash, missing entries misreported.
Corpus seeds:
- empty config
- complete exact config
- duplicate canonical kind
- wildcard allowlist
- invalid regex/pattern if patterns are user-configurable
- contradictory token mappings
Expected outcome: exact `ScanConfig` for valid config, `NamingScanError::InvalidConfiguration` or `NamingScanError::PatternCompilationFailed` for invalid config; never panic.

## 6. Kani Harnesses

### Kani Harness: one-based line and column construction is panic-free
Property: for bounded text up to 4 KiB and any occurrence offset within bounds, line and column construction returns values >= 1 and never performs unchecked indexing, slicing, casts, or arithmetic.
Bound: text length <= 4096 bytes, line count <= 256, occurrence count <= 64.
Rationale: exact finding location is contract-critical and off-by-one/overflow bugs are easy to miss with example tests.

### Kani Harness: occurrence classification is total
Property: for every bounded spelling class and context tag, `classify_occurrence` returns either a canonical/allowed/invalid class or a typed `NamingScanError`, never panics.
Bound: token/context enum exhaustive; text fragment length <= 256 bytes.
Rationale: classifier is the verified kernel boundary and must be total over all public inputs.

### Kani Harness: finding sort key is total and stable
Property: for bounded finding lists, sorting by path, line, column, spelling class produces a deterministic total order equivalent to tuple order and preserves equality for identical keys.
Bound: findings <= 16, path length <= 128, line/column <= 10_000.
Rationale: deterministic reporting is a release-gate guarantee and complements Lean `THM-INV-008` with Rust data constructors.

### Kani Harness: report data construction avoids unchecked collection operations
Property: constructing `ScanReport` and `NamingFinding` from bounded valid inputs cannot panic and cannot use unchecked indexing/slicing/casts/arithmetic.
Bound: findings <= 64, remediation length <= 256, path length <= 256.
Rationale: `PRE-005` and `INV-007` require all fallible/data operations to remain typed and safe.

## 7. Mutation Testing Checkpoints

Minimum command: `moon run :verify-deep` must run `cargo-mutants` or the repository-approved mutation lane. Kill threshold: >=90% overall and 100% for critical mutants below.

Critical mutants that must be killed:
- Change canonical product token from `velvet-ballastics` to `velvet-ballistics`; caught by `canonical_table_returns_ballastics_for_product_binary_package_and_rig_when_loaded`.
- Change canonical crate token from `velvet_ballastics` to `velvet_ballistics`; caught by `canonical_table_returns_underscore_ballastics_for_crate_module_and_database_when_loaded`.
- Change language version from `velvet-ballastics/v1` to any other version; caught by `canonical_table_returns_ballastics_v1_when_language_version_is_loaded`.
- Remove generic missing-entry validation; caught by `validate_scan_config_returns_invalid_configuration_when_product_kind_is_missing`, `validate_scan_config_returns_invalid_configuration_when_crate_module_kind_is_missing`, and `validate_scan_config_returns_invalid_configuration_when_language_version_kind_is_missing`.
- Remove empty-config validation; caught by `validate_scan_config_returns_invalid_configuration_when_config_is_empty`.
- Remove product-kind missing validation; caught by `validate_scan_config_returns_invalid_configuration_when_product_kind_is_missing`.
- Remove crate/module-kind missing validation; caught by `validate_scan_config_returns_invalid_configuration_when_crate_module_kind_is_missing`.
- Remove language-version-kind missing validation; caught by `validate_scan_config_returns_invalid_configuration_when_language_version_kind_is_missing`.
- Remove duplicate-kind validation; caught by `validate_scan_config_returns_invalid_configuration_when_kind_is_duplicated` and `validate_scan_config_returns_invalid_configuration_when_kind_count_is_one_above_required`.
- Remove contradictory-token validation; caught by `validate_scan_config_returns_invalid_configuration_when_canonical_token_contradicts_kind`.
- Remove wildcard allowlist validation; caught by `validate_scan_config_returns_invalid_configuration_when_allowlist_contains_wildcard`.
- Remove prefix-only allowlist validation; caught by `validate_scan_config_returns_invalid_configuration_when_allowlist_contains_prefix_only_rule`.
- Remove substring allowlist validation; caught by `validate_scan_config_returns_invalid_configuration_when_allowlist_contains_substring_rule`.
- Map invalid pattern to `InvalidConfiguration`; caught by `validate_scan_config_returns_pattern_compilation_failed_when_scan_pattern_is_invalid`.
- Replace exact allowlist match with substring match; caught by `classify_occurrence_returns_invalid_legacy_when_exception_substring_appears_in_unrelated_text` and allowlist proptest.
- Treat unlabeled migration-like text as allowed; caught by `classify_occurrence_returns_invalid_legacy_when_exception_substring_appears_in_unrelated_text` and fuzz target.
- Skip invalid legacy spelling; caught by `scan_repository_returns_invalid_canonical_spelling_when_legacy_spelling_is_outside_allowlist`.
- Return empty findings for invalid spelling; caught by `scan_repository_returns_invalid_canonical_spelling_when_legacy_spelling_is_outside_allowlist`.
- Swap `path` and `text` arguments in classification/scan assembly; caught by exact path assertions in `scan_file_returns_exact_finding_fields_when_invalid_occurrence_is_present` and exact occurrence-class payload tests.
- Swap `line` and `column` arguments; caught by `scan_file_returns_exact_finding_fields_when_invalid_occurrence_is_present`, `scan_file_returns_column_one_when_invalid_occurrence_starts_at_first_column`, and CRLF/final-line location kernel tests.
- Use wrong config when classifying an occurrence; caught by `scan_repository_report_kernel_preserves_config_identity_when_report_is_successful` and exact remediation-token tests.
- Return `Ok(Default::default())` for successful repository scan; caught by `scan_repository_returns_zero_finding_report_when_inputs_are_canonical_or_allowed_exceptions`, which asserts root, config fingerprint, selected input count, scanned text input count, and empty findings.
- Emit wrong remediation token for product/package contexts; caught by `scan_file_returns_exact_finding_fields_when_invalid_occurrence_is_present`.
- Emit wrong remediation token for crate/database contexts; caught by `given_wrong_crate_or_module_spelling_when_repository_is_scanned_then_gate_fails_closed` / `wrong_crate_repo_fixture`.
- Emit wrong remediation token for language-version contexts; caught by language-version classifier and generated file-scan proptest.
- Change line/column to zero-based; caught by `scan_file_returns_exact_finding_fields_when_invalid_occurrence_is_present` and `scan_file_returns_column_one_when_invalid_occurrence_starts_at_first_column`.
- Report only first invalid occurrence on a line; caught by `scan_file_returns_all_findings_when_multiple_invalid_occurrences_share_one_line`.
- Remove sorting before render; caught by `render_scan_report_orders_findings_by_path_line_column_and_class_when_findings_are_unsorted`.
- Make traversal order affect report; caught by `scan_repository_returns_identical_report_when_same_inputs_are_discovered_in_different_orders`.
- Silently ignore unreadable files; caught by `scan_file_returns_input_read_failed_when_selected_input_is_unreadable_or_undecodable`.
- Map discovery failure to wrong error variant; caught by `discover_scan_inputs_returns_file_discovery_failed_when_traversal_cannot_complete`.
- Map report write failure to wrong error variant; caught by `render_scan_report_returns_report_write_failed_when_destination_is_unwritable`.
- Exclude eligible source/doc/manifest/script path; caught by `discover_scan_inputs_returns_source_docs_manifests_scripts_and_configured_beads_when_present`.
- Exclude docs but include source; caught by `discover_scan_inputs_returns_source_docs_manifests_scripts_and_configured_beads_when_present` because expected path set includes docs separately.
- Exclude manifests but include source; caught by `discover_scan_inputs_returns_source_docs_manifests_scripts_and_configured_beads_when_present` because expected path set includes manifests separately.
- Exclude scripts but include docs; caught by `discover_scan_inputs_returns_source_docs_manifests_scripts_and_configured_beads_when_present` because expected path set includes scripts separately.
- Exclude configured bead references; caught by `discover_scan_inputs_returns_source_docs_manifests_scripts_and_configured_beads_when_present` because expected path set includes configured bead references separately.
- Include `.beads/dolt`, build output, or binary blob; caught by `discover_scan_inputs_excludes_vcs_build_binary_embedded_database_and_runtime_artifacts_when_present`.
- Include `.git`; caught by excluded-surface discovery test with exact excluded path set.
- Include `target`; caught by excluded-surface discovery test with exact excluded path set.
- Include `.beads/backup` or `.beads/embeddeddolt`; caught by excluded-surface discovery test with exact excluded path set.
- Remove Moon fast-gate wiring; caught by `moon_verify_fast_fails_when_invalid_naming_fixture_is_present`.
- Add forbidden dependency or forbidden construct; caught by static governance tests and `moon run :verify-fast`.

## 8. Combinatorial Coverage Matrix

### Config validation and canonical table

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| empty config | zero entries | `Err(NamingScanError::InvalidConfiguration { reason: empty config })` | unit |
| minimum complete table | all required exact tokens and exact exceptions only | `Ok(ScanConfig)` with exact canonical entries, exact allowlist payloads, config fingerprint | unit |
| maximum bounded valid config | all required exact tokens, exact exceptions, documented path/pattern rules | `Ok(ScanConfig)` with exact canonical entries, exact allowlist payloads, exact rule count | unit |
| one-below required kind count | one required kind absent | `Err(NamingScanError::InvalidConfiguration { reason: one-below/missing kind })` | unit/proptest |
| missing product token | malformed config | `Err(NamingScanError::InvalidConfiguration { reason: missing product })` | unit/proptest |
| missing crate token | malformed config | `Err(NamingScanError::InvalidConfiguration { reason: missing crate/module })` | unit/proptest |
| missing language-version token | malformed config | `Err(NamingScanError::InvalidConfiguration { reason: missing language version })` | unit/proptest |
| duplicate kind | malformed config | `Err(NamingScanError::InvalidConfiguration { reason: duplicate kind })` | unit/proptest |
| one-above required kind count | duplicate extra kind | `Err(NamingScanError::InvalidConfiguration { reason: one-above/duplicate kind })` | unit/proptest |
| contradictory token | malformed config | `Err(NamingScanError::InvalidConfiguration { reason: contradiction })` | unit/proptest |
| wildcard allowlist | broad config | `Err(NamingScanError::InvalidConfiguration { reason: broad wildcard })` | unit/proptest |
| prefix-only allowlist | broad config | `Err(NamingScanError::InvalidConfiguration { reason: prefix-only allowlist })` | unit/proptest |
| substring allowlist | broad config | `Err(NamingScanError::InvalidConfiguration { reason: substring allowlist })` | unit/proptest |
| invalid pattern | malformed pattern | `Err(NamingScanError::PatternCompilationFailed { pattern, source })` | unit |

### Occurrence classification

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| canonical product | `velvet-ballastics` valid context | `Ok(OccurrenceClass::CanonicalProduct { canonical: "velvet-ballastics", kind: ProductBinaryPackageRig })` | unit |
| canonical crate | `velvet_ballastics` valid context | `Ok(OccurrenceClass::CanonicalCrateModule { canonical: "velvet_ballastics", kind: CrateModuleDatabase })` | unit |
| canonical language version | `velvet-ballastics/v1` | `Ok(OccurrenceClass::CanonicalLanguageVersion { canonical: "velvet-ballastics/v1", kind: LanguageVersion })` | unit |
| repository path exception | exact configured external path | `Ok(OccurrenceClass::AllowedLegacy { exception: RepositoryPath { path } })` with exact path payload | unit/Lean |
| master filename exception | exact configured master filename | `Ok(OccurrenceClass::AllowedLegacy { exception: MasterFilename { filename } })` with exact filename payload | unit/Lean |
| migration reference | explicitly labeled migration reference | `Ok(OccurrenceClass::AllowedLegacy { exception: MigrationReference { artifact, label, legacy_text } })` with exact payloads | unit |
| empty text | no occurrence | `Ok(OccurrenceClass::NoOccurrence)` | unit |
| token at start/end | boundary position | exact class with column `1` or exact end column | unit |
| case variant | hostile spelling | `Ok(OccurrenceClass::InvalidLegacy { spelling_class: LegacyProjectSpelling, remediation })` | unit/proptest |
| Unicode confusable | hostile spelling | invalid/confusable class per documented normalization policy, never canonical | unit/fuzz |
| exception substring | prefix/suffix/neighbor path | invalid legacy class with exact remediation | unit/proptest |
| unlabeled migration text | legacy spelling without label | invalid legacy class with exact remediation | unit/fuzz |

### File scanning

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| empty supported text | empty file | `Ok(Vec::new())` | integration |
| canonical-only file | valid text | `Ok(Vec::new())` | integration/proptest |
| invalid occurrence | one disallowed token | `Ok(vec![exact NamingFinding])` or repository-level `Err(InvalidCanonicalSpelling)` | integration |
| invalid at first column | boundary column | finding has `column == 1` | unit/integration/Kani |
| CRLF columns | Windows newlines | exact one-based line and column ignoring carriage return as column content per documented policy | unit/integration |
| final line no newline | boundary EOF | exact line and column for final occurrence | unit/integration |
| maximum line length | bounded long line | exact finding count and no truncation | unit/proptest |
| maximum occurrence count | bounded many tokens | exact all-findings vector length and exact last finding | unit/proptest |
| multiple same line | two invalid tokens | two exact findings with exact columns | integration |
| unreadable selected file | filesystem failure | `Err(NamingScanError::InputReadFailed { path, source })` | integration/manual |
| undecodable selected file | hostile bytes | `Err(NamingScanError::InputReadFailed { path, source })` | fuzz/integration |

### Repository scanning and discovery

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| missing root | invalid root | `Err(NamingScanError::InvalidRoot { root })` | integration/manual |
| external root | outside workspace | `Err(NamingScanError::InvalidRoot { root })` | integration/manual |
| symlink escape root | symlink resolves outside workspace | `Err(NamingScanError::InvalidRoot { root })` | integration/manual |
| empty eligible set | no selected inputs | `Ok(ScanReport { selected_input_count: 0, scanned_text_input_count: 0, findings: [] })` | unit/integration |
| one eligible file | minimum repo | exact one `ScanInput` and successful scan result count `1` | integration |
| maximum bounded tree | max generated repo fixture | deterministic exact sorted input list | integration/proptest |
| eligible source surfaces | source/docs/manifests/scripts/beads | exact included `ScanInput` set | integration/proptest |
| excluded surfaces | `.git`, `target`, binary, `.beads/dolt`, locks | exact excluded path set | integration/proptest |
| symlink loop | cyclic path | `Err(NamingScanError::FileDiscoveryFailed { path, source })` or documented skip if loop is explicitly excluded | integration/manual |
| discovery failure | traversal error | `Err(NamingScanError::FileDiscoveryFailed { path, source })` | integration/manual |
| canonical repo | valid contents | `Ok(ScanReport { root, config_fingerprint, selected_input_count, scanned_text_input_count, findings: [] })` | integration/proptest |
| invalid repo | disallowed legacy spelling | `Err(NamingScanError::InvalidCanonicalSpelling { findings: nonempty exact list })` | integration/mutation |
| all-invalid repo | every selected file invalid | `Err(NamingScanError::InvalidCanonicalSpelling { findings: exact all-invalid list })` | integration/proptest |
| mixed repo | valid and invalid selected files | `Err(NamingScanError::InvalidCanonicalSpelling { findings: exact invalid-only list })` | integration/proptest |
| shuffled inputs | same contents different order | exactly equal `ScanReport` and rendered bytes | integration/proptest/Lean |
| read-only scan | clean git repo | `git diff --exit-code == 0` and unchanged hashes | E2E/manual |

### Report rendering and quality gate

| Scenario | Input Class | Expected Output | Layer |
|----------|-------------|-----------------|-------|
| empty report | zero findings | exact empty rendered body/header per report format | unit/integration |
| single finding | one exact finding | rendered output preserves path, line, column, class, remediation exactly | unit/integration |
| unsorted findings | bounded finding list | rendered order is path/line/column/class | integration/Kani/Lean |
| duplicate sort keys | equal path/line/column with different class | deterministic class-order output | unit/Kani |
| maximum bounded findings | report upper test bound | exact first and last rendered finding and no truncation | unit/proptest |
| non-ASCII paths | UTF-8 path names | exact path rendering or documented typed path error | integration/fuzz |
| missing parent destination | report path parent absent | `Err(NamingScanError::ReportWriteFailed { path, source })` | integration/manual |
| permission-denied destination | filesystem write failure | `Err(NamingScanError::ReportWriteFailed { path, source })` | integration/manual |
| invalid fixture in fast lane | Moon quality gate | nonzero exit plus exact finding output | E2E |
| fixed fixture in fast lane | canonical replacement | exit code `0` | E2E |
| forbidden source construct | source contains forbidden token | `moon run :verify-fast` nonzero with token name | static |
| forbidden runtime dependency | runtime core includes YAML/JSON/HTTP | `moon run :verify-fast` nonzero with dependency name | static |

## 9. Fixtures and Test Data

- `canonical_repo_fixture`: temp repo containing eligible source/docs/manifests/scripts/bead references with only `velvet-ballastics`, `velvet_ballastics`, `velvet-ballastics/v1`, and exact documented legacy exceptions.
- `invalid_legacy_repo_fixture`: temp repo with one invalid legacy project spelling outside migration context in an eligible file; expected exact path/line/column is fixed in fixture notes.
- `wrong_crate_repo_fixture`: temp repo with wrong crate/module spelling in Rust source or manifest; expected remediation is `velvet_ballastics`.
- `exception_near_miss_fixture`: texts where exception strings appear as substrings, child paths, generated names, and unlabeled migration-like prose; all must fail closed.
- `discovery_surface_fixture`: temp tree with eligible files and excluded surfaces `.git/`, `target/`, binary blobs, `.beads/dolt`, `.beads/backup`, `.beads/embeddeddolt`, locks, and runtime DB state.
- `unreadable_input_fixture`: permission-denied or otherwise unreadable selected file for platforms that support permissions; if platform cannot model permission denial, use a documented broken selected input fake at the filesystem boundary.
- `unwritable_report_destination_fixture`: unwritable directory or destination path used only by the shell/report layer.

All fixtures must be hermetic and created under test temp directories except the manual QA real-workspace run.

## 10. Commands and Evidence Expectations

- Red-phase focused checks: run the specific failing unit/integration tests by name after writing each test. Expected result before implementation: test compiles if API stubs exist and fails with exact missing/wrong value or exact missing error variant.
- Standard test lane: `moon run :verify-standard` must run full unit and integration suite and produce `formal-verification-report.md` evidence for PRE/POST/INV/ERR scenarios.
- Deep test lane: `moon run :verify-deep` must run proptest extended cases, fuzz smoke/regression corpus, cargo-mutants, and coverage evidence.
- Proof lane: `moon run :verify-proof` must run Lean obligations `THM-INV-001`, `THM-INV-002`, `THM-INV-003`, `THM-INV-004`, `THM-INV-005`, `THM-INV-008` and Kani harnesses above.
- Fast gate: `moon run :verify-fast` must include static forbidden-token/dependency scans and the canonical spelling quality gate.
- All gate: `moon run :verify-all` must roll up fast, standard, deep, proof, and manual QA evidence.
- Manual QA evidence: `manual-qa-spelling-scan.md` must include invalid root, real workspace pass/fail examples, unwritable report destination, and before/after `git diff --exit-code` evidence.
- Coverage evidence: `cargo-llvm-cov` report must show every `NamingScanError` variant and every occurrence class executed.

## 11. Error Variant Coverage

Every error variant must have at least one exact scenario and exact assertion:

| Error Variant | Required Scenario | Exact Assertion |
|---------------|-------------------|-----------------|
| `NamingScanError::InvalidRoot` | `scan_repository_returns_invalid_root_when_root_is_missing_or_outside_workspace` | returned error variant is `InvalidRoot` and contains rejected root |
| `NamingScanError::InvalidConfiguration` | `validate_scan_config_returns_invalid_configuration_when_config_is_empty`; `...product_kind_is_missing`; `...crate_module_kind_is_missing`; `...language_version_kind_is_missing`; `...kind_is_duplicated`; `...canonical_token_contradicts_kind`; `...allowlist_contains_wildcard`; `...allowlist_contains_prefix_only_rule`; `...allowlist_contains_substring_rule` | returned error variant is `InvalidConfiguration` and reason identifies exact defect |
| `NamingScanError::FileDiscoveryFailed` | traversal failure test | returned error variant is `FileDiscoveryFailed` and contains failing path |
| `NamingScanError::InputReadFailed` | unreadable/undecodable selected input test | returned error variant is `InputReadFailed` and contains selected path |
| `NamingScanError::PatternCompilationFailed` | `validate_scan_config_returns_pattern_compilation_failed_when_scan_pattern_is_invalid` | returned error variant is `PatternCompilationFailed` and contains invalid pattern |
| `NamingScanError::InvalidCanonicalSpelling { findings }` | invalid spelling repository test | returned error variant is `InvalidCanonicalSpelling` and findings equal nonempty expected list |
| `NamingScanError::ReportWriteFailed` | unwritable destination test | returned error variant is `ReportWriteFailed` and contains destination path |

## Open Questions

1. The final command surface is still open in the contract (`moon ci`, dedicated Moon task, `just`, or script invoked by Moon). This plan assumes `moon run :verify-fast`, `moon run :verify-standard`, `moon run :verify-deep`, `moon run :verify-proof`, and `moon run :verify-all` per `verification-layers.md`; update only command names if downstream implementation chooses aliases.
2. Bead historical records scanning scope is open. This plan requires configured bead references to be included and `.beads/dolt`/runtime database state to be excluded. If historical bead records are excluded by default, add a fixture proving the documented scope boundary.
