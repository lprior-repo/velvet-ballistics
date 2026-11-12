# Verification Layers: vb-5xs4

## Boundary
- Verified kernel: discovery normalization, loop-pattern classification, disposition validation, and report validation contracts.
- Lean contract projection: pure classification and disposition algebra only.
- Runtime shell: filesystem, parser integration, report writing, bead creation/update commands, Moon lanes, and manual workflow.
- External systems excluded from formal proof: bd/Dolt/git/Moon process behavior, filesystem permissions, OS scheduling, and mutation engine internals.

## Layer Assignment
- PRE-001 -> manual-qa + static-scan: workspace/root unreadable paths return `WorkspaceUnreadable`.
- PRE-002 -> proptest + kani: arbitrary paths outside bounded roots are rejected without traversal.
- PRE-003 -> static-scan + proptest: external/vendor/generated paths are excluded unless whitelisted.
- PRE-004 -> fuzz + miri: arbitrary file bytes/text never panic and produce typed errors.
- PRE-005 -> Fowler error scenarios + mutation: missing assignment sink fails closed.
- PRE-006 -> unit/Fowler contract scenarios + mutation: missing/invalid labeling policy fails classification.
- POST-001 -> proptest + coverage: every risky pattern appears with path, location, kind, reason, and action.
- POST-002 -> lean + kani + proptest: exactly one disposition for every risky finding.
- POST-003 -> lean + kani + proptest + fuzz/bolero + mutation: unlabeled/ambiguous failure context maps to repair required, and Rust realization matches the pure label-sufficiency predicate.
- POST-004 -> Fowler tests + coverage: exceptions include reason, scope, owner, expiry/review trigger.
- POST-005 -> lean (THM-POST-005) + kani + proptest + coverage + mutation: safe labeling proof includes behavior and case evidence, and Rust constructors cannot create safe proof without both fields.
- POST-006 -> lean + proptest: same normalized input and policy produce identical output.
- POST-007 -> lean + kani + mutation: quality gate fails closed for unassigned risky findings.
- POST-008 -> proptest + mutation: non-risky loops do not suppress risky findings.
- INV-001 -> lean + kani + mutation.
- INV-002 -> lean + kani + proptest.
- INV-003 -> lean + kani + proptest + fuzz/bolero + mutation.
- INV-004 -> lean + proptest.
- INV-005 -> static-scan + manual-qa + mutation: deletion is never counted as repair.
- INV-006 -> manual-qa + static-scan: no mutation-improvement claim without mutation evidence.
- INV-007 -> static-scan: runtime core contains no YAML, JSON, or HTTP dependencies for this feature.
- INV-008 -> static-scan + cargo-llvm-cov: all fallible paths use typed `Result<T, InventoryError>`.
- ERR-001 -> manual-qa + coverage: unreadable workspace maps to `WorkspaceUnreadable`.
- ERR-002 -> kani + proptest: out-of-scope roots map to `InputRootOutOfScope` before traversal.
- ERR-003 -> manual-qa + coverage: unreadable candidate file maps to `FileReadFailed`.
- ERR-004 -> fuzz/bolero + coverage: invalid UTF-8 maps to `InvalidUtf8` without panic.
- ERR-005 -> fuzz/bolero + coverage: unrecoverable Rust syntax maps to `ParseFailed`.
- ERR-006 -> lean (THM-ERR-006) + kani + proptest + mutation: ambiguous labels map to `AmbiguousCaseLabel` or repair-required, never safe proof.
- ERR-007 -> lean + kani + mutation: unassigned risky pattern maps to `UnassignedRiskyPattern`.
- ERR-008 -> lean + kani + mutation: duplicate disposition maps to `ConflictingDisposition`.
- ERR-009 -> manual-qa + mutation: destructive deletion maps to `DestructiveChangeDetected`.
- ERR-010 -> fuzz/bolero + manual-qa: untraceable generated source maps to `UnsupportedGeneratedSource`.
- ERR-011 -> static-scan + manual-qa: repository rule breach maps to `PolicyViolation`.

## Required Evidence Commands
- Fast lane: `moon run :verify-fast` for formatting, static scans, and fast contract checks.
- Standard lane: `moon run :verify-standard` for unit/integration/proptest coverage expected by downstream implementation.
- Deep lane: `moon run :verify-deep` for fuzz, Miri/cargo-careful, mutation, and coverage evidence.
- Proof lane: `moon run :verify-proof` for Lean/Kani obligations over classification and disposition kernels.
- Full lane: `moon run :verify-all` before closing the parent mutation-refresh quality chain.
- If these lanes are not yet wired, downstream agents must use the formal-verifier templates referenced by the `rust-contract` skill and record the gap as a bead.

## Layer Details
- Lean: prove disposition completeness, exactly-one disposition, label sufficiency, monotonic evidence refinement, deterministic classification, and fail-closed gate logic.
- Kani: bounded checks for state constructors, disposition validation, path-scope predicates, and panic-free validation transitions.
- Miri/cargo-careful: runtime checks for parser/scanner integration and allocation/aliasing-sensitive traversal code.
- Proptest: generated inventories, labels, paths, and pattern lists to explore invariants.
- Fuzz/Bolero: hostile Rust-like source text, malformed UTF-8 boundaries, macro-shaped input, and parser recovery surfaces.
- Loom/Shuttle/Lockbud: waived unless implementation introduces concurrency; if parallel scanning is added, this becomes mandatory.
- cargo-mutants: kill mutants that remove assignment validation, weaken label sufficiency, invert path-scope checks, or treat deletion as repair.
- cargo-llvm-cov: prove all error variants and invariant branches are exercised.
- Static scans: reject `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`, unchecked indexing/slicing/casts/arithmetic, and runtime-core YAML/JSON/HTTP use.
- Manual QA: inspect generated inventory against representative real `tests/**` and `crates/**` surfaces and confirm repair/exception/proof assignments are actionable.

## Waivers
- WAIVER-001:
  - Clauses: PRE-001, PRE-003, PRE-004, ERR-001, ERR-003, ERR-004, ERR-005, ERR-010.
  - Waived layer: Lean.
  - Reason: filesystem traversal, file decoding, parser recovery, macro/source mapping, and OS errors are runtime-shell behavior, not pure deterministic kernel behavior.
  - Compensating evidence: manual-qa, fuzz/bolero, Miri/cargo-careful, coverage, and static-scan obligations listed for those clauses.
  - Owner: downstream vb-5xs4 implementation owner.
  - Expiration/follow-up: expires if a pure parser/normalizer model is introduced; then a Lean parser/normalizer theorem or explicit renewed waiver is required.
- WAIVER-002:
  - Clauses: POST-001, POST-004, INV-006.
  - Waived layer: Lean.
  - Reason: report rendering and human-readable exception/report metadata are shell presentation concerns.
  - Compensating evidence: Fowler scenarios, cargo-llvm-cov branch evidence, mutation checks for omitted metadata, and manual QA against real `tests/**` and `crates/**` inventory output.
  - Owner: downstream vb-5xs4 implementation owner.
  - Expiration/follow-up: expires if report validation becomes a pure typed renderer with semantic formatting claims; then add Lean or Kani renderer obligations.
- WAIVER-003:
  - Clauses: all clauses in this bead.
  - Waived layer: Loom/Shuttle/Lockbud.
  - Reason: State 1 contract requires a sequential inventory workflow; no concurrency, async tasks, shared mutable state, or background workers are in scope.
  - Compensating evidence: static review for absence of concurrency primitives plus `moon run :verify-fast`; if implementation adds parallel scanning, this waiver is void.
  - Owner: downstream vb-5xs4 implementation owner.
  - Expiration/follow-up: immediately expires upon adding parallel scanning, async execution, channels, mutexes, atomics, background workers, or shared mutable state.
- WAIVER-004:
  - Clauses: all clauses in this bead.
  - Waived layers: performance, assembly-ir, api-compat, release-provenance.
  - Reason: this bead makes no speed, zero-cost abstraction, vectorization, public API compatibility, or release artifact claim.
  - Compensating evidence: explicit non-goal in `contract.md` plus normal gauntlet evidence for correctness layers.
  - Owner: downstream vb-5xs4 implementation owner.
  - Expiration/follow-up: expires if downstream work adds performance/API/release claims; then exact benchmark, assembly, semver, SBOM, or provenance obligations are mandatory.

## Review Gate
- Independent review must write `contract-verification-review.md` with `STATUS: APPROVED` before downstream states consume these artifacts.
