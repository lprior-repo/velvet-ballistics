# Black-Hat Review: vb-m5gp

STATUS: APPROVED

## Startup Doctrine Cited

- Read `/home/lewis/.claude/skills/black-hat-reviewer/SKILL.md`: requires contract/bead parity first, Farley rigor, Holzman Rust, DDD, and rejection before aesthetics when contract parity fails.
- Read `/home/lewis/.agents/skills/black-hat-reviewer/SKILL.md`: same content observed; this agents copy wins on conflict.

## Findings First

No blocking defects found in this retry.

## Prior Rejection Repairs Verified

1. **Dependency-edge blocker repaired.**
   - Previous `mod_compile_errors -> mod_compile_validation` edge is gone: `crates/vb_compile/src/mod_compile_errors/kind.rs:1-6` and `crates/vb_compile/src/mod_compile_errors/collection.rs:1-7` import no validation module.
   - Previous `mod_compile_validation -> mod_compile_core` edge is gone: `crates/vb_compile/src/mod_compile_validation/part_01.rs:1-10` imports `crate::limits::YamlLimits`, not the compile facade.
   - `YamlLimits` now lives in a shared private value-object module: `crates/vb_compile/src/limits.rs:1-38`; crate-root API is preserved by `crates/vb_compile/src/mod_compile_core.rs:3` and `crates/vb_compile/src/lib.rs:43-48`.

2. **Executable dependency gate now exists and passed.**
   - `crates/workspace_tests/tests/vb_m5gp_compile_split_contract.rs:645-680` rejects the exact forbidden edges from the prior rejection.
   - I reran `rtk cargo test -p velvet-ballistics-workspace-tests --test vb_m5gp_compile_split_contract mod_compile_dependency_edges_remain_acyclic_and_diagnostic_leaf`: PASS, 1 passed.

3. **Real split and hidden-include repairs hold.**
   - `crates/vb_compile/src/lib.rs:14-26` declares private split modules directly.
   - `crates/workspace_tests/tests/vb_m5gp_compile_split_contract.rs:605-643` rejects missing private split modules, doc-only shells, `include!` bodies, and resurrected `compile_core_impl.rs`.
   - Independent scan found no `include!` or `compile_core_impl` in `crates/vb_compile/src`.

4. **Recursive line-limit repair holds.**
   - `scripts/check-source-length.sh:96-127` recursively scans `crates/vb_compile/src/mod_compile_*/**/*.rs`.
   - `crates/workspace_tests/tests/vb_m5gp_compile_split_contract.rs:703-752` recursively enforces bead-local `lib.rs` and `mod_compile_*` sources below `SOURCE_LINE_LIMIT`.
   - Independent count found no bead-local oversized split source; max observed was `crates/vb_compile/src/mod_compile_errors/collection.rs` at 286 lines.
   - I reran `rtk cargo test -p velvet-ballistics-workspace-tests --test vb_m5gp_compile_split_contract vb_compile_production_sources_remain_under_agreed_line_limit`: PASS, 1 passed.
   - I reran `bash scripts/check-source-length.sh`: PASS with only DEFERRED_GLOBAL pre-existing unrelated files.

5. **Formal evidence updated after repair.**
   - `.beads/vb-m5gp/formal-verification-report.md:36-53` records the rerun State 11 obligation results with `STRUCT-002`, `LEN-001`, `KANI-001`, and canonical gates passing.
   - `.beads/vb-m5gp/verification-ledger.jsonl:6` now records the repaired dependency-edge evidence instead of the earlier false pass.

## Gate Decision

- Contract parity: PASS for the bead-local split contract.
- Public API parity: PASS by reported State 11 gates and local `rtk cargo check -p vb_compile` rerun.
- Real split/no hidden include: PASS.
- Recursive source-length governance: PASS for bead-local sources; unrelated oversized files remain correctly classified as `DEFERRED_GLOBAL`.
- Forbidden dependency edges: PASS for the edges that caused prior rejection.
- Formal evidence: PASS for required scope; optional direct Miri remains non-blocking `DEFERRED_GLOBAL` per `.beads/vb-m5gp/formal-verification-report.md:48` and `.beads/vb-m5gp/verification-ledger.jsonl:11`.

## Mandated Fixes

None for State 12. Do not expand scope; proceed to evidence packaging.

## Verdict

APPROVED. The prior dependency-edge rejection has been repaired with real code changes, an executable gate, recursive line-limit enforcement, and updated formal evidence.
