# Proof Evidence - vb-qi37.4.2

Timestamp: `2026-05-15T22:33:47Z`

State: 5 proof-writer repair attempt 3 after State 4 plan repair.

## Workspace Evidence

### `pwd -P`

Exit: 0

```text
/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-qi37-4-2
```

## Artifact Writes

- `.beads/vb-qi37.4.2/proof-writer-report.md`: refreshed attempt 3 summary after State 4 plan repair.
- `.beads/vb-qi37.4.2/proof-evidence.md`: refreshed attempt 3 command evidence and planned downstream evidence-policy boundaries.
- `.beads/vb-qi37.4.2/STATE.md`: appended State 5 attempt 3 transition/completion.
- No production source, tests, proof/model logic, dependency files, or CI configuration were changed.

## Tool Discovery Evidence

### `which java`

Exit: 0

```text
/home/lewis/.local/share/mise/installs/java/26.0.1/bin/java
```

### `which tlc`

Exit: 0

```text
/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc
```

### `which verus`

Exit: 0

```text
/home/lewis/.local/bin/verus
```

### `verus --version`

Exit: 0

```text
Verus
  Version: 0.2026.05.05.d03e906
  Profile: release
  Platform: linux_x86_64
  Toolchain: 1.95.0-x86_64-unknown-linux-gnu
```

### `cargo kani --version`

Exit: 0

```text
cargo-kani 0.67.0
```

### `cargo fuzz --version`

Exit: 0

```text
cargo-fuzz 0.13.1
```

### `cargo +nightly miri --version`

Exit: 0

```text
miri 0.1.0 (e0e95a7187 2026-04-04)
```

### `cargo flux --version`

Exit: non-zero

```text
error: no such command: `flux`

help: a command with a similar name exists: `fix`

help: view all installed commands with `cargo --list`
help: find a package to install `flux` with `cargo search cargo-flux`
```

Status: `BLOCKED_TOOLING` for Flux discovery only. Flux is not applicable for this bead per `PO-017`; no Flux proof is claimed.

## TLA+ Evidence

All TLC runs used:

```text
TMPDIR=target/tmp
JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp
```

### `PO-001` / `TLA-ADMIT-001`

Command:

```text
TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/vb-qi37-4-2-tlc-all -config verification/tla/CapabilityLifecycleAll.cfg verification/tla/CapabilityLifecycle.tla
```

Exit: 0

Relevant output:

```text
Model checking completed. No error has been found.
478 states generated, 220 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 3.
```

### `PO-002` / `TLA-GATE-002`

Command:

```text
TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/vb-qi37-4-2-tlc-gate -config verification/tla/CapabilityLifecycleGateMismatch.cfg verification/tla/CapabilityLifecycle.tla
```

Exit: 0

Relevant output:

```text
Model checking completed. No error has been found.
478 states generated, 220 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 3.
```

### `PO-003` / `TLA-CAP-003` Excess Grant

Command:

```text
TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/vb-qi37-4-2-tlc-excess -config verification/tla/CapabilityLifecycleExcessGrant.cfg verification/tla/CapabilityLifecycle.tla
```

Exit: 0

Relevant output:

```text
Model checking completed. No error has been found.
478 states generated, 220 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 3.
```

### `PO-003` / `TLA-CAP-003` Exact Profile

Command:

```text
TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/vb-qi37-4-2-tlc-exact -config verification/tla/CapabilityLifecycleExactProfile.cfg verification/tla/CapabilityLifecycle.tla
```

Exit: 0

Relevant output:

```text
Model checking completed. No error has been found.
478 states generated, 220 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 3.
```

### `PO-004` / `TLA-BYPASS-004`

Command:

```text
TMPDIR=target/tmp JAVA_TOOL_OPTIONS=-Djava.io.tmpdir=target/tmp tlc -metadir target/tmp/vb-qi37-4-2-tlc-legacy -config verification/tla/CapabilityLifecycleLegacyBypass.cfg verification/tla/CapabilityLifecycle.tla
```

Exit: 0

Relevant output:

```text
Model checking completed. No error has been found.
478 states generated, 220 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 3.
```

## Verus Evidence

### `PO-005` / `VERUS-CAP-005`

Command:

```text
TMPDIR=target/tmp verus verification/verus/capability_artifact_model.rs
```

Exit: 0

Output:

```text
verification results:: 8 verified, 0 errors
```

### `PO-006` / `VERUS-ENV-006`

Command:

```text
TMPDIR=target/tmp verus verification/verus/accepted_envelope_model.rs
```

Exit: 0

Output:

```text
verification results:: 8 verified, 0 errors
```

## Downstream Policy, Blockers, And Not-Run Items

- `PO-007` Kani digest harness: `planned` downstream evidence-policy row with `waiver_policy`; `cargo-kani` exists, but `verification/kani/digest_admission_harness.rs` is absent. No Kani pass or contract-time waiver is claimed.
- `PO-008` accepted-envelope fuzz target: `planned` downstream evidence-policy row with `waiver_policy`; `cargo-fuzz` exists, but `fuzz/fuzz_targets/accepted_artifact_envelope.rs` is absent. No fuzz pass or contract-time waiver is claimed.
- `PO-009` broad invalid-space proptest: `planned` downstream evidence-policy row with `waiver_policy`; no confirmed proptest feature/target in State 5 scope. No proptest pass or contract-time waiver is claimed.
- `PO-010` static scan/lint: `NOT_RUN` in State 5 because repaired plan keeps owner state 8; no static/lint pass is claimed.
- `PO-011` diagnostic mutation: `planned` downstream evidence-policy row with `waiver_policy` until diagnostic tests and bounded cargo-mutants target exist. No mutation pass or contract-time waiver is claimed.
- `PO-012` canonical CI: `planned` downstream evidence-policy row with `waiver_policy`; formal-verifier/landing must run `moon ci` or record downstream `WAIVED`/`DEFERRED` before any CI pass claim. No contract-time waiver is claimed.
- `PO-013` Lean/Aeneas/Hax: `not_applicable`; no theorem-kernel proof claimed.
- `PO-014` TLA+ liveness: `not_applicable`; this bead's admission gate scope is safety-only.
- `PO-015` Loom: `not_applicable`; no concurrency interleaving risk in scope.
- `PO-016` Miri: `not_applicable`; no unsafe/FFI/raw-pointer UB trigger in scope.
- `PO-017` Flux: `not_applicable`; Flux tooling is also unavailable. No Flux coverage claimed.
- `PO-018` dependency audit/geiger: `not_applicable`; no dependency manifests or build scripts are in bead scope.
- `PO-019` strict admission tests: `NOT_RUN` in State 5 because owner state is 8; no test pass is claimed.

## Assumptions

- TLA+ uses finite safety abstraction with `GateCounts={0,2,CanonicalGate}`, `CapabilityCounts=0..2`, and `CanonicalGate=15`.
- TLA+ proves admission lifecycle safety, not byte decoding, runtime storage implementation, production constructor wiring, or liveness.
- Verus `capability_artifact_model.rs` proves exact capability predicates on decoded domain values.
- Verus `accepted_envelope_model.rs` proves decoded accepted-envelope predicates only.
- Raw hostile bytes, digest storage equality, production strict-path wiring, diagnostic preservation, integration behavior, mutation resistance, and canonical CI remain deferred to their explicit planned downstream evidence-policy lanes.
