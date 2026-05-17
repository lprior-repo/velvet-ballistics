# Lean Contract Projection: vb-37lc

## Boundary
- Lean-owned kernel: pure naming table model, pure legacy-exception predicate, pure occurrence classification, and pure finding ordering.
- Rust/runtime shell: filesystem traversal, text decoding, report writing, Moon/CI integration, command-line process exit behavior.
- External systems excluded from Lean proof: operating system filesystem, Git state, bead database storage, Moon task runner, terminal output.

## Lean-Owned Clauses
- INV-001 -> `VelvetBallastics.Naming.Canonical::product_binary_package_rig_are_ballastics`
- INV-002 -> `VelvetBallastics.Naming.Canonical::crate_module_database_are_underscore_ballistics`
- INV-003 -> `VelvetBallastics.Naming.Canonical::language_version_is_v1`
- INV-004 -> `VelvetBallastics.Naming.Allowlist::legacy_occurrence_requires_documented_exception`
- INV-005 -> `VelvetBallastics.Naming.Allowlist::path_and_master_filename_do_not_generalize`
- INV-008 -> `VelvetBallastics.Naming.Findings::sort_key_is_total_and_stable`

## Abstract Model
- `CanonicalNameKind`: product, binary, package, crate_module, bead_rig, bead_database, language_version.
- `CanonicalName`: ASCII token associated with one `CanonicalNameKind`.
- `LegacyOccurrence`: path, line, column, matched token, and surrounding context label.
- `LegacyException`: repository_root_path, master_filename, or migration_reference.
- `OccurrenceClass`: canonical, allowed_legacy, invalid_legacy, irrelevant.
- `FindingKey`: normalized path, line, column, spelling class.

## Theorem Obligations

### THM-INV-001
- Contract clause: INV-001
- Rust/spec target: `canonical_spelling_table`
- Lean module: `VelvetBallastics.Naming.Canonical`
- Theorem shape: `product_binary_package_rig_are_ballastics`
- Model: finite mapping from canonical name kind to ASCII token.
- Refinement: Rust `CanonicalSpellingTable` validates into the Lean finite mapping with exact string equality for product, binary, package, and bead rig.
- Shell exclusions: filesystem, CLI parsing, report rendering, Moon integration.
- Evidence command: `moon run :verify-proof`

### THM-INV-002
- Contract clause: INV-002
- Rust/spec target: `canonical_spelling_table`
- Lean module: `VelvetBallastics.Naming.Canonical`
- Theorem shape: `crate_module_database_are_underscore_ballistics`
- Model: finite mapping from canonical name kind to ASCII token.
- Refinement: Rust validated table maps crate/module and bead database to the underscore spelling exactly.
- Shell exclusions: filesystem, CLI parsing, report rendering, Moon integration.
- Evidence command: `moon run :verify-proof`

### THM-INV-003
- Contract clause: INV-003
- Rust/spec target: `canonical_spelling_table`
- Lean module: `VelvetBallastics.Naming.Canonical`
- Theorem shape: `language_version_is_v1`
- Model: finite mapping from canonical name kind to ASCII token.
- Refinement: Rust validated table maps language version to canonical v1 token exactly.
- Shell exclusions: filesystem, CLI parsing, report rendering, Moon integration.
- Evidence command: `moon run :verify-proof`

### THM-INV-004
- Contract clause: INV-004
- Rust/spec target: `classify_occurrence`
- Lean module: `VelvetBallastics.Naming.Allowlist`
- Theorem shape: `legacy_occurrence_requires_documented_exception`
- Model: `LegacyOccurrence -> List LegacyException -> OccurrenceClass`.
- Refinement: Rust occurrence classification returns allowed legacy only when the Lean predicate returns true for a documented exception.
- Shell exclusions: filesystem traversal, text decoding, report rendering, Moon integration.
- Evidence command: `moon run :verify-proof`

### THM-INV-005
- Contract clause: INV-005
- Rust/spec target: `classify_occurrence`
- Lean module: `VelvetBallastics.Naming.Allowlist`
- Theorem shape: `path_and_master_filename_do_not_generalize`
- Model: exact exception constructors, not substring patterns.
- Refinement: Rust allowlist entries validate to exact Lean exception constructors and cannot authorize neighboring paths, generated names, diagnostics, or package fields.
- Shell exclusions: filesystem traversal, text decoding, report rendering, Moon integration.
- Evidence command: `moon run :verify-proof`

### THM-INV-008
- Contract clause: INV-008
- Rust/spec target: `scan_repository`
- Lean module: `VelvetBallastics.Naming.Findings`
- Theorem shape: `sort_key_is_total_and_stable`
- Model: list of finding keys sorted by path, line, column, class.
- Refinement: Rust `NamingFinding` exposes a validated key equivalent to the Lean tuple order before report rendering.
- Shell exclusions: filesystem traversal, text decoding, terminal formatting, Moon integration.
- Evidence command: `moon run :verify-proof`

## Waivers
- LEAN-WAIVER-001
  - Clause IDs: PRE-001, ERR-001
  - Waived layer: Lean
  - Reason: repository-root existence and workspace containment depend on filesystem resolution and process environment, not a pure deterministic kernel.
  - Compensating evidence: `PRE-001` and `ERR-001` proof obligations require manual QA and `moon run :verify-standard` evidence for invalid-root behavior.
  - Owner: downstream State 2/3 implementer for `vb-37lc`.
  - Follow-up/complete condition: complete when `manual-qa-spelling-scan.md` and `formal-verification-report.md` show invalid roots fail closed with `NamingScanError::InvalidRoot`.
- LEAN-WAIVER-002
  - Clause IDs: PRE-004, ERR-003, ERR-004
  - Waived layer: Lean
  - Reason: file discovery, file reads, and text decoding are shell I/O behavior outside Lean scope.
  - Compensating evidence: `PRE-004`, `ERR-003`, `ERR-004`, and `FUZZ-001` obligations require standard/deep gauntlet, fuzz, and manual QA evidence.
  - Owner: downstream State 2/3 implementer for `vb-37lc`.
  - Follow-up/complete condition: complete when include/exclude discovery tests pass and hostile/unreadable inputs fail closed without panic.
- LEAN-WAIVER-003
  - Clause IDs: ERR-007
  - Waived layer: Lean
  - Reason: report destination failures are shell write failures, not pure report rendering semantics.
  - Compensating evidence: `ERR-007` obligation requires manual QA of an unwritable report destination.
  - Owner: downstream State 2/3 implementer for `vb-37lc`.
  - Follow-up/complete condition: complete when manual QA records `NamingScanError::ReportWriteFailed` for an unwritable destination.
- LEAN-WAIVER-004
  - Clause IDs: POST-006
  - Waived layer: Lean
  - Reason: Moon/CI quality-gate integration is process orchestration outside the pure kernel.
  - Compensating evidence: `POST-006`, `GATE-001`, and `GATE-002` obligations require gauntlet evidence.
  - Owner: downstream State 2/3 implementer for `vb-37lc`.
  - Follow-up/complete condition: complete when quality-lane evidence shows invalid naming blocks the configured gate.
