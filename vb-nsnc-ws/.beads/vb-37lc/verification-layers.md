# Verification Layers: vb-37lc Canonical Spelling Scan

## Boundary
- Verified kernel: canonical table validation, allowlist predicate, occurrence classification, stable finding ordering.
- Lean contract projection: exact canonical names, exact legacy exception semantics, deterministic ordering.
- Runtime shell: filesystem discovery, text decoding, report writing, process exit, Moon/CI wiring.
- External systems excluded from formal proof: OS filesystem, Git internals, bead storage backend, terminal emulator.

## Layer Assignment
- PRE-001 -> unit/Fowler contract test + manual QA for invalid root + static scan for typed `Result` handling.
- PRE-002 -> Lean THM-INV-001/002/003 + unit/Fowler contract tests + proptest generated malformed configs.
- PRE-003 -> Lean THM-INV-004/005 + proptest allowlist generation + mutation testing of exception predicates.
- PRE-004 -> unit/Fowler contract tests for include/exclude surfaces + manual QA with real repo sample.
- PRE-005 -> static scan forbidding unwrap/expect/panic/todo/unimplemented/dbg and ignored `Result` + cargo-careful/Miri for runtime checks.
- POST-001 -> happy-path Fowler tests + proptest over canonical-only generated inputs.
- POST-002 -> error-path Fowler tests + mutation testing for invalid spelling detection.
- POST-003 -> contract tests for exact path/line/column/class/remediation fields.
- POST-004 -> proptest with shuffled input ordering + Lean THM-INV-008.
- POST-005 -> manual QA confirms no file diffs after scan + static review of read-only shell operations.
- POST-006 -> manual QA and `moon ci`/quality-lane evidence once wired.
- INV-001 -> Lean + exact-value contract tests.
- INV-002 -> Lean + exact-value contract tests.
- INV-003 -> Lean + exact-value contract tests.
- INV-004 -> Lean + proptest + mutation testing.
- INV-005 -> Lean + boundary tests for similar but non-exception paths.
- INV-006 -> static scan for runtime-core YAML/JSON/HTTP exclusion.
- INV-007 -> static scan and clippy/source lint gates.
- INV-008 -> Lean + proptest shuffled findings.
- INV-009 -> error-path tests for unreadable/undecodable selected inputs.
- ERR-001 through ERR-007 -> Fowler error scenarios + mutation testing of each branch.

## Five-Lane Gauntlet Mapping
- `moon run :verify-fast`: formatting, source lint, static forbidden-token scans, focused naming scan tests.
- `moon run :verify-standard`: full unit/integration suite, Miri or cargo-careful where configured, coverage summary.
- `moon run :verify-deep`: proptest extended cases, fuzz/Bolero malformed text/config inputs, cargo-mutants, cargo-llvm-cov.
- `moon run :verify-proof`: Lean theorem obligations and Kani bounded panic/indexing checks.
- `moon run :verify-all`: release-critical rollup proving all prior lanes and manual QA evidence are present.

## Tool-Specific Requirements
- Lean: prove pure canonical table, exact allowlist predicate, and stable ordering only.
- Kani: bounded checks for line/column arithmetic, path segment iteration, and panic-free classification/report data construction.
- Miri/cargo-careful: run scan tests under interpreter/runtime checks for aliasing, bounds, and invalid assumptions.
- proptest: generate canonical-only content, invalid legacy occurrences, random file ordering, and malformed configs.
- cargo-fuzz or Bolero: hostile text bytes and path-like inputs for decoder/classifier boundary; invalid UTF-8 must fail closed or be excluded by documented binary detection.
- Loom/Shuttle/Lockbud: waived; scan is specified as sequential and no concurrency claim is made.
- cargo-mutants: mutants must be killed for canonical table entries, allowlist branches, error mapping, and gate failure behavior.
- cargo-llvm-cov: coverage evidence must include all error variants and all occurrence classes.
- static-scan: forbid unsafe, unwrap, expect, panic, todo, unimplemented, dbg, unchecked indexing/slicing/casts/arithmetic, ignored Result, and runtime-core YAML/JSON/HTTP dependencies.
- manual QA: run scan against the real workspace and verify expected pass/fail examples plus no repository file changes.

## Waivers
- CONCURRENCY-WAIVER-001
  - Clause IDs: POST-004, INV-008
  - Waived layer: Loom/Shuttle/Lockbud
  - Reason: the scan is specified as sequential and makes no thread interleaving, cancellation, deadlock, or shared-state claim.
  - Compensating evidence: Lean `THM-INV-008`, proptest shuffled input ordering, and deterministic report tests.
  - Owner: downstream State 2/3 implementer for `vb-37lc`.
  - Follow-up/complete condition: complete when verification evidence shows deterministic ordering for shuffled inputs; waiver expires if implementation introduces concurrency.
- PERFORMANCE-WAIVER-001
  - Clause IDs: POST-001, POST-002, POST-006
  - Waived layer: performance/assembly-ir
  - Reason: this bead makes no latency, throughput, allocation, vectorization, zero-cost, or assembly claim.
  - Compensating evidence: correctness gates remain mandatory through fast/standard/deep/proof/all verification lanes.
  - Owner: downstream State 2/3 implementer for `vb-37lc`.
  - Follow-up/complete condition: complete when no performance claim is added; waiver expires if any speed or assembly claim is introduced.
- API-COMPAT-WAIVER-001
  - Clause IDs: Contract Signatures section
  - Waived layer: api-compat
  - Reason: no public library API stability claim is made; signatures are contract targets for downstream design, not a published semver API.
  - Compensating evidence: downstream implementation must keep the scan behind the selected internal quality-gate interface and review any public API exposure separately.
  - Owner: downstream State 2/3 implementer for `vb-37lc`.
  - Follow-up/complete condition: complete when implementation remains internal; waiver expires if a public crate API or CLI compatibility guarantee is declared.
- RELEASE-PROVENANCE-WAIVER-001
  - Clause IDs: STATE-001
  - Waived layer: release-provenance
  - Reason: State 1 produces planning artifacts only and no release binary, package, SBOM, or provenance artifact.
  - Compensating evidence: release provenance remains governed by repository-wide release gates before any release artifact is produced.
  - Owner: downstream release owner for `velvet-ballastics`.
  - Follow-up/complete condition: complete when State 1 handoff occurs with no release artifact; waiver expires at release-candidate packaging.

## Independent Review Gate
- Downstream test planning, test writing, implementation, and formal proof work must not consume these artifacts until an independent reviewer writes `contract-verification-review.md` with `STATUS: APPROVED`.
