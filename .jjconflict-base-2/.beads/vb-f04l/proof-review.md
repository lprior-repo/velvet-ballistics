# Proof Review: vb-f04l State 6 Retry

STATUS: APPROVED

## Scope

- Bead: `vb-f04l`.
- Role: go-skill State 6 proof-reviewer retry after State 5 attempt 4 repair.
- Workspace verified by `pwd -P`: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l`.
- Source checkout isolation: current path is not `/home/lewis/src/velvet-ballistics` and is not nested under it.
- Review writes: `.beads/vb-f04l/proof-review.md`, `.beads/vb-f04l/proof-findings.jsonl`, and appended completion evidence in `.beads/vb-f04l/STATE.md` only.
- Proof/code edits: none by this review.

## Decision

The repaired State 5 proof artifacts satisfy State 6 proof-review for owner-state 5 obligations. The previous blockers are repaired: the canonical Verus proof function exists and verifies, the TLA+ model now checks bounded target variation rather than one fixed representative layout, and raw verifier evidence was rerun in the isolated workspace.

## Findings

- No blocking proof-review findings for State 5 owner obligations.
- Non-blocking residual risk: `verification/verus/v1_primitive_lowering.rs` remains an abstract source-input/plan proof, not a direct proof over `compile_source`. This matches the stated trusted boundary and must be consumed later by implementation/test/formal gates before landing.
- Non-blocking residual risk: `verification/tla/V1PrimitiveLowering.tla` proves lifecycle properties over bounded prevalidated target choices `0..2` with configured finite bounds, not arbitrary graph sizes or malformed production emissions. This is acceptable for State 5 proof scope because malformed/concrete compiler behavior remains assigned to later tests and State 11 gates.

## Evidence Run This Review

- `pwd -P && test "$(pwd -P)" = "/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l" && case "$(pwd -P)" in "/home/lewis/src/velvet-ballistics"|"/home/lewis/src/velvet-ballistics"/*) exit 1;; esac`: exit 0, output `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-f04l`.
- Artifact and JSONL gate: `test -s ... && jq -c . ... >/dev/null`: exit 0 for `proof-obligations.jsonl`, `proof-obligations.planned.jsonl`, `traceability-matrix.jsonl`, `proof-writer-report.md`, `proof-evidence.md`, Verus artifact, TLA+ model, and TLA+ config.
- Tool discovery: `which verus && which tlc && which java`: exit 0, tools found at `/home/lewis/.local/bin/verus`, `/home/lewis/.local/share/mise/installs/http-tla2tools/1.7.4/tlc`, and `/home/lewis/.local/share/mise/installs/java/26.0.1/bin/java`.
- Verus rerun: `TMPDIR=target/tmp verus verification/verus/v1_primitive_lowering.rs`: exit 0, `verification results:: 15 verified, 0 errors`.
- TLC rerun: `mkdir -p "target/tmp" && TMPDIR="$PWD/target/tmp" JAVA_TOOL_OPTIONS="-Djava.io.tmpdir=$PWD/target/tmp" tlc -config verification/tla/V1PrimitiveLowering.cfg verification/tla/V1PrimitiveLowering.tla`: exit 0, `Model checking completed. No error has been found.`, `5909760 states generated`, `3491424 distinct states found`, `0 states left on queue`, depth `7`.

## Obligation Review

- `PRE-007`, `POST-003`, `POST-004`, `POST-005`, `INV-001`, `INV-003`, `INV-004`, `INV-005`: approved for State 5 Verus scope through `SourceInputs`, `source_inputs_valid`, `construct_plan`, and verified source-derived proof functions.
- `POST-006-VERUS` through `POST-012-VERUS`: approved for State 5 Verus scope through exact proof function `proof_lowering_plan_preserves_primitive_shapes(source: SourceInputs, tag: int)` at `verification/verus/v1_primitive_lowering.rs:286`.
- `POST-006-TLA` through `POST-012-TLA` and `INV-002`: approved for State 5 TLA+ scope through bounded varied target choices and TLC completion with no errors.

## Next Gate

State 6 proof-review is approved. Downstream states must still prove concrete compiler behavior, production bridge fidelity, tests, `moon ci`, static scans, and implementation-level acceptance before landing.
