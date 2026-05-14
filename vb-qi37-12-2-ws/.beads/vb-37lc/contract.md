# Contract Specification: vb-37lc Canonical Spelling Scan

## Context
- Feature: mechanical repository scan that enforces canonical `velvet-ballastics` naming and rejects legacy misspellings outside documented exceptions.
- Domain terms:
  - Canonical product, binary, package, and bead rig: `velvet-ballastics`.
  - Canonical crate/module and bead database: `velvet_ballastics`.
  - Canonical language version: `velvet-ballastics/v1`.
  - Legacy exception: an occurrence of the legacy project spelling allowed only when it is the current external repository path, the master filename, or an explicitly labeled migration reference to a pre-existing external artifact.
  - Finding: a deterministic record containing file path, line number, column, matched spelling class, and remediation text.
- Assumptions:
  - The scan is a cold quality gate, not runtime core logic.
  - The repository root and master filename exceptions are data in scan configuration, not implicit ad hoc behavior.
  - Binary/generated/vendor/embedded database state is excluded by documented path rules.
  - No production code, tests, proof code, or harness code is part of State 1.
- Open questions:
  - Which exact command name will downstream implementation expose: `moon ci`, a dedicated Moon task, `just`, or a script invoked by Moon?
  - Will bead historical records be scanned by default or only current source/docs/manifests/scripts?

## Preconditions
- PRE-001: The scan receives an explicit repository root path that exists and resolves inside the active workspace.
- PRE-002: The scan configuration contains the complete canonical spelling table for product, binary, package, crate/module, bead rig, bead database, and language version.
- PRE-003: The scan configuration contains the complete allowlist of documented legacy exceptions: current external repository path, current master filename, and explicit migration references.
- PRE-004: The scan input file set is deterministic and excludes only documented non-source surfaces such as VCS internals, build outputs, binary blobs, embedded database state, and generated lock/runtime artifacts.
- PRE-005: All fallible filesystem, decoding, and pattern compilation operations are represented as `Result<T, NamingScanError>`.

## Postconditions
- POST-001: If every scanned occurrence matches a canonical spelling or a documented legacy exception, the scan returns `Ok(ScanReport)` with zero findings.
- POST-002: If any invalid spelling is present outside the allowlist, the scan returns `Err(NamingScanError::InvalidCanonicalSpelling { findings })` and the quality gate fails closed.
- POST-003: Each finding identifies the exact path, one-based line, one-based column, matched spelling class, and canonical replacement guidance.
- POST-004: The scan result is reproducible for identical repository contents and configuration regardless of filesystem traversal order.
- POST-005: The scan never modifies repository files, bead records, manifests, scripts, or generated artifacts.
- POST-006: Integration into the canonical quality flow makes invalid naming a blocking gate for source, docs, manifests, scripts, and configured bead references.

## Invariants
- INV-001: Canonical product, binary, package, and bead rig spelling is always `velvet-ballastics`.
- INV-002: Canonical crate/module and bead database spelling is always `velvet_ballastics`.
- INV-003: Canonical language version spelling is always `velvet-ballastics/v1`.
- INV-004: Legacy project spelling is invalid unless the occurrence matches a documented exception with explicit migration context.
- INV-005: The current external repository path and master filename are allowed legacy exceptions only; they must not expand into a blanket substring allowlist.
- INV-006: The scan has no runtime-core dependencies on YAML, JSON, or HTTP parsing.
- INV-007: The scan contract forbids unsafe code, unwrap, expect, panic, todo, unimplemented, dbg, unchecked indexing/slicing/casts, and unchecked arithmetic in downstream Rust implementation.
- INV-008: Finding ordering is stable: path order, then line, then column, then spelling class.
- INV-009: Unknown or unreadable text inputs fail closed with typed errors rather than being silently skipped.

## Error Taxonomy
- ERR-001: `NamingScanError::InvalidRoot` when PRE-001 is violated.
- ERR-002: `NamingScanError::InvalidConfiguration` when the canonical spelling table or allowlist is missing, duplicated, contradictory, or contains broad wildcards.
- ERR-003: `NamingScanError::FileDiscoveryFailed` when deterministic file discovery cannot complete.
- ERR-004: `NamingScanError::InputReadFailed` when a selected file cannot be read or decoded as supported text.
- ERR-005: `NamingScanError::PatternCompilationFailed` when scan patterns cannot be compiled.
- ERR-006: `NamingScanError::InvalidCanonicalSpelling { findings }` when disallowed spelling occurrences are found.
- ERR-007: `NamingScanError::ReportWriteFailed` when a configured report destination cannot be written by the shell layer.

## Contract Signatures
- `fn canonical_spelling_table() -> CanonicalSpellingTable`
- `fn validate_scan_config(config: RawScanConfig) -> Result<ScanConfig, NamingScanError>`
- `fn discover_scan_inputs(root: RepoRoot, config: &ScanConfig) -> Result<Vec<ScanInput>, NamingScanError>`
- `fn classify_occurrence(path: RepoPath, line: LineNumber, column: ColumnNumber, text: &str, config: &ScanConfig) -> Result<OccurrenceClass, NamingScanError>`
- `fn scan_file(input: ScanInput, config: &ScanConfig) -> Result<Vec<NamingFinding>, NamingScanError>`
- `fn scan_repository(root: RepoRoot, config: ScanConfig) -> Result<ScanReport, NamingScanError>`
- `fn render_scan_report(report: &ScanReport) -> Result<RenderedReport, NamingScanError>`

## Lean-Owned Clauses
- INV-001, INV-002, INV-003: canonical spelling table contains exact required names.
- INV-004, INV-005: pure allowlist predicate accepts only documented legacy exceptions and rejects all other legacy occurrences.
- INV-008: pure finding sort key yields deterministic ordering.

## Non-goals
- No production implementation, test code, Lean proof code, Kani harnesses, or fuzz harnesses in State 1.
- No performance claim beyond deterministic gate behavior.
- No runtime-core behavior change.
- No approval of these artifacts by their author; independent review must write `contract-verification-review.md` with `STATUS: APPROVED` before downstream use.
