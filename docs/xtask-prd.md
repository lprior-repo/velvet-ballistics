# Xtask PRD

## Status

Draft product requirements document for extracting the `velvet-ballistics` `xtask` lessons into a reusable Rust-only AI software delivery harness.

## Summary

Xtask is an opinionated Rust-only assurance orchestrator for AI-assisted software delivery. It turns human intent and agent activity into scoped work, determines the verification and review evidence required for that work, runs or routes the required gates, and produces an explicit admission decision.

Xtask is not a generic task runner, CI wrapper, or agent chat shell. It is the software factory gate for Rust teams that want AI to write code without trusting vibes.

The product standard is:

> Latest stable Rust for production code, safe first-party code, blessed.rs-first dependency selection, audited high-performance dependencies, typed railway errors, Holzmann-bounded resources, the full go-skill delivery lifecycle, formal proof lanes where warranted, static analysis, mutation testing, fuzzing, Miri, Loom, Kani, Flux, Prusti/Creusot, Verus, TLA+, benchmark gates, and fail-closed evidence.

## Problem

AI coding agents can generate code quickly, but the acceptance process is usually informal. Teams get diffs, chat transcripts, and partial CI output. They rarely get a durable answer to the questions that matter:

- What was the intended change?
- What files and APIs were in scope?
- What commands was the agent allowed to run?
- What did the agent actually change?
- Which proof, test, static-analysis, and performance gates were required?
- Which gates actually ran?
- What raw evidence proves the result?
- Which residual risks remain?
- Is this change accepted, rejected, deferred, or waiting for human review?

Existing CI can answer whether commands passed. It does not own the AI delivery lifecycle from intent through admission.

## Product Thesis

Xtask owns AI change admission for Rust.

The goal is not to make every change run every verifier. The goal is to decide what the change needs, make that need explicit, orchestrate the right specialists and tools, and refuse admission until the required evidence exists.

Verus, TLA+, Kani, Flux, Prusti/Creusot, Loom, Miri, fuzzing, property testing, mutation testing, static analysis, black-hat review, truth-serum, and manual QA are capabilities behind the orchestrator. Xtask is the policy engine that chooses the necessary subset from scope, criticality, hazards, contracts, and claims.

The harness sits above AI agents and below the repository:

```text
human intent
  -> xtask lifecycle
  -> scoped agent execution
  -> repository diff
  -> gates / proofs / reviews / evidence
  -> admission decision
```

The core product object is not a command. It is a `ChangeAdmission` record.

## Target Users

- Rust developers using AI agents for production code.
- Staff/principal engineers who need repeatable acceptance rules for generated code.
- Safety-conscious teams that want proof, fuzzing, static analysis, and mutation gates as normal delivery tools.
- Solo builders who want one strict local harness instead of a pile of scripts, prompts, and CI fragments.
- Agent fleet operators who need machine-readable status, evidence, and failure reasons.

## Non-Goals

- Supporting non-Rust implementation languages in core.
- Being a general CI system.
- Being a generic workflow orchestrator.
- Allowing arbitrary user-defined standards to replace the core policy.
- Trusting synthetic evidence as gate evidence.
- Rewriting GitHub Actions, Buildkite, or local developer tools.
- Hiding failures behind best-effort automation.

## Product Principles

1. Rust only in first-party implementation.
2. Latest stable Rust for production code.
3. No first-party `unsafe` by default.
4. No `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` in production code.
5. Typed errors and railway flow instead of panic control flow.
6. Bounded resources: loops, retries, queues, fanout, memory growth, subprocesses, and timeouts are explicit.
7. Real evidence only: a pass requires raw command output plus validator acceptance.
8. Proofs are scoped to the blast radius of the change.
9. Performance claims require benchmarks.
10. Static analysis, mutation testing, fuzzing, and formal methods are first-class gates, not optional polish.
11. Configuration can select scope and adapters, but cannot disable the safety model without an explicit waiver artifact.
12. The CLI should dictate the lifecycle rather than expose a bag of unrelated commands.
13. The go-skill state machine is the normative delivery lifecycle for non-trivial changes.
14. blessed.rs is the default crate discovery baseline, not a substitute for local audit and measurement.
15. Assurance is risk-tiered so routine changes move quickly and high-consequence changes get NASA-grade evidence.
16. Every admitted change reduces uncertainty or explicitly records the remaining uncertainty.

## Stable Rust Policy

Production code uses the latest stable Rust toolchain.

Recommended default:

```toml
# rust-toolchain.toml
[toolchain]
channel = "stable"
profile = "minimal"
components = ["rustfmt", "clippy", "rust-src"]
targets = ["x86_64-unknown-linux-gnu"]
```

Rules:

- No `#![feature(...)]` in production crates.
- No `RUSTC_BOOTSTRAP`.
- No nightly-only production APIs.
- No first-party `std::arch` intrinsics.
- SIMD is allowed through stable compiler auto-vectorization or audited dependency APIs only.
- Miri, Verus, Kani, and other verifier lanes may install tool-specific or nightly toolchains as isolated analysis tools, but they must not leak unstable source features into production crates.
- Release evidence records the exact `rustc --version --verbose` output used for builds and gates.

## NASA-Grade Quality / Throughput Model

Xtask should pursue NASA-level assurance without turning every typo fix into a launch review. Rigor must be proportional, mechanical, and cheap to reuse.

The product separates three concerns:

- change criticality
- evidence strength
- throughput lane

The harness selects the lightest lane that can honestly discharge the risk. AI proposes the change; deterministic evidence gates dispose of it.

### Criticality Tiers

Every work item receives a criticality tier during `xtask scope`.

| Tier | Name | Examples | Required posture |
| --- | --- | --- | --- |
| C0 | Cosmetic | copy, docs, comments, non-contract screenshots | fast lane, source checks, no proof unless touched contract demands it |
| C1 | Local behavior | pure function, parser branch, CLI flag | fast lane plus targeted tests/properties |
| C2 | State or persistence | durable event log, recovery path, config migration | deep lane plus model/property/fuzz evidence |
| C3 | Concurrency or scheduling | queues, cancellation, worker lifecycle, locks | deep lane plus Loom/TLA+ obligation |
| C4 | Security or secrets | auth, capability, redaction, taint, secret storage | release lane plus threat model and negative fixtures |
| C5 | Safety kernel | admission decision, proof selection, artifact trust, replay truth | full go-skill, formal review, black-hat, truth-serum, release gate |

Rules:

- The scope engine proposes the tier; the user or policy may raise it, never silently lower it.
- A lower tier inherits a higher gate when it touches a high-criticality file or public contract.
- Criticality is stored in `delivery-scope.jsonl` and copied into `ChangeAdmission`.
- Tier changes after implementation invalidate affected proof, test, and review states.

### Throughput Lanes

Xtask needs lanes, not one giant quality wall.

| Lane | Goal | Max wait target | Typical tiers | Gate posture |
| --- | --- | --- | --- | --- |
| edit-loop | keep agent moving | seconds to minutes | C0-C1 | fmt, check, scoped clippy, focused tests |
| pre-review | stop bad PRs early | minutes | C1-C3 | fast profile, scope proof plan, property tests |
| merge-candidate | protect main | tens of minutes | C2-C4 | deep profile, static analysis, scoped formal lanes |
| release-candidate | prove shipment | hours allowed | C3-C5 | release profile, mutation, fuzz, full evidence pack |
| incident-hotfix | restore safely | bounded emergency SLA | any | minimum safe lane plus mandatory post-incident completion debt |

Incident-hotfix is not a loophole. It creates an `assurance-debt` artifact with owner, deadline, and a blocked future release if unpaid.

### Assurance Budget

Each run gets an assurance budget that defines what must be proven now and what can be deferred.

Fields:

- criticality tier
- changed files and owners
- public API impact
- state/durability impact
- security/secret impact
- concurrency impact
- performance claim impact
- dependency impact
- maximum acceptable gate time
- required evidence strength

The budget is a contract between throughput and risk, not just a timeout.

### Evidence Strength Ladder

Evidence has strength levels. Xtask reports the highest level reached for each claim.

| Level | Evidence | Meaning |
| --- | --- | --- |
| E0 | assertion only | not acceptable for admission |
| E1 | static source check | shape looks right |
| E2 | unit/integration test | behavior observed for named cases |
| E3 | property/fuzz/mutation | adversarial input or test adequacy exercised |
| E4 | model check/bounded proof | state space or bounded domain searched |
| E5 | deductive proof or temporal model plus implementation binding | contract has mathematical backing |
| E6 | operational replay/benchmark/provenance | production-relevant behavior measured and reproducible |

Release evidence exposes claim-to-evidence mapping:

```text
claim -> criticality -> required strength -> actual evidence -> decision
```

### Hazard Ledger

NASA-style assurance needs hazards, not only tests.

Xtask maintains a hazard ledger per repository and per change.

Hazard fields:

- hazard ID
- unsafe state or loss scenario
- trigger conditions
- affected assets
- severity
- likelihood
- detection method
- prevention/mitigation control
- linked proof obligations
- linked tests/fuzz/property suites
- residual risk
- owner and review date

Example hazards:

- admitted change lacks required raw evidence
- replay reconstructs a different admission decision
- secret value appears in logs or artifacts
- cancellation loses a child process or leaves a lock held
- overflow changes a scheduling or retry decision
- dependency update introduces unsafe transitive code in the trust boundary

### FMEA And FRACAS

Xtask should encode lightweight Failure Mode and Effects Analysis for C2-C5 changes.

Required FMEA fields:

- component
- failure mode
- effect
- cause
- detection
- severity
- occurrence
- detection score
- mitigation
- residual risk

FRACAS behavior:

- every escaped defect creates a failure record
- every failure record links to the missed gate or missing gate
- recurring failures raise criticality heuristics
- repaired failures add regression evidence to the affected lane

This keeps the system improving instead of repeating the same blind spots.

### Quality Ratchets

The harness should ratchet quality upward without blocking unrelated work forever.

Ratcheted metrics:

- count of forbidden constructs in first-party production code
- unclassified mutation survivors
- uncovered high-criticality hazards
- waiver count and age
- flaky gate count
- unsupported verifier findings
- dependency exceptions
- average evidence strength by criticality

Rules:

- new local debt blocks admission unless explicitly waived
- pre-existing global debt is recorded as `deferred-global`
- C4-C5 work cannot add new waivers without owner and expiration
- expired waivers block release-candidate admission

### Full Skill Quality Bar Canon

Xtask should preserve the high-value skill lifecycle, but encode it as typed product behavior instead of prompt folklore.

Required specialist roles:

- `go-skill` owns the canonical state machine and phase transitions.
- `rust-contract` writes the contract, assumptions, invariants, verification layers, and traceability targets.
- `contract-verification-reviewer` rejects weak or unbound contracts before tests or implementation consume them.
- `proof-planner` selects risk-appropriate verifier lanes and proof obligations.
- `proof-writer` writes only verification artifacts and repairs them when the implementation is wrong or the proof is weak.
- `proof-reviewer` reviews proofs and rejects vacuous, mirror-only, or unbound verification.
- `test-planner` derives tests from the contract, hazards, proof obligations, and scope.
- `test-writer` writes failing-first tests, properties, fuzz targets, and harnesses.
- `test-reviewer` rejects weak assertions, overfitted tests, nondeterminism, and missing negative paths.
- `formal-verifier` executes accepted proof obligations and classifies evidence without authoring the proof.
- `black-hat-reviewer` performs adversarial contract parity, DDD, safety, and simplicity review.
- `truth-serum` audits the evidence pack for hallucinated claims, missing raw logs, and unverifiable assertions.
- `landing-skill` owns commit, issue sync, remote push, cleanup, and final handoff.

Default lifecycle posture:

- explore before contract work
- contract before proof planning
- proof planning before test planning
- proof review before implementation
- tests before implementation where feasible
- implementation before formal execution
- formal execution before black-hat review
- black-hat review before truth-serum
- truth-serum before landing

Short-circuit rules:

- C0 doc-only changes may skip proof and test authoring when scope proves no executable behavior changed.
- C1 local behavior may use proof planning as a lightweight lane-selection artifact rather than full formal writing.
- C2-C5 changes require explicit proof/test/review artifacts or an accepted waiver.
- Any public contract, safety kernel, security, persistence, or concurrency change raises the minimum lane.

### Proof Lifecycle Rationale

The proof lifecycle is worth the work because it moves expensive discovery earlier.

Risk without early proof planning:

- tests encode the wrong model
- implementation bakes in an unprovable design
- formal tools get bolted on after the shape is already wrong
- reviewers argue about style instead of invariants

Benefit with early proof planning:

- impossible states are found before code exists
- proof obligations shape API boundaries
- test plans target the real hazards
- reviewer effort moves from taste to contract parity
- AI output is constrained by typed artifacts instead of vibes

The stack is strong but not omniscient. Xtask must never claim that tools prove the product is correct. It can claim only that selected obligations passed under stated assumptions with raw evidence.

### Formal Stack Completeness Boundary

No single Rust verification tool covers the whole problem.

Tool boundaries:

- TLA+ models temporal behavior, distributed state, scheduling, cancellation, and resource transitions.
- Verus handles deductive proof over implementation-shaped Rust where specs bind to exec logic.
- Kani explores bounded Rust executions and panic/index/overflow/state bugs.
- Flux refines local value relationships and constructor/API invariants.
- Prusti/Creusot can cover alternate deductive contracts when Verus is not the best fit.
- Loom explores concurrent interleavings.
- Miri catches undefined behavior and invalid Rust execution under interpreter-supported paths.
- Proptest and fuzzing pressure broad input spaces.
- Mutation testing tests the tests.
- Static analysis catches source and dependency hazards before runtime.

Selection rules:

- Use TLA+ when behavior depends on time, order, retries, cancellation, leases, queues, or crash/recovery.
- Use Verus or another deductive lane when a pure invariant must hold for all values, not sampled values.
- Use Kani when a bounded implementation domain can expose panics, overflows, or illegal transitions.
- Use Flux when value-level refinement is cheap and directly binds to public constructors/APIs.
- Use Loom when interleaving order can invalidate correctness.
- Use Miri when undefined behavior, raw layout assumptions, or interpreter-supported unsafe dependencies are in scope.
- Use fuzz/property tests when input shape or parser behavior is the risk.
- Use mutation when test adequacy is the risk.

Failure rules:

- A failing proof or model reveals a product/design issue until proven otherwise.
- Do not weaken the contract or harness just to satisfy the tool.
- If a proof obligation is impossible, record the rejected design and route back to contract or implementation.
- If a tool cannot express the obligation, record the limitation and choose the next strongest evidence.

### Black-Hat Review Scope

Black-hat review remains mandatory for C2-C5 and any change with a waiver.

Required questions:

- Does the implementation satisfy the accepted contract, not a weaker local interpretation?
- Did any evidence come from a fake, fixture, dry-run, or stale command?
- Can an agent bypass scope, command capability, or admission policy?
- Are there hidden panics, unchecked arithmetic, unbounded loops, or ambient secrets?
- Are tests asserting observable behavior or implementation trivia?
- Are formal artifacts bound to production logic, or are they mirror-world proofs?
- Is the simplest domain model being used, or did ceremony hide missing invariants?

Findings route back to the owning state and invalidate downstream approvals.

### Truth-Serum Evidence Audit

Truth-serum is not final QA. It is an evidence-integrity gate.

It checks:

- every claim has raw evidence
- every required artifact exists
- every tool output is current for the final diff
- every skipped gate has a policy reason
- every waiver has owner, scope, expiration, and residual risk
- every proof/test/review artifact maps to a contract item or hazard
- every benchmark claim has baseline and result evidence
- every issue state and git state agrees with the admission decision

Truth-serum can approve evidence integrity, reject it, or require human review. It must not silently convert missing evidence into a pass.

### Conditional QA

Manual QA is conditional, not a default phase.

Run hands-on QA when:

- the change affects user-visible CLI/API/UI behavior
- an incident-hotfix path was used
- gate behavior itself changed
- an external service or OS integration changed
- a release-candidate includes migration or install behavior

Do not run manual QA as a ritual for pure internal refactors with stronger machine evidence.

## Architecture

Xtask should be split into a reusable harness core and repository-specific adapters.

```text
crates/
  xtask_core          lifecycle state machine, policies, admission model
  xtask_policy        Rust/Holzmann/go-skill policy definitions
  xtask_runner        typed command execution, timeouts, logs, exit codes
  xtask_rust          cargo, clippy, nextest, miri, kani, loom, fuzz adapters
  xtask_proof         TLA+, Verus, Prusti/Creusot, Kani, Flux, Miri, Loom proof planning and ledgers
  xtask_static        Clippy, Dylint, dependency and source-policy scans
  xtask_mutation      cargo-mutants orchestration and survivor reporting
  xtask_perf          benchmark plans, Criterion output parsing, deferred profile-guided optimization eligibility
  xtask_agents        opencode, Claude Code, Codex, Cursor, and local agent adapters
  xtask_git           diff, status, scope, branch, commit metadata
  xtask_store         run event log, evidence store, replay index
  xtask_cli           user-facing command surface
  xtaskd              optional local daemon for long-running agent/gate sessions
```

The core must not depend on a specific repository, issue tracker, UI fixture, or bead ID. `velvet-ballistics` becomes one profile/adaptor, not the product kernel.

## Core Domain Model

```text
Intent
Scope
PolicySet
WorkItem
AgentRun
CommandSpec
CommandResult
ProofObligation
GateProfile
GateResult
ReviewFinding
Waiver
EvidencePack
AdmissionDecision
ChangeAdmission
```

### Admission Decisions

- `accepted`: all required gates passed and no blocking findings remain.
- `rejected`: local or regression failure blocks the change.
- `needs-human-review`: the harness cannot decide safely.
- `deferred-global`: unrelated pre-existing global debt was observed and recorded.
- `waived`: an explicit waiver artifact accepted a known risk.

## CLI Requirements

The CLI must be opinionated and lifecycle-oriented.

```bash
xtask init
xtask policy doctor
xtask work start <id-or-intent>
xtask scope
xtask contract
xtask proof-plan
xtask implement --agent opencode
xtask gates --profile fast
xtask gates --profile deep
xtask gates --profile release
xtask review
xtask evidence pack
xtask admit
xtask status --jsonl
xtask explain-failure <run-id>
xtask replay <run-id>
```

### CLI Behavior

- Every command emits human-readable output by default and supports `--jsonl` for agents.
- Every mutating command creates an event in the local run store.
- Every subprocess is represented by a typed `CommandSpec`.
- No gate command may use `sh -c` or stringly shell execution.
- Exit codes are stable and documented.
- `--dry-run` may render planned commands but must never emit passing evidence.
- Fixture-backed evidence must be labeled `fixture`, never `pass`.

## Go-Skill Lifecycle

Xtask should encode the full go-skill delivery pipeline as a first-class state machine. The CLI may expose shorter aliases, but the persisted run state uses the canonical state IDs below.

```text
State 1: Claim work, isolate workspace, capture baseline, record path proof.
State 2: Explore codebase, map touched files/APIs/crates, write delivery scope.
State 3: Write contract, assumptions, invariants, verification layers, traceability.
State 4: Plan proof obligations and verifier lanes.
State 5: Write or repair verification artifacts only.
State 6: Review proofs and contract parity; reject weak or vacuous proofs.
State 7: Plan tests from contract, scope, and approved proof obligations.
State 8: Write failing-first tests.
State 9: Review test plan and suite; reject weak assertions or missing behavior.
State 10: Implement safe Rust against accepted contract, proofs, and tests.
State 11: Execute formal obligations and machine gates, then classify failures.
State 12: Run adversarial black-hat review and route defects to owning state.
State 13: Package evidence and run truth-serum audit.
State 14: Land accepted work, sync issue state, and push to remote.
State 15: Verify landing, cleanup, and final handoff state.
```

### Lifecycle Rules

- State transitions require raw evidence, not conversational claims.
- Every required artifact must exist and be non-empty before the next state consumes it.
- Proof review rejection routes back to State 5.
- Test review rejection routes back to State 7 or State 8.
- Black-hat defects route back to the owning state and invalidate affected downstream approvals.
- Each failed gate or review loop gets at most seven attempts.
- Failures are classified as `block-local`, `block-regression`, `block-release`, `required-obligation-fail`, `deferred-global`, or `waived`.
- Red Queen is not part of the default lifecycle.
- Truth-serum approval is mandatory before landing.
- Landing is not complete until the accepted code and issue state are pushed to their remotes.

### Required Go-Skill Artifacts

Xtask must be able to persist and validate the canonical artifacts for each state:

- `STATE.md`
- `baseline-report.md`
- `codebase-map.md`
- `delivery-scope.jsonl`
- `contract.md`
- `domain-model-review.md`
- `verification-layers.md`
- `proof-obligations.jsonl`
- `traceability-matrix.jsonl`
- `proof-strategy.md`
- `proof-writer-report.md`
- `proof-evidence.md`
- `proof-review.md`
- `contract-verification-review.md`
- `test-plan.md`
- `test-writer-report.md`
- `test-plan-review.md`
- `test-suite-review.md`
- `implementation.md`
- `formal-verification-report.md`
- `verification-ledger.jsonl`
- `machine-gate-report.md`
- `regression-diff.md`
- `black-hat-review.md`
- `assurance-bundle.md`
- `truth-serum-report.md`
- `final-evidence-decision.md`
- `landing-report.md`
- `cleanup-report.md`

Xtask may store these as Markdown/JSONL files for local transparency, but the internal model should treat them as typed artifacts with schema validation and provenance.

## CommandSpec

All external command execution goes through a typed command model.

Required fields:

- program
- args
- working directory
- environment allowlist
- timeout
- expected outputs
- log path
- redaction policy
- retry policy
- required capabilities

Forbidden:

- shell string execution by default
- inherited ambient secrets by default
- unbounded stdout/stderr capture
- unbounded retries
- ignored exit codes
- pass status without raw log capture

## Gate Profiles

### Fast

Purpose: cheap local feedback for normal agent loops.

Required gates:

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --all-features`
- strict `cargo clippy` over production targets
- `cargo nextest run --workspace --all-features`
- forbidden-construct scan
- source-length and module-boundary scan
- dependency-boundary scan

### Deep

Purpose: high-confidence validation before review or merge.

Required gates:

- all fast gates
- scoped Miri lane for pure/domain/perf crates where supported
- scoped Kani harnesses from proof plan
- scoped Loom models for concurrency changes
- scoped Flux/Prusti/Creusot obligations when selected by proof plan
- proptest/property suites
- fuzz smoke for touched parsers/decoders/admission logic
- static analysis lane
- dependency policy lane
- coverage report

### Release

Purpose: fail-closed admission for shipped changes.

Required gates:

- all deep gates
- TLA+ model checks for temporal/concurrency/resource obligations
- Verus proof checks for deductive obligations
- Flux/Prusti/Creusot proof checks when selected by proof obligations
- full cargo-mutants or scoped mutation gate
- cargo-audit
- cargo-deny
- cargo-vet where configured
- cargo-geiger dependency unsafe report
- cargo-machete unused dependency scan
- cargo-hack feature-power-set check
- cargo-semver-checks for public API changes
- benchmark gate for performance-sensitive changes
- release provenance bundle

## Formal Verification Lanes

Xtask does not run every formal tool for every change. It selects proof obligations from scope.

### TLA+

Use for:

- lifecycle state machines
- admission protocols
- queue scheduling
- retry/cancel/finalize behavior
- crash/recovery flows
- bounded resource state transitions

Requirement:

- specs must model bounded machine limits where relevant
- overflow/error states must be explicit
- TLC output must be captured as evidence

### Verus

Use for:

- pure Rust invariants
- state transition kernels
- bounded arithmetic contracts
- proof-carrying constructors

Requirement:

- proof artifacts must bind to production logic, not standalone mirrors
- trusted boundaries are listed in evidence

### Kani

Use for:

- bounded panic freedom
- bounded state-machine transitions
- index/access safety
- parser/admission invariants

Requirement:

- no hardcoded single-shape proofs for core structures
- use `kani::Arbitrary` or exhaustive bounded generation where practical

### Flux

Use for:

- refinement-friendly local invariants
- length and bound relationships
- typestate-like value constraints
- illegal-state exclusion at construction boundaries

Requirement:

- refinements must bind to constructors or public APIs
- tautological refinements are rejected

### Prusti / Creusot

Use for:

- deductive Rust verification when Verus is not the right fit
- preconditions and postconditions
- loop invariants
- panic/overflow and functional properties where supported

Requirement:

- partial-correctness limitations must be recorded
- shell behavior and termination assumptions must be explicit

### Miri

Use for:

- pure logic crates
- parser/decoder logic
- data layout sensitive code
- dependency-sensitive undefined-behavior checks where supported

Requirement:

- Miri failures are classified as local, regression, unsupported dependency, or deferred global

### Loom

Use for:

- cancellation races
- worker shutdown
- bounded queues
- command-runner coordination
- durable event append ordering

Requirement:

- each concurrency primitive has a named model or a documented waiver

## Static Analysis

Static analysis is a first-class lane.

Required tools:

- rustc warnings as errors
- Clippy hard-deny profile
- Dylint custom lints for Xtask-specific policy
- cargo-deny for dependency policy
- cargo-audit for known vulnerability reports
- cargo-vet for supply-chain trust where configured
- cargo-geiger for unsafe dependency inventory
- cargo-machete for unused dependencies
- cargo-semver-checks for public API compatibility
- cargo-hack for feature combinations

Optional text/policy scans:

- Semgrep for repository policy and generated-artifact patterns
- custom `rg` scans for forbidden strings, secrets, or architecture drift

Static analysis findings must be classified:

- `block-local`
- `block-regression`
- `block-release`
- `deferred-global`
- `waived`

## Mutation Testing

Mutation testing uses `cargo-mutants`.

Requirements:

- Fast profile does not run mutation testing.
- Deep profile may run scoped mutation tests for touched crates.
- Release profile requires mutation evidence unless a waiver explains scope, runtime, or tool limitation.
- Survivors are not automatically acceptable.
- Every surviving mutant is classified as killed-later, equivalent, out-of-scope, or test-gap.
- Test-gap survivors create follow-up work.

## Fuzzing And Property Testing

Required targets:

- config parsing
- command spec parsing
- evidence parsing
- admission decision state machine
- redaction
- artifact digesting
- binary event log decoding
- proof obligation selection

Rules:

- fuzz smoke runs in deep profile for touched fuzzable surfaces
- release profile records corpus path, duration, and command output
- proptest covers boundary values, invalid states, and resource-limit edges
- fuzzers and properties must assert typed errors, not just non-crash behavior

## Performance Policy

Xtask is performance-conscious but does not optimize by vibes.

Rules:

- no performance claim without benchmark evidence
- no new hot-path dependency without benchmark or design justification
- no custom allocator without heap profile evidence
- no Rayon without CPU-bound workload and scaling evidence
- no Tokio CPU-heavy loops
- no unbounded spawn/fanout/channel
- no `async_trait`, `Box<dyn Trait>`, `Arc<Mutex<_>>`, `clone`, formatting, or heap allocation in hot paths unless justified

Stable Rust performance levers:

- data-oriented design
- bounded collections
- `try_reserve` for fallible growth
- dense IDs and prevalidated artifacts
- `criterion` benchmarks
- `iai-callgrind` where available for CI-friendly instruction regression
- deferred PGO only for future representative production workloads
- audited high-performance dependencies

SIMD policy under stable Rust:

- prefer compiler auto-vectorization first
- use audited safe dependency APIs where SIMD is required
- do not write first-party unsafe SIMD
- do not require nightly `portable_simd` in production code until stable

## Dependency Policy

Xtask is library-heavy but dependency-strict.

blessed.rs is the default crate shortlist for common Rust ecosystem needs. A blessed.rs recommendation is not automatic approval. It means the candidate starts in the preferred evaluation lane instead of the exception lane.

### blessed.rs Selection Rules

- Prefer blessed.rs-recommended crates for common categories before searching crates.io broadly.
- Use latest compatible crate versions by default.
- Commit `Cargo.lock` for binaries and harness workspaces.
- Record whether a dependency is blessed.rs-recommended, house-standard, or exception-approved.
- House-standard crates may override blessed.rs when Xtask has stricter evidence or domain needs.
- Exception-approved crates require a written reason, maintenance check, license check, unsafe/dependency review, and replacement plan.
- blessed.rs does not bypass cargo-audit, cargo-deny, cargo-vet, cargo-geiger, cargo-machete, feature review, or benchmark gates.

### Initial Dependency Baseline

| Need | blessed.rs baseline | Xtask policy |
| --- | --- | --- |
| CLI parsing | `clap`, `lexopt`, `pico-args` | Default to `clap` for UX; use `lexopt` for tiny hot tools. |
| Errors | `thiserror`, `anyhow`, `color-eyre` | Use `thiserror` in libraries; prefer typed errors over `anyhow` in core; allow rich app diagnostics at CLI boundary. |
| Logging/tracing | `tracing`, `log` | Default to `tracing`; `log` only for compatibility. |
| Serialization | `serde`, `serde_json`, `toml`, `postcard`, `rkyv` | Use JSONL for agent output, TOML for local config, `postcard` for compact internal artifacts; `rkyv` requires audit. |
| Filesystem walking | `ignore`, `walkdir`, `globset` | Prefer `ignore` plus `globset` for repo-aware scans. |
| Temp files | `tempfile` | Allowed for tests and controlled local scratch. |
| IDs | `uuid` | Allowed; sortable IDs need separate house standard. |
| Digests | `blake3`, `sha2` | Prefer `blake3` for artifact identity unless interoperability requires `sha2`. |
| Secrets | `zeroize` | Pair with house-standard secret wrapper such as `secrecy`. |
| Async runtime | `tokio` | I/O shell only; no CPU-heavy work on async workers. |
| HTTP client | `reqwest`, `ureq` | Prefer `ureq` for simple sync calls and `reqwest` for async/Tokio adapters. |
| HTTP server | `axum`, `actix-web` | Prefer `axum` for service mode; benchmark before choosing performance-specific alternatives. |
| Channels | `crossbeam-channel`, `flume`, `tokio` | Use bounded channels only unless waiver explains the bound substitute. |
| CPU parallelism | `rayon` | Use only with workload and scaling evidence. |
| Fixed/inline buffers | `arrayvec`, `smallvec`, `tinyvec` | Prefer fixed capacity when bounds are known; justify spill-to-heap behavior. |
| Benchmarking | `criterion`, `divan`, `gungraun`, `hyperfine` | Use Criterion or Divan for local benches; use Gungraun/Iai-style tools for stable instruction regression where practical. |
| Profiling | `cargo-flamegraph`, `dhat`, `cargo-show-asm` | Use when performance claims need CPU, heap, or assembly evidence. |
| Testing | `cargo-nextest`, `insta` | Default to nextest; use snapshots only for stable contracts and reports. |
| Release automation | `cargo-release`, `release-plz` | Optional; Xtask admission remains separate from publish automation. |
| Cross compilation | `cross`, `cargo-zigbuild` | Optional adapter tools; do not assume cross-target behavior without CI evidence. |

### House-Standard Overrides

- `miette` is allowed at CLI/user diagnostic boundaries even though the blessed.rs error section highlights `anyhow` and `color-eyre`.
- `fjall` is the preferred local durable store when append-only files are not enough.
- `cargo-mutants`, `cargo-fuzz`, Kani, Verus, TLA+, Loom, and Miri are required verification tools even when not represented as blessed.rs application dependencies.
- `Dylint` is the preferred custom static-analysis path for Xtask-specific Rust policy.

Preferred crates by role:

- CLI: `clap`, `miette`, `thiserror`
- serialization: `serde`, `serde_json` for JSONL, `postcard` for compact binary evidence/events
- config: `toml` or `toml_edit`
- storage: `fjall` for local durable run/evidence stores
- hashing/digests: `blake3`
- time: `jiff` or a documented project-standard time crate
- IDs: `uuid` or a documented project-standard sortable ID crate
- observability: `tracing`, `tracing-subscriber`, OpenTelemetry exporter crates
- secrets: `secrecy`, `zeroize`
- concurrency: `crossbeam`, `rayon`, `tokio` only by layer and workload
- testing: `proptest`, `insta`, `trybuild`, `cargo-nextest`, `cargo-fuzz`, `cargo-mutants`

Rules:

- use latest compatible crate versions by default
- commit `Cargo.lock`
- forbid duplicate dependency families unless justified
- audit high-unsafe or high-criticality dependencies before acceptance
- keep fast hashers away from adversarial/user-controlled keys unless threat-modeled
- record dependency policy evidence in release profile

## Evidence Model

Evidence is append-only and replayable.

Each `EvidencePack` contains:

- intent
- scope
- agent identity
- repository state before and after
- command specs
- command outputs and exit codes
- gate results
- proof obligations and proof results
- review findings
- waivers
- performance evidence if claimed
- admission decision
- residual risks

Evidence status rules:

- `pass`: command ran, validator accepted output, raw log exists
- `fail`: command ran and failed or validator rejected output
- `skipped`: allowed only with explicit reason and profile rules
- `unsupported`: tool cannot run in environment; may block depending on profile
- `fixture`: synthetic or fixture-backed output, never release evidence by itself

## Agent Integration

Xtask should control agents rather than be controlled by them.

Agent adapters provide:

- launch command
- allowed filesystem scope
- allowed shell capabilities
- prompt/context injection
- progress events
- produced diff capture
- transcript capture where available
- failure normalization

Supported initial adapters:

- opencode
- Claude Code
- Codex
- Cursor
- local command adapter for tests

Agent-generated code is never accepted directly. It enters the admission lifecycle.

## Security And Capability Model

Capabilities are explicit.

Examples:

- read repository
- write scoped files
- run cargo commands
- run verifier tools
- access network
- access secrets
- mutate git state
- publish release artifacts

Rules:

- default deny for secrets and network
- secrets are redacted in logs and evidence
- commands receive environment variables from an allowlist
- capability use is recorded in events
- release publishing requires explicit capability and identity evidence

## Local Daemon

`xtaskd` is optional but expected for serious use.

Responsibilities:

- durable run queue
- cancellation
- retries
- bounded concurrency
- event streaming
- long-running verifier/fuzzer sessions
- agent process supervision
- local IPC

Non-goals:

- distributed cluster scheduler in v1
- SaaS control plane in v1
- arbitrary workflow engine in v1

## Storage

The store records events, artifacts, and evidence packs.

Requirements:

- append-only run events
- crash-safe writes
- digest-addressed artifacts
- bounded log sizes or externalized large blobs
- redaction before persistence for sensitive outputs
- replay from event log to `ChangeAdmission`

## Observability

Every run emits tracing spans.

Required span hierarchy:

```text
xtask.run
  xtask.scope
  xtask.agent
  xtask.command
  xtask.gate
  xtask.proof
  xtask.review
  xtask.admission
```

Release evidence records trace/export status. OTLP export is optional in local mode and required in managed/service mode.

## MVP Requirements

MVP must support one Rust repository end-to-end:

1. Initialize policy files.
2. Start a work item from a human intent string.
3. Compute changed-file scope from git.
4. Generate required gate profile from scope.
5. Launch one configured agent adapter.
6. Capture diff and command transcript.
7. Run real fast-profile gates.
8. Emit JSONL status.
9. Pack evidence.
10. Produce an admission decision.

MVP must not emit synthetic pass evidence.

## Roadmap

### Phase 1: Local Harness Kernel

- typed domain model
- stable CLI lifecycle
- command runner
- fast profile
- evidence pack
- JSONL status
- git scope adapter

### Phase 2: Proof And Static Analysis

- proof obligation planner
- Kani adapter
- Flux adapter
- Prusti/Creusot adapter
- Loom adapter
- Miri adapter
- Dylint/static-analysis lane
- cargo-deny/audit/vet/geiger/machete integration

### Phase 3: Agent Control

- opencode adapter
- transcript capture
- scoped filesystem write policy
- agent run event stream
- explain-failure output

### Phase 4: Deep And Release Profiles

- TLA+ adapter
- Verus adapter
- fuzz smoke
- mutation testing
- cargo-hack
- cargo-semver-checks
- benchmark evidence
- release provenance

### Phase 5: Daemon Mode

- `xtaskd`
- local IPC
- durable run queue
- cancellation
- bounded concurrency
- event replay

## Acceptance Criteria For Xtask Product

Xtask is ready to use on real Rust repositories when:

- production code builds on latest stable Rust without source feature gates
- `xtask gates --profile fast` runs real commands and captures raw logs
- `xtask admit` fails closed when any required evidence is missing
- agent adapters cannot silently exceed declared scope
- command execution never uses shell strings by default
- proof plans are generated from changed-file scope
- Miri/Kani/Flux/Prusti/Loom/TLA+/Verus results are captured as typed evidence where configured
- mutation survivors are reported and classified
- static-analysis findings are classified by blast radius
- performance claims require benchmark evidence
- every run can be replayed into the same admission decision

## Open Questions

- Should the public binary name be `xtask`, `harness`, or a distinct product name?
- Should `xtaskd` be bundled in v1 or delayed until after local CLI admission is proven?
- Should the first issue-tracker adapter be beads only, or should GitHub Issues/Linear/Jira land in v1?
- Should Verus/TLA+ proof requirements be configured per repository, or should Xtask ship strict default proof heuristics?
- Should evidence storage use Fjall in v1 or start with append-only files and migrate later?

## Product Positioning

Xtask is the Rust-only operating system for AI software delivery.

It is for teams that want AI speed with mission-critical acceptance discipline:

```text
stable Rust
safe first-party code
typed errors
bounded resources
formal proof lanes
static analysis
mutation testing
fuzzing
benchmarks
real evidence
fail-closed admission
```

The product promise:

> AI can propose the code. Xtask decides whether the code is allowed to become software.
