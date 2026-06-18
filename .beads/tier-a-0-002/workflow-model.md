# Workflow Model — Residue Quarantine CI Gate

bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
phase: 1
state: 3 (rust-contract)
skill: rust-contract
attempt: 1-of-7
updated_at: 2026-06-17T22:00:00.000000+00:00

## 1. Workflow as a State Machine

The gate is a single execution of one well-typed state machine. Every
execution begins in `Init` and terminates in `Done` with a
`GateDecision` payload. There is no "indeterminate" terminal state;
a malformed input transitions to `ContractViolation` and exits 2.

```
                              +-----------+
                              |   Init    |
                              +-----+-----+
                                    |
                                    | init(policy, root, allowlist)
                                    v
                              +-----------+
                              |  Loaded   |
                              +-----+-----+
                                    |
                                    | walk()
                                    v
                              +-----------+
                              |  Walked   |
                              +-----+-----+
                                    |
                                    | match_lines()
                                    v
                              +-----------+
                              |  Matched  |
                              +-----+-----+
                                    |
                                    | diff_against_allowlist()
                                    v
                              +-----------+
                              | Differed  |
                              +-----+-----+
                                    |
                                    | decide()
                                    v
                              +-----------+
                              |   Done    |
                              +-----+-----+
                                    |
                                    | returns GateDecision
                                    v
                       (script exits with mapped code)
```

### 1.1 States

| State | Pre-condition | Post-condition |
|-------|---------------|----------------|
| `Init` | Process start; no resources held. | — |
| `Loaded` | `ResiduePolicy` and `AllowlistRef` are constructed and validated. | The aggregate owns a valid policy and a parsed allowlist. The source tree root is recorded. |
| `Walked` | The set of hot paths and cold paths is enumerated. | The aggregate owns two `Vec<SourcePath>`; total file count is recorded in `ScanReport::files_scanned`. |
| `Matched` | Every hot path has been read line-by-line and classified. | The aggregate owns a `Vec<ResidueMatch>` plus per-line classification metadata. Cold paths contribute zero matches. |
| `Differed` | Each `ResidueMatch` is annotated with its allowlist status. | `ResidueMatch` is moved into either `active` or `allowlisted`; `active_count` and `allowlisted_count` are recorded. |
| `Done` | A `GateDecision` is computed and emitted to stderr/stdout. | The bash wrapper translates the decision to an exit code and the process exits. |

### 1.2 Transitions

| Transition | Trigger | Guard | Effect |
|------------|---------|-------|--------|
| `Init -> Loaded` | `ResidueQuarantine::init` | policy is non-empty; allowlist file is readable and parseable; quarantined crates exist as directories | constructs the aggregate, returns `Result<Self, GateError>` |
| `Loaded -> Walked` | `ResidueQuarantine::walk` | source root is a directory | enumerates `.rs` files under each quarantined crate; partitions into hot vs cold paths; updates `ScanReport::files_scanned` |
| `Walked -> Matched` | `ResidueQuarantine::match_lines` | every hot path is readable; unreadable cold path is skipped silently | reads each hot path; emits `ResidueMatch` for each matching line |
| `Matched -> Differed` | `ResidueQuarantine::diff_against_allowlist` | `AllowlistRef` is loaded | for each match, look up the `(file, line_no, name)` key in the allowlist; move to `active` or `allowlisted` |
| `Differed -> Done` | `ResidueQuarantine::decide` | always | reduces to `GateDecision::Pass` (active == 0) or `GateDecision::Fail(active)` |

## 2. Legal-State Matrix

A `match` over `ResidueQuarantineState` is total. Adding a new state
is a contract change that requires a master amendment.

```
match state {
    Init       => ...,
    Loaded     => ...,
    Walked     => ...,
    Matched    => ...,
    Differed   => ...,
    Done       => ...,
}
```

The implementation MUST NOT add a wildcard arm. The state machine is
exhaustive and the type system enforces it.

## 3. Error and Failure Sub-Workflows

The state machine has three failure sub-paths. Each is a single
transition to a terminal `ContractViolation` state with a `GateError`
payload.

### 3.1 Pattern file missing (master amendment stale)

If the master document is missing, unreadable, or no longer contains
the expected closed sets, the policy constructor fails with
`GateError::PatternFileMissing`. The bash wrapper exits 2.

```
Init --(init fails)--> ContractViolation(exit=2)
```

### 3.2 Glob unreadable (filesystem failure)

If a quarantined crate's path is unreadable, the walker fails with
`GateError::GlobUnreadable`. The bash wrapper exits 2.

```
Loaded --(walk fails)--> ContractViolation(exit=2)
```

### 3.3 Allowlist parse failure (allowlist malformed)

If the allowlist file is unreadable or contains a malformed line, the
init step fails with `GateError::AllowlistParseFailure`. The bash
wrapper exits 2.

```
Init --(init fails)--> ContractViolation(exit=2)
```

### 3.4 Unhandled panic (catch-all)

The bash wrapper uses `set -euo pipefail` and the scanner's `main`
function returns a `Result<GateDecision, GateError>`. Any unhandled
panic is caught by the bash `trap` and translated to
`GateError::ScriptInvocationFailure` with exit 2.

```
ANY --(panic)--> ContractViolation(exit=2)
```

## 4. Bash Wrapper Sub-Workflow

The `scripts/forbid-runtime-fmt.sh` script is itself a small state
machine. Its states are bash-local and are not part of the Rust
aggregate, but they correspond 1:1 to the aggregate's states.

| Bash state | Aggregate state | Effect |
|------------|-----------------|--------|
| `Pre-flight` | (none) | Verify the script is run from the repo root (exit 64 if not). |
| `Build` | (none) | `rustc --edition=2024 scripts/forbid-runtime-fmt.rs -o target/gate-tools/forbid-runtime-fmt` |
| `Run` | `Init -> Done` | Invoke the binary, capture stdout and stderr, propagate exit code. |
| `Report` | (terminal) | Print the captured stderr to the parent moon task's stderr; print the captured stdout to the parent moon task's stdout. |

```
Pre-flight --(cd to repo root)--> Build --(rustc exit 0)--> Run --(binary exit)--> Report --> exit
                                       \-- (rustc exit != 0)--> Report(exit=2)
```

The `Run` step's exit code is the binary's exit code. The binary's
exit code is the `GateDecision::exit_code` translation:

| Aggregate decision | Bash exit code |
|--------------------|----------------|
| `GateDecision::Pass` | 0 |
| `GateDecision::Fail(_)` | 1 |
| `GateError::NewResidueDetected` | 1 |
| `GateError::PatternFileMissing(_)` | 2 |
| `GateError::GlobUnreadable { .. }` | 2 |
| `GateError::AllowlistParseFailure { .. }` | 2 |
| `GateError::ScriptInvocationFailure(_)` | 2 |

## 5. Moon Pipeline Sub-Workflow

The new gate participates in the `moon run :check` workflow. The
moon pipeline is itself a state machine, and the new gate is one
node in the dependency graph.

```
fmt
  -> lint-src
  -> check
       -> forbid-runtime-fmt   <-- NEW
       -> nightly-feature-gate
       -> hot-cold-forbidden-apis
       -> check-removed-crate-residue
       -> check-removed-feature-residue
       -> ... (other deps)
       -> cargo check --workspace --all-targets --all-features
```

The new gate is added as a `deps:` of `:check`, ordered *before* the
heavier compile gates (matching the `hot-cold-forbidden-apis`
placement at `.moon/tasks/all.yml` line 117). This is the
`test_moon_ci_quarantine_dependency_correctly_ordered` invariant.

### 5.1 Moon task contract

```
forbid-runtime-fmt:
  command: 'bash scripts/forbid-runtime-fmt.sh'
  toolchains:
    - rust
  inputs:
    - 'scripts/forbid-runtime-fmt.sh'
    - 'scripts/forbid-runtime-fmt.rs'
    - 'scripts/forbid-runtime-fmt.allow'
    - 'fixtures/forbid-runtime-fmt/**/*'
    - 'crates/vb_core/src/**/*'
    - 'crates/vb_runtime/src/**/*'
    - 'crates/vb_storage/src/**/*'
    - 'crates/vb_ipc/src/**/*'
    - '.moon/tasks/all.yml'
  options:
    runInCI: true
```

The `inputs:` list intentionally enumerates the four hot crate source
roots (matching the sibling `hot-cold-forbidden-apis` at line 633)
so moon's cache invalidation observes a change to any hot crate as
a gate re-run. The `Cargo.lock` and `Cargo.toml` files are
deliberately NOT in the inputs list — the gate does not scan them
per OQ-004 / OQ-006.

## 6. Decision Outcomes

| Outcome | What the user sees | What `moon ci` does | What the bead stores |
|---------|--------------------|----------------------|----------------------|
| `Pass` (active == 0) | `summary: active=0 ...` on stdout, no other output | proceeds to the next task in the pipeline | n/a (no evidence stored) |
| `Fail` (active >= 1) | one or more `RUNTIME-FMT:` lines on stderr, `summary: active>=1 ...` on stderr | aborts the pipeline, surfaces the stderr to the developer | n/a (failure is the failure) |
| `ContractViolation` (exit 2) | one `GateError:*` line on stderr | aborts the pipeline, surfaces the stderr to the developer | n/a (the bash wrapper has a bug or the master changed) |

## 7. Workflow Invariants

The following invariants hold for every execution of the gate:

1. The state machine is total. Every `ResidueQuarantineState` is
   either an intermediate state on the happy path or a terminal
   `ContractViolation` state. There is no other terminal state.
2. The `Init -> Loaded` transition validates the policy and the
   allowlist *before* the walker is constructed. A bad allowlist
   never reaches the walker.
3. The `Walked -> Matched` transition only reads hot paths. Cold
   paths contribute zero matches. A cold path with a read error is
   silently skipped; a hot path with a read error transitions to
   `ContractViolation`.
4. The `Matched -> Differed` transition is deterministic. Given a
   fixed `Vec<ResidueMatch>` and a fixed `AllowlistRef`, the
   partition into `active` vs `allowlisted` is identical across
   runs. (See RQ-005 in `proof-seeds.jsonl`.)
5. The `Differed -> Done` transition is deterministic. Given a
   fixed `ScanReport`, the `GateDecision` is identical across runs.
6. The stderr output is line-ordered: the scanner emits all
   residue-match lines, then the summary line, then nothing else.
   (See RQ-005.)
7. The state machine has no cycles. `Init` and `ContractViolation`
   are the only states with no incoming transition.

## 8. Workflow Sequence (Happy Path)

For one `bash scripts/forbid-runtime-fmt.sh` invocation against a
clean tree, the sequence is:

1. `Pre-flight`: bash verifies the script is run from the repo root.
2. `Build`: bash runs `rustc --edition=2024 scripts/forbid-runtime-fmt.rs -o target/gate-tools/forbid-runtime-fmt`.
3. `Init`: scanner constructs `ResiduePolicy` from the master
   document, validates the closed sets.
4. `Loaded`: scanner constructs `AllowlistRef` from
   `scripts/forbid-runtime-fmt.allow` (which may be empty).
5. `Walked`: scanner walks the four quarantined crates and
   partitions `.rs` files into hot vs cold paths.
6. `Matched`: scanner reads each hot path line-by-line and emits
   zero or more `ResidueMatch` values.
7. `Differed`: scanner partitions matches into `active` and
   `allowlisted`.
8. `Done`: scanner reduces to `GateDecision::Pass` (active == 0) and
   emits `summary: active=0 allowlisted=0 files_scanned=N hot_paths=H cold_paths=C`.
9. `Report`: bash exits 0; the moon task succeeds; the pipeline
   proceeds.

For a tree with a single forbidden import:

- Step 6 emits one `ResidueMatch`.
- Step 7 places it in `active` (no allowlist entry matches).
- Step 8 reduces to `GateDecision::Fail(vec![match])` and emits
  `<file>:<line>: RUNTIME-FMT: <name>: <snippet>` followed by
  `summary: active=1 allowlisted=0 files_scanned=N hot_paths=H cold_paths=C`.
- Step 9 bash exits 1; the moon task fails; the pipeline aborts.

## 9. Workflow Sequence (Contract Violation)

For a `GateError::PatternFileMissing` (master document is missing):

1. `Pre-flight`: bash verifies the script is run from the repo root.
2. `Build`: bash runs `rustc` (which succeeds, since the scanner
   does not embed the master document at compile time).
3. `Init`: scanner calls `ResiduePolicy::from_master` and the
   master file does not exist; the function returns
   `Err(GateError::PatternFileMissing("serde_json"))`.
4. `Done`: scanner emits `GateError:PatternFileMissing: serde_json`
   on stderr and exits 2.
5. `Report`: bash exits 2; the moon task fails; the pipeline
   aborts with a contract-violation message.

The same pattern holds for `GlobUnreadable`,
`AllowlistParseFailure`, and `ScriptInvocationFailure`.

## 10. Workflow Sequence (Allowlisted Residue)

For a tree with one forbidden import and one matching allowlist
entry:

1. Steps 1-5 are unchanged.
2. Step 6 emits one `ResidueMatch`.
3. Step 7 looks up the match's `(file, line_no, name)` key in
   `AllowlistRef::entries` and finds a match. The `ResidueMatch` is
   moved to `allowlisted` and the `AllowlistEntry` is attached.
4. Step 8 reduces to `GateDecision::Pass` (active == 0) and emits
   `<file>:<line>: allowlisted: <reason>: <snippet>` followed by
   `summary: active=0 allowlisted=1 files_scanned=N hot_paths=H cold_paths=C`.
5. Step 9 bash exits 0; the moon task succeeds; the pipeline
   proceeds.

The allowlisted line is on stderr; the summary is on stdout. (See
`boundary-map.md` §4 for the exact stdout/stderr split.)

## 11. Workflow Concurrency

The gate is a single-threaded, single-process workflow. There is no
concurrency. The aggregate is not `Send` or `Sync` and the scanner
does not spawn threads.

If the State 11 implementation needs parallelism, the contract
permits a per-quarantined-crate parallel walk, but the
`ResidueQuarantine` aggregate MUST remain a single owner of the
final `ScanReport`. The implementation must use message-passing
channels to fan out, never shared mutable state.

This restriction is documented for completeness; the State 11
implementation is expected to be single-threaded for simplicity.

## 12. Workflow Performance

The contract binds a 30-second wall-clock budget for a full scan of
the four hot crates on the current source tree. The budget is
verified by the `test-forbid-runtime-fmt.sh` test that runs the gate
against the real repository and asserts the elapsed time is below
30 seconds.

The budget is generous: the scanner's line-by-line walk is O(N) in
the total source line count, and the four hot crates together have
on the order of 30,000 lines of Rust source. A naive `rustc` scanner
should complete in well under 1 second; the 30-second budget is
a sanity check against future regressions.
