# Proof Writer Report: vb-f04l

## Scope

- Bead: `vb-f04l`.
- State: 5 proof-writer repair after State 6 attempt 3 rejection.
- Attempt: 4-of-7.
- Timestamp: `2026-05-15T17:57:59-05:00`.
- Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l`.
- Source checkout writes: none; `/home/lewis/src/velvet-ballistics` was not edited.
- Production/test/dependency/CI edits: none.

## Repair Delta

- `verification/verus/v1_primitive_lowering.rs`: added `SourceInputs`, `source_inputs_valid`, `construct_plan`, exact required `proof_lowering_plan_preserves_primitive_shapes`, and source-derived proof functions for dense nodes, target range, slot coverage, bounds, determinism, and primitive shape obligations.
- `verification/tla/V1PrimitiveLowering.tla`: replaced fixed representative target fields with bounded variation via `TargetChoices == 0..2` and target variables selected from that set in `InitLoweredPrimitiveGraph`.
- `.beads/vb-f04l/proof-writer-report.md`: refreshed this report.
- `.beads/vb-f04l/proof-evidence.md`: refreshed raw command evidence and mapping.
- `.beads/vb-f04l/STATE.md`: appended State 5 repair transition and completion evidence.

## Obligation Coverage

| Canonical obligations | Artifact | Status | Evidence |
| --- | --- | --- | --- |
| `PRE-007`, `INV-004` | `verification/verus/v1_primitive_lowering.rs` | PASS | `verus verification/verus/v1_primitive_lowering.rs`, exit 0, `15 verified, 0 errors` |
| `POST-003`, `INV-005` | `verification/verus/v1_primitive_lowering.rs` | PASS | `same_source` to `construct_plan` determinism proof, same Verus command |
| `POST-004`, `INV-001` | `verification/verus/v1_primitive_lowering.rs` | PASS | source-derived dense node and target range proofs, same Verus command |
| `POST-005`, `INV-003` | `verification/verus/v1_primitive_lowering.rs` | PASS | source-derived slot allocator closure proof, same Verus command |
| `POST-006-VERUS` through `POST-012-VERUS` | `verification/verus/v1_primitive_lowering.rs` | PASS | exact required function `proof_lowering_plan_preserves_primitive_shapes`, same Verus command |
| `POST-006-TLA` through `POST-012-TLA`, `INV-002` | `verification/tla/V1PrimitiveLowering.tla`, `.cfg` | PASS | TLC over varied `TargetChoices`, exit 0, no errors |

## Commands Run

### Workspace And Gates

- Command: `pwd -P`
- Exit: 0
- Output: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l`

- Command: `test -s .beads/vb-f04l/proof-obligations.jsonl && test -s .beads/vb-f04l/proof-obligations.planned.jsonl && test -s .beads/vb-f04l/proof-writer-report.md && test -s .beads/vb-f04l/proof-evidence.md && test -s verification/verus/v1_primitive_lowering.rs && test -s verification/tla/V1PrimitiveLowering.tla && test -s verification/tla/V1PrimitiveLowering.cfg`
- Exit: 0
- Output: none.

- Command: `jq -c . .beads/vb-f04l/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-f04l/proof-obligations.planned.jsonl >/dev/null && jq -c . .beads/vb-f04l/traceability-matrix.jsonl >/dev/null`
- Exit: 0
- Output: none.

### Tool Discovery

- Command: `which verus && which tlc && which java`
- Exit: 0
- Output:

```text
/home/lewis/.local/bin/verus
/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc
/home/lewis/.local/share/mise/installs/java/26.0.1/bin/java
```

### Verus

- Command: `TMPDIR=target/tmp verus verification/verus/v1_primitive_lowering.rs`
- Exit: 0
- Output: `verification results:: 15 verified, 0 errors`
- Status: PASS

### TLA+

- Command: `TMPDIR=target/tmp tlc -config verification/tla/V1PrimitiveLowering.cfg verification/tla/V1PrimitiveLowering.tla`
- Exit: non-zero before SANY completion.
- Output: `java.io.IOException: Disk quota exceeded` while TLC tried to read/write standard modules under `/tmp`.
- Status: NOT PASS EVIDENCE.

- Command: `mkdir -p "target/tmp" && TMPDIR="$PWD/target/tmp" JAVA_TOOL_OPTIONS="-Djava.io.tmpdir=$PWD/target/tmp" tlc -config verification/tla/V1PrimitiveLowering.cfg verification/tla/V1PrimitiveLowering.tla`
- Exit: 0
- Key output:

```text
Running breadth-first search Model-Checking with fp 32 and seed -5441496190825684737 with 1 worker on 32 cores with 30688MB heap and 64MB offheap memory.
Finished computing initial states: 1632960 distinct states generated at 2026-05-15 17:55:01.
Model checking completed. No error has been found.
5909760 states generated, 3491424 distinct states found, 0 states left on queue.
The depth of the complete state graph search is 7.
Finished in 02min 48s at (2026-05-15 17:57:47)
```

- Status: PASS

## Assumptions And Trusted Boundaries

- Verus proves an abstract source-input constructor surface, not `compile_source` directly.
- Required production bridge remains future work: later implementation must map emitted node count, target fields, slot refs, primitive tag, branch count, attempt count, and page bound into `SourceInputs` and `construct_plan`.
- TLA+ proves lifecycle/progress and no-deadlock over bounded prevalidated target choices `0..2` with `MaxNodes = 12`, `BranchBound = 4`, `AttemptBound = 4`, `PageBound = 4`, and `InputBound = 4`.
- TLA+ target variation is non-vacuous for all six route fields but remains a bounded model, not a proof over arbitrary `MaxNodes`.
- TLA+ does not prove YAML parsing, dense index construction, slot coverage, heap allocation, digest internals, runtime execution, wall-clock time, human answer service, or event delivery implementation.
- Focused cargo tests, `moon ci`, static scans, and implementation-level checks remain owner-state 8 or 11 obligations per `.beads/vb-f04l/proof-obligations.planned.jsonl`.

## Blockers

- `BLOCKED_TOOLING`: none for required State 5 Verus and TLA+ lanes after using workspace-local Java temp storage.
- `PRODUCTION_DESIGN_BLOCKER`: none introduced by this State 5 repair, but implementation cannot claim concrete compiler proof until the production bridge maps concrete emitted structures into `SourceInputs`.
- `NOT_RUN`: all owner-state 8/11 cargo-test, static-scan, and `moon ci` obligations were not run because they are not State 5 proof-writer obligations.

## Reviewer Guidance

- Review `proof_lowering_plan_preserves_primitive_shapes` as the exact canonical mapping for `POST-006-VERUS` through `POST-012-VERUS`.
- Review TLA+ target variation through `TargetChoices == 0..2`; this is intentionally broader than one representative layout but still bounded.
