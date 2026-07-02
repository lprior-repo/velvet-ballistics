# Domain Model Review: vb-qi37.12.4

## Status

REVIEW_ONLY: This is a State 3 domain/type-model review, not an independent contract-verification approval.

## Model Boundaries

- Core domain: quality-gate classification of source patterns that may discard fallible results.
- Runtime shell: filesystem traversal, Moon task execution, shell exit codes, and clippy execution.
- External systems: Moon, Cargo/clippy, shell, optional JSONL validation command.

## Type Model Candidates

- `RepoRoot`: validated repository root.
- `ProductionPath`: path proven to be inside the scan domain and outside non-production exclusions.
- `NonProductionPath`: excluded path with explicit exclusion reason.
- `DiscardClass`: one of DISCARD-001 through DISCARD-006 from `contract.md`.
- `ExceptionRecord`: path-bound justification with discard class, owner, reason, and expiry/non-production classification.
- `GateReport`: deterministic list of violations, exceptions used, skipped non-production paths, and exit status.
- `Classification`: exactly one of `Violation`, `JustifiedException`, or `NonProductionExcluded` for each source match after scan-domain and exception validation.
- `ValidatedExceptionSet`: exception records proven syntactically valid, path-bound, owned, expiry/follow-up bounded, and narrower than the production scan domain.

## Illegal States To Exclude

- A violation reported without path, line/range, and discard class.
- A production path skipped only because it contains a magic comment.
- A global exception for all files or all discard classes.
- A zero exit status with unreadable inputs, malformed exceptions, or missing Moon wiring.
- A gate that passes only because clippy workspace lints are configured but the direct reproducible command is absent.
- A source match that is simultaneously classified as accepted and rejected.
- An exception record without owner, expiry/follow-up, path binding, discard class, or narrow non-production classification.

## Review Findings

- The model must keep scan-domain classification separate from discard classification to prevent broad regex exceptions from hiding production code.
- Existing production discard candidates noted in `codebase-map.md` must be treated as fail cases or separately remediated; they are not proof of acceptance.
- Because current State 3 has no Rust-local scanner/classifier artifact, Verus is waived with explicit metadata in `contract.md` and `verification-layers.md`; deterministic classifier and exception-validation behavior remain required through executable obligations. If downstream implementation creates Rust scanner logic, the waiver expires and Verus-first obligations must be added before approval.

## Required Downstream Checks

- Validate JSONL exception/trace artifacts if introduced.
- Prove direct gate command fails on production-like negative fixtures.
- Prove canonical Moon path executes the same gate.
