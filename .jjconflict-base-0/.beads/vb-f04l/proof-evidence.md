# Proof Evidence: vb-f04l

## Summary

- Timestamp: `2026-05-15T17:57:59-05:00`.
- State: proof-writer State 5 repair after State 6 attempt 3 rejection.
- Attempt: 4-of-7.
- Result: PASS for required State 5 Verus and TLA+ proof artifacts only.
- `BLOCKED_TOOLING`: none for State 5 proof lanes after workspace-local Java temp configuration.
- Source checkout writes: none.

## Artifact Ledger

| Artifact | Obligations | Status |
| --- | --- | --- |
| `verification/verus/v1_primitive_lowering.rs` | `PRE-007`, `POST-003`, `POST-004`, `POST-005`, `POST-006-VERUS`..`POST-012-VERUS`, `INV-001`, `INV-003`, `INV-004`, `INV-005` | PASS via Verus |
| `verification/tla/V1PrimitiveLowering.tla` | `POST-006-TLA`..`POST-012-TLA`, `INV-002` | PASS via TLC |
| `verification/tla/V1PrimitiveLowering.cfg` | `POST-006-TLA`..`POST-012-TLA`, `INV-002` | PASS via TLC |

## Canonical Mapping

- `POST-006-VERUS` through `POST-012-VERUS`: exact required proof function now exists at `verification/verus/v1_primitive_lowering.rs` as `proof_lowering_plan_preserves_primitive_shapes(source: SourceInputs, tag: int)`.
- `PRE-007`, `POST-004`, `POST-005`, `INV-001`, `INV-003`, `INV-004`: source-derived proofs use `source_inputs_valid(source)` and `construct_plan(source)` rather than requiring the final `constructor_inputs_valid(plan)` predicate directly.
- `POST-003`, `INV-005`: determinism proof maps equal source inputs through deterministic `construct_plan` output equality.
- `POST-006-TLA` through `POST-012-TLA`, `INV-002`: TLC now checks target route variation through `TargetChoices == 0..2`, not a single fixed layout.

## Raw Command Evidence

### `pwd -P`

- Exit: 0

```text
/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l
```

### Artifact Existence Gate

- Command: `test -s .beads/vb-f04l/proof-obligations.jsonl && test -s .beads/vb-f04l/proof-obligations.planned.jsonl && test -s .beads/vb-f04l/proof-writer-report.md && test -s .beads/vb-f04l/proof-evidence.md && test -s verification/verus/v1_primitive_lowering.rs && test -s verification/tla/V1PrimitiveLowering.tla && test -s verification/tla/V1PrimitiveLowering.cfg`
- Exit: 0
- Output: none.

### JSONL Gate

- Command: `jq -c . .beads/vb-f04l/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-f04l/proof-obligations.planned.jsonl >/dev/null && jq -c . .beads/vb-f04l/traceability-matrix.jsonl >/dev/null`
- Exit: 0
- Output: none.

### Tool Discovery

- Command: `which verus && which tlc && which java`
- Exit: 0

```text
/home/lewis/.local/bin/verus
/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc
/home/lewis/.local/share/mise/installs/java/26.0.1/bin/java
```

### Mapping Scan

- Command: grep for `proof_lowering_plan_preserves_primitive_shapes` under `verification/verus`.
- Exit: 0

```text
verification/verus/v1_primitive_lowering.rs:286: pub proof fn proof_lowering_plan_preserves_primitive_shapes(source: SourceInputs, tag: int)
```

### `TMPDIR=target/tmp verus verification/verus/v1_primitive_lowering.rs`

- Exit: 0

```text
verification results:: 15 verified, 0 errors
```

### `TMPDIR=target/tmp tlc -config verification/tla/V1PrimitiveLowering.cfg verification/tla/V1PrimitiveLowering.tla`

- Exit: non-zero before model check.
- Status: NOT PASS EVIDENCE.

```text
java.io.IOException: Disk quota exceeded
Fatal errors while parsing TLA+ spec in file V1PrimitiveLowering
```

### Workspace-Local Java Temp TLC Rerun

- Command: `mkdir -p "target/tmp" && TMPDIR="$PWD/target/tmp" JAVA_TOOL_OPTIONS="-Djava.io.tmpdir=$PWD/target/tmp" tlc -config verification/tla/V1PrimitiveLowering.cfg verification/tla/V1PrimitiveLowering.tla`
- Exit: 0

```text
Picked up JAVA_TOOL_OPTIONS: -Djava.io.tmpdir=/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l/target/tmp
TLC2 Version 2.19 of 08 August 2024 (rev: 5a47802)
Running breadth-first search Model-Checking with fp 32 and seed -5441496190825684737 with 1 worker on 32 cores with 30688MB heap and 64MB offheap memory [pid: 1589501].
Finished computing initial states: 1632960 distinct states generated at 2026-05-15 17:55:01.
Model checking completed. No error has been found.
5909760 states generated, 3491424 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 7.
Finished in 02min 48s at (2026-05-15 17:57:47)
```

## Non-Run Obligations

- Owner-state 8 cargo-test/proptest obligations: NOT_RUN in State 5 by instruction.
- Owner-state 11 `moon ci`, static scan, forbidden construct, dependency boundary, and legacy inventory obligations: NOT_RUN in State 5 by instruction.
- Kani, Loom, Miri, Flux, fuzz, and Lean rows remain not-applicable/waived exactly as planned by State 4; no new waiver was created here.

## Evidence Integrity

- No production source files were edited.
- No tests were edited.
- No dependencies or CI config were edited.
- No verifier result is inferred from an unrun command.
- The TLC disk-quota failure is recorded as a failed setup run only; PASS evidence comes from the workspace-local Java temp rerun.
