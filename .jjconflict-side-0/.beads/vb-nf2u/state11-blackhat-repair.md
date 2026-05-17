STATUS: FAIL

# State 11 Black-Hat Rejection Repair — vb-nf2u

## Files changed
- `xtask/src/main.rs`
- `xtask/src/evidence.rs`
- `xtask/tests/integration_gates.rs`

## Root causes repaired
- `cargo xtask ai-release --bead <unknown>` previously fell through to generic green profile evidence. It now rejects unknown/missing `ai-release` bead IDs before evidence directory creation/report generation.
- `xtask` release profile code used raw bead strings plus optional workflow fields. It now uses `ReleaseBeadId`, `NegativeFixtureWorkflow`, `ReleaseArtifactWorkflow`, and `ReleaseParityClaim` sum types for the repaired release boundary surface.
- Marker constants in `xtask/src/evidence.rs` were removed so tests cannot use marker arrays as proof substitutes.
- Redaction test seam now checks all required raw secret classes, not only `password=hunter2`.
- `xtask/tests/integration_gates.rs` now asserts unknown `ai-release` beads fail closed and do not create `.evidence/<unknown>/ai-release.yaml`.

## Command evidence
- PASS: `rtk cargo check -p xtask`
- PASS: `rm -rf ".evidence/vb-nf2u-missing" && cargo xtask ai-release --bead vb-nf2u-missing; code=$?; test "$code" -ne 0 && test ! -e ".evidence/vb-nf2u-missing/ai-release.yaml"`
  - Observed product error: `Error: unknown ai-release bead id: vb-nf2u-missing`.
- PASS: `cargo nextest run -p velvet-ballastics-workspace --test vb_nf2u_ui_release_acceptance` — 8 passed.
- PASS: `cargo nextest run -p xtask` — 91 passed.
- PASS: `cargo nextest run -p vb_ui_snapshot -p vb_ui_makepad -p xtask` — 131 passed.
- PASS: `rtk cargo fmt --all --check`.
- PASS: `rtk cargo clippy -p xtask --tests --all-features -- -D warnings` — 0 errors.
- FAIL: `moon run :verify-all`.
  - Exact blocker: Kani installation is broken: `error: Unable to find kani_lib.c. Looked for /home/lewis/.local/library/kani/kani_lib.c`.
  - Diagnostics: `command -v cargo-kani` found `/home/lewis/.cargo/bin/cargo-kani`; `cargo kani --version` reported `cargo-kani 0.67.0`; `/home/lewis/.local/library/kani` does not exist.
- PASS: `moon ci --base HEAD --head HEAD` — `Tasks: 20 completed (2 cached)`, output captured at `/home/lewis/.local/share/opencode/tool-output/tool_e1067a77f001SwsJCahG5S29oq`.

## Residual risks / waivers
- `moon run :verify-all` remains red because the local Kani runtime library is absent. I did not claim formal proof pass.
- Fixture-backed evidence still does not prove live Makepad/core parity; output continues to state `core_runtime_parity_claim: unsupported`.
- Acceptance tests still contain legacy substring/block assertions in some helpers; the repaired command-boundary blocker and xtask release ID behavior are covered by executable tests, but a full typed-YAML rewrite of the root acceptance file remains follow-up work.
- Function-length findings in pre-existing/reviewed UI acceptance and image-check helpers were not fully eliminated in this scoped repair; no new production hot-path performance claim is made.

## Performance-layer decision
- No performance claim made; no benchmark/profiler evidence required.

## Second-ring evidence
- Formal second-ring lane attempted through `moon run :verify-all`; blocked by missing Kani library as recorded above.
