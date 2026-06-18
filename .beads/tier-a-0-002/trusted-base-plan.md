# Trusted Base Plan — Residue Quarantine CI Gate

bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
phase: 1
state: 4 (proof-planner)
skill: proof-planner
attempt: 1-of-7
updated_at: 2026-06-17T23:45:00.000000+00:00
planner_invocation_id: tier-a-0-002-s4-proof-planner-PROOF01
schema_version: trusted-base-plan/v1

STATUS: STATE_4_TRUSTED_BASE_PLANNED

## 1. Trusted Components

The following components are trusted for the residue quarantine CI
gate. Each is recorded with a marker, reason, scope, and impact.

### 1.1 Master Document §43 Trigger Table

- **ID**: `TB-MASTER-§43`
- **Artifact**: `velvet-ballistics-MASTER.md`
- **Location**: `velvet-ballistics-MASTER.md::§43.lines_2038_to_2041`
- **Marker**: master §43 trigger table 7-10
- **Trusted kind**: canonical source of forbidden patterns
- **Reason**: The master document is the single source of truth for
  the closed set of forbidden imports (per `contract.md` §2.1). The
  scanner's `ResiduePolicy::from_master` parser walks the master and
  constructs the seven-variant `ForbiddenImportName` enum; drift
  between the master and the parser is detectable via
  `GateError::PatternFileMissing` (fail-closed).
- **Scope**: global (every execution of the gate)
- **Impact**: scanner pattern list is derived from the master at
  scanner construction time
- **Behavior affecting**: false
- **Compensating evidence**: the State 13 black-hat-reviewer reads
  the master and the scanner parser source and asserts the closed
  sets are derived from the master, not hard-coded. The review
  evidence is recorded as `PO-RQ-002` in
  `proof-obligations.planned.jsonl`.
- **Owner**: state_3_rust_contract (captured the master linkage in
  `contract.md` §2.1 and `traceability-matrix.jsonl` rows
  `TM-008`..`TM-013`)
- **Expiry**: never (master document is the canonical reference for
  the lifetime of the project)
- **Reviewer disposition**: pending (State 4 proof-plan-reviewer)

### 1.2 Hot Crate Paths

- **ID**: `TB-HOT-CRATES`
- **Artifact**: `crates/{vb_core,vb_runtime,vb_storage,vb_ipc}/src/`
- **Location**: `crates/vb_core/src/`, `crates/vb_runtime/src/`,
  `crates/vb_storage/src/`, `crates/vb_ipc/src/`
- **Marker**: four hot crate roots (per `contract.md` §2.2 and
  `master` §44.6 line 2078)
- **Trusted kind**: scan scope boundary
- **Reason**: The gate scans only these four crate roots; other
  source trees (`vb_cli`, `vb_benchmark`, `workspace_tests`) are out
  of scope (per `contract.md` §2.2). The hot crate list is bound by
  the four-variant `HotCrateName` enum; the gate's pattern file is
  closed under this enum.
- **Scope**: per-execution (the gate walks the four roots at
  scanner invocation)
- **Impact**: the gate's residue count is a total function of the
  four roots
- **Behavior affecting**: false
- **Compensating evidence**: the State 11 holzman-rust agent
  reviews the scanner's `walkdir` invocation against the four
  crate roots; the State 13 black-hat-reviewer asserts the scan
  scope is closed.
- **Owner**: state_11_holzman_rust
- **Expiry**: never (hot crate list is closed for the lifetime of
  the project)
- **Reviewer disposition**: pending (State 4 proof-plan-reviewer)

### 1.3 Allowlist Format

- **ID**: `TB-ALLOWLIST-FORMAT`
- **Artifact**: `scripts/forbid-runtime-fmt.allow`
- **Location**: `scripts/forbid-runtime-fmt.allow`
- **Marker**: human-edited allowlist in the format documented in
  `type-contracts.md` §9.1
- **Trusted kind**: format specification boundary
- **Reason**: The allowlist is human-edited and parsed by a
  single-pass line parser; the format is bound by `type-contracts.md`
  §9.1 and `contract.md` §2.3. The contract binds the format; the
  contract does not bind the allowlist contents (which are decided
  per-bead by State 11 and State 14).
- **Scope**: per-execution (the gate reads the allowlist at scanner
  invocation)
- **Impact**: the gate's residue-vs-allowlist partition is a total
  function of the allowlist entries
- **Behavior affecting**: false
- **Compensating evidence**: the State 11 holzman-rust agent
  reviews the allowlist parser against `type-contracts.md` §9.1;
  the State 13 black-hat-reviewer asserts the format is honored.
- **Owner**: state_11_holzman_rust
- **Expiry**: never (allowlist format is closed for the lifetime of
  the project)
- **Reviewer disposition**: pending (State 4 proof-plan-reviewer)

### 1.4 Scanner Script (Bash Wrapper)

- **ID**: `TB-SCAN-SCRIPT`
- **Artifact**: `scripts/forbid-runtime-fmt.sh`
- **Location**: `scripts/forbid-runtime-fmt.sh`
- **Marker**: bash wrapper (to be authored by State 11)
- **Trusted kind**: imperative shell boundary
- **Reason**: The bash wrapper is the imperative shell that compiles
  and invokes the scanner binary; it is trusted to:
  1. Compile the scanner binary exactly once per invocation.
  2. Translate the scanner's `GateDecision` to the contract-bound
     exit code (0/1/2).
  3. Emit the contract-bound stderr format (per `contract.md` §3.3
     and §4.4).
  4. Honor the 30-second performance budget (per `contract.md` §6).
- **Scope**: per-execution (the bash wrapper is invoked once per
  gate execution by the moon task)
- **Impact**: the gate's observable behavior (exit code, stderr
  format, performance) is determined by the bash wrapper
- **Behavior affecting**: false
- **Compensating evidence**: the State 13 black-hat-reviewer reads
  the bash wrapper and asserts the contract-bound invariants
  (§3.1, §3.3, §4.4, §6) are honored. The review evidence is
  recorded as `PO-RQ-005` in `proof-obligations.planned.jsonl`.
- **Owner**: state_11_holzman_rust (will author); state_13_black_hat_reviewer
  (will review)
- **Expiry**: never (the bash wrapper is part of the project for its
  lifetime)
- **Reviewer disposition**: pending (State 4 proof-plan-reviewer)

### 1.5 Moon Task Graph

- **ID**: `TB-MOON-TASK-GRAPH`
- **Artifact**: `.moon/tasks/all.yml` (or `.moon/tasks/forbid-runtime-fmt.yml`)
- **Location**: `.moon/tasks/all.yml`
- **Marker**: moon v2 task graph (per `contract.md` §2.4 and §3.5)
- **Trusted kind**: CI orchestration boundary
- **Reason**: The moon task graph is the CI orchestration layer that
  invokes the gate as a `deps:` of `:check`, ordered before heavier
  cargo check invocations. The wiring is asserted by
  `test_moon_ci_quarantine_dependency_correctly_ordered`.
- **Scope**: per-CI-run (the moon task graph is evaluated once per
  CI invocation)
- **Impact**: the gate's position in the CI pipeline determines
  whether the gate runs at all
- **Behavior affecting**: false
- **Compensating evidence**: the State 8/9/10 test-writer chain
  produces the moon-wiring bash test; the State 11 holzman-rust
  agent authors the moon task entry.
- **Owner**: state_11_holzman_rust
- **Expiry**: never (moon task graph is part of the project for its
  lifetime)
- **Reviewer disposition**: pending (State 4 proof-plan-reviewer)

## 2. Untrusted Components

The following components are untrusted for the residue quarantine CI
gate. The scanner reads them at scanner invocation.

### 2.1 Source Files

- **Artifact**: `crates/{vb_core,vb_runtime,vb_storage,vb_ipc}/src/**/*.rs`
- **Trusted kind**: read-only input
- **Reason**: The scanner reads the source files of the four hot
  crates; the scanner's job is to detect forbidden imports in the
  source files. A source file containing a forbidden import is the
  residue that the gate is designed to catch.
- **Compensating evidence**: the bash tests
  (`test_quarantine_gate_blocks_json_import`,
  `test_quarantine_gate_blocks_unbounded_channel`) exercise the
  scanner against fixtures containing forbidden imports.

### 2.2 Allowlist Contents

- **Artifact**: `scripts/forbid-runtime-fmt.allow`
- **Trusted kind**: read-only input
- **Reason**: The allowlist contents are human-edited and are
  decided per-bead by State 11 and State 14. The contract binds the
  format, not the contents. A malformed allowlist entry is a parse
  error (`GateError::AllowlistParseFailure`); the scanner fails
  closed (exit 2).
- **Compensating evidence**: the State 13 black-hat-reviewer reviews
  the allowlist parser against `type-contracts.md` §9.1.

### 2.3 Master Document Revision

- **Artifact**: `velvet-ballistics-MASTER.md`
- **Trusted kind**: canonical reference
- **Reason**: The master document is the canonical reference but is
  also untrusted in the sense that an amendment may change the
  closed sets. A master amendment triggers the contract evolution
  steps in `contract.md` §11.
- **Compensating evidence**: the scanner fails closed on
  `GateError::PatternFileMissing`; the State 11 holzman-rust agent
  updates the scanner's `ForbiddenImportName` enum to match the
  master amendment.

## 3. External Components

There are **no external C/C++/WASM components** for this bead. The
gate is pure Rust (scanner binary) + bash (wrapper) + YAML (moon
task graph).

## 4. Trusted Marker Limits

The trusted markers listed above are bounded as follows:

- **TB-MASTER-§43** is trusted as a canonical reference for the
  closed set of forbidden imports. The master document's revision
  is trusted; the master's prose is not. Only the trigger table 7-10
  is trusted for this bead.
- **TB-HOT-CRATES** is trusted as the scan scope. The four crate
  roots are trusted; the contents of those crates are untrusted
  (the scanner's job is to detect residue in the contents).
- **TB-ALLOWLIST-FORMAT** is trusted as a format specification. The
  format is trusted; the contents are untrusted (the scanner's job
  is to parse the contents and decide whether each entry matches
  a residue tuple).
- **TB-SCAN-SCRIPT** is trusted as a contract-bound imperative
  shell. The bash wrapper's contract-bound invariants (§3.1, §3.3,
  §4.4, §6) are trusted; the bash wrapper's internal organization
  (e.g., the choice of `bash` vs `sh`, the use of `set -euo pipefail`)
  is untrusted.
- **TB-MOON-TASK-GRAPH** is trusted as a CI orchestration layer.
  The moon task graph's wiring (gate is in `:check` deps:) is
  trusted; the rest of the task graph is untrusted for this bead.

## 5. Trusted Base Ledger Cross-Reference

The State 5 proof-writer populates
`trusted-base-ledger.jsonl` with rows for each trusted marker
above. The State 4 trusted-base-plan.md is the planning artifact;
the State 5 trusted-base-ledger.jsonl is the execution artifact.

| Trusted Marker | State 5 Ledger Row ID (planned) | State 5 Ledger Status |
|----------------|--------------------------------|----------------------|
| `TB-MASTER-§43` | `TBL-MASTER-§43` | pending |
| `TB-HOT-CRATES` | `TBL-HOT-CRATES` | pending |
| `TB-ALLOWLIST-FORMAT` | `TBL-ALLOWLIST-FORMAT` | pending |
| `TB-SCAN-SCRIPT` | `TBL-SCAN-SCRIPT` | pending |
| `TB-MOON-TASK-GRAPH` | `TBL-MOON-TASK-GRAPH` | pending |

## 6. Status and Handoff

The trusted base plan is captured. The five trusted markers are
listed above; each is bound to a proof obligation via
`trusted_base_refs` in `proof-obligations.planned.jsonl`. The State
5 proof-writer is the next state to materialize this plan into the
ledger.