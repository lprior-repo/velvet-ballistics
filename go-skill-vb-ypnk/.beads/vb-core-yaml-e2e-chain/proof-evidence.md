# Proof Evidence: vb-core-yaml-e2e-chain

## Evidence Summary

- `PO-001`: PASS by TLC on `verification/tla/YamlE2eChain.tla` and `verification/tla/YamlE2eChain.cfg`; exit status 0.
- `PO-002`: PASS by TLC on ordered sequence journal model; exit status 0; model checks append-order prefix durability for `RunAccepted <= RunAdmission <= RunFinished`.
- `PO-003`: PASS by TLC on YAML-free recovery and persisted-only recovery inputs; exit status 0.
- `PO-004`: PASS_WITH_COMPENSATION by Verus pure proof plus `vb_storage` and `cli_integration` executable compensation; all commands exit 0.
- `PO-005`: PASS_WITH_COMPENSATION by Verus pure proof plus Kani admission proof, `vb_runtime`, and `vb_storage` executable compensation; all commands exit 0.
- `PO-006`: PASS_COMPENSATING_SUITE by `cargo test -p vb_storage -- --nocapture`; exit status 0.
- `PO-007`: PASS_COMPENSATING_SUITE by `cargo test -p velvet_ballastics --test cli_integration -- --nocapture`; exit status 0.
- `PO-012`: PASS by `cargo kani -p vb_runtime --harness yaml_e2e_admission_matrix`; exit status 0; 1 harness verified, 0 failures.
- `PO-008`, `PO-009`, `PO-010`, `PO-013`, `PO-017`: NOT_RUN in State 5 because their planned artifacts are downstream production integration/static/Miri/CI gates.
- `PO-011`: PARTIAL_COMPENSATION only; storage/runtime/CLI suites passed, but the full planned chained command was not run in this State 5 repair.
- `PO-014`: WAIVED_BY_PLAN.
- `PO-015`, `PO-016`: NOT_APPLICABLE_BY_PLAN.

## Raw Command Evidence

### Workspace Check

```text
Command: pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain" && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; esac && test -d .beads/vb-core-yaml-e2e-chain && printf 'isolation-ok\n'
Exit status: 0
Output:
/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-core-yaml-e2e-chain
isolation-ok
```

### TLC

```text
Command: mkdir -p target/tmp/tlc && TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/tlc -config verification/tla/YamlE2eChain.cfg verification/tla/YamlE2eChain.tla; code=$?; printf '\nEXIT_STATUS=%s\n' "$code"; exit "$code"
Tool: TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)
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

### Verus

```text
Command: mkdir -p target/tmp && TMPDIR=target/tmp verus verification/verus/yaml_e2e_digest_roles.rs; code=$?; printf '\nEXIT_STATUS=%s\n' "$code"; exit "$code"
Tool: /home/lewis/.local/bin/verus
Exit status: 0
Output:
verification results:: 8 verified, 0 errors
EXIT_STATUS=0
```

### Kani

```text
Command: mkdir -p target/tmp && TMPDIR=target/tmp cargo kani -p vb_runtime --harness yaml_e2e_admission_matrix; code=$?; printf '\nEXIT_STATUS=%s\n' "$code"; exit "$code"
Tool: cargo-kani 0.67.0
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

### Storage Compensation

```text
Command: mkdir -p target/tmp crates/vb_storage/target/tmp crates/vb_runtime/target/tmp crates/velvet_ballastics/target/tmp; RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo test -p vb_storage -- --nocapture; code=$?; printf '\nEXIT_STATUS=%s\n' "$code"; exit "$code"
Exit status: 0
Output excerpt:
cargo test: 983 passed (7 suites, 43.30s)
EXIT_STATUS=0
```

### Runtime Compensation

```text
Command: RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo test -p vb_runtime -- --nocapture; code=$?; printf '\nEXIT_STATUS=%s\n' "$code"; exit "$code"
Exit status: 0
Output excerpt:
cargo test: 1460 passed (10 suites, 1.16s)
EXIT_STATUS=0
```

### CLI Compensation

```text
Command: RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo test -p velvet_ballastics --test cli_integration -- --nocapture; code=$?; printf '\nEXIT_STATUS=%s\n' "$code"; exit "$code"
Exit status: 0
Output excerpt:
cargo test: 86 passed (1 suite, 1.24s)
EXIT_STATUS=0
```

### Formatting

```text
Command: mkdir -p target/tmp && TMPDIR=target/tmp rtk cargo fmt --check; code=$?; printf '\nEXIT_STATUS=%s\n' "$code"; exit "$code"
Exit status: 0
Output:
EXIT_STATUS=0
```

## Proof Artifact Map

- `verification/tla/YamlE2eChain.tla`: finite one-run TLA+ lifecycle abstraction for strict admission, persist-before-ack, ordered journal projection safety, YAML-free recovery, persisted-only recovery inputs, mismatch fail-closed behavior, and restart resolution.
- `verification/tla/YamlE2eChain.cfg`: TLC invariant and temporal property selection for PO-001 through PO-003. Deadlock checking remains enabled.
- `verification/verus/yaml_e2e_digest_roles.rs`: pure Verus proof for source/artifact digest role separation, mismatch classification, invalid-artifact denial, deterministic same-input recovery classification, and pure shell-target mapping.
- `crates/vb_runtime/src/yaml_e2e_admission_matrix.rs`: crate-local Kani-discoverable admission matrix proof for PO-012.
- `verification/kani/yaml_e2e_admission_matrix.rs`: standalone reference copy; not used as crate-discovery evidence.

## Assumptions And Bounds

- TLA+ uses finite boolean abstractions for digest validity, artifact validity, gate validity, proof validity, capability validity, and replay validity.
- TLA+ represents journal evidence as an ordered sequence and proves accepted/admitted/finished event order when those events exist.
- TLA+ limits the model to one crash/restart attempt; repeated crash of failed terminal states is outside this finite State 5 proof model.
- Verus proof is a pure model. BLAKE3, Fjall I/O, postcard decode, CLI formatting, runtime scheduling, and concrete target calls are trusted shell boundaries supported here by focused executable compensation, not by Verus alone.
- Kani matrix bounds gate count representatives to `0`, `2`, `14`, `15`, and `16`.

## Non-Pass Claims

- No claim is made that State 7/8/11/12 obligations are globally complete.
- No claim is made that `moon ci`, Miri, static boundary/clippy, strict YAML tests, full error taxonomy chained command, or workspace recovery integration passed in State 5.
- No production runtime behavior was intentionally changed; the crate-source edit is gated behind `#[cfg(kani)]` for proof discovery.
