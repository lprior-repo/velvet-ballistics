# Verification Layers: vb-kkvb

## Boundary
- Verified kernel: command registry data, typed route mapping, structured status schema, placeholder status classification, and dependency-boundary validation logic.
- Lean contract projection: pure registry, route, and status-schema properties only.
- Runtime shell: Clap parser, process argv, stdout/stderr, file/manifests, subprocess/tool invocation, and exit status.
- External systems excluded from formal proof: OS, terminal, Moon/Cargo execution, network, clocks, and future quality tools.

## Five-Lane Gauntlet Mapping
- `moon run :verify-fast`: compile, unit/integration smoke, lint/static source scans for immediate feedback.
- `moon run :verify-standard`: standard tests plus proptest and dependency-boundary checks.
- `moon run :verify-deep`: Miri/cargo-careful, fuzz/Bolero where command text or manifests are parsed, mutation, coverage, and manual QA evidence collation.
- `moon run :verify-proof`: Lean and Kani obligations for pure routing/schema invariants and panic-freedom.
- `moon run :verify-all`: release-quality aggregate for all above lanes.

## Layer Assignment
- PRE-001 -> integration test + manual QA + static scan for stdin/TTY/editor/network prompt sources.
- PRE-002 -> proptest over command names + Kani bounded route lookup + Clap integration scenarios + Bolero/cargo-fuzz hostile argv exploration.
- PRE-003 -> unit/integration validation tests + Bolero/cargo-fuzz malformed option exploration + mutation.
- PRE-004 -> static dependency scan + cargo tree/cargo deny evidence + gauntlet-standard.
- PRE-005 -> Rust type checking + Miri for ownership-sensitive paths.
- POST-001 -> Lean + integration help-output test + mutation.
- POST-002 -> Lean + Kani + proptest.
- POST-003 -> Lean + renderer contract tests + schema snapshot/approval test.
- POST-004 -> Lean + integration scenario + mutation.
- POST-005 -> integration CLI scenario + manual QA + Bolero/cargo-fuzz hostile command-name exploration + mutation.
- POST-006 -> integration CLI scenario + proptest invalid options + Bolero/cargo-fuzz malformed option exploration + mutation.
- POST-007 -> regression tests for legacy commands + cargo llvm-cov coverage.
- POST-008 -> static dependency scan + cargo deny/tree evidence + gauntlet-standard.
- INV-001 -> Lean + Kani + proptest.
- INV-002 -> Lean + Kani + proptest duplicate registry generation.
- INV-003 -> Lean + static golden list test.
- INV-004 -> Lean + static scan rejecting stringly post-parse dispatch patterns by review.
- INV-005 -> clippy lint gates + static source scans for forbidden constructs + cargo-mutants.
- INV-006 -> Lean + schema tests + proptest over command families.
- INV-007 -> manual QA + static scan for stdin reads/editor/spawn prompts.
- INV-008 -> dependency scan + cargo deny + cargo tree diff.
- INV-009 -> `unsafe_code = forbid` + cargo geiger/static unsafe scan.
- ERR-001 -> integration error scenario + Bolero/cargo-fuzz hostile command-name exploration + mutation.
- ERR-002 -> integration error scenario + unit validation test + Bolero/cargo-fuzz malformed option exploration + mutation.
- ERR-003 -> proptest invalid input + integration scenario + Bolero/cargo-fuzz malformed option exploration.
- ERR-004 -> renderer failure injection test + Miri/cargo-careful for I/O boundary handling.
- ERR-005 -> static dependency-boundary test + cargo tree evidence.
- ERR-006 -> integration placeholder scenario + mutation.
- ERR-007 -> Kani/proptest registry validation + mutation.

## Parser/Codec/Fuzz Scope
- The command-name and option parser boundary must receive executable Bolero or cargo-fuzz coverage for arbitrary command names, malformed option shapes, missing values, duplicate flags, delimiter-like tokens, and invalid Unicode-adjacent values where Rust APIs permit them.
- Required downstream obligation: a fuzz/Bolero target exercises the xtask parse wrapper, not Clap internals directly, and asserts no panic plus fail-closed `UnknownCommand`, `MissingRequiredInput`, or `InvalidInput` classification.
- Full Clap internals are out of scope; the Rust wrapper behavior must still prove unknown and malformed inputs fail closed.

## Concurrency Scope
- No concurrency is required by this bead. Loom/Shuttle/Lockbud are waived for vb-kkvb unless implementation introduces threads, async tasks, channels, locks, or subprocess fan-out.

## Performance Scope
- No speedup claim is made. Performance evidence is limited to a non-regression smoke budget for help and placeholder commands if downstream agents choose to add it.

## API Compatibility Scope
- Public Rust API stability is not the external contract. CLI command spelling and output field names are the compatibility surface; test via CLI scenarios and golden schema checks.

## Release Provenance Scope
- Because `xtask` participates in quality automation, `cargo deny`, supply-chain scans, and SBOM/auditable evidence belong in `verify-all` before release-critical use.

## Waivers
- CONC-WAIVER-001:
  - Contract clauses: PRE-001, POST-005, POST-006, INV-007, ERR-001, ERR-002, ERR-003, ERR-004, ERR-006
  - Verification layer waived: Loom/Shuttle/Lockbud
  - Owner: contract agent for State 1; downstream owner is vb-kkvb implementer
  - Reason: contracted behavior is synchronous CLI routing and output with no threads, async tasks, locks, channels, cancellation, or shared mutable concurrent state.
  - Compensating evidence: static scan for thread/async/channel/lock usage plus integration and manual CLI scenarios.
  - Expiry/follow-up: expires immediately if implementation introduces threads, async tasks, channels, locks, subprocess fan-out, or background workers; then add Loom/Shuttle/Lockbud obligations before merge.
- PERF-WAIVER-001:
  - Contract clauses: all PRE/POST/INV/ERR clauses for vb-kkvb
  - Verification layer waived: performance and assembly-ir
  - Owner: contract agent for State 1; downstream owner is vb-kkvb implementer
  - Reason: this bead makes no speed, allocation, vectorization, zero-cost, or code-size claim.
  - Compensating evidence: functional CLI smoke tests and gauntlet-fast/standard/deep as applicable.
  - Expiry/follow-up: expires if any performance, latency, allocation, binary-size, or assembly claim is added; then add exact benchmark/assembly obligations with baseline and threshold.
- LEAN-SHELL-WAIVER-001:
  - Contract clauses: PRE-001, PRE-002, PRE-003, PRE-004, PRE-005, POST-005, POST-006, POST-007, POST-008, INV-005, INV-007, INV-008, INV-009, ERR-001, ERR-002, ERR-003, ERR-004, ERR-005
  - Verification layer waived: Lean for Rust/runtime shell behavior only
  - Owner: contract agent for State 1; downstream owner is vb-kkvb implementer
  - Reason: Lean is scoped to pure deterministic registry, route, and structured-status schema kernels; I/O, Clap shell behavior, manifests, stdout/stderr, process exits, and lint/toolchain properties are outside theorem scope.
  - Compensating evidence: Bolero/cargo-fuzz, Kani, proptest, Miri/cargo-careful, integration tests, static scans, cargo-deny/tree/geiger, mutation, coverage, manual QA, and gauntlet lanes assigned above.
  - Expiry/follow-up: expires for any listed clause if its behavior is factored into a pure deterministic kernel; then add a Lean theorem obligation and traceability row before downstream implementation consumes the changed contract.

## Required Evidence Artifacts
- `formal-verification-report.md` with `moon run :verify-fast`, `:verify-standard`, `:verify-deep`, `:verify-proof`, and `:verify-all` results as applicable.
- CLI transcript proving help includes all required families.
- CLI transcript proving representative placeholder command emits structured non-interactive status.
- CLI transcript proving unknown command exits non-zero with diagnostic.
- Dependency-boundary report proving runtime core crates did not gain `xtask`, Clap, JSON/YAML/HTTP, or tooling-only dependencies.
- Independent `contract-verification-review.md` with `STATUS: APPROVED` before implementation consumes this plan.
