# vb-8eocj.5 Rust regression qualification contract

Authoritative scope: velvet-ballistics-MASTER.md, backend/IR interpreter. Rust contract -> implementation -> behavioral tests -> independent CODE review -> independent TEST review. Local5090/3090 author repairs; no Go-skill or proof-planning stage. F1 remains open beyond this increment.

## Evidence and ownership

Sole native integration root is /home/lewis/src/isoloated/vb-8eocj-fleet-v22-counter-tests-3090; both Git/JJ roots must agree. Main coordination checkout stays clean. Frozen full RED receipts e8d612f079adbdbc99b2d61290d34a657ab91982 preserve V131/V137/V142. Generated Verus report and original failing Collect/RunId property seeds remain preserved. V142 ran all14726:14607PASS119FAIL27skipped; generated RunId seed changed map, wrapper125/raw100. No blanket expectation migration.

## Invariants for accepted language changes

- Empty flat Repeat remains an exact StepFieldShape at step0/steps/at least one set or do step. Original negative YAML bytes/nonempty errors remain.
- Flat multi-Set Repeat is accepted. Replace obsolete rejection with independent exact layout/edges/distinct actual output slots, constant values, cross-public-API IR parity, digest sensitivity/idempotency. Preserve PO-009 behavior intent; this is behavioral evidence, not a formal-proof claim.
- ForEach/Collect/Reduce still reject multiple body steps in this increment. Their three exact negative fixtures must all execute after extracting Repeat's positive case. No early return hides later cases.
- Choose conditions must be boolean. Fix successful fixture data to boolean expression strings; preserve strict production guard, body/branch/otherwise/width/fallthrough assertions and property strategies.
- Canonical ForEach uses distinct input/iterator/item/body-result slots and consistent Start/Next ownership. Manual positive fixture must reserve a separate iterator, preserve body->control and body/done edges. Keep original disconnected-body rejection.
- Canonical Collect uses CollectNext with fresh collector distinct from source/body-result, shared collector at Next/Finish, checked pages*items total limit, page_size items/default1, exact offsets. Public lower_collect intentionally retains legacy raw CollectPage behavior; no global replacement.
- Canonical Reduce preserves input via Copy and separate state; Repeat uses RepeatCheck. Nested Do numeric external input1 stays input1; assertions may migrate only to independently specified roles/edges, never rebind external input to scratch state.
- Typed expression errors remain exact variants and payloads. Unknown input Finish reference remains rejected; supported input expression Save must compile and actually execute edge values with exact Secret taint. No generic is_err/is_ok substitution for behavioral assertions.
- Raw lower_repeat allocation is independent of raw node IDs. Slot limits remain checked; invalid raw node IDs are rejected on workflow admission. Review allocation-domain Kani harnesses separately and preserve arbitrary structural domains/GOD RULES. No proof success inferred from setup/build or stale id+1 harnesses.

## Findings requiring classification

Four nesting-depth budget failures must be reproduced/understood through actual production validation before any repair. They cannot be changed to success merely because current validation accepts them. RunId0 is reserved: preserve arbitrary zero/nonzero domain and assert exact rejection for all affected encoders, preserve success byte assertions for valid IDs. CLI tests must invoke the binary built for the exact source in the configured Cargo target directory; explicit supported binary env is acceptable configuration, with raw native execution required.

## Exit criteria

Every119 failure accounted for; genuine production defects fixed in production; no ignored tests, filters masquerading as full coverage, weaker generic error checks, guard waivers or property domain reduction. Preserve original seeds and exact native commands/exits/source maps. Focused groups, full workspace nextest and canonical moon ci pass at final source; independent CODE/TEST reviews bind to exact diff and raw results. Push reviewed candidate to main only after acceptance; record remaining F1 source-language gaps as open beads.
