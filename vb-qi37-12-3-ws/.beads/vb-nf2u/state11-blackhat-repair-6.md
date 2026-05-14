STATUS: PASS

# State 11 Black-Hat Repair 6 — vb-nf2u

## Reference files read

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`

## Code changes made

- Repaired `xtask/src/evidence.rs` so positive `ai-release` evidence is produced, read back, validated, and then reported from typed domain artifacts instead of self-certified strings.
- Added typed evidence parsing/validation APIs consumed by acceptance and xtask tests.
- Replaced surrogate `sha256:`/FNV-style digest evidence with `blake3:` digests computed from read artifact bytes.
- Split overlap and secret negative fixture states into distinct typed shapes and made required fixture absence fail closed.
- Split cited oversized reviewed functions in `xtask/src/evidence.rs` and `crates/vb_ui_snapshot/src/checks.rs` below the reviewed-surface threshold.
- Updated `tests/vb_nf2u_ui_release_acceptance.rs` and `xtask/tests/ui_release_gates.rs` to assert typed evidence documents instead of local YAML-ish line slicing.
- Added `blake3` to `xtask/Cargo.toml` and exposed `xtask` as a root dev-dependency for the workspace acceptance test.

## Black-hat blockers addressed

1. **Manufactured provenance** — repaired by deriving facts from emitted/read artifacts, storing read path/bytes/digest provenance, and validating the bundle after readback.
2. **Fake digest label** — repaired by using `blake3:` over read artifact bytes. No SHA-256 claim remains for these artifacts.
3. **Hard-coded layout evidence** — repaired by parsing screen geometry from emitted artifact text and using that geometry in layout predicates.
4. **Placeholder-only redaction scan** — repaired by scanning read artifact bytes plus fixture-visible text.
5. **Deterministic constants** — repaired by validating typed metadata read from emitted artifact content.
6. **Functional core / imperative shell** — repaired by separating artifact production, artifact readback, domain validation, and report serialization paths.
7. **Illegal evidence states** — repaired with typed artifact provenance and separate overlap/secret negative fixture types.
8. **String-theater tests** — repaired by consuming typed evidence parser APIs in tests.

## Power-of-Ten / zero-panic rules affected

- Rule 4, short functions: cited reviewed functions were split; residual oversized functions found by broad scan are pre-existing test functions in `xtask/src/evidence.rs`, not the cited production/reviewed blockers.
- Rule 5, invariant density: evidence invariants moved into typed constructors/parsers and domain validators.
- Rule 7, checked returns/parameters: missing fixture/artifact/readback failures now return errors instead of empty-vector or neutral defaults.
- Zero forbidden constructs: no production `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` were introduced by this repair.

## Commands run

- `rtk cargo fmt --all` — PASS.
- `rtk cargo check -p xtask -p vb_ui_snapshot --tests --all-features` — PASS.
- `cargo xtask ai-release --bead vb-nf2u` — PASS; emitted six passing UI release subgates.
- `if cargo xtask ai-release --bead vb-nf2u-missing; then exit 64; else true; fi` — PASS as negative check; unknown bead fails closed.
- `rm -rf "target/vb-nf2u-negative-fixtures" && if cargo xtask ai-release --bead vb-nf2u; then exit 64; else true; fi` — PASS as negative check; missing required fixture fails closed.
- `cargo nextest run -p velvet-ballastics-workspace --test vb_nf2u_ui_release_acceptance` — PASS.
- `cargo nextest run -p vb_ui_snapshot -p xtask` — PASS; 130 tests.
- `cargo kani -p vb_ui_snapshot --harness inventory` — PASS; output `/home/lewis/.local/share/opencode/tool-output/tool_e114f2b19001yyl1106y4HPu5N`.
- `cargo kani -p vb_ui_snapshot --harness layout_` — PASS; output `/home/lewis/.local/share/opencode/tool-output/tool_e114f3cd2001kwaJ49A4LR9pPx`.
- `rtk cargo clippy -p vb_ui_snapshot -p xtask --tests --all-features -- -D warnings` — PASS.
- `rtk cargo fmt --all --check` — PASS.
- reviewed-surface function-size scan — PASS for black-hat-cited functions; residual pre-existing oversized test functions recorded below.
- `moon run :verify-all` — PASS; output `/home/lewis/.local/share/opencode/tool-output/tool_e1153392c0010b1oHnjxRtkWVm`.
- `moon ci --base HEAD --head HEAD` — PASS; final rerun exit marker `MOON_CI_EXIT:0`; output `/home/lewis/.local/share/opencode/tool-output/tool_e1169c813001HeUzwlpHxf4HNZ`.

## Benchmark / profiler evidence

- No performance claim made. This was correctness/provenance/verification repair, not an optimization.

## Performance-layer decision

- No claim made. No benchmark/profiler evidence required for this repair.

## Second-ring evidence

- Kani proof lanes executed for `vb_ui_snapshot` inventory and layout harnesses.
- Moon `:verify-all` executed and passed, preserving the bead-scoped Lockbud waiver policy established earlier.
- No assembly/IR/vectorization/API-compatibility claim was made.

## Skipped gates

- No requested gate was skipped.

## Residual risks

- Evidence remains fixture-backed surrogate UI artifact evidence, not live Makepad rendering; `core_runtime_parity_claim` remains explicitly unsupported.
- Broad function-size scan still reports pre-existing oversized test functions in `xtask/src/evidence.rs`; these are outside the black-hat-cited production/reviewed blockers.
- Full repository has many unrelated JJ working-copy changes from the bead lifecycle; this repair did not close or land the bead.
