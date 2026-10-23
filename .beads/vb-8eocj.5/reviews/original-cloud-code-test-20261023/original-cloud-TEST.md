# Velvet Ballistics independent test and receipt review

## Reviewer and scope

- Reviewer identity: `/root/verify_velvet_f1_acceptance`.
- Report date: 2026-10-23 UTC.
- Repository reviewed: `lprior-repo/velvet-ballistics`.
- Recipient: the implementation coordinator and Master for evidence preservation and landing decisions.
- Scope: my completed, bounded TEST/oracle and native-receipt reviews covering V131 through V193. This is my original assessment, not a restatement of an implementation owner's acceptance claim.
- Publication mode: a standalone Markdown file for the coordinator to transfer. I did not create a repository commit or branch for this report, change source/tests, or update the main branch.

**Conclusion:** The V193 receipt supports an unfiltered workspace result of **14,728 passed, 27 skipped**, at exact source `daa75236facecac24c30be18ecb733d604fbf5d8`, with the recorded source map unchanged. It does not establish a successful canonical CI invocation, a Kani proof verdict, or complete F1 feature acceptance. Historical failures and incomplete runs remain part of the evidence, rather than being retrospectively relabeled green.

I independently read the native command results and raw output, checked receipt hashes and recorded source maps, compared available map entries with exact Git blobs, and reviewed the specified test/source diffs. I did **not** run Cargo, nextest, rustfmt, Clippy, Kani, native builds, or runtime tests in the cloud. All native execution results below were produced elsewhere and verified as receipts. Static equivalence findings are explicitly separate from those native results.

The read-only review workspace was `/workspace/shared/velvet-f2-workspaces/velvet-f2-verification`. Its pre-existing authorized F2 test addition was left untouched. This report does not require checking out a different revision or rerunning tests.

## Evidence locator and manifest register

Every path in this report is relative to the repository at the associated full receipt pin. A manifest hash authenticates the bytes I inspected; it is not a cryptographic attestation of the native machine or of facts absent from the receipt.

| Receipt group | Exact receipt commit | Evidence directory | Manifest and SHA256 |
| --- | --- | --- | --- |
| V131 through V142 | `e8d612f079adbdbc99b2d61290d34a657ab91982` | `.beads/vb-8eocj.5/native-red-v131-v142/` | `sha256.json`: `558713bcb4a9c09728749ad141ca42dc0bf71a8c349a7919b2adc563327794bb` |
| V151 through V155 | `b41ef9be4c971618fc317651b181b4f20e1dd3d5` | `.beads/vb-8eocj.5/native-migrations-v151-v155/` | `sha256.json`: `a142a9e727ea0ed4a62fc5a74390be98cb488f5ddac826fda18731952312c500` |
| V158 through V164 | `f019bfba0dd466a26e287e18315822e556da6b6e` | `.beads/vb-8eocj.5/native-migrations-v158-v164/` | `manifest.sha256.json`: `d51518f3f15d7b3810eb2da6fee3b72aebd1b43cdf682146e7d5b19654716385` |
| V167 | `52436aa4e68f3e1bb5ca0449ea2544bb89330aae` | `.beads/vb-8eocj.5/native-v1-v167-red/` | Manifest SHA256: `8131576db2df9ccfe4f190ca7a3b43466c1eecf323ae0e7e60ad96c691c28c46` |
| V168 through V179 | `9aaa36d8ff29a9e8e3163cd8a1d145c0334cbcb8` | `.beads/vb-8eocj.5/native-migrations-v168-v179/` | `manifest.sha256.json`: `a2834913f317cff072494e5da0d9cf68cfb84e0af0164a72b217e7fe1af3cd9a` |
| V181 through V193 | `3de0bcfb9b95eaa99c172d8827a9a957e8d67660` | `.beads/vb-8eocj.5/native-qualification-v181-v193/` | `manifest.sha256.json`: `65225c5e3eb785ac37a2dc9cdbe6be0387b035124b56dfaa344ba92982c8d83f` |

The V131–V142 manifest has 28 entries; the V151–V155 manifest has 32 entries. The later verified manifest sets contain 46 entries for V158–V164, 14 for V167, 48 for V168–V179, and 65 for V181–V193. **Entries are not synonymous with raw command logs:** in particular, the 48-entry V168–V179 manifest includes the receipt, and the 65-entry V181–V193 manifest includes `receipt.md`. The latter directory has 64 other manifested files plus the receipt and manifest, 66 files in total. I found no hash mismatch in the verified sets.

The V168–V179 receipt is an evidence-only child of `ba5724b6c860d946dd4b9ee9bc3d77381b244fcc`. The V181–V193 receipt is an evidence-only child of `daa75236facecac24c30be18ecb733d604fbf5d8`. No source/configuration delta outside Beads was present in those receipt commits.

## V131 and V142 preserve the original failures

The baseline was `b3ed204cf71e22feee878e27929af8d30d1f1616`. The later RED inventory contract was supplied at `c7fd6475c773e6c66f5fab7d51db8df16714ed04`, path `.beads/vb-8eocj.5/contract.md`.

| Run | Raw outcome | Important limit |
| --- | --- | --- |
| V131 canonical `moon ci` | Exit 1; 1,728.302 seconds; not a timeout. Nextest: 571 passed, 1 failed, 14,154 not run, 27 skipped. | The owned process was deliberately stopped after the concrete Repeat diagnostic failure. This was not a complete suite. |
| V137 unchanged-code no-fail-fast inventory | Exit 124; 600.111 seconds; `timed_out=true`. 12,811 run: 12,776 passed, 35 failed; 1,915 not run; 27 skipped. | Two failures were SIGTERM cancellations, leaving 33 actual assertion failures. It was not a complete inventory. |
| V142 complete no-fail-fast inventory | Exit 100; 293.829 seconds; not a timeout. 14,726 run: 14,607 passed, 119 failed; 27 skipped. | This was the completed RED inventory, not acceptance. |

The 119 V142 failures partition exactly into 31 compiler failures, 2 Finish/input failures, 1 typed RunId failure, 1 workspace Choose failure, 80 CLI process `ENOENT` failures, and 4 depth/budget failures. `V142_FAILED_NAMES.json` preserves the names. The counts sum to 119; I did not classify all of them as allocator or runtime bugs.

V131 also retained a formatting failure involving 11 files. The canonical report recorded failed format and test tasks, not an overall PASS. Successful sanitizer or other task statuses cannot override those failures. The retained task summary was 21 completed, 3 cached, 2 failed, and 4 skipped. I specifically corrected the earlier characterization of formatting as passed: the retained format last-run result had `exitCode: 1`.

### Recorded drift and unavailable post-run object

- V131 before and after source maps contained 1,857 source files and had equal SHA256 `1d1fbe4ffc05fb1c6cc0e83e9b5d5083309c966c93f320a0026ea507000d67cd`.
- Its before commit was `b3ed204cf71e22feee878e27929af8d30d1f1616`; its recorded after commit was `aee06b2414d2e664e8da74189a0892fdcf2d6443`. The latter object was unavailable through the original remote route. I did not claim an independently executed Git comparison against it.
- V131 regenerated tracked `.evidence/verus/trust-scan.txt`. Equal source maps therefore did not mean an entirely unchanged repository tree.
- V137 generated the Collect proptest regression seed, taking the source map from 1,857 to 1,858 files. The published reproduction state `21479a76c9470f99d628db2a5d07f54752cd729f` had source-map SHA256 `ef1112326384373ba2d2afa71b3bb4435ed71912efd554eac73206b5d767d023`.
- V142 changed the existing RunId property regression seed. The published reproduction pin `8c2944b1627c0d7cfc385f612ff412bcde2fb8b3` had 1,858 files and map SHA256 `6aaa95ea18df1a63e7773c9a49f68bb254ae5e2fa675b81d9246d7ad4d725cd1`, matched to actual Git blobs.
- The source-drift wrapper result 125 must not replace the underlying native exit 100 or be called a timeout. Generated evidence and seeds were preserved. The original production and authored test bytes did not change during these inventory runs.

### Original diagnostic classification

`digest_repeat_unit::test_repeat_empty_body_rejected_with_step_field_shape` still supplied an empty Repeat body. Its intended rejection remained valid; the implementation now accepted multiple flat Set/Do body nodes, so the old diagnostic text was obsolete.

The permissible migration was narrowly from `(step 0, field steps, exactly one set step)` to `(step 0, field steps, at least one set or do step)`. The fixture had to remain empty, rejection had to remain typed and exact, and both `step` and `field` had to remain fixed. Accepting the workflow, changing the input, using an error substring, or accepting any error would not have preserved that oracle.

## V151 through V155 migration review

The completed TEST review covered the exact transition from `f8099f52f09b04930bf638718c13eef583966b44` to `10288e00b4fb096288c89bf6c26c975aebaa244c` and the associated receipts.

| Run | Exact tested source | Observed native result |
| --- | --- | --- |
| V151 | `f8099f52f09b04930bf638718c13eef583966b44` | ForEach topology: **11** passed, zero failed/ignored/filtered; exit 0; 1.394 seconds. Earlier coordination prose saying 10 was incorrect. |
| V153 | `10288e00b4fb096288c89bf6c26c975aebaa244c` | Repeat digest 13 + integration Choose 2 + emission 2 + fallthrough 1 + otherwise 2 + width 2 = **22** passed; exit 0; 2.153 seconds. |
| V155 | `10288e00b4fb096288c89bf6c26c975aebaa244c` | CLI 80 + compile/parse/validate 48 = **128** passed; exit 0; 0.454 seconds. |

The ForEach map was unchanged at SHA256 `3017c0444ff24ec5326fbb3f43f86500296a3fe67718b725355d4c0b4ff72252`. V153 and V155 maps were unchanged at `e000f48805858b379b3b210895564f1e117f46dffe1afad913b9bdc64d2c1cc4`.

My bounded oracle findings:

- The ForEach changes corrected fixture topology/roles while preserving the body-to-latch case and the disconnected-body negative. They were not permission to accept arbitrary invalid ownership.
- The Repeat positive retained the original three-Set YAML and asserted concrete values, complete layout, digest behavior, and compilation parity. The V150 generated draft's invented `RepeatFinish.output` member was corrected to the actual `CompiledNode.output` before qualification. V150 is not evidence that the invented API compiled.
- Choose fixtures used actual boolean expression strings, while retaining branch/layout/property assertions and ranges. This did not authorize coercing arbitrary non-Bool values into conditions.
- V155 selected the supported CLI binary via `CARGO_BIN_EXE_velvet-ballistics`; no CLI behavior or test-body edits were needed. This demonstrated that those 80 `ENOENT` failures were a harness path problem in that invocation.
- V155 alone did not establish the later, stronger binary provenance recorded for V164/V193. I retained the earlier executable-provenance gap instead of inferring that a file at the selected path necessarily came from the tested source.
- These migrations were predominantly compile/shape tests. They were not a replacement for the already-existing public source-to-runtime Repeat tests, which remained unchanged.

## Zero ID and stable event kind tests

I reviewed `b41ef9be4c971618fc317651b181b4f20e1dd3d5` to `6d6c0cfa91ceacb598e2ab7d595690fdae005a89`, limited to `restate_typed_partitioned_id_tests.rs`.

The RunId generator remained full-domain `any::<u64>()`, including zero. All four encoders explicitly required the exact typed `InvalidRunId { run: 0 }` error at zero. The nonzero byte assertions remained unchanged. Sequence generation retained its original `0..u64::MAX` range, which excludes `u64::MAX`; I did not describe that separate generator as full-domain. The checked sequence-addition and unknown-kind checks remained after the zero/nonzero branch, not behind an early return.

The independently literal known-kind table remained `1..=4 | 10..=41 | 50`. The nine explicit stable mappings retained IDs 4, 33–39, and 41 for RunInputs, StepSucceeded, ActionScheduledTicket, ActionCompletedEnvelope, RunAdmissionBound, RetryCheckpoint, PolicyDecision, RunAdmissionPinned, and RunAdmissionBoundPinned. The assertions were not derived from the decoder being tested.

I approved this bounded test correction. I did not claim decoder roundtrip coverage, acceptance of all known kinds by a decoder, or an executed zero case solely because a randomized generator can produce it. The exact zero-error branch and the retained generator have distinct roles. Existing regression seeds were preserved.

## V158 through V164 receipts and bounded coverage

All five retained native gate maps were unchanged before/after and matched available Git blobs. The receipt commit was an evidence-only child of `a2f08fb4d8d68e9538b3ac29ac8ef6831ebbf4dd`.

| Run | Exact source | Native outcome | Source map SHA256 |
| --- | --- | --- | --- |
| V158 | `6d6c0cfa91ceacb598e2ab7d595690fdae005a89` | Zero-ID target: 2 passed; exit 0; 0.401 seconds. | `260d1657ef15f985e1e9d0f5117882e22cf8e022bd33e7f5ff030b77ce40ab9d` |
| V159 | `b18734c02c64560bf267ea449b44984970444ae4` | Collect target: 17 passed; exit 0; 0.732 seconds. | `23c33ad1a3c12ba5b5d8c74e3311e4b14a9e797c27721a39b2de919af2e4abb6` |
| V162 | `a2f08fb4d8d68e9538b3ac29ac8ef6831ebbf4dd` | Input target: 12 passed; exit 0; 0.522 seconds. | `c90025df43d159dc2a7b3b61d01fd1e797a8c1a4df76ad366405afc812002ce0` |
| V164 build | Same `a2f08fb4d8d68e9538b3ac29ac8ef6831ebbf4dd` | Explicit CLI all-features build: exit 0; 0.086 seconds. | Same `c90025df…` map. |
| V164 CLI | Same `a2f08fb4d8d68e9538b3ac29ac8ef6831ebbf4dd` | 80 passed; exit 0; 0.159 seconds. | Same `c90025df…` map. |

These source maps had 1,858 files. V164's recorded binary before/after SHA256 was `c3308b8350a9ed42584e49e0a09e5f30dc50d6ff9b6ac2b6f0a6927691020aa7`, size 96,635,288 bytes. The binary path matched the command's override. This is independently checked receipt content, not my own rehash of the native executable.

### Collect contract classification

The canonical lowering now advances with `CollectNext`, not the legacy nonadvancing `CollectPage` opcode. A narrow canonical opcode/layout expectation migration was justified. Raw helper tests that intentionally exercised `lower_collect` and its `CollectPage` shape were not blanket-rewritten.

The reviewed canonical tests preserved source/private collector separation, distinct state roles, ownership, concrete body/done/finish links, count and bound assertions. The deterministic 3-by-5 case asserted 15, and exact overflow coverage was added. Existing property strategies and their conditional-on-success structure were retained; that structure can leave unsupported generated cases without positive assertions. I did not call it full-domain source admission or runtime/taint proof. The reviewed helper did not add an exact Set-42 payload assertion; the original property did not have one either.

### Input and Finish contract classification

The former unsupported-Save fixture became a positive test using the original YAML through `parse_workflow_source` then `compile_source`, followed by the core `run_until_blocked` path on unchanged returned IR. The test manually populated the declared input port with Secret taint. It asserted declared port 1, `EvalExpr` loading slot 1 into output 0, `Finish` slot 0, and exact finished value/Secret taint for `i64::MIN`, -1, 0, 42, and `i64::MAX`. The missing-input case retained exact `UnknownReferenceName` fields.

This is not a `compile_workflow` plus full RuntimeEngine/journal admission test. Its positive value coverage is meaningful but narrower, and I kept that limit explicit.

## V167 through V174 primitive migration and preserved RED

V167 tested `4980066ebb11e3a07d5a3394e3ccb111d8528e2d`. Its 1,858-file before/after map matched Git and was unchanged at SHA256 `09aea36576a140d1b1f429ccb000d00de02ffe6dbfa8d860cc17982a21491ecf`.

The actual result was 47 run, 42 passed, 5 failed; exit 101; 0.918 seconds. Three public-API shape tests stopped at the first ForEach output mismatch, 3 versus the old expected 1. Another expected old SetConst condition lowering rather than EvalExpr. The raw maximum-ID helper test expected the wrong failure. Assertions after those first failures were **not reached** in V167.

I reviewed the subsequent fixture corrections at `aa3c654f4bfdd6cee5c3481c5f3ebac3a3505948`, including the preceding semantic changes at `4980066ebb11e3a07d5a3394e3ccb111d8528e2d`. The corrected tests inspected actual constants, private slots, edges, Collect count 15, Reduce's private Copy, RepeatCheck, and `RepeatStart.next == None`. No invented `Start.next` field or behavior was accepted as tested evidence.

The original `not-an-integer` YAML now produced an exact `ExpressionUnknownIdentifier` for `an` at index 4; a separate max-zero fixture retained an exact shape error. The empty-Repeat diagnostic was distinguished from the three other primitive restrictions. The Together header-output and full two-Set value gaps initially remained coverage gaps, and were later strengthened at `daa75236facecac24c30be18ecb733d604fbf5d8` rather than silently assumed covered.

| Run | Exact source | Native result | Unchanged map SHA256 |
| --- | --- | --- | --- |
| V170 | `aa3c654f4bfdd6cee5c3481c5f3ebac3a3505948` | 47 run, 46 passed, 1 failed; exit 101; 0.635 seconds. Old MAX expectation still RED. | `2c7c590110f123152ab296edc9e25bf387cbf9839981c1ba06d7cef787271297` |
| V172 | `66896b386a2361c349386f3c1ab71a0478a3d105` | 47 run, 46 passed, 1 failed; exit 101; 0.765 seconds. Draft expected NodeIdMismatch, but actual guard failed earlier. | `b676f8783996577bc961f2a4d699d2d3e9f94be4341e8b6846dd467193a52f30` |
| V174 | `8d1b23b562dae48123186b3639d397089d0bb895` | 47 passed; exit 0; 0.656 seconds. | `a492f6075401b5c263093f1d4bc8dabe25ec76a600fbf9950c1e299a55537935` |

The maximum-ID raw layout remained explicitly rejected when passed through `lower_steps_to_ir`, with exact `CompileErrors([Validation(LoopBodyStepOutOfRange { step: 65535, node_count: 3, source_node: 0, label: "repeat body" })])`. A separate NodeIdMismatch test remained. Returning a raw vector from a helper does not mean its nodes constitute an admitted workflow.

The allocator helper boundary tests retained prefixes 0 and 65,533 as successful two-slot allocations, ending at 2 and 65,535. Prefixes 65,534 and 65,535 require exact `SlotIndexOutOfRange(65535)` with final count 65,535. The 65,534 case intentionally records **partial allocation**. These are not transaction-atomicity assertions.

## Depth and nested Do migrations

### Depth fixtures

The two-file import at `014d72abc10e49f3e3e88f87ce038c8acfb9d208` was byte-identical to the assigned source `78dda98968b6cc87f19eb547608cffa628f24749`:

- `crates/workspace_tests/tests/vb_qi37_2_4_integration_budget_errors.rs`, SHA256 `6aad35683123efaaf1680b65520a8e7a0f8379e9c6936f3e0219f2895409649d`.
- Its `depth_fixtures_tests.rs` helper, SHA256 `bc266a7f607338d0b9d201439b15a9e4e73aa7bd72af37277377a296ff00146d`.

All four original rejection names and limit pairs were retained: 10/5, 10/5, 5/4, and 3/2. The new graphs distinguish actual contained depth D from siblings at depth 1; exact D succeeds, D−1 fails with the exact expected budget error, and the original rejection limit remains tested. These are raw budget graphs, not constructor or runtime admission tests.

V176 used the wrong package name and exited 101 in 0.082 seconds, with **no tests run**. V177 then ran the actual depth target: 47 passed, exit 0, 0.317 seconds at `014d72abc10e49f3e3e88f87ce038c8acfb9d208`. Its 1,859-file unchanged map SHA256 was `3ff4d2aebf28dcee7d2340ebac2223d8d5c69bbb935a91a0bda0aa3257b8d6f5`.

### Nested Do compile-only fixtures

The reviewed `vb_xi2f_nested_do_lowering.rs` migration retained all six original YAML literals, action IDs, and numeric external operands. The exact 99,999 `SlotIndexOutOfRange` negative was unchanged. Five positives called `compile_workflow` and compared the returned nodes with complete expected fields, including counts, entry, private source/state/output roles, Reduce initial constant 10, and Together first-branch output None versus second-branch result output.

V178 at `3ea4c30138f1fd963984e615446ba777347138d2` failed compilation, exit 101 in 0.151 seconds: unused imports and Vec versus boxed-slice API mismatch. Its unchanged map SHA256 was `6d44834f0de2f32bbbfd8dff7e8a54a50973144d7ca954bfea7419bfa5262ac9`. No tests ran.

The V178-to-V179 correction was mechanical: remove unused imports and use `.into_boxed_slice()`. It did not change the source fixtures or expected node fields. V179 at `ba5724b6c860d946dd4b9ee9bc3d77381b244fcc` passed all six tests, exit 0 in 0.216 seconds. The 1,859-file map was unchanged at SHA256 `aa5502124a7ce4b64124ec950009f34d45f79c8d07bd959c51c38222c92c4265`.

These tests prove compiler layout for the tested source forms. They do **not** execute the actions, establish repeated real-ticket behavior, or verify runtime taint propagation.

## Mechanical Repeat split and final oracle strengthening

### Exact two-file split

I reviewed `08001dd5b8743473d29c51eab92ba6a5a485a081`, whose exact parent was `aa3c654f4bfdd6cee5c3481c5f3ebac3a3505948`.

Only two files changed. `digest_repeat_unit.rs` split `test_repeat_multi_step_body_compiles_and_changes_digest` into a 27-line main test and seven helpers of 21, 17, 17, 17, 20, 21, and 29 lines. `vb_8mdp_7_collect_lowering_props.rs` changed only a comment describing the private collector relationship.

I mechanically expanded the seven unconditional helper calls and compared the 25 assertion macro byte strings. They were identical in bytes and order. YAML, test name and annotation, compile paths, equality checks, constants 10/20/30, edges, outputs, counter and Finish checks, digest inequality after changing 30 to 31, and recompilation equality were all retained. Helpers performed immutable lookups and propagated errors; they did not mutate IR or add skip/accept-any-error behavior.

The resulting `digest_repeat_unit.rs` SHA256 was `0f1e2c48f23198d307f6e49b541cf656869916ed29db4d5f828dfec5768980cf`. Those exact split/comment files remained byte-identical in the later `daa75236facecac24c30be18ecb733d604fbf5d8` candidate. V191 subsequently qualified the 13-test digest target, but the split approval itself was static equivalence, not a native execution claim.

### Stronger final primitive assertions

At `daa75236facecac24c30be18ecb733d604fbf5d8`, Together explicitly checks first-branch output None and second-branch output `Some(slot 2)`.

The two-Set Repeat fixture now requires six nodes, slot count 4, entry 0, two constants with values 1 and 2, and these independent literal expectations:

- Start 0: counter 1, max 3, body 1, done 4.
- Set 1: constant index 0, output 2, next 2.
- Set 2: constant index 1, output 3, next 3.
- Check 3: counter 1, output 1, next 1, done 4.
- RepeatFinish 4: result 3, output 3, next 5.
- Terminal Finish 5: result 0, no next, no output.
- Every node has absent `on_error` and `error_slot`.

Thus counter/output isolation is not inferred only from a node count. The three other multi-body primitive negatives still require exact step-0, `steps`, `exactly one set step` errors, with an explicit negative count of three. All four original YAML strings remain unchanged. I found no oracle weakening in the combined `aa3c654f…` to `daa75236…` review.

V190 at `405a7e83d27a928e8fb8e154df48bc452e7d367b` is preserved as a compile failure: exit 101, 0.626 seconds, no tests. It had three StepIdx Display E0277 errors and an index-type E0308 error. The subsequent correction used `.get()` and `usize::from(id)` without removing assertions. Its unchanged 1,860-file map SHA256 was `e44f3b6b68894665b5b4e7d5260f9352d802d2566bc188d27a17af6a35eb5c25`.

## Formatting review and its exact qualification pin

I independently compared formatting commit `446511bc53310f4daad640cb66cdeaf61b995eeb`. The exact scope was 11 files:

1. Compiler `canonical_repeat.rs`.
2. Compiler `canonical_repeat_tests.rs`.
3. Compiler `canonical_types.rs`.
4. Compiler lowering `part_02.rs`.
5. Compiler lowering `part_04.rs`.
6. Compiler `source_slot_reservation_tests.rs`.
7. Workspace scalar-Do `negative.rs`.
8. Workspace scalar-Do `positive.rs`.
9. Workspace scalar-Do `type_identity.rs`.
10. Workspace `vb_f1_scalar_do_support/mod.rs`.
11. Workspace `vb_f2_source_slot_preservation.rs`.

Token/literal comparison, function order, normalized assertion expressions and order, and test counts were preserved. I manually reviewed the remaining import-order and equivalent expression/match/closure-brace differences. The frozen F2 target retained 19 tests and 37 assertion macros; Repeat compiler units retained 8 tests/24 assertions; reservation tests retained 9/26; scalar negative, positive, and type-identity groups retained 11, 8, and 7 tests. No expected value, limit, gate, or skip was changed.

V181 applied `rustup run nightly-2026-04-28 cargo fmt --all`, raw exit 0 in 1.500 seconds. The receipt prose describes an expected source-drift wrapper 125, but the retained machine gate records raw exit 0; I do not manufacture an additional machine receipt for wrapper 125.

The before-map named `3107f1e157916de3733a3e9dad3bcf60a8183a37`. That object was unavailable via the original route. Its recorded 1,859-file contents were nevertheless fully matched to actual published `ba5724b6c860d946dd4b9ee9bc3d77381b244fcc` blobs, map SHA256 `aa5502124a7ce4b64124ec950009f34d45f79c8d07bd959c51c38222c92c4265`. This is a content comparison against the available commit, **not** a claim that I retrieved or compared the missing commit.

The after-map matched exact `446511bc53310f4daad640cb66cdeaf61b995eeb`, with 1,859 files and SHA256 `3353421896cda13ce8fa663cfdd9947acfee43bfccae6c6a30896fde5570721b`. Exactly the reviewed 11 source files changed.

V182 then ran `rustup run nightly-2026-04-28 cargo fmt --all --check`: exit 0, 1.563 seconds, no timeout, empty stdout/stderr, identical before/after maps at that exact formatting pin. This is not automatically a format-check receipt for later `daa75236…` or a later Kani extraction.

## Kani review remains bounded and inconclusive

I statically reviewed `c0d3e4ccf68f7299fb1aaddfa7bfaa865ea9284b`, bookmark `codex/vb-8eocj-repeat-kani-v187-source`.

The three-file source change removed two stale id-plus-one Repeat proof helpers from `kani_lower_control`, added `kani_lower_repeat`, and registered its `cfg(kani)` module. The old control module remained disabled under `cfg(any())` as before; unrelated Ask/Choose helpers were retained. The new proof used five independent full-u16 symbols for prefix, max, id, body, and done, without a user `kani::assume` or stub.

Its intended assertions cover success with two fresh slots for prefix at most 65,533 and exact overflow/final-count behavior at 65,534 and 65,535, including the existing partial-allocation behavior. It checks concrete fields of all three raw nodes, with 12 covers and unwind 12. This is not a proof of validated workflow admission, nested runtime activation, allocator transaction atomicity, or complete F1.

The proof listing (`kani-list-v184`) identifies exactly one standard harness, `kani_lower_repeat::lower_repeat_allocates_fresh_slots_for_symbolic_prefix`, with zero contract harnesses and zero function contracts. Listing a harness or compiling it is not a proof verdict.

V186 failed setup before proving anything: invalid systemd `WorkingDirectory` assignment, exit 1, 0.003 seconds.

V187 invoked Kani 0.67.0 / CBMC 6.8.0 under a user systemd scope with MemoryHigh 20G, MemoryMax 24G, MemorySwapMax 0, one job, and the exact harness. It timed out after 360.101 seconds with exit 124 and `timed_out=true`. Its before/after 1,860-file maps were unchanged and matched actual source at SHA256 `9c8edbcb6bd56934ced64890a685440404e6e82f303d36c4c477ce0cc8f2975f`.

The retained 11,100-byte stdout begins bounded model checking and includes unsupported caller-location/foreign-function warnings and generated `aborting path on assume(false)` lines from standard-library control paths. There is **no final verification success or failure verdict**, no successful cover-result evidence, and no clearance that unsupported constructs are unreachable. Those generated lines are not evidence that a user assumption was added to the source.

The separate CODE reviewer later reported a reviewability-preserving extraction at the actual 40-character pin `dd823160331dfac4280289541de419563cf0521f`; the earlier spelling with a trailing `a` was incorrect. I did not perform that extraction review or run V196. Its reported 37 normalized assertions, 12 covers, five independent symbols, and smaller functions are CODE-review findings from that reviewer, not my own completed native verdict. Any later proof outcome requires a separate receipt assessment.

## V191 through V193 final verified native results

All three runs below used exact source `daa75236facecac24c30be18ecb733d604fbf5d8`. Their 1,860-file source maps were unchanged at SHA256 `f029a24e79f9b3b4ac863e84ff8d499bdc9e3dc6f96add195318566a5a78c784`. I matched every mapped path hash against actual Git blobs at that pin.

### V191 focused tests

The strict-warning command selected `v1_primitive_lowering`, `digest_repeat_unit`, and `vb_8mdp_7_collect_lowering_props`, with one test thread. Actual totals were 47 + 13 + 17 = **77 passed**, zero failed/ignored/filtered. Exit 0; 1.051 seconds; no timeout.

This qualifies those three targets at that pin. It does not convert V170/V172/V190 failures into passes, nor establish all unrelated source/runtime requirements.

### V192 binary build and V193 executable identity

V192 ran `cargo build -p velvet-ballistics --bin velvet-ballistics --all-features` with strict warnings. Exit 0; 0.857 seconds; no timeout.

V193 selected the exact native binary path `/home/lewis/src/isoloated/vb-qz2yn-integration-local5090-v12/target/debug/velvet-ballistics`, matching its `CARGO_BIN_EXE_velvet-ballistics` override. Recorded before/after identity was:

- SHA256 `d00ddb0d065d14be8c77029c89bcee8d6e76e9697e15183fdd07ffd70612e6dc`.
- Size 96,635,296 bytes.
- Both records attributed the binary to source `daa75236facecac24c30be18ecb733d604fbf5d8`.
- Recorded before time 1792773357928810009 and after time 1792773885727856375.

I checked these receipt bytes and their hashes. I did not access or independently rehash the native executable from the cloud. This is the appropriate provenance distinction, rather than claiming direct native filesystem verification.

### V193 workspace run

The command was `cargo nextest run --cargo-quiet --workspace --all-features --no-fail-fast`, with explicit shared target/tmp configuration, empty `RUSTC_WRAPPER`, `RUSTFLAGS=-Dwarnings`, `RUST_TEST_THREADS=1`, `NEXTEST_TEST_THREADS=8`, and the pinned CLI override.

The gate recorded exit 0, 527.295 seconds, timeout limit 1,800 seconds, and `timed_out=false`. Raw nextest stderr recorded:

- Starting 14,728 tests across 309 binaries, with 27 skipped.
- Summary after 498.881 seconds: 14,728 run, 14,728 passed, 21 slow, 27 skipped.

The difference between process elapsed time and nextest's test summary time is retained, not conflated. There was no generated regression-seed drift in these maps.

**Evidence limit:** stdout is empty, and stderr is only the 298-byte aggregate summary. Individual PASS names and the identities/reasons for the 27 skips were not retained in that raw output. The unfiltered command and aggregate summary support the whole-workspace result, but I cannot honestly claim to have matched every historical failed test name to a V193 per-test PASS line. I also cannot call the skipped tests passed.

## Preserved limitations and approval boundary

1. My approval is for the specified test migrations, equivalent refactors, and receipt claims at exact pins. It is not approval to weaken unrelated guards or to declare F1/F2 fully finished.
2. Canonical `moon ci` did not acquire a successful terminal receipt merely because V193 nextest passed. V131 remains a failed, deliberately stopped canonical run with a separate formatting failure.
3. V187 remains a timeout with no Kani verdict. A later active proof is not evidence until its terminal output and provenance are reviewed.
4. `aee06b2414d2e664e8da74189a0892fdcf2d6443` and `3107f1e157916de3733a3e9dad3bcf60a8183a37` were not obtainable through the original Git route. Recorded map content can be compared with available published blobs, but missing commits must not be represented as fetched or independently tree-compared. I did not seek an alternate route around those missing-object results.
5. Earlier source-map drift from generated seeds and Verus evidence was real and preserved. An unchanged-code statement must identify its file set rather than implying an unchanged complete repository.
6. The compile-only nested Do, primitive layout, raw budget, raw allocation, and typed-ID tests do not prove runtime actions, taints, complete lexical admission, or decoder roundtrips beyond their explicit oracles.
7. Input tests described above use `compile_source` and the core drive path, not the complete `compile_workflow`/RuntimeEngine/journal admission route.
8. Raw slot-allocation overflow at prefix 65,534 preserves partial allocation. No transaction-atomicity claim follows.
9. Formatting-equivalent changes preserve the original test meanings, but their native format receipt belongs to exact `446511bc…`, not every descendant automatically.
10. Full F1 requirements including arbitrary bounded nested/recovery flows, text/object values, secret/result/example admission, and related cross-layer contracts remain outside the closure established here. A green existing suite is not a substitute for missing requirements-to-source/runtime tests.
11. The frozen public Repeat max-1/max-3 source-to-runtime tests were preserved by the reviewed migration work. Earlier vertical qualification is separate evidence; these compile-only migrations do not replace it.
12. No native execution, production action, source edit, main push, or merge was performed by this reviewer for this report. No independent CODE review performed by another worker is presented here as my own work.

## Reviewer attestation

I authored this report as `/root/verify_velvet_f1_acceptance` from my completed reviews and their retained evidence. The PASS, FAIL, NOT RUN, timeout, unavailable-object, and static-only qualifications above are intentional. The report does not add a new implementation review, erase historical failures, or grant broader acceptance than the cited tests and receipts support.

Publication identity is this reviewer task identity, not a Git commit signature. The coordinator may preserve this file byte-for-byte in an evidence-only commit; its file SHA256 should be recorded externally so the report does not contain a self-referential digest.
