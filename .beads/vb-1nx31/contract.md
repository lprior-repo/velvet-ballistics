# vb-1nx31 — Exhaustive stopped-agent branch audit + missing direct-API integration

Frozen base: `90ab770633c2130501ae2db889374c205a960d69` (`@-`).

## Scope

Sole-owner lane: exhaustive patch disposition of every origin/codex branch ref and
integration of genuinely missing stopped-agent source. No canonical digest edits
(owned by c789b1b, reviewed elsewhere), no broad `moon ci`, no source fixes outside
the audit blast radius.

## Inputs

- `/tmp/vb-wlnl4-source-inventory.json` — 13 source checkpoints, all
  `ancestry_included` in candidate 62ff5544/90ab.
- `/tmp/vb-wlnl4-unmerged-branch-audit.json` — 43 unmerged refs (14 with source
  deltas).

## Acceptance

1. Every one of the 43 audit refs plus any new source ref seen since the audit has a
   disposition with tipSHA, scope, actual files/hunks compared, status
   (integrated/redundant/superseded/blocked), exact reason/evidence, current
   counterpart, and verification test.
2. The genuinely missing direct-API source (bb1245638843) is integrated on top of the
   authentic AskTicket/public contract: `lookup_pending_action_ticket`,
   struct-update negative rejections, `ActionCompletedEnvelope` journal assertion,
   fabricated `action_ticket` helper removed, previously-`#[ignore]`d test un-ignored.
3. `cargo test -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance`
   reports 14 passed / 0 failed / 0 ignored with a true exit.
4. No `unsafe`/`unwrap`/`expect`/`panic`/`dbg`/unchecked ops introduced; canonical
   AskTicket durableseq / run_attempt / checkpoint / retained-input-digest / artifact
   trust contracts preserved (only test-file edits in this lane).

## Out of scope (explicitly excluded, owned elsewhere)

- Canonical digest files (`crates/vb_compile/src/mod_compile_lowering/part_05_digest.rs`,
  `crates/vb_compile/tests/digest_framing_collision.rs`, and digest unit tests):
  branch `29b9f85a5d59` (vb-a2l02-digest-boundaries-candidate) and new ref
  `18ef1b9c2b91` (vb-7x0ol-independent-integration-review). Owned by c789b1b digest
  repair (pending independent review); integrator will inherit.

## Delivery

Work in new child commits from the frozen base; publish fast-forward-only bookmark
`codex/vb-1nx31-missing-branch-integration` via `jj git push --bookmark`. Evidence
tracked with `jj file track --include-ignored`. Keep bead open pending independent
review. No main push.
