STATUS: PASS

# State 11 Black-Hat Repair 7 — vb-nf2u

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

- `Cargo.toml`
- `xtask/src/evidence.rs`
- `tests/vb_nf2u_ui_release_acceptance.rs`
- `xtask/tests/ui_release_gates.rs`

## Blocker-by-blocker repair summary

1. Replaced release-critical evidence document line scraping with `serde_saphyr::from_str` domain parsing into typed raw structs and validated domain newtypes/enums.
2. Replaced `require_document_shape` magic-word `contains()` checks with typed parse/validation of the rendered snapshot, ai-release, negative-fixture, determinism, and animation-freeze documents.
3. Strengthened parsed evidence types with canonical screen IDs, subgate names, layout check names, redaction classes, fixture-backed state, core parity claim, and separate overlap/secret negative fixture evidence structs.
4. Deleted empty fallback parsed-document helpers from acceptance tests; parser errors now fail immediately instead of fabricating empty documents.
5. Replaced local command diagnostic line slicing with a `CommandDiagnostic` boundary over the command result text; no `lines()` / `split_once("Gate '")` remains in the acceptance test.
6. Renamed positive generated UI release artifacts from fake `.png` names to explicit `.fixture.txt` fixture evidence, changed report provenance to `artifact_path`, and cleanup removes stale surrogate `.png` files.
7. Added `target/miri-tmp` workspace exclusion so Moon Miri's temporary sysroot package is not incorrectly treated as a member of this JJ/Git workspace.

## Focused parser/validation scan

- PASS: `parse_snapshot_document`, `parse_ai_release_document`, `parse_negative_fixture_document`, and `require_document_shape` no longer use `text.lines()`, `strip_prefix()`, indentation-prefix scanning, or `contains()` magic-word validation.
- Remaining `lines()`/`strip_prefix()` uses in `xtask/src/evidence.rs` are key/value parsing for fixture/artifact payload formats and tests, not release-critical evidence-document parsing.
- Remaining `contains()` uses are raw-secret/placeholder scans or non-release tests, not release document acceptance.

## Commands run

- `cargo xtask ai-release --bead vb-nf2u` — PASS.
- `if cargo xtask ai-release --bead vb-nf2u-missing; then exit 64; else true; fi` — PASS; unknown bead fails closed.
- `rm -rf "target/vb-nf2u-negative-fixtures" && if cargo xtask ai-release --bead vb-nf2u; then exit 64; else true; fi` — PASS; missing required fixture fails closed.
- `cargo nextest run -p velvet-ballistics-workspace --test vb_nf2u_ui_release_acceptance` — PASS; 8/8 tests.
- `cargo nextest run -p vb_ui_snapshot -p xtask` — PASS; 130/130 tests.
- `cargo kani -p vb_ui_snapshot --harness inventory` — PASS; non-zero harness execution.
- `cargo kani -p vb_ui_snapshot --harness layout_` — PASS; non-zero harness execution.
- `moon run :verify-all` — PASS; output `/home/lewis/.local/share/opencode/tool-output/tool_e11947cdb001XOz0xjj2cVITPl`.
- `moon ci --base HEAD --head HEAD` — PASS; `Tasks: 20 completed (1 cached)`, output `/home/lewis/.local/share/opencode/tool-output/tool_e11b7219e001QUcX4AlC43jlPy`.
- `rtk cargo fmt --all --check` — PASS.
- `rtk cargo clippy -p vb_ui_snapshot -p xtask --tests --all-features -- -D warnings` — PASS; 0 errors, 2 warnings.
- `moon run velvet-ballistics:miri` — PASS after adding `target/miri-tmp` workspace exclusion; output `/home/lewis/.local/share/opencode/tool-output/tool_e11b06b05001tXWyOVVjO6tRIR`.

## Power-of-Ten / zero-panic rules affected

- Rule 5 invariant density: release evidence acceptance now validates typed structure and domain invariants before/after serialization.
- Rule 7 checked results: parser/test helpers fail closed on parse errors; missing fixtures and unknown beads remain non-zero.
- Zero forbidden constructs: no production `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` introduced.

## Performance-layer decision

- No performance claim made; no benchmark/profiler evidence required.

## Second-ring evidence

- Kani inventory and layout harnesses ran and passed.
- Moon `:verify-all` and `moon ci --base HEAD --head HEAD` ran and passed.
- No assembly/IR/vectorization/API-compatibility claim made.

## Skipped gates

- No requested gate was skipped.

## Residual risks

- Evidence remains explicit fixture-backed text artifact evidence, not live Makepad rendering.
- Core runtime parity remains explicitly `unsupported`.
- Broad workspace has unrelated parent Git/JJ state outside this bead workspace; this repair did not close or land the bead.
