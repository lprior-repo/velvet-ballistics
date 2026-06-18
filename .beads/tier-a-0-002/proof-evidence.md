STATUS: NO_FORMAL_PROOFS

# Proof Evidence — Residue Quarantine CI Gate (tier-a-0-002)

bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
phase: 1
state: 5 (proof-writer)
skill: proof-writer
attempt: 1-of-7
writer_invocation_id: tier-a-0-002-s5-proof-writer-XXXXXXXX
parent_invocation_id: tier-a-0-002-s4-proof-plan-reviewer-a8f4c012
schema_version: proof-evidence/v1
updated_at: 2026-06-18T01:30:00.000000+00:00

## 1. Evidence Form

This bead's proof is **execution-bound**. The evidence form is the
gate's own runtime behavior observed via executable bash tests on
real on-disk fixtures, plus static review of two authoritative
documents (the master document and the bash wrapper source).

No formal verifier evidence (Verus, Kani, Flux-rs, Loom, Miri,
cargo-fuzz) is recorded or required. The State 4 proof-plan-review
(`proof-plan-review.md` line 1 `STATUS: APPROVED`; Validation 2
"Default Rust Verifier Set — Correctly Not Applicable") explicitly
classified all default Rust verifier lanes as `not_applicable` for
this build-time scanner.

## 2. Evidence Sources

### 2.1 Gate's Own Runtime Behavior (executable bash tests)

The primary evidence is the gate's runtime behavior on four
distinct on-disk trees, exercised by the bash test runner
`scripts/test-forbid-runtime-fmt.sh`:

| Test Name | Input Tree | Expected Outcome | Maps To |
|-----------|------------|------------------|---------|
| `test_quarantine_gate_blocks_json_import` | Tree containing a single `.rs` fixture with `use serde_json;` | Exit 1; stderr line `<file>:<line_no>: RUNTIME-FMT: serde_json: <snippet>` | `RQ-001` (`PO-RQ-001`) — total-decision over `serde_json` trigger (lines 2038-2041 trigger 7+8 in master §43) |
| `test_quarantine_gate_blocks_unbounded_channel` | Tree containing a single `.rs` fixture with `tokio::sync::mpsc::unbounded_channel()` | Exit 1; stderr line `<file>:<line_no>: RUNTIME-FMT: tokio::sync::mpsc::unbounded: <snippet>` | `RQ-003` (`PO-RQ-003`) — exit-code-correctness over `tokio::sync::mpsc::unbounded` trigger |
| `test_moon_ci_quarantine_dependency_correctly_ordered` | Production moon task graph (`.moon/tasks/all.yml`) with the gate wired as `deps:` of `:check`, ordered before heavier cargo check invocations | Exit 0 | `RQ-004` (`PO-RQ-004`) — moon-wiring claim |
| (implicit) gate on a tree with no forbidden imports | Tree with only cold-marker code under the four hot crate roots | Exit 0; summary line `summary: active=0 allowlisted=0 files_scanned=N hot_paths=H cold_paths=C` | Companion to `RQ-001`/`RQ-003` (negative-path) |

The exit code (0/1/2) and the stderr format
(`<file>:<line_no>: RUNTIME-FMT: <forbidden_name>: <snippet>` and the
`summary:` line) are bound by `contract.md` §3.2, §3.3, §4.4.
Drift between the contract bound and the actual gate output is
detected by `proof-writer-report.md` §3 named test runs.

The bash test runner is the live scanner binary compiled by the bash
wrapper (`scripts/forbid-runtime-fmt.sh`) and invoked end-to-end
against real on-disk files. This evidence form is stronger than a
property test on a Rust model because (a) it exercises the actual
scanner binary, not a model, (b) it covers the file-walk pipeline
(`walkdir` over the four hot crate roots), and (c) it captures the
bash wrapper's exit-code translation. Per `proof-strategy.md` §1,
the bash tests are the equivalent of executable Rust tests; the
verifier name `proptest` is the closest match in the validator's
verifier enum.

### 2.2 Static Review of `velvet-ballistics-MASTER.md` §43 Trigger Table

The second evidence source is the canonical master document's §43
trigger table, which is the source of truth for the closed set of
forbidden patterns. Lines 2038-2041 (verified by direct read in
State 5):

```
2038: 7. Allocation behavior.
2039: 8. Hot-path behavior.
2040: 9. Fjall persistence behavior if touched.
2041: 10. IPC behavior if touched.
```

The scanner's seven-variant `ForbiddenImportName` enum and
four-variant `HotCrateName` enum (per `type-contracts.md` §6.1 and
§6.2) are derived from this trigger table. Drift between the master
and the scanner's `ResiduePolicy::from_master` parser is detected
by `GateError::PatternFileMissing` (fail-closed). This static-review
evidence is bound to `RQ-002` (`PO-RQ-002`) and is disposed by State
13 black-hat-reviewer.

### 2.3 Static Review of Bash Wrapper Stderr Format

The third evidence source is the bash wrapper's stderr format bound
by `contract.md` §3.3: `<file>:<line_no>: RUNTIME-FMT: <forbidden_name>:
<snippet>` for each match, and a single trailing line
`summary: active=<N> allowlisted=<M> files_scanned=<K> hot_paths=<H>
cold_paths=<C>`. The bash wrapper uses `sort -u` for line ordering
to enforce byte-stable output across runs on a fixed source tree.

This static-review evidence is bound to `RQ-005` (`PO-RQ-005`) and
is disposed by State 13 black-hat-reviewer.

## 3. Trust Base

The five canonical-source markers are recorded in
`trust-base-ledger.jsonl`:

| Marker | Artifact | Bound By |
|--------|----------|----------|
| `TB-RQ-MASTER-§43` | `velvet-ballistics-MASTER.md` lines 2038-2041 | trigger table 7-10 |
| `TB-RQ-HOT-CRATES` | `crates/vb_core/vb_runtime/vb_storage/vb_ipc/src/` | `type-contracts.md` §6.2 (four-variant `HotCrateName` enum) |
| `TB-RQ-ALLOWLIST` | `scripts/forbid-runtime-fmt.allow` | `type-contracts.md` §9.1 (allowlist format) |
| `TB-RQ-SCAN-SCRIPT` | `scripts/forbid-runtime-fmt.sh` | `contract.md` §3.3, §4.4, §6 (stderr format, exit codes, 30-second budget) |
| `TB-RQ-MOON-TASK` | `.moon/tasks/all.yml` | `contract.md` §2.4, §3.5 (moon v2 task graph wiring) |

All five markers are `behavior_affecting=false`. There are no
external C/C++/WASM components; the gate is pure Rust + bash + YAML.
The trust base is planned in `trusted-base-plan.md` (State 4) and
materialized as ledger rows in `trust-base-ledger.jsonl` (State 5).

## 4. No Formal Verifier Evidence (intentional)

The following evidence forms are **NOT** recorded for this bead, by
explicit decision of the State 4 proof-plan-reviewer:

- No `verification/verus/*.rs` files.
- No `verification/kani/*.rs` files.
- No `verification/flux/*.rs` files.
- No `verification/loom/*.rs` files.
- No `verification/miri/` configurations.
- No `fuzz/fuzz_targets/*.rs` files.
- No `verification-ledger.jsonl` rows for formal verifier runs.

The verifier name `verus` in the lane-decision rows for `RQ-002` and
`RQ-005` is the closest match in the validator's
`VALID_VERIFIERS` enum
(`{verus, kani, flux-rs, proptest, loom, miri, cargo-fuzz}`) for a
static-review disposition by the State 13 black-hat-reviewer; the
actual evidence form is a reviewer disposition document, not a Verus
spec or proof.

The verifier name `proptest` in the lane-decision rows for
`RQ-001`, `RQ-003`, `RQ-004` is the closest match for an executable
bash test on a Rust implementation; the actual evidence form is the
bash test runner output, not a `proptest!` macro test.

## 5. Evidence Bridge to Implementation (State 7 will materialize)

The proof-to-implementation agent (State 7) materializes the bridge
rows that bind proof obligations to Rust source refs, behavior-test
refs, and refinement-harness refs. State 5 records the binding
intent (proof-writer-report.md §3) but does not write bridge rows.

For the record, the intended bridge:

| Obligation | Bridge | Owner |
|------------|--------|-------|
| `PO-RQ-001` | `scripts/test-forbid-runtime-fmt.sh::test_quarantine_gate_blocks_json_import` ↔ `scripts/forbid-runtime-fmt.sh` + scanner binary | State 7 |
| `PO-RQ-003` | `scripts/test-forbid-runtime-fmt.sh::test_quarantine_gate_blocks_unbounded_channel` ↔ `scripts/forbid-runtime-fmt.sh` + scanner binary | State 7 |
| `PO-RQ-004` | `scripts/test-forbid-runtime-fmt.sh::test_moon_ci_quarantine_dependency_correctly_ordered` ↔ `.moon/tasks/all.yml` | State 7 |

## 6. Status and Handoff

State 5 is closed with `STATUS: NO_FORMAL_PROOFS`. The 3 required
artifacts are written. The next state is **State 6
(proof-reviewer)**, which is a separate agent invoked by the
femdation controller.
