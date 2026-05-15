# Contract Specification: vb-qi37.12.4

## Context

- Feature: reproducible quality gate for ignored fallible production results and silent-discard patterns.
- Source of truth consumed: `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-qi37.12.4 --json`, `.beads/vb-qi37.12.4/codebase-map.md`, `.beads/vb-qi37.12.4/delivery-scope.jsonl`.
- Scope: first-party production Rust and gate wiring in the isolated workspace only.
- Non-production exclusions: tests, benches, fuzz, Kani/proof artifacts, reference artifacts, bead evidence, and fixtures unless a production-like negative fixture is explicitly used to prove the gate.

## Domain Terms

- Fallible result: an expression whose type is `Result<_, _>` or another Rust `#[must_use]` fallible outcome where discarding hides success/failure.
- Silent discard: code that observes neither success nor error and permits execution to continue as if no fallible operation occurred.
- Production scan domain: `crates/*/src/**/*.rs` plus `xtask/src/**/*.rs`, minus declared non-production exclusions.
- Exception: a path-bound, machine-readable justification that is either non-production or semantically safe because the discarded outcome cannot change correctness, durability, observability, or user-visible status.
- Deterministic classifier: the gate decision procedure that maps `(source match, scan-domain classification, exception set)` to exactly one of `Violation`, `JustifiedException`, or `NonProductionExcluded` with stable ordering and stable error taxonomy.
- Exception validation: the gate decision procedure that rejects malformed, missing-owner, missing-expiry, overbroad, class-wide, and production-hiding exception records before any zero-exit report can be emitted.

## Assumptions

- `xtask/src` is first-party tooling and remains in the scan domain unless implementation records explicit path-bound exclusions.
- Existing observed `crates/velvet_ballastics/src/main.rs` discard candidates are not accepted as clean; final acceptance must either fail on them or remove/replace them in another state/bead before the gate passes.
- No new external dependencies are required for this contract; if implementation adds one, supply-chain verification becomes required.

## Preconditions

- PRE-001: The gate command is run from the repository root of the isolated workspace.
- PRE-002: The scan domain includes first-party production Rust under `crates/*/src/**/*.rs` and `xtask/src/**/*.rs`.
- PRE-003: Non-production paths are excluded only by explicit path patterns, not by file contents or comments alone.
- PRE-004: Any exception is represented in a deterministic, reviewable artifact with path, pattern/class, reason, owner bead or owner, and expiry or non-production classification.

## Postconditions

- POST-001: A reproducible command exits non-zero when production code contains an unhandled ignored `Result` or other configured silent-discard pattern.
- POST-002: The gate exits zero only when every production match is absent or justified by a valid exception record.
- POST-003: The canonical Moon verification path runs the gate so `moon run :verify-standard` cannot pass while the gate fails.
- POST-004: `moon run :lint-src` preserves hard denial of `unused_must_use` and `clippy::let_underscore_must_use` for production/source targets.
- POST-005: Negative production-like fixtures demonstrate failure for each contracted silent-discard class without requiring production code to retain violations.

## Invariants

- INV-001: No first-party production path silently discards a fallible result without a valid explicit exception.
- INV-002: The gate taxonomy is deterministic: the same tree and exception artifact produce the same pass/fail result and same reported violation set.
- INV-003: Non-production exclusions cannot hide production files.
- INV-004: Exception records are narrower than the scan domain and cannot globally disable a discard class.
- INV-005: Gate wiring is fail-closed: missing scan script, malformed exception data, or unreadable production paths causes failure.
- INV-006: The deterministic classifier has total, mutually exclusive outcomes: every production-like match is either a violation or covered by exactly one valid path-bound exception; no match can be both accepted and rejected.
- INV-007: Exception validation is fail-closed and complete for malformed syntax, missing owner, missing expiry/follow-up, overbroad path, overbroad class, and production-hiding non-production claims.

## Silent-Discard Taxonomy

- DISCARD-001: Bare ignored `Result` or must-use expression statement.
- DISCARD-002: `let _ = <fallible>;` or equivalent binding that intentionally ignores a must-use/fallible value.
- DISCARD-003: `<fallible>.ok()` or `<fallible>.err()` where the converted option is unused or discarded.
- DISCARD-004: `match`/branch arm such as `Err(_) => {}` or `Ok(()) | Err(_) => {}` that swallows failure without typed propagation, visible diagnostic, or justified recovery.
- DISCARD-005: `drop(<fallible>)` or equivalent explicit discard of a fallible value.
- DISCARD-006: documented silent-discard comments or allow markers that lack a valid exception record.

## Error Taxonomy

- GateError::ViolationFound - one or more production silent-discard violations were found.
- GateError::InvalidInvocation - command not run from repo root or required arguments are invalid.
- GateError::UnreadableInput - scan domain or exception artifact cannot be read.
- GateError::MalformedException - exception artifact is syntactically invalid or missing required fields.
- GateError::OverbroadException - exception record disables a whole class or broad subtree without valid non-production classification.
- GateError::MissingGateWiring - canonical Moon verification path does not execute the gate.

## Contract Signatures

- `run_ignored_fallible_results_gate(repo_root: RepoRoot, scan_domain: ScanDomain, exceptions: ExceptionSet) -> Result<GateReport, GateError>`
- `classify_silent_discard(match: SourceMatch, exceptions: ExceptionSet) -> Result<Classification, GateError>`
- `validate_exception_set(exceptions: ExceptionSet, scan_domain: ScanDomain) -> Result<ValidatedExceptionSet, GateError>`
- `verify_moon_wiring(tasks: MoonTaskGraph) -> Result<WiringReport, GateError>`

## Verus-Owned Clauses

- VERUS-WAIVER-001: Verus is waived for the current State 3 artifact because there is no bead-local Rust classifier, parser, exception validator, data structure, loop, arithmetic bound, or typestate surface to verify; current behavior is a future executable shell/static-gate contract over filesystem, Moon, clippy, and process exit status. Owner: State 3 rust-contract. Expiry/follow-up: expires immediately if State 8/11 introduces any Rust-local classifier, parser, exception validator, or reusable report model for `classify_silent_discard` or `validate_exception_set`. Concrete limitation: Verus cannot verify absent Rust code or external shell/Moon/filesystem semantics. Compensating evidence: required executable obligations `GATE-CLASSIFIER-001`, `GATE-EXC-001`, `GATE-EXC-VALIDATION-001`, `GATE-DETERMINISM-001`, and `GATE-FAIL-CLOSED-001` must prove deterministic classifier and exception-validation behavior through negative fixtures, malformed exception fixtures, repeated-run comparison, and fail-closed command evidence.
- VERUS-FOLLOWUP-001: If implementation introduces Rust-local deterministic classifier or exception-validation logic, downstream contract repair must add Verus-first obligations for INV-002, INV-004, INV-006, and INV-007 before proof/test/implementation approval. Required proof surfaces must cover total classification, mutual exclusion, deterministic ordering, narrow path-bound exception acceptance, malformed/overbroad exception rejection, and panic/overflow/indexing freedom.

## TLA+-Owned Clauses

- None. This bead has no lifecycle, scheduler, queue, retry, lease, or concurrent protocol behavior. See `tla-spec.md`.

## Theorem-Owned Clauses

- None. No tiny theorem kernel beyond possible Rust-local validation logic is identified.

## Non-goals

- Repairing runtime/storage silent-discard production behavior in State 3.
- Proving current production tree is clean before dependencies resolve known debt.
- Adding production code, tests, or proof/model code in State 3.
- Treating waiver text as proof evidence; waiver validity must be represented in waiver sections, while executable behavior is proven by runnable obligations.
