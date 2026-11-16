# Unqualified engine work preserved for review

This draft preserves the remaining frozen source and experiments after the passing delivery on main 881258ecc458a71cd5a4238706600f73c7d4d4e9. It adds review artifacts only. It does not activate the partial migration, replace accepted production/proof files, or claim engine completion. No workspaces or original references were removed.

## Actual code review

- [canonical-current](https://github.com/lprior-repo/velvet-ballistics/compare/5ef82f5c44ca72c1ebb7608ac53a7b4c52ad04f9...codex/velvet-held-canonical-r30) — UNQUALIFIED: fixture/guard/partial IR compatibility source frozen; final helper split not verified, formal/runtime projections and integrated acceptance remain incomplete.
- [cli-compatibility](https://github.com/lprior-repo/velvet-ballistics/compare/c05fd5253aa0a997e91b8cfb9e105d83078325a5...codex/velvet-held-cli-r30) — UNQUALIFIED independent review/integration; exact scoped native1868/1868, source Clippy/fmt passed.
- [formal-alternative](https://github.com/lprior-repo/velvet-ballistics/compare/cbaf0c038c21fce34ae8ffac58984662c48f0ac2...codex/velvet-held-formal-r30) — BLOCKED: scoped projection Verus21/0 and native parity168/168 do not close production binding; mandatory gate rejects shape.
- [taint-compatibility-contract](https://github.com/lprior-repo/velvet-ballistics/compare/af7f13aa5b5e0bc7390903183b1d81f97598ee4e...codex/velvet-held-taint-boundary-r30) — BLOCKED: contract-only; unselected source candidate and full667-input graph preserved in packet, actual Verus compiler ICE.
- [wal-repair-design](https://github.com/lprior-repo/velvet-ballistics/compare/af7f13aa5b5e0bc7390903183b1d81f97598ee4e...codex/velvet-held-wal-design-r30) — BLOCKED/DESIGN ONLY: no Rust implementation; original helper compilation failure silently reports test pass; no actual kill/reopen proof.
- [proof28-exact-held-postimages](https://github.com/lprior-repo/velvet-ballistics/compare/881258ecc458a71cd5a4238706600f73c7d4d4e9...codex/velvet-held-proof28-r30) — UNQUALIFIED source-only snapshot of exact73 held postimages; includes explicit PR503 drift, must not replace accepted main.

## Exact reconstruction

[manifest.json](manifest.json) binds base/tip identities, complete binary Git patches, source postimages, original filenames, SHA-256 values, byte counts and qualification boundaries. The patches in patches/ reproduce each frozen source delta. Review-only postimages in sources/ are verbatim text snapshots; binary or whitespace-sensitive records are losslessly compressed. Frozen proof variants, the full 667-input compatibility graph, raw failures and the five held acceptance payloads are in packets/. The archive is deliberate: incompatible alternatives must not overwrite accepted main.

Run node .evidence/unqualified-preservation-r30/verify-preservation.cjs from the repository root to verify every recorded byte and nested source/patch roundtrip. The publication manifest additionally binds this index and checker. No compiled libraries, build caches or credentials are included.

## Qualification boundaries

- Canonical final helper split and combined caller/runtime migration not qualified on current main.
- Canonical runtime proof projections stale and production-binding gate failure.
- Current owning-core proof fails; unselected taint alternative reaches Verus Deserialize ICE.
- WAL process-kill test repair unimplemented; original test falsely passes helper build failure.
- Full MASTER/Miri/powerset/mutation and disabled fuzz behavior incomplete; held scanner proposal and mutant-generated regressions not activated.

The CLI candidate passed its scoped 1868/1868 native cases, source Clippy and formatting, but independent review and integrated acceptance are unfinished. Earlier proof counts apply only to their recorded input graphs. The WAL work is contract and reproduced-failure evidence only; no repaired implementation or kill/reopen result exists.

Existing separately preserved drafts [#502](https://github.com/lprior-repo/velvet-ballistics/pull/502) and [#504](https://github.com/lprior-repo/velvet-ballistics/pull/504) are not duplicated. Related open work [#505](https://github.com/lprior-repo/velvet-ballistics/pull/505), [#506](https://github.com/lprior-repo/velvet-ballistics/pull/506) and [#507](https://github.com/lprior-repo/velvet-ballistics/pull/507) remains separately reviewable.
