STATUS: APPROVED

# vb-m5gp compile split global repair

## Scope

- Workspace: `/home/lewis/src/bd-vb-kyyf-bdd`
- Forbidden source checkout was not used.
- Manifest: `.beads/vb-kyyf/dispatch-state14-vb-m5gp-global-repair-attempt1.json`

## Decision

The implementation did not regress for the minimal accepted workflow. The expected generated-Rust digest was stale after compile/codegen behavior legitimately changed the emitted source text. The accepted workflow IR/artifact baseline stayed byte-for-byte unchanged, and the generated output still satisfies the semantic shape checks for the accepted workflow:

- artifact bytes still equal `EXPECTED_MINIMAL_ARTIFACT_BYTES`;
- artifact digest still equals `EXPECTED_MINIMAL_ARTIFACT_DIGEST`;
- generated source still contains the required no-unsafe/no-ignored-must-use header;
- generated source still has one slot, one node, empty constants, step-0 dispatch, finish reading slot 0, and unknown action rejection;
- the focused test passed twice with the updated digest, proving deterministic emission for this accepted workflow in this workspace.

## Change made

- Updated only `EXPECTED_MINIMAL_GENERATED_DIGEST` in `crates/workspace_tests/tests/vb_m5gp_compile_split_contract.rs` from `[63, 64, 128, 60, 49, 67, 227, 251, 100, 242, 87, 255, 194, 142, 170, 33, 138, 122, 104, 168, 72, 30, 170, 234, 117, 111, 72, 178, 103, 206, 33, 147]` to `[152, 194, 152, 219, 82, 80, 114, 24, 247, 103, 7, 27, 252, 205, 180, 223, 186, 178, 227, 64, 210, 126, 204, 76, 201, 252, 105, 224, 220, 57, 118, 8]`.

## Commands and results

- `pwd -P` → PASS, output `/home/lewis/src/bd-vb-kyyf-bdd`.
- `TMPDIR=/tmp/opencode/vb-kyyf-moon-ci-tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p velvet-ballastics-workspace-tests --test vb_m5gp_compile_split_contract -- --nocapture` before repair → FAIL, 6 passed / 2 failed, both stale generated digest mismatch.
- `TMPDIR=/tmp/opencode/vb-kyyf-moon-ci-tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p velvet-ballastics-workspace-tests --test vb_m5gp_compile_split_contract -- --nocapture` after repair → PASS, 8 passed.
- Same focused test repeated after repair → PASS, 8 passed.
- `TMPDIR=/tmp/opencode/vb-kyyf-moon-ci-tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo fmt --check` → PASS.
- `TMPDIR=/tmp/opencode/vb-kyyf-moon-ci-tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 moon ci` → PASS, 21 completed / 4 cached; notable output: `velvet-ballastics:test` ran 11026 tests, all passed; `source-length` reported pre-existing `DEFERRED_GLOBAL` oversized files.

## Residual risk

- No production Rust changed.
- No performance claim made.
- The digest baseline is text-sensitive by design; future intentional codegen text changes will require the same artifact-stability and deterministic-focused-test evidence before updating this constant.

## Landing recommendation

vb-kyyf State 14 landing can rerun.
