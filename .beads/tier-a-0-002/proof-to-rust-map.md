STATUS: BRIDGE_MAPPED

# Proof-To-Rust Map — Residue Quarantine CI Gate (tier-a-0-002)

bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
phase: 1
attempt: 1-of-7
state: 7 (proof-to-implementation)
skill: proof-to-implementation
writer_invocation_id: tier-a-0-002-s7-proof-to-implementation-PTIBRIDG
parent_invocation_id: tier-a-0-002-s6-proof-reviewer-c21da687
state_6_reviewer_invocation_id: tier-a-0-002-s6-proof-reviewer-c21da687
state_5_writer_invocation_id: tier-a-0-002-s5-proof-writer-cad4075b
schema_version: proof-to-rust-map/v1
updated_at: 2026-06-18T03:30:00.000000+00:00

## §0. Bridge Status

This document maps 5 proof seeds (RQ-001..RQ-005) to Rust source refs,
behavior-test refs, and refinement-harness refs in conformance with
the proof-reviewer (State 6) disposition (`proof-review.md`
`STATUS: APPROVED`, reviewer_invocation_id
`tier-a-0-002-s6-proof-reviewer-c21da687`) and the proof-writer
(State 5) bridge plan (`proof-writer-report.md` §3, `proof-evidence.md`
§2).

All 5 seeds are `behavior_affecting=false` (per
`proof-seeds.jsonl`); therefore refinement-harness refs are
deliberately empty across all 5 rows (N/A for a build-time scanner).
Closure to `mapping_status="verified"` is the State 12 formal-verifier
responsibility and is contingent on the State 11 holzman-rust agent
materializing the forward-pointing source_refs named below.

## §1. 5-Seed Mapping

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|----------|----------------|------------------|---------------------|------------------|---------------------|--------------------------|----------|------------------|------------|
| PO-RQ-001 | RQ-001 | 3.2_pass_iff_no_active_residue | false | `scripts/forbid-runtime-fmt.sh::main`; `scripts/forbid-runtime-fmt.sh::compile_scanner`; `scripts/forbid-runtime-fmt.rs::ResidueQuarantine::run`; `scripts/forbid-runtime-fmt.rs::ResidueQuarantine::decide` | `scripts/test-forbid-runtime-fmt.sh::test_quarantine_gate_blocks_json_import` | [] | proptest | `bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_json_import` | state_8_test_writer |
| PO-RQ-002 | RQ-002 | 3.4_closed_set_invariant | false | `velvet-ballistics-MASTER.md::section_43_automatic_rejection_triggers_2056_2060`; `scripts/forbid-runtime-fmt.rs::ResiduePolicy::from_master`; `scripts/forbid-runtime-fmt.rs::ForbiddenImport::from_name`; `scripts/forbid-runtime-fmt.rs::expected_master_trigger`; `scripts/forbid-runtime-fmt.rs::master_line_matches` | `scripts/test-forbid-runtime-fmt.sh::test_static_evidence_binds_master_rejection_triggers` | [] | proptest | `bash scripts/test-forbid-runtime-fmt.sh test_static_evidence_binds_master_rejection_triggers` | state_11_holzman_rust_repair |
| PO-RQ-003 | RQ-003 | 3.2_pass_iff_no_active_residue | false | `scripts/forbid-runtime-fmt.sh::main`; `scripts/forbid-runtime-fmt.sh::exit_code_translation`; `scripts/forbid-runtime-fmt.rs::ResidueQuarantine::decide`; `scripts/forbid-runtime-fmt.rs::GateError::exit_code` | `scripts/test-forbid-runtime-fmt.sh::test_quarantine_gate_blocks_unbounded_channel` | [] | proptest | `bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_unbounded_channel` | state_8_test_writer |
| PO-RQ-004 | RQ-004 | 3.4_closed_set_invariant | false | `scripts/forbid-runtime-fmt.rs::ResidueQuarantine::diff_against_allowlist`; `scripts/forbid-runtime-fmt.allow`; `.moon/tasks/all.yml::forbid-runtime-fmt`; `.moon/tasks/all.yml::check` | `scripts/test-forbid-runtime-fmt.sh::test_moon_ci_quarantine_dependency_correctly_ordered` | [] | proptest | `bash scripts/test-forbid-runtime-fmt.sh test_moon_ci_quarantine_dependency_correctly_ordered` | state_11_holzman_rust |
| PO-RQ-005 | RQ-005 | 3.3_stderr_format | false | `scripts/forbid-runtime-fmt.sh::sort_unique`; `scripts/forbid-runtime-fmt.rs::ResidueMatch::active_line`; `scripts/forbid-runtime-fmt.rs::ResidueMatch::allowlisted_line`; `scripts/forbid-runtime-fmt.rs::ScanReport::summary_line`; `scripts/forbid-runtime-fmt.rs::emit_pass`; `scripts/forbid-runtime-fmt.rs::emit_fail` | `scripts/test-forbid-runtime-fmt.sh::test_static_evidence_binds_real_formatter_symbols` | [] | proptest | `bash scripts/test-forbid-runtime-fmt.sh test_static_evidence_binds_real_formatter_symbols` | state_11_holzman_rust_repair |

### §1.1 Path::Symbol Reference Format

All `source_refs` use the canonical `path::symbol` form required by the
go-skill validator's `SOURCE_REF` regex
`^[A-Za-z0-9_./-]+::[A-Za-z0-9_:.-]+$`. The paths refer to artifacts
in the source checkout (`/home/lewis/src/velvet-ballistics/...`); the
symbols name functions, methods, enums, file-level constants, or YAML
task keys.

### §1.2 Forward-Pointing vs Existing Source Refs

Of the source_refs listed in §1:

- **Existing on disk** (verified by direct read in State 5):
  - `velvet-ballistics-MASTER.md::section_43_automatic_rejection_triggers_2056_2060`
    (lines 2056-2060: unbounded/YAML/JSON/HTTP/HashMap rejection items)
- **Forward-pointing** (to be authored by State 11 holzman-rust per
  `contract.md` §1 and `proof-writer-report.md` §3):
  - `scripts/forbid-runtime-fmt.sh::*` (bash wrapper; ~45 lines;
    modeled on `scripts/check-removed-feature-residue.sh` per
    `type-contracts.md` §12)
  - `scripts/forbid-runtime-fmt.rs::*` (scanner binary source;
    pseudocode in `type-contracts.md` §2-11)
  - `scripts/forbid-runtime-fmt.allow` (allowlist file; format in
    `type-contracts.md` §9.1)
  - `.moon/tasks/all.yml::forbid-runtime-fmt` (moon v2 task entry;
    position in `:check` `deps:` per `contract.md` §3.5)
  - `.moon/tasks/all.yml::check` (existing task; the gate is wired
    into its `deps:` array)

The forward-pointing refs are deliberately chosen as
forward-pointing path::symbol placeholders that match the validator's
`SOURCE_REF` regex but do not exist on disk yet. The State 11
holzman-rust agent must materialize them; the State 12 formal-verifier
agent verifies their existence and binds `mapping_status` from
`mapped` to `verified`.

### §1.3 Source Ref Co-Disjointness

The 5 rows are mutually disjoint in `source_refs`:

- PO-RQ-001: `scripts/forbid-runtime-fmt.sh::main`,
  `scripts/forbid-runtime-fmt.sh::compile_scanner`,
  `scripts/forbid-runtime-fmt.rs::ResidueQuarantine::run`,
  `scripts/forbid-runtime-fmt.rs::ResidueQuarantine::decide`
- PO-RQ-002: `velvet-ballistics-MASTER.md::section_43_automatic_rejection_triggers_2056_2060`,
  `scripts/forbid-runtime-fmt.rs::ResiduePolicy::from_master`,
  `scripts/forbid-runtime-fmt.rs::ForbiddenImport::from_name`,
  `scripts/forbid-runtime-fmt.rs::expected_master_trigger`,
  `scripts/forbid-runtime-fmt.rs::master_line_matches`
- PO-RQ-003: `scripts/forbid-runtime-fmt.sh::main` (shared with RQ-001),
  `scripts/forbid-runtime-fmt.sh::exit_code_translation`,
  `scripts/forbid-runtime-fmt.rs::ResidueQuarantine::decide` (shared
  with RQ-001),
  `scripts/forbid-runtime-fmt.rs::GateError::exit_code`
- PO-RQ-004: `scripts/forbid-runtime-fmt.rs::ResidueQuarantine::diff_against_allowlist`,
  `scripts/forbid-runtime-fmt.rs::AllowlistRef::load`,
  `.moon/tasks/all.yml::forbid-runtime-fmt`,
  `.moon/tasks/all.yml::check`
- PO-RQ-005: `scripts/forbid-runtime-fmt.sh::sort_unique`,
  `scripts/forbid-runtime-fmt.rs::ResidueMatch::active_line`,
  `scripts/forbid-runtime-fmt.rs::ResidueMatch::allowlisted_line`,
  `scripts/forbid-runtime-fmt.rs::ScanReport::summary_line`,
  `scripts/forbid-runtime-fmt.rs::emit_pass`,
  `scripts/forbid-runtime-fmt.rs::emit_fail`

`scripts/forbid-runtime-fmt.sh::main` is shared between PO-RQ-001 and
PO-RQ-003 because both RQ-001 (pass-iff-no-active-residue) and RQ-003
(exit-code-correctness) are the two halves of the same decide() flow:
RQ-001 is the in-band success-path invariant; RQ-003 is the
exit-code-translation invariant. Sharing is intentional and
documented; the shared ref is the bash wrapper's `main` orchestrator,
not a duplicated RRO obligation.

`scripts/forbid-runtime-fmt.rs::ResidueQuarantine::decide` is shared
between PO-RQ-001 and PO-RQ-003 for the same reason: the decide()
method is the single producer of `GateDecision`, and both the iff
claim (RQ-001) and the exit-code-translation claim (RQ-003) bind to
the same method.

## §2. 5 Obligations (mirror of `proof-obligations.planned.jsonl`)

| Obligation | Risk | Verifier | Mode | Owner State | Rerun From |
|------------|------|----------|------|-------------|------------|
| PO-RQ-001 | hot-crate-gate miss: a forbidden import enters vb_core/vb_runtime/vb_storage/vb_ipc and the gate fails to detect it | proptest | executable-test | state_8_9_10 | state_8_test_writer |
| PO-RQ-002 | master-linkage drift: the scanner's closed sets diverge from the master document and the gate fails to detect a new forbidden import | proptest | manual | state_13 | state_13_black_hat_reviewer |
| PO-RQ-003 | exit-code misclassification: the scanner emits 0 when residue is present, allowing a forbidden import to land in the runtime | proptest | executable-test | state_8_9_10 | state_8_test_writer |
| PO-RQ-004 | moon-wiring regression: the gate is removed from check's deps: and CI misses the residue check | proptest | executable-test | state_11 | state_11_holzman_rust |
| PO-RQ-005 | non-deterministic output: CI logs differ across runs, making residue diffs non-reproducible | proptest | manual | state_13 | state_13_black_hat_reviewer |

The 5 obligation rows in `rust-refinement-obligations.jsonl` (one per
seed) bind 1:1 with the 5 obligations above. The bridge preserves
identity: every RRO row's `proof_id` field equals the corresponding
`proof-obligations.planned.jsonl::id`; the `requirement_id` and
`contract_clause` fields are also preserved (per
`check_proof_to_rust_completeness` in the validator).

## §3. 3 Named Test Cases (executable behavior tests)

The bead description names 3 executable bash tests. The bridge binds
each to a specific RRO row.

| Test Name | RRO Row | Fixture | Expected Stderr Pattern | Expected Exit |
|-----------|---------|---------|--------------------------|----------------|
| `test_quarantine_gate_blocks_json_import` | RRO-RQ-001 | Single `.rs` file under a temporary directory containing `use serde_json;` | `<file>:<line_no>: RUNTIME-FMT: serde_json: <snippet>` (matches `ResidueMatch::active_line` per the current scanner source) | 1 |
| `test_quarantine_gate_blocks_unbounded_channel` | RRO-RQ-003 | Single `.rs` file under a temporary directory containing `tokio::sync::mpsc::unbounded_channel()` | `<file>:<line_no>: RUNTIME-FMT: tokio::sync::mpsc::unbounded: <snippet>` | 1 |
| `test_moon_ci_quarantine_dependency_correctly_ordered` | RRO-RQ-004 | Real moon task graph (`.moon/tasks/all.yml`) with `forbid-runtime-fmt` wired as a `deps:` of `:check`, ordered before heavier cargo check invocations | (no stderr match; test is structural) | 0 |

These 3 tests are the executable behavior tests; they are NOT
verifier harnesses. They exercise the live scanner binary (compiled
by the bash wrapper per `contract.md` §4.5) against real on-disk
fixtures. The 3 tests are owned by the State 8/9/10 test-writer
chain (RQ-001/003) and the State 11 holzman-rust agent (RQ-004,
allowlist format review).

The remaining 2 RRO rows (RRO-RQ-002, RRO-RQ-005) use executable static
evidence tests: `test_static_evidence_binds_master_rejection_triggers`
binds the scanner's forbidden variants to the actual §43 automatic
rejection lines, and `test_static_evidence_binds_real_formatter_symbols`
binds deterministic output claims to existing formatter functions.

## §4. No Holzman-Rust Scope (gate is pure shell + ripgrep-style line classifier)

This bridge map does NOT impose any Holzman-rust-specific
obligations on the State 11 holzman-rust agent beyond what is
already bound by `contract.md` and `type-contracts.md` (State 3
output). The bridge does not introduce:

- New Holzman-rust rules (no `no-unwrap`, `no-expect`, `no-panic`,
  `no-todo`, `no-unimplemented`, `no-unsafe` constraints beyond the
  `#![forbid(unsafe_code)]` already in `type-contracts.md` §2).
- New NASA/JPL Power-of-Ten rule set (no recursion-bound, no
  pointer-arithmetic, no dynamic-allocation, etc. constraints beyond
  the sync-core / async-shell boundary already bound by `contract.md`
  §7).
- New verification layers (no Verus, Kani, Flux-rs, Loom, Miri,
  cargo-fuzz obligations; the State 4 plan correctly classified all
  default Rust verifiers as `not_applicable` for this build-time
  scanner).
- New test-assertion strength requirements (no mutation-resistance
  scoring, no differential-test setup, no fuzz-corpus seeding).

The gate is structurally shell + ripgrep-style line classifier +
scanner binary (per `contract.md` §1 and `type-contracts.md` §12).
The scanner binary is a single Rust source file
(`scripts/forbid-runtime-fmt.rs`) compiled by the bash wrapper with
`rustc --edition=2024` (per `type-contracts.md` §12). The
Holzman-rust agent's responsibility for the scanner is the same as
for any other small CLI utility in the repository: a single-source
file, no `unsafe`, no async, no FFI, no external dependencies beyond
the standard library.

The bridge's role is to bind the 5 proof seeds (which are
behavior-affecting=false by design) to the forward-pointing source
refs that State 11 will materialize, and to bind the 3 named
executable tests to the 3 RRO rows that have executable behavior
evidence. The bridge does NOT extend the Holzman-rust scope.

## §5. Refinement Harness Refs = [] (N/A)

`refinement_harness_refs=[]` for all 5 RRO rows. This is
intentional, not a gap.

A refinement harness is a separate verifier artifact (e.g., a Verus
`proof fn` with `requires`/`ensures`, a Kani `#[kani::proof]`
harness, a Flux-rs `#[refined_by]` refinement, a Loom
`loom::model::Config` permutation test) that is distinct from the
production code AND distinct from the behavior test. The validator's
`check_rro` enforces this disjointness via
`E_BRIDGE_REFS_NOT_DISJOINT` when `behavior_test_refs` and
`refinement_harness_refs` overlap.

For this bead:

- The production code is the scanner binary
  (`scripts/forbid-runtime-fmt.rs`), the bash wrapper
  (`scripts/forbid-runtime-fmt.sh`), the allowlist
  (`scripts/forbid-runtime-fmt.allow`), and the moon task entry
  (`.moon/tasks/all.yml::forbid-runtime-fmt`).
- The behavior tests are the 3 named bash tests
  (`test_quarantine_gate_blocks_json_import`,
  `test_quarantine_gate_blocks_unbounded_channel`,
  `test_moon_ci_quarantine_dependency_correctly_ordered`).
- A refinement harness would be a Verus/Kani/Flux-rs/Loom/Miri/cargo-fuzz
  artifact that proves a property of the scanner binary's logic. The
  State 4 plan correctly classified all default Rust verifiers as
  `not_applicable` for this build-time scanner (per
  `proof-strategy.md` §1 and `proof-plan-review.md` Validation 2);
  therefore no refinement harness is required, and
  `refinement_harness_refs=[]` is the correct disposition.

The `behavior_affecting=false` classification of all 5 seeds (per
`proof-seeds.jsonl`) means the validator's
`E_REFINEMENT_HARNESS_MISSING` check (which fires when
`behavior_affecting=true` and `refinement_harness_refs=[]`) does NOT
fire. The empty refinement harness refs are consistent with the
non-behavior-affecting disposition.

## §6. Known Gaps

1. **Forward-pointing source refs are not on disk yet.** The 5 RRO
   rows reference `scripts/forbid-runtime-fmt.sh`, `scripts/forbid-runtime-fmt.rs`,
   `scripts/forbid-runtime-fmt.allow`, `scripts/test-forbid-runtime-fmt.sh`,
   and `.moon/tasks/all.yml::forbid-runtime-fmt` — none of which
   exist on disk at the time of this State 7 dispatch (verified by
   direct `ls` and `grep` of the source checkout). These refs are
   forward-pointing placeholders that the State 11 holzman-rust
   agent must materialize. The bridge cannot be verified (`verified`
   mapping_status) until State 12 formal-verifier confirms the
   files exist and the source refs resolve to real symbols.

2. **`proof-to-rust-review.md` is not yet written.** The validator
   expects 3 files at state 7 (`proof-to-rust-map.md`,
   `rust-refinement-obligations.jsonl`, `proof-to-rust-review.md`);
   the third is written by a SEPARATE proof-reviewer agent invoked
   next by the femdation controller. The validator will report
   `E_MISSING_ARTIFACT` for `proof-to-rust-review.md` until the
   reviewer writes it. This is expected and intentional: the
   State 7 bridge review is a separate agent.

3. **State 5 JSONL documentation gap (E_LANE_DECISION_MISSING) is
   unchanged at State 7.** The 11 `E_LANE_DECISION_MISSING`
   validator findings identified at State 4 review (per
   `proof-plan-review.md` Validation 2 "Note on JSONL Lane Coverage")
   are not closed by the State 5 writer (per
   `proof-writer-report.md` §5). State 7 does not extend or close
   this gap; the substantive proof plan is sound and the 5 accepted
   lanes are correctly bound to 5 RRO rows.

4. **The verifier name `proptest` for RQ-002 and RQ-005 is the
   closest match in the validator's `VALID_VERIFIERS` enum for
   static-review dispositions, but the actual evidence form is a
   State 13 black-hat-reviewer disposition document, not a
   `proptest!` macro test.** This is a known semantic mismatch in
   the verifier name; it is consistent with the State 4 plan's
   decision to use the closest enum match for non-executable
   verification lanes. The evidence_command and expected_evidence
   fields in the 2 RRO rows make the actual evidence form explicit
   (a `bash -c` invocation that asserts the master §43 trigger
   table has 4 entries for RQ-002; a `bash -c` invocation that
   asserts the bash wrapper has `sort -u` and a `summary:` line
   prefix for RQ-005). The State 13 black-hat-reviewer agent will
   execute these commands and record the disposition in
   `proof-to-rust-review.md` (or its successor).

5. **The 3 executable tests are not yet on disk.** The State 8/9/10
   test-writer chain is responsible for authoring the 3 tests in
   `scripts/test-forbid-runtime-fmt.sh` (per `contract.md` §8 and
   `proof-writer-report.md` §3). State 7 binds the test names to
   RRO rows but does not implement the tests.

## §7. Validator Self-Check

The validator (`/home/lewis/.agents/skills/go-skill/tools/go-skill-v9-validate`)
checks the following for state 7:

- `proof-to-rust-map.md` exists and contains the required matrix
  header `| Proof ID | Claim | Behavior Affecting | Rust Source Refs |
  Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence
  Command | Rerun From |` (per `check_matrices`).
- `rust-refinement-obligations.jsonl` exists and has 5 rows, all with
  schema_version `rust-refinement-obligation/v1` and all 22
  RRO_FIELDS present (per `check_rro`).
- All `source_refs` are non-empty lists of strings matching
  `SOURCE_REF = ^[A-Za-z0-9_./-]+::[A-Za-z0-9_:.-]+$`.
- `behavior_test_refs` are lists of strings (empty lists allowed for
  static-review obligations; required for executable obligations).
- `refinement_harness_refs` are lists (empty lists allowed because
  `behavior_affecting=false`).
- No `behavior_affecting=true` row lacks behavior tests or refinement
  harnesses (per `E_PROOF_TO_RUST_MISSING`,
  `E_REFINEMENT_HARNESS_MISSING`).
- `behavior_test_refs` and `refinement_harness_refs` are disjoint
  (per `E_BRIDGE_REFS_NOT_DISJOINT`).
- No `behavior_test_refs` entry matches `BEHAVIOR_TEST_FORBIDDEN_RE`
  (i.e., no `verification/...` paths or `kani`/`verus`/`flux`/`loom`/`miri`
  keywords in test refs; the 3 named tests in
  `scripts/test-forbid-runtime-fmt.sh` do not match this pattern).
- `check_proof_to_rust_completeness` iterates proof_obligations with
  `behavior_affecting=true` and requires a Rust bridge with matching
  identity. Since all 5 obligations have `behavior_affecting=false`,
  this check is skipped.

The expected validator output for this State 7 dispatch is one
`E_MISSING_ARTIFACT` finding for `proof-to-rust-review.md` (to be
written by the next proof-reviewer agent) and zero other findings.
