# Proof Writer Report: vb-core-yaml-e2e-chain

## Scope

- State: 5 proof/model/harness writing repair after State 6 rejection.
- Attempt: 3-of-7.
- Timestamp: 2026-05-15T22:44:55Z.
- Skill: proof-writer.
- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain`.
- Boundary: proof/model/harness artifacts and `.beads` evidence only. The only crate-source touch is `#[cfg(kani)]` verifier target wiring in `crates/vb_runtime/src/lib.rs`; it has no non-Kani runtime behavior.

## Inputs Read

- `.beads/vb-core-yaml-e2e-chain/STATE.md` through State 6 attempt 3 rejection.
- `.beads/vb-core-yaml-e2e-chain/proof-review.md`.
- `.beads/vb-core-yaml-e2e-chain/proof-findings.jsonl`.
- `.beads/vb-core-yaml-e2e-chain/proof-repair-guide.md`.
- `.beads/vb-core-yaml-e2e-chain/contract-verification-review.md`.
- `.beads/vb-core-yaml-e2e-chain/proof-obligations.planned.jsonl`.
- `.beads/vb-core-yaml-e2e-chain/proof-strategy.md`.
- `.beads/vb-core-yaml-e2e-chain/contract.md`.
- `.beads/vb-core-yaml-e2e-chain/traceability-matrix.jsonl`.

## Artifacts Written

- `verification/tla/YamlE2eChain.tla` for PO-001, PO-002, PO-003.
- `crates/vb_runtime/src/yaml_e2e_admission_matrix.rs` for PO-012.
- `crates/vb_runtime/src/lib.rs` adds only `#[cfg(kani)] pub mod yaml_e2e_admission_matrix;` target wiring for PO-012 discovery.
- `.beads/vb-core-yaml-e2e-chain/proof-writer-report.md`.
- `.beads/vb-core-yaml-e2e-chain/proof-evidence.md`.
- `.beads/vb-core-yaml-e2e-chain/STATE.md` appended with State 5 attempt 3 transition and completion evidence.

## Repairs Applied

- PO-002/TLA-DUR-002: replaced the set-valued journal with an ordered `Seq(Event)` journal, changed event writes to `Append`, added event index/order predicates, and strengthened `JournalPrefixDurable` to prove `RunAccepted <= RunAdmission <= RunFinished` when those events exist.
- PO-002/TLA-DUR-002: bounded the finite one-run restart abstraction by removing `Failed` from `CrashRestart`; the old set model masked an unbounded failure/restart append loop.
- PO-012/KANI-ADMIT-023: added a crate-local `#[cfg(kani)]` harness module so `cargo kani -p vb_runtime --harness yaml_e2e_admission_matrix` discovers and verifies the required harness.
- PO-004/PO-005: did not weaken Verus. Pure Verus still proves only digest/admission abstractions; shell linkage is supported by the compensating executable commands below.

## Obligation Status

| Obligation | Artifact | Command | Status |
| --- | --- | --- | --- |
| PO-001 | `verification/tla/YamlE2eChain.tla`; `verification/tla/YamlE2eChain.cfg` | `tlc -metadir target/tmp/tlc -config verification/tla/YamlE2eChain.cfg verification/tla/YamlE2eChain.tla` | PASS |
| PO-002 | `verification/tla/YamlE2eChain.tla`; `verification/tla/YamlE2eChain.cfg` | `tlc -metadir target/tmp/tlc -config verification/tla/YamlE2eChain.cfg verification/tla/YamlE2eChain.tla` | PASS |
| PO-003 | `verification/tla/YamlE2eChain.tla`; `verification/tla/YamlE2eChain.cfg` | `tlc -metadir target/tmp/tlc -config verification/tla/YamlE2eChain.cfg verification/tla/YamlE2eChain.tla` | PASS |
| PO-004 | `verification/verus/yaml_e2e_digest_roles.rs` plus storage/CLI compensation | Verus + `cargo test -p vb_storage` + `cargo test -p velvet_ballastics --test cli_integration` | PASS_WITH_COMPENSATION |
| PO-005 | `verification/verus/yaml_e2e_digest_roles.rs` plus Kani/runtime/storage compensation | Verus + Kani + `cargo test -p vb_runtime` + `cargo test -p vb_storage` | PASS_WITH_COMPENSATION |
| PO-006 | production test artifact `crates/vb_storage/src/recovery/tests.rs` | `cargo test -p vb_storage -- --nocapture` | PASS_COMPENSATING_SUITE |
| PO-007 | production test artifact `crates/velvet_ballastics/tests/cli_integration.rs` | `cargo test -p velvet_ballastics --test cli_integration -- --nocapture` | PASS_COMPENSATING_SUITE |
| PO-008 | production workspace recovery integration | not run in State 5 | NOT_RUN_OWNER_STATE_7 |
| PO-009 | static boundary plus clippy | not run in State 5 | NOT_RUN_OWNER_STATE_8 |
| PO-010 | production compile/YAML tests | not run in State 5 | NOT_RUN_OWNER_STATE_7 |
| PO-011 | production focused suites | storage/runtime/CLI compensation run; full chained command not run | PARTIAL_COMPENSATION |
| PO-012 | crate-local `vb_runtime` Kani harness | `cargo kani -p vb_runtime --harness yaml_e2e_admission_matrix` | PASS |
| PO-013 | production Miri codec/recovery paths | not run in State 5 | NOT_RUN_OWNER_STATE_12 |
| PO-014 | fuzz target if present | no bead-specific target discovered by plan | WAIVED_BY_PLAN |
| PO-015 | not applicable | not applicable | NOT_APPLICABLE_BY_PLAN |
| PO-016 | not applicable | not applicable | NOT_APPLICABLE_BY_PLAN |
| PO-017 | workspace release gate | `moon ci` | NOT_RUN_OWNER_STATE_11 |

## Command Evidence

### Workspace Check

```text
Command: pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain" && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; esac && test -d .beads/vb-core-yaml-e2e-chain && printf 'isolation-ok\n'
Exit status: 0
Output:
/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain
isolation-ok
```

### Formatting

```text
Command: mkdir -p target/tmp && TMPDIR=target/tmp rtk cargo fmt --check; code=$?; printf '\nEXIT_STATUS=%s\n' "$code"; exit "$code"
Exit status: 0
Output: EXIT_STATUS=0
```

### TLA+ PO-001/PO-002/PO-003

```text
Command: mkdir -p target/tmp/tlc && TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/tlc -config verification/tla/YamlE2eChain.cfg verification/tla/YamlE2eChain.tla; code=$?; printf '\nEXIT_STATUS=%s\n' "$code"; exit "$code"
Exit status: 0
Output excerpt:
Parsing file target/tmp/Naturals.tla
Parsing file target/tmp/Sequences.tla
Checking temporal properties for the complete state space with 990 total distinct states
Model checking completed. No error has been found.
2728 states generated, 990 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 13.
EXIT_STATUS=0
```

### Verus PO-004/PO-005

```text
Command: mkdir -p target/tmp && TMPDIR=target/tmp verus verification/verus/yaml_e2e_digest_roles.rs; code=$?; printf '\nEXIT_STATUS=%s\n' "$code"; exit "$code"
Exit status: 0
Output:
verification results:: 8 verified, 0 errors
EXIT_STATUS=0
```

### Kani PO-012

```text
Command: mkdir -p target/tmp && TMPDIR=target/tmp cargo kani -p vb_runtime --harness yaml_e2e_admission_matrix; code=$?; printf '\nEXIT_STATUS=%s\n' "$code"; exit "$code"
Exit status: 0
Output excerpt:
Checking harness yaml_e2e_admission_matrix::yaml_e2e_admission_matrix...
SUMMARY:
 ** 0 of 7 failed
VERIFICATION:- SUCCESSFUL
Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
EXIT_STATUS=0
```

### Storage Compensation For PO-004/PO-005/PO-006/PO-011

```text
Command: mkdir -p target/tmp crates/vb_storage/target/tmp crates/vb_runtime/target/tmp crates/velvet_ballastics/target/tmp; RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo test -p vb_storage -- --nocapture; code=$?; printf '\nEXIT_STATUS=%s\n' "$code"; exit "$code"
Exit status: 0
Output excerpt:
cargo test: 983 passed (7 suites, 43.30s)
EXIT_STATUS=0
```

### Runtime Compensation For PO-005/PO-011

```text
Command: RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo test -p vb_runtime -- --nocapture; code=$?; printf '\nEXIT_STATUS=%s\n' "$code"; exit "$code"
Exit status: 0
Output excerpt:
cargo test: 1460 passed (10 suites, 1.16s)
EXIT_STATUS=0
```

### CLI Compensation For PO-004/PO-007/PO-011

```text
Command: RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo test -p velvet_ballastics --test cli_integration -- --nocapture; code=$?; printf '\nEXIT_STATUS=%s\n' "$code"; exit "$code"
Exit status: 0
Output excerpt:
cargo test: 86 passed (1 suite, 1.24s)
EXIT_STATUS=0
```

## Failed Attempts And Classifications

- TLC first rerun: `BLOCK_LOCAL`; Java/TLC wrote standard-library files to quota-exhausted `/tmp`; repaired by `JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp` and `-metadir target/tmp/tlc`.
- Compensating cargo tests with default sccache: `BLOCK_LOCAL`; `/tmp/sccache*/deps.d` hit disk quota.
- Compensating cargo tests with `RUSTC_WRAPPER=` only: `BLOCK_LOCAL`; C compiler wrote `/tmp/cc*.s` and hit disk quota.
- Compensating cargo tests with `CC_FORCE_DISABLE=1`: `BLOCK_LOCAL`; `blake3` build script exited when C compilation was disabled.
- Compensating cargo tests with `CFLAGS=-pipe` before crate-local temp parents: `BLOCK_LOCAL`; test tempdirs failed because `TMPDIR=target/tmp` resolved under each crate and parents did not exist.
- Final compensating cargo tests with crate-local temp parents: PASS.

## Assumptions And Bounds

- TLA+ remains a finite one-run model with one crash/restart attempt; it now models journal order as a sequence rather than a set.
- TLA+ abstracts Fjall durability as source/artifact/header booleans plus ordered journal events, not byte-level storage internals.
- Verus trusts BLAKE3, Fjall I/O, postcard decode, CLI formatting, runtime scheduling, and concrete target calls; executable compensation above is the current shell-link evidence.
- Kani bounds gate count representatives to `0`, `2`, `14`, `15`, and `16`.
- No claim is made for unrun owner_state 7/8/11/12 obligations beyond the focused compensating suites listed above.

## Reviewer Guidance

- Re-review PO-002 against the ordered sequence journal and one-restart finite abstraction.
- Re-review PO-012 using the now-discoverable crate-local Kani harness evidence.
- Do not treat this State 5 repair as full State 7/8/11/12 execution; it only supplies focused compensation for the expired Verus shell waivers and required Kani/TLA repairs.
