# vb-1nx31 branch disposition — all 43 audit refs + 1 new source ref

Base: `90ab770633c2130501ae2db889374c205a960d69`. 44 refs total.

| status | count |
|---|---|
| integrated | 1 |
| redundant | 2 |
| superseded | 7 |
| blocked | 2 |
| review | 28 |
| contract | 3 |
| base | 1 |

Method: per-file `git diff --quiet <tip> <base> -- <path>` (IDENTICAL/DIFFERS) for every
branch code_path; ancestry via `git merge-base --is-ancestor`; own-delta (vs parent)
for review/evidence classification. Classification is content-based, never name/date-based.

## Integrated (this change) (1)

### origin/codex/vb-cvuau-direct-api-candidate
- tipSHA: `bb124563884340d7fe050f101595a0625a4f1c12`
- date: 2026-10-21T17:54:25-05:00
- subject: vb-cvuau: direct-api isolated specialist delivery
- scope: source · code_paths: 1
- status: **integrated**
- reason: vb-cvuau direct-api specialist delivery. Genuinely MISSING: current base had test ignored + fabricated action_ticket helper (dummy seq/attempt/idempotency_key/capacity) violating vb-xb62s authority boundary. Integrated authentic version: lookup_pending_action_ticket + struct-update negative rejections + ActionCompletedEnvelope journal assertion; removed fake helper; un-ignored test. 14/14 pass, 0 ignored.
- current counterpart: this change (crates/workspace_tests/tests/vb_vt2f_direct_runtime_api_acceptance.rs)
- verification: cargo test -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance -> 14 passed 0 failed 0 ignored


## Redundant (byte-identical to base) (2)

### origin/codex/vb-xx331-ci-candidate
- tipSHA: `fd06cd11de9f37689db105e9e7b048410dfb1ecd`
- date: 2026-10-21T17:54:10-05:00
- subject: vb-xx331: integrate compiler guard candidate for canonical CI
- scope: source · code_paths: 6
- status: **redundant**
- reason: vb-xx331 compiler guard for canonical CI. All 6 code_paths (naming_scan/classify.rs, legacy.rs, types.rs + vb_37lc_canonical_spelling_red common_part_02/part_03/part_04) byte-IDENTICAL to base.
- current counterpart: base naming_scan + canonical spelling tests
- verification: vb_37lc_canonical_spelling_red suite

### origin/codex/vb-xx331-isolation-review
- tipSHA: `b994b99537d338bf8ea8778cb52dd2d36bee271e`
- date: 2026-10-21T18:00:13-05:00
- subject: vb-xx331: independently review JJ worktree isolation repair
- scope: source · code_paths: 6
- status: **redundant**
- reason: vb-xx331 isolation review (JJ worktree isolation repair). Same 6 files as fd06cd11, byte-IDENTICAL to base.
- current counterpart: base naming_scan + canonical spelling tests
- verification: vb_37lc_canonical_spelling_red suite


## Superseded (base is newer / rewrite-equivalent) (7)

### origin/codex/vb-3l40a-runtime
- tipSHA: `a5b1338af17e982275273b983fc3cd750e7dc696`
- date: 2026-10-21T12:16:42-05:00
- subject: vb-3l40a: restore typed runtime command and admission compilation
- scope: source · code_paths: 44
- status: **superseded**
- reason: vb-3l40a typed runtime command + admission compilation (1148 ins, 44 files). Canonical crate structure and typed run_compiled/admission compilation are present in base; base compiles clean (direct API workspace test builds vb_core/vb_runtime).
- current counterpart: 856603b8 delivery checkpoint (ancestor of base) + later reviewed patches
- verification: cargo test -p velvet-ballistics-workspace-tests --test vb_vt2f_direct_runtime_api_acceptance

### origin/codex/vb-3l40a-test-assertions
- tipSHA: `4ae3e8cb6ac0fc082112bb2732f6ec63cebd84bc`
- date: 2026-10-21T12:14:58-05:00
- subject: vb-3l40a: sharpen local recovery test assertions
- scope: source · code_paths: 42
- status: **superseded**
- reason: vb-3l40a sharpened recovery test assertions (23 ins / 38 del, 3 files). Base carries later-reviewed recovery assertion refinements; recovery/full.rs and admission.rs are newer in base.
- current counterpart: later reviewed recovery tests in base
- verification: vb_runtime recovery suites

### origin/codex/vb-8iilq-aligned-candidate
- tipSHA: `59aa6d2e7b32a2505bae7943541756403d34d1c0`
- date: 2026-10-21T15:08:16-05:00
- subject: vb-8iilq: align parity repair with reviewed runtime, CLI and lint baseline
- scope: source · code_paths: 28
- status: **superseded**
- reason: vb-8iilq parity alignment. Base is NEWER: recovery/full.rs +helpers +impl_parts/chunk_001 +lifecycle/chunk_001 net +220/-37 (188 extra lines) carrying reviewed recovery/Ask authority logic. commands_incident.rs, input_mapping_boundary.rs, stale_guards.rs, ipc_frame.rs, ipc_target.rs, fuzz/lib.rs byte-identical.
- current counterpart: base recovery/Ask authority lineage (afb002df ask-ticket + shutdown 6a5a4d4e)
- verification: ticket_authority_matrix.rs + stale_guards.rs present in base

### origin/codex/vb-eyij7-integration
- tipSHA: `f82f834e6e2fb986bdd88a6edd47f9c9ebff9cb9`
- date: 2026-10-21T12:17:39-05:00
- subject: vb-eyij7: integrate runtime prerequisite and repair migration test findings
- scope: source · code_paths: 276
- status: **superseded**
- reason: vb-eyij7 integration of runtime prerequisite + migration findings (1554 ins / 333 del, 56 files). Migration regression test vb_eyij7_migration_regression.rs (25876 bytes) present in base; reviewed delivery checkpoint 856603b8 ancestor of base.
- current counterpart: 856603b8 delivery checkpoint + vb_eyij7_migration_regression.rs in base
- verification: vb_eyij7_migration_regression.rs

### origin/codex/vb-eyij7-path-consumers
- tipSHA: `d5e703045eb01eb3d4f6a02a49ee8f6fb82e4e79`
- date: 2026-10-21T12:09:02-05:00
- subject: vb-eyij7: update renamed crate consumers and CI paths
- scope: source · code_paths: 238
- status: **superseded**
- reason: vb-eyij7 renamed-crate consumers + CI path updates (272 ins / 270 del). Canonical crate paths already in base; Verus production mirrors regenerated later.
- current counterpart: canonical naming base + mirror-regen lineage

### origin/codex/vb-eyij7-workspace
- tipSHA: `e0a8330442c2b232c5e5ed9bc229bf92ac94ac71`
- date: 2026-10-21T11:29:23-05:00
- subject: vb-eyij7: exercise canonical crate and workspace migration
- scope: source · code_paths: 234
- status: **superseded**
- reason: vb-eyij7 canonical crate/workspace migration exercise (1245 ins). Canonical crate structure (vb_core/vb_runtime/vb_compile/vb_expr/vb_yaml) present and compiling in base.
- current counterpart: canonical workspace structure in base
- verification: direct API test compiles canonical crates

### origin/codex/vb-netau-abi-recording-candidate
- tipSHA: `8a474d7051ad411c05ea1a8ae537055ac1312abe`
- date: 2026-10-21T18:24:14-05:00
- subject: fix(runtime): record actual action ABI digest at schedule boundary
- scope: source · code_paths: 2
- status: **superseded**
- reason: vb-netau actual action ABI digest at schedule boundary (63 ins / 2 del). schedule_action_abi_digest + action_abi_digest emission present in base chunk_005.rs; base further evolved with prepare_awaiting_schedule/run_attempt threading (newer).
- current counterpart: 652a6073 clean re-implementation + base chunk_005.rs
- verification: runtime_fjall_pending_action_recovery.rs in base (52624 bytes)


## Blocked (out of scope — canonical digest) (2)

### origin/codex/vb-7x0ol-independent-integration-review
- tipSHA: `18ef1b9c2b91d5d3c8f497d07d262567daea10d9`
- scope: source · code_paths: 5
- status: **blocked**
- reason: New non-ancestor source ref seen since audit. 5 source files all canonical DIGEST (part_05_digest.rs + digest/foreach_digest/wait_digest unit tests). DO-NOT-TOUCH: owned by c789b1b digest repair reviewed in separate workspace.
- current counterpart: c789b1b digest repair

### origin/codex/vb-a2l02-digest-boundaries-candidate
- tipSHA: `29b9f85a5d596371791c0f9e66dc7522e282fda4`
- date: 2026-10-21T18:52:27-05:00
- subject: vb-a2l02: add boundary-safe framing to canonical semantic digest
- scope: source · code_paths: 2
- status: **blocked**
- reason: vb-a2l02 boundary-safe framing for canonical semantic digest (part_05_digest.rs + digest_framing_collision.rs). DO-NOT-TOUCH: canonical digest repair owned by c789b1b, reviewed in separate workspace; integrator will inherit. Explicitly excluded from this audit lane.
- current counterpart: c789b1b digest repair (pending independent review)


## Review / evidence (no source delta) (28)

### origin/codex/vb-014sy-code-review
- tipSHA: `73199a581fc7adbfc8518e1256b642f9d9144950`
- date: 2026-10-21T18:03:26-05:00
- subject: vb-014sy: independent code review verdict
- scope: review · code_paths: 0
- status: **review**
- reason: Independent code-review verdict (vb-014sy); own delta is .beads/* evidence only, zero source files.
- current counterpart: reviewed commit consumed by integration base

### origin/codex/vb-13j40-artifact-rereview
- tipSHA: `fa9e13df830ed37eb86607409117dfabae4a49bf`
- date: 2026-10-21T18:18:48-05:00
- subject: vb-13j40: independent artifact repair review 57f307da and helper follow-up 63119912
- scope: review · code_paths: 0
- status: **review**
- reason: Independent artifact repair review (57f307da + 63119912); .beads/* evidence only.
- current counterpart: admission-serializer lineage (ancestor of base)

### origin/codex/vb-56k2u-abi-test-review
- tipSHA: `8702d25fcb52bd801ac2a86ff373a5b647681c1f`
- date: 2026-10-21T18:28:16-05:00
- subject: vb-56k2u: independent ABI schedule regression test review
- scope: review · code_paths: 0
- status: **review**
- reason: Independent ABI schedule regression test review (vb-56k2u); .beads/* only.

### origin/codex/vb-9agmk-artifact-test-review-vb-5yimw
- tipSHA: `fb0e74d93fce552f635597feb981b27ddb339d09`
- date: 2026-10-21T18:13:03-05:00
- subject: vb-5yimw: independent artifact test suite review — rejected
- scope: review · code_paths: 0
- status: **review**
- reason: vb-5yimw artifact test suite review — REJECTED; .beads/* only.

### origin/codex/vb-9agmk-test-rereview-vb-iuxt8
- tipSHA: `7424ec9349348557ebf12350634e573a7519c9c5`
- date: 2026-10-21T18:25:44-05:00
- subject: vb-iuxt8: independent artifact test re-review rejected at 729778e
- scope: review · code_paths: 0
- status: **review**
- reason: vb-iuxt8 artifact test re-review — REJECTED at 729778e; .beads/* only.

### origin/codex/vb-9agmk-test-review-luna
- tipSHA: `5c2278da400ba4a67a4517b17ecb2a6880362d4b`
- date: 2026-10-21T18:01:27-05:00
- subject: vb-9agmk.1: independent test suite review
- scope: review · code_paths: 0
- status: **review**
- reason: vb-9agmk.1 test suite review; .beads/* only.

### origin/codex/vb-9agmk.1-code-review
- tipSHA: `832840178a4de0a8d2dc391ad2bdc2ec7607da6b`
- date: 2026-10-21T18:05:57-05:00
- subject: vb-9agmk.1: independently review accepted artifact candidate 948f19cb
- scope: review · code_paths: 0
- status: **review**
- reason: vb-9agmk.1 code review of 948f19cb; .beads/* only.

### origin/codex/vb-9pycz-shutdown-code-review
- tipSHA: `490a65d3485f9be71ae24f98778afe0465356504`
- date: 2026-10-21T18:33:18-05:00
- subject: vb-9pycz: approve scoped production shutdown with independent durable-prefix retry probe
- scope: review · code_paths: 0
- status: **review**
- reason: vb-9pycz shutdown code review — APPROVED; .beads/* only. Production shutdown already in base (6a5a4d4e shutdown ancestor).
- current counterpart: 6a5a4d4e shutdown (ancestor of base)
- verification: shutdown suites in base

### origin/codex/vb-aaqp7-shutdown-test-review-evidence
- tipSHA: `b7053b8defc1b1ef0c310c27bfd96a26a123172a`
- date: 2026-10-21T18:33:36-05:00
- subject: vb-zd8hq independent shutdown test review evidence
- scope: review · code_paths: 0
- status: **review**
- reason: vb-zd8hq shutdown test review evidence; .beads/* only.

### origin/codex/vb-cste8-abi-code-review
- tipSHA: `b5a0e13211e6d21e977c5eb3e96355cadc3aad8c`
- date: 2026-10-21T18:29:23-05:00
- subject: vb-cste8: independent ABI scheduling code review
- scope: review · code_paths: 0
- status: **review**
- reason: vb-cste8 ABI scheduling code review; .beads/* only.

### origin/codex/vb-cyz6d-ipc-review-evidence
- tipSHA: `d634616f5af52a205db95a8602c2cd97ec03084e`
- date: 2026-10-21T18:26:26-05:00
- subject: review: record independent IPC candidate rejection and focused test evidence
- scope: review · code_paths: 0
- status: **review**
- reason: IPC candidate rejection + focused test evidence (vb-cyz6d); .beads/* only.

### origin/codex/vb-ezymz-test-review-root
- tipSHA: `99fc261583655d26d10b9efce185acb093d464b1`
- date: 2026-10-21T18:13:25-05:00
- subject: vb-z28qg: independently test atomic admission and identify pending runtime recovery oracle
- scope: review · code_paths: 0
- status: **review**
- reason: vb-z28qg atomic admission test + pending recovery oracle; .beads/* only.

### origin/codex/vb-f8vdd-audit
- tipSHA: `c3340b3cfd5eb29060922be822f489d069ffc72d`
- date: 2026-10-21T17:54:01-05:00
- subject: vb-f8vdd: map full v1 acceptance gaps for bounded parallel delivery
- scope: audit · code_paths: 0
- status: **review**
- reason: vb-f8vdd map of v1 acceptance gaps for bounded parallel delivery; .beads/* only.

### origin/codex/vb-gy80b-final-test-review
- tipSHA: `6b9c4ac667028da96945c7a65a289ef31c059f8e`
- date: 2026-10-21T18:24:24-05:00
- subject: vb-gy80b.1: independent final compiler format and test re-review
- scope: review · code_paths: 0
- status: **review**
- reason: vb-gy80b.1 final compiler format + test re-review; .beads/* only.

### origin/codex/vb-gy80b-test-review-root
- tipSHA: `db786985b74f6750ec505ad6bfc644c47ec18f8b`
- date: 2026-10-21T18:08:05-05:00
- subject: vb-gy80b.1: independently verify migration doctest including failing broken import
- scope: review · code_paths: 0
- status: **review**
- reason: vb-gy80b.1 migration doctest verification; .beads/* only.

### origin/codex/vb-kcfc6-review-v1
- tipSHA: `475e4d3d07ae9af8267370a7b880938a1d144293`
- date: 2026-10-21T14:37:09-05:00
- subject: vb-kcfc6: preserve independent review and rejected precedence test coverage finding
- scope: review · code_paths: 0
- status: **review**
- reason: vb-kcfc6 independent review + rejected precedence coverage finding; .beads/* only.

### origin/codex/vb-mryfc-budget-rereview-evidence-root
- tipSHA: `37e24f9355766458e6d5449f12fab7e1fa9ee9d5`
- date: 2026-10-21T18:10:13-05:00
- subject: Independent test review evidence for vb-mryfc.2
- scope: review · code_paths: 0
- status: **review**
- reason: vb-mryfc.2 budget re-review evidence; .beads/* + .evidence/* only.

### origin/codex/vb-mryfc-lifecycle-code-review-evidence
- tipSHA: `db2696d58294e33cae52fa75b33fbfc3b737a2c7`
- date: 2026-10-21T18:30:44-05:00
- subject: vb-mryfc.3 independent lifecycle code review evidence
- scope: review · code_paths: 0
- status: **review**
- reason: vb-mryfc.3 lifecycle code review evidence; .beads/* only.

### origin/codex/vb-mryfc-test-review-root
- tipSHA: `da6bb12b70ffea5e049b3b767d7990c0f6893ff1`
- date: 2026-10-21T18:01:16-05:00
- subject: vb-mryfc.2: independently execute lifecycle suites and reject budget-insensitive oracle
- scope: review · code_paths: 0
- status: **review**
- reason: vb-mryfc.2 lifecycle suite execution + budget-insensitive oracle rejection; .beads/* only.

### origin/codex/vb-n7umv-admission-code-review
- tipSHA: `6bc5bd4d441f583c7d146bbc84edf43bdb446b1a`
- date: 2026-10-21T18:12:43-05:00
- subject: vb-n7umv: independent admission code review of 9f0557e5
- scope: review · code_paths: 0
- status: **review**
- reason: vb-n7umv admission code review of 9f0557e5; .beads/* only.

### origin/codex/vb-rcizj-independent-review
- tipSHA: `a536e6f162441843d01a8df8a9d6b997af49bc78`
- date: 2026-10-21T13:27:26-05:00
- subject: vb-rcizj: independent code and test review of real ticket repair
- scope: review · code_paths: 0
- status: **review**
- reason: vb-rcizj real-ticket code+test review; .beads/* only.

### origin/codex/vb-rcizj-review-v2
- tipSHA: `6e74a77a93a46b945fbcea678c36950955fb873f`
- date: 2026-10-21T14:21:43-05:00
- subject: vb-rcizj: record independent test review rejection of stale-guard coverage
- scope: review · code_paths: 0
- status: **review**
- reason: vb-rcizj stale-guard coverage rejection; .beads/* only.

### origin/codex/vb-t3cp3-ipc-code-review
- tipSHA: `c50eb400eb44ccbbaa2900d587ec3a9125a3a470`
- date: 2026-10-21T18:28:50-05:00
- subject: vb-t3cp3: clarify evidence annotations and exact helper counts
- scope: review · code_paths: 0
- status: **review**
- reason: vb-t3cp3 IPC code review + evidence annotations; .beads/* only.

### origin/codex/vb-te8re-cancel-test-review
- tipSHA: `6ede201105b2eaf0c3753f18a22cd022a9948824`
- date: 2026-10-21T18:15:47-05:00
- subject: vb-te8re: independently execute cancellation suite and reject incomplete journal oracle
- scope: review · code_paths: 39
- status: **review**
- reason: vb-te8re cancellation suite execution + incomplete journal oracle rejection. Own delta is .beads/* evidence only (5 files); code_paths list files under reviewed commit 1e12ef61, not its own edits.
- current counterpart: 1e12ef61 cancel/kill lattice repair

### origin/codex/vb-te8re-final-cancel-review
- tipSHA: `ea55d54be324a7e6bdba9938c8571536168a21c3`
- date: 2026-10-21T18:30:54-05:00
- subject: vb-te8re: independently re-review full cancellation journal assertions
- scope: review · code_paths: 0
- status: **review**
- reason: vb-te8re final cancellation journal assertion re-review; .beads/* only.

### origin/codex/vb-ttpoa-serializer-review
- tipSHA: `831f6e9dccb7c2394ecbd715be643211ae496862`
- date: 2026-10-21T18:28:15-05:00
- subject: vb-ttpoa: independently review final admission serializer extraction
- scope: review · code_paths: 0
- status: **review**
- reason: vb-ttpoa admission serializer extraction review; .beads/* only.

### origin/codex/vb-um4oi-code-review-wip
- tipSHA: `1900853e778a8838e924b80765151702e9388ad0`
- date: 2026-10-21T18:36:02-05:00
- subject: vb-um4oi: REJECTED source identity collision; stopped WIP evidence
- scope: review · code_paths: 0
- status: **review**
- reason: vb-um4oi REJECTED source identity collision; stopped WIP evidence; .beads/* only.

### origin/codex/vb-ymb0f-code-review
- tipSHA: `b29dd7ce3ccea830d5edf5ba2f6ff671d6a4d7c3`
- date: 2026-10-21T18:16:07-05:00
- subject: vb-ymb0f: independent code review approved 1e12ef61 (cancel/kill lattice repair)
- scope: review · code_paths: 39
- status: **review**
- reason: vb-ymb0f cancel/kill lattice code review — APPROVED 1e12ef61. Own delta is .beads/* evidence only (1 file); code_paths list files under reviewed commit.
- current counterpart: 1e12ef61 cancel/kill lattice repair


## Contract draft (doc only) (3)

### origin/codex/vb-3ruqy-contract-draft
- tipSHA: `aeb8f4f0a72a7beb49d2c332e2fac93337a34771`
- date: 2026-10-21T18:13:49-05:00
- subject: vb-3ruqy: analyze consumer defaults and owned input contract
- scope: contract · code_paths: 0
- status: **contract**
- reason: vb-3ruqy contract draft (consumer defaults / owned input); .beads/* doc only.

### origin/codex/vb-3ruqy-contract-refinement
- tipSHA: `4a85402461fa9caee13b93b424c2c05cc37d13f4`
- date: 2026-10-21T18:31:51-05:00
- subject: vb-3ruqy: refine consumer allocation and versioned owned input contract
- scope: contract · code_paths: 0
- status: **contract**
- reason: vb-3ruqy contract refinement (consumer allocation + versioned owned input); .beads/* doc only.

### origin/codex/vb-7r7jv-contract-draft
- tipSHA: `7aa5b9dd482d229d44d911e9ecbd836a3e52c29e`
- date: 2026-10-21T18:14:19-05:00
- subject: vb-7r7jv: model bounded executor conformance and receipt recovery
- scope: contract · code_paths: 0
- status: **contract**
- reason: vb-7r7jv contract draft (bounded executor conformance / receipt recovery); .beads/* only.


## Base (self) (1)

### origin/codex/vb-wlnl4-local-integration-candidate
- tipSHA: `90ab770633c2130501ae2db889374c205a960d69`
- date: 2026-10-21T18:59:06-05:00
- subject: integration: fix review blockers (relaxed policy exact digest assertion, shutdown DuplicateEvent cause assertion, IPC WriteResult enum)
- scope: base · code_paths: 12
- status: **base**
- reason: Frozen integration base (vb-wlnl4-local-integration-candidate); this is @- parent. Self.


## Landed refs (ancestors of base) — 80 origin/codex refs

80 additional `origin/codex/*` refs are ancestors of the frozen base and are therefore
already integrated (no source delta outstanding). This includes the 13 source
checkpoints from `/tmp/vb-wlnl4-source-inventory.json`, all confirmed ancestors:

| source | sha | ancestor |
|---|---|---|
| reviewed-base | `e36e4c385b6b` | YES |
| artifact-fixtures | `729778e06599` | YES |
| admission-serializer | `6d6ce25c0836` | YES |
| semantic-source-schema3 | `376979e05670` | YES |
| IPC-helpers | `521559b3b58c` | YES |
| shutdown | `6a5a4d4e9861` | YES |
| yaml-event-grammar | `c849f7bdc173` | YES |
| codex/vb-mdi3w-retained-input-recovery-wip-stop | `45693ea51b53` | YES |
| codex/vb-clp3q-artifact-fixtures-wip-stop | `6fc6e0ed1a96` | YES |
| codex/vb-r5c1c-checkpoint-wip-stop | `de13504f655d` | YES |
| codex/vb-qzxto-ask-ticket-wip-stop | `afb002df4493` | YES |
| codex/vb-7r7jv-wip-stop | `da2da5e2eb72` | YES |
| codex/vb-idxsn-source-length-wip-20261021 | `eba9e5fe8ae2` | YES |

The single non-ancestor new source ref (`vb-7x0ol-independent-integration-review`,
`18ef1b9c2b91`) is canonical digest scope and is recorded under Blocked above.
