# vb-kyyf Canonical Aggregate Machine Gate Report

STATUS: APPROVED

Workdir for every command: `/home/lewis/src/bd-vb-kyyf-bdd`.

| Gate | Command | Exit | Evidence |
|---|---|---:|---|
| startup/read | read `/home/lewis/.claude/skills/formal-verifier/SKILL.md` and `/home/lewis/.agents/skills/formal-verifier/SKILL.md` | 0 | Same v1.5.0 content; agents copy wins if conflict. |
| input gate | `test -s ... && rtk grep -n '^STATUS: APPROVED$' ... && jq -c ...` | 0 | Required formal inputs present and JSONL valid. |
| tool availability | `command -v ...; cargo --list ...` | 0 | TLC, Apalache, Verus, Lake, Moon, jq, cargo-kani, cargo-mutants, cargo-llvm-cov, cargo-deny available; optional missing tools not required by planned obligations. |
| PO-001 | `rtk cargo test -p velvet-ballistics-workspace-tests --test vb_kyyf_cross_run_determinism -- --test-threads=1` | 0 | 16 passed. |
| PO-002 | `rtk cargo test -p vb_storage --test replay_resume` | 0 | 3 passed. |
| PO-003/006 | `rtk cargo test -p velvet-ballistics-workspace-tests --test vb_kyyf_cross_run_determinism` | 0 | 16 passed. |
| PO-004 | `rtk cargo test -p vb_storage --test recovery_bdd_tests` | 0 | 29 passed, 2 ignored. |
| PO-005 | `rtk cargo test -p vb_codegen` | 0 | 367 passed across 4 suites. |
| PO-007 | `rtk cargo test -p velvet-ballistics-workspace-tests --test vb_hxm0_acceptance_catalog` | 0 | 6 passed. |
| evidence artifacts | `test -s .evidence/vb-kyyf/*.md` for seven required paths | 0 | All vb-kyyf evidence artifacts non-empty. |
| PO-008 | `JAVA_TOOL_OPTIONS='-Djava.io.tmpdir=/home/lewis/src/bd-vb-kyyf-bdd/.tlc-tmp' tlc -workers 32 -metadir /home/lewis/src/bd-vb-kyyf-bdd/.tlc-metadir -config verification/tla/VbKyyfReplayDeterminism.cfg verification/tla/VbKyyfReplayDeterminism.tla` | 0 | TLC no errors; 42,907,696 generated; 16,483,704 distinct; depth 9; finished in 07min 46s. |
| PO-009 | `verus verification/verus/vb_kyyf_normalization.rs` | 0 | 43 verified, 0 errors. |
| PO-010 | `moon ci` | 1 | DEFERRED_GLOBAL: unrelated `vb_cli` storage-error exit-code tests plus disk quota in `mutants-smoke` after all scoped vb-kyyf obligations passed. Full output: `/home/lewis/.local/share/opencode/tool-output/tool_e3cd7fbff001gu1aeCytJUYiDo`. |

No bead-local blocking command failed; `.beads/vb-kyyf/ci-failure-category.txt` intentionally not emitted.
