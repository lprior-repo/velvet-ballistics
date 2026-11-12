# BD Closure Audit: vb-scxh

STATUS: PASS_WITH_SCOPE_NOTES

## Commands

- Workdir: `/home/lewis/src/vb-scxh`
- Primary command: `bd --db /home/lewis/src/.beads/dolt show vb-scxh --json`
- List command: `bd --db /home/lewis/src/.beads/dolt list --json`
- Per-ID command form: `bd --db /home/lewis/src/.beads/dolt show <id> --json`

## Raw Extraction

- `vb-scxh` raw BD JSON status: `in_progress`.
- `vb-scxh` raw BD JSON dependency count: `12`.
- `vb-scxh` raw BD JSON dependent count: `1` (`vb-engine-yaml`, dependency_type `blocks`).
- `EXACT_FALSE_CLOSURE_COUNT=12`.

## Exact False-Closure IDs From Raw BD Dependencies

| id | raw status | priority | raw dependency_type on vb-scxh | labels marker |
|---|---:|---:|---|---|
| `vb-4ki5` | `open` | 2 | `related` | `cli,master-gap,planner-shred,release-plan` |
| `vb-7nr3` | `open` | 3 | `related` | `blocked-by-core,master-gap,planner-generated,release-plan,ui-paused` |
| `vb-b1hq` | `open` | 3 | `related` | `blocked-by-core,master-gap,planner-generated,release-plan,ui-deferred,ui-paused` |
| `vb-gvmt` | `open` | 1 | `related` | `blocked-by-runtime,codegen-deferred,master-gap,maxperf,mvp-post-core-codegen,performance,release-blocker` |
| `vb-j0m0` | `open` | 1 | `related` | `fuzz,master-gap,planner-shred,quality,release-plan` |
| `vb-qi37.10` | `open` | 1 | `related` | `blocked-by-runtime,codegen,codegen-deferred,generated-rust,master-gap,maxperf,mvp-post-core-codegen,performance,release-blocker,release-plan` |
| `vb-qi37.12.2` | `open` | 0 | `related` | `master-gap,mvp-feature-now,release-plan,reliability,runtime,storage` |
| `vb-qi37.14.2` | `open` | 1 | `related` | `cli,master-gap,mvp-post-core-cli,operator,release-plan` |
| `vb-qi37.15.3` | `open` | 1 | `related` | `cli,master-gap,mvp-post-core-cli,observability,operator,release-plan` |
| `vb-qi37.17.1` | `open` | 1 | `related` | `cli,diagnostics,master-gap,operator,release-plan` |
| `vb-qi37.5.2` | `closed` | 0 | `related` | `idempotency,master-gap,mvp-feature-now,release-plan,verifier` |
| `vb-qi37.9.2` | `open` | 0 | `related` | `expr,master-gap,mvp-feature-now,release-plan,semantics` |

## Per-ID Query Evidence

The per-ID loop executed `bd --db /home/lewis/src/.beads/dolt show <id> --json` for all 12 IDs and exited without command failure. The command emitted only repeated permissions warnings for `/home/lewis/src/vb-scxh/.beads` being `0755`; no per-ID lookup failed.

## Classification

- `BD-SCXH-001`: `PASS` for exact count, ID extraction, and per-ID raw query completion.
- `BD-SCXH-002`: `PASS` for raw-source-only status/link basis.
- `ERR-SCXH-005`: `PASS` for exact ID/count/status presence in this State 11 artifact.

Note: `vb-qi37.5.2` is currently `closed` in raw BD output, while the rest are `open`. This is captured as raw status, not normalized or inferred.
