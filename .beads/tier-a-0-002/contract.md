# Contract — Residue Quarantine CI Gate

bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
phase: 1
state: 3 (rust-contract)
skill: rust-contract
attempt: 1-of-7
updated_at: 2026-06-17T22:00:00.000000+00:00

## 1. Contract Scope

This contract binds the behavior of the new `scripts/forbid-runtime-fmt.sh`
CI gate and its supporting artifacts. The contract is the boundary
between the State 3 rust-contract agent and the State 11 holzman-rust
implementation agent. The State 11 agent MUST satisfy this contract;
the State 13 black-hat-reviewer MUST reject any implementation that
violates it.

The contract is intentionally narrow: it binds the gate's *observable
behavior* (exit code, stderr format, moon task wiring) and its
*internal invariants* (closed sets, exhaustive state machine, fail-closed
on contract violation). It does NOT bind implementation choices that
are not observable from outside the gate (e.g., the choice of `rustc`
vs `clippy-driver`, the use of `BTreeMap` vs `HashMap` for the
allowlist, the exact line-by-line walk algorithm).

## 2. Preconditions

The contract holds under the following preconditions. Violating a
precondition is the caller's responsibility, not the gate's.

### 2.1 Master contract is canonical

`velvet-ballistics-MASTER.md` is the single source of truth for:

- The closed set of forbidden imports (drawn from §2 lines 99-102
  and §12 lines 405-439).
- The closed set of hot crates (`vb_core`, `vb_runtime`, `vb_storage`,
  `vb_ipc`, per §44.6 line 2078).
- The closed set of cold markers (derived from the sibling
  `check-hot-cold-forbidden-apis.rs::COLD_MARKERS`).

If the master is amended, the gate's policy table MUST be regenerated
from the master. The contract binds the regeneration, not the
specific line numbers in the master.

### 2.2 Source tree is the four hot crate roots

The gate reads `crates/{vb_core,vb_runtime,vb_storage,vb_ipc}/src/**/*.rs`
and only those paths. Other source trees (e.g., `vb_cli`, `vb_benchmark`,
`workspace_tests`) are out of scope. The contract does NOT bind the
gate to scan them.

### 2.3 Allowlist is human-edited

`scripts/forbid-runtime-fmt.allow` is a human-edited file in the
format documented in `type-contracts.md` §9.1. The contract binds
the format; the contract does NOT bind the allowlist contents (which
are decided per-bead by the State 11 holzman-rust agent and
State 14 evidence-packaging).

### 2.4 Moon task graph is well-formed

`.moon/tasks/all.yml` (or `.moon/tasks/forbid-runtime-fmt.yml`
plus `.moon.yml`) is a well-formed moon v2 task graph. The contract
binds the gate's position in the `check` task's `deps:` array; the
contract does NOT bind the rest of the task graph.

## 3. Postconditions

The contract guarantees the following postconditions for every
execution of the gate.

### 3.1 Total decision

Every execution of the gate produces exactly one of:

- `GateDecision::Pass` (exit 0).
- `GateDecision::Fail(Vec<ResidueMatch>)` (exit 1).
- `GateError::NewResidueDetected` (exit 1).
- `GateError::PatternFileMissing` (exit 2).
- `GateError::GlobUnreadable` (exit 2).
- `GateError::AllowlistParseFailure` (exit 2).
- `GateError::ScriptInvocationFailure` (exit 2).

There is no other decision. The contract is exhaustive.

### 3.2 Pass iff no active residue

`GateDecision::Pass` is returned **if and only if** the set of
`active` matches in `ScanReport` is empty. Equivalently, the gate
fails (exit 1) iff at least one residue match is in `active`.

### 3.3 Stderr format

For every execution, the gate emits:

- Zero or more `<file>:<line_no>: RUNTIME-FMT: <forbidden_name>: <snippet>`
  lines (one per active match).
- Zero or more `<file>:<line_no>: allowlisted: <reason>: <snippet>`
  lines (one per allowlisted match).
- Exactly one `summary: active=<N> allowlisted=<M> files_scanned=<K>
  hot_paths=<H> cold_paths=<C>` line.
- Zero or one `GateError:<VariantName>: <args>` line (only on contract
  violation).

The format is a contract. Changes to the format are breaking changes
to `test-forbid-runtime-fmt.sh`.

### 3.4 Closed set invariant

The set of forbidden imports, hot crates, and cold markers is
closed. The gate does not add or remove items from the closed sets
at runtime. The sets are derived from the master document at
scanner construction time and are immutable for the duration of
one scan.

### 3.5 Moon task wiring

The gate is declared as a moon task with:

- `command: 'bash scripts/forbid-runtime-fmt.sh'`
- `options.runInCI: true`
- `deps:` of `:check`, ordered before the heavier cargo check
  invocations.

The wiring is asserted by `test_moon_ci_quarantine_dependency_correctly_ordered`.

## 4. Invariants

The following invariants hold for every execution of the gate. A
violation is a contract violation.

### 4.1 Closed-set invariants

- The set of `ForbiddenImportName` variants is exactly 7:
  `SerdeJson`, `SerdeYaml`, `Hyper`, `Reqwest`, `Axum`,
  `HashMapStringGeneric`, `TokioSyncMpscUnbounded`.
- The set of `HotCrateName` variants is exactly 4:
  `VbCore`, `VbRuntime`, `VbStorage`, `VbIpC`.
- The set of `ColdMarker` variants is exactly 15 (matching the
  sibling `check-hot-cold-forbidden-apis.rs::COLD_MARKERS`).
- The set of `GateError` variants is exactly 5:
  `PatternFileMissing`, `GlobUnreadable`, `AllowlistParseFailure`,
  `ScriptInvocationFailure`, `NewResidueDetected`.

### 4.2 Allowlist invariants

- The allowlist is append-only within one scan. Removing an
  allowlist entry mid-scan is invalid.
- An allowlist entry is matched to exactly one `(file_path, line_no,
  forbidden_name)` tuple. Duplicate keys are a parse error.
- An allowlist entry's `forbidden_name` field must be one of the 7
  `ForbiddenImportName::as_str()` forms. An unknown name is a parse
  error.

### 4.3 State machine invariants

- The state machine is total: every `ResidueQuarantineState` is
  either an intermediate state or a terminal `ContractViolation`
  state. There is no other terminal state.
- The state machine has no cycles. `Init` and `ContractViolation`
  are the only states with no incoming transition.
- Every transition is one of the 5 documented transitions in
  `workflow-model.md` §1.2.

### 4.4 Output invariants

- Stderr is line-ordered: the scanner emits all residue-match
  lines, then the summary line (or error line), then nothing.
- The summary line is emitted exactly once per execution.
- The error line is emitted at most once per execution.
- The scanner emits no other output.

### 4.5 Lifecycle invariants

- The scanner is compiled once per gate execution by the bash
  wrapper. The binary is re-compiled only when the source file's
  mtime changes (per moon's cache invalidation).
- The scanner does not persist state across executions. Every
  execution is independent.

## 5. Error Contract

The error contract is documented in full in `error-taxonomy.md`.
Summary:

| Class | Variants | Exit code | Stderr prefix |
|-------|----------|-----------|---------------|
| Failure | (none; the gate works as designed) | 1 | `RUNTIME-FMT:` |
| Error | `PatternFileMissing`, `GlobUnreadable`, `AllowlistParseFailure`, `ScriptInvocationFailure` | 2 | `GateError:` |
| Pre-flight | (bash wrapper) | 64 | (bash error) |

The contract is exhaustive: every non-zero exit code is one of the
documented classes.

## 6. Performance Contract

The gate's wall-clock time on the current source tree (the four hot
crates with ~30,000 lines of Rust source) MUST be under 30 seconds.
The contract binds a 30-second budget as a sanity check; the actual
expected time is well under 1 second.

The budget is verified by the `test-forbid-runtime-fmt.sh` perf
assertion. A regression is a hard failure of the test.

The contract does NOT bind the scanner's memory usage. A scanner
that uses O(N) memory (where N is the total source line count)
satisfies the contract.

## 7. Trust Contract

The contract treats the following components as trusted:

- The moon pipeline (orchestration).
- The bash wrapper (imperative shell).
- The scanner binary (post-build, pre-run).

The contract treats the following components as untrusted:

- The file system (source files, allowlist, master document).

The trust contract is documented in `boundary-map.md` §1.

## 8. Contract Tests (State 9)

The State 9 test-writer MUST implement three failing-first tests
in `scripts/test-forbid-runtime-fmt.sh`:

1. `test_quarantine_gate_blocks_json_import` — exits 1 with a
   `RUNTIME-FMT: serde_json:` stderr line on a fixture containing
   `use serde_json;`.
2. `test_quarantine_gate_blocks_unbounded_channel` — exits 1 with
   a `RUNTIME-FMT: tokio::sync::mpsc::unbounded:` stderr line on a
   fixture containing `tokio::sync::mpsc::unbounded_channel()`.
3. `test_moon_ci_quarantine_dependency_correctly_ordered` — exits 0
   on a moon task graph where the gate is correctly wired as a
   `deps:` of `:check`.

The State 11 holzman-rust agent MUST satisfy all three tests before
the State 11 work is complete.

## 9. Out-of-Scope (Explicitly Excluded)

The contract does NOT bind the following:

- The choice of `rustc` vs `clippy-driver` for compiling the
  scanner.
- The choice of separate `.moon/tasks/forbid-runtime-fmt.yml` vs
  a new entry in `.moon/tasks/all.yml`.
- The internal data structure choices (BTreeMap vs HashMap, etc.).
- The exact algorithm for cold-path classification (substring
  containment is bound; the data structure is not).
- The specific line numbers in the master document (the closed
  sets are bound; the line numbers are not).
- The contents of the allowlist (the format is bound; the entries
  are not).
- The contents of the stderr summary line beyond the documented
  format (e.g., the exact wording of the summary is bound; the
  per-residue snippet is not).

## 10. Contract Violation Reporting

A contract violation is reported by the State 13 black-hat-reviewer
as a `STATUS: REJECTED` finding. The State 11 holzman-rust agent
MUST fix the violation before the State 11 work is complete.

The most common contract violations are:

- The scanner uses a regex (H-04 in `hazard-analysis.md`).
- The scanner does not parse the master document (H-06, H-08).
- The allowlist parser does not skip comments (H-09).
- The scanner emits output in a non-canonical format (§3.3).
- The moon task is not in `check`'s `deps:` (H-07).
- The scanner uses async I/O (violates the
  sync-core/async-shell boundary).

## 11. Contract Evolution

The contract is versioned implicitly by the master document
revision. A master amendment that changes the closed sets (e.g.,
adding a new forbidden import) requires:

1. A code change to the scanner's `ForbiddenImportName` enum.
2. A code change to the scanner's `ForbiddenImportKind` enum.
3. An update to the `type-contracts.md` §6.1 pattern table.
4. An update to the `domain-model.md` §4.1 enumeration.
5. An update to the `proof-seeds.jsonl` RQ-002 contract clause.
6. An update to the `traceability-matrix.jsonl` row for the
   new forbidden import.

The contract evolution steps are the State 11 holzman-rust agent's
responsibility. The State 3 rust-contract agent has bound the
*structure* of the evolution but not the *content*.

## 12. Contract Signatures

The contract is signed by:

- The State 3 rust-contract agent: `STATUS: STATE_3_CONTRACT_CAPTURED`.
- The State 11 holzman-rust agent: must be `STATUS: PASS` before
  the State 13 black-hat-review.
- The State 13 black-hat-reviewer: must be `STATUS: APPROVED`
  before the State 14 evidence-packaging.
- The State 14 evidence-packaging: must be `STATUS: APPROVED`
  before the State 15 landing.

A missing or non-`PASS` status at any state is a contract violation
that blocks the next state.

## 13. Cross-Reference to Other Artifacts

| Concern | Artifact |
|---------|----------|
| Ubiquitous language | `domain-model.md` §1 |
| Entities, VOs, aggregates | `domain-model.md` §2-5 |
| Type pseudocode | `type-contracts.md` |
| State machine | `workflow-model.md` §1 |
| Error variants | `error-taxonomy.md` §2 |
| File system I/O | `boundary-map.md` §3 |
| Hazards | `hazard-analysis.md` §2-11 |
| Proof seeds | `proof-seeds.jsonl` |
| Master contract traceability | `traceability-matrix.jsonl` |
| Sibling gate patterns | `domain-model.md` §3.1, `type-contracts.md` §3.1, §12 |
