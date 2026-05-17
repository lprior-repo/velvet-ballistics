STATUS: PASS

# State 11 Black-Hat Blocker Repair 8 — vb-nf2u

## Reference files read
- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`

## Files changed
- `.beads/vb-nf2u/contract.md`
- `.beads/vb-nf2u/verification-layers.md`
- `.beads/vb-nf2u/proof-obligations.jsonl`
- `.beads/vb-nf2u/traceability-matrix.jsonl`
- `.beads/vb-nf2u/test-plan.md`
- `xtask/src/evidence.rs`
- `tests/vb_nf2u_ui_release_acceptance.rs`
- `xtask/fixtures/vb-nf2u-ui/*.fixture.txt`

## Blocker-by-blocker repair
1. Contract parity: amended contract, verification, proof, traceability, and test-plan language from PNG evidence to explicit fixture-text artifacts with `blake3:` digest/readback for this no-live-Makepad boundary.
2. Positive evidence: `ai-release` now copies checked-in source fixture inputs from `xtask/fixtures/vb-nf2u-ui/` and validates readback bytes instead of generating screen proof solely from in-code format constants.
3. Function size: split the cited `RawNegativeFixtureEntry::into_overlap` / `into_secret` paths into expected-failed and rejected constructors.
4. Rejected negative evidence: replaced neutral/default rejected rows with explicit `ParsedRejectedFixtureEvidence` sum-type variants requiring error, variant, code, expected gate, and actual status.
5. String-key bag API: removed release-critical `field(&self, key: &str)` and changed acceptance tests to assert typed variants/fields directly.
6. Test panic vector: parser helpers now return `Result`; tests propagate parse errors instead of `unwrap_or_else` + `panic_any`.
7. Command diagnostic: false-pass checks parse an exact command diagnostic boundary instead of rubber-stamping any non-empty output.
8. Functional core/shell: split release model construction from filesystem persistence/readback verification.

## Commands run
- `bd prime` — PASS; Dolt auto-push warning was unrelated to local repair.
- `rtk cargo fmt --all --check` — PASS after formatting.
- `rtk cargo clippy -p vb_ui_snapshot -p xtask --tests --all-features -- -D warnings` — PASS; tool reported 0 errors, 2 warnings.
- `if cargo xtask ai-release --bead vb-nf2u-missing; then exit 64; else true; fi` — PASS; unknown bead fails closed.
- `rm -rf "target/vb-nf2u-negative-fixtures" && if cargo xtask ai-release --bead vb-nf2u; then exit 65; else true; fi` — PASS; missing fixtures fail closed.
- `cargo nextest run -p velvet-ballastics-workspace --test vb_nf2u_ui_release_acceptance && cargo xtask ai-release --bead vb-nf2u` — PASS; 8/8 acceptance tests and positive release path passed.
- `cargo nextest run -p velvet-ballastics-workspace --test vb_nf2u_ui_release_acceptance` — PASS as part of the positive-path command; 8/8.
- `cargo nextest run -p vb_ui_snapshot -p xtask` — PASS; 130/130.
- `cargo kani -p vb_ui_snapshot --harness inventory && cargo kani -p vb_ui_snapshot --harness layout_` — PASS; output captured at `/home/lewis/.local/share/opencode/tool-output/tool_e11cd638e001XvN3572NEEDP8S`.
- `moon run :verify-all` — PASS; output captured at `/home/lewis/.local/share/opencode/tool-output/tool_e11debd65001E0fTPMZrhH3rdM`.
- `moon ci --base HEAD --head HEAD` — PASS; `Tasks: 20 completed (2 cached)`, output captured at `/home/lewis/.local/share/opencode/tool-output/tool_e11e02420001G4xUgmTCooj3wi`.

## Power-of-Ten / zero-panic impact
- Rule 4: split reviewed release-critical functions toward <=25-line reviewability.
- Rule 5: moved rejected fixture invariants into explicit sum types rather than neutral strings.
- Rule 7: parser and command-boundary failures are returned and propagated.
- Zero-panic: no production `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` introduced.

## Performance-layer decision
- No performance claim made; no benchmark/profiler evidence required.

## Second-ring evidence
- Kani inventory and layout harnesses passed.
- No assembly/IR/vectorization/API-compatibility/release-provenance claim was made.

## Skipped gates
- No requested gate was skipped. The first combined `moon run :verify-all && moon ci --base HEAD --head HEAD` attempt timed out at the shell tool limit during long Moon work, so both commands were rerun separately and passed.

## Residual risks
- Evidence remains explicitly fixture-backed text evidence, not live Makepad rendering.
- Core/runtime parity remains explicitly unsupported.
- Existing repository-wide warnings about duplicate cargo package/target metadata remain outside this bead repair.
