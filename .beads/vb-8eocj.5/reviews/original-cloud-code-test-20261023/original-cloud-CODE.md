# Independent Velvet CODE review record

Reviewer: `/root/review_velvet_f2_composites`  
Recorded: 2026-10-23, UTC  
Disposition: the bounded source/test migrations below are approved, with the stated limits. This is not whole-F1/F2 acceptance, a formal-proof result, or permission to bypass any gate.

## Authority, method, and execution limits

This is my own review record, authored from my completed read-only inspections, not another reviewer's conclusions. The authorized scope was Velvet F1/F2 cloud CODE review. I inspected exact Git objects and diffs in the existing cloud integration clone, read the applicable repository instructions and NASA/JPL/runtime-architecture references, and used read-only text/token/hash comparisons. I did not edit production or test sources, check out/reset shared work, run Rust builds/tests/rustfmt/Kani, or change gates. Git diff-checks mentioned below were actually performed; they are not compilation evidence.

Native counts and process states supplied by the owner are explicitly identified as such. Independent TEST receipt verification belongs to the separate TEST reviewer. No native result is attributed to execution by me. Main was reported still at `d0645504a432552301d43094859e1261318a1bef`; I make no landing claim.

## 1. Combined assertions: approved

Review pin: `daa75236facecac24c30be18ecb733d604fbf5d8`  
Combined comparison base: `aa3c654f4bfdd6cee5c3481c5f3ebac3a3505948`  
New assertion commits: `405a7e83d27a928e8fb8e154df48bc452e7d367b` and `daa75236facecac24c30be18ecb733d604fbf5d8`  
Published bookmark: `codex/vb-8eocj-review-fixes-v191-source`

The already-reviewed raw-allocation, depth, and nested-Do migrations remain intact. In `crates/vb_compile/tests/v1_primitive_lowering.rs`, the replacement two-Set Repeat helper strengthens, rather than relaxes, the former positive:

- Entry 0, six nodes, four slots, and two constants are exact.
- Set nodes 1/2 have literal payloads 1/2, private outputs 2/3, and next targets 2/3.
- Full expected `CompiledNode` values pin Start 0, Check 3, and RepeatFinish 4, including counter slot 1, body 1, done 4, result slot 3, next edges, and absent error fields.
- Finish 5 remains result slot 0 with no output/fallthrough. Dense IDs and absent error fields are checked throughout.
- The three other exact multi-body rejection cases and their count remain. No generator or source operand was rebound.
- Together now explicitly checks first-branch output `None` and later-branch output slot 2, closing the specific inherited coverage gap I previously reported.

The final `.get()` changes repair formatting of typed IDs, and `usize::from(id)` repairs checked indexing; neither alters expected semantics. The Digest Repeat split and Collect comment are byte-identical to the separately approved `08001dd5` artifact. Owner-reported V191: 77 PASS. This review does not substitute for that receipt or for full CI.

## 2. Eleven-file formatting: approved as source-equivalent

Pin: `446511bc53310f4daad640cb66cdeaf61b995eeb`  
Parent: `9aaa36d8ff29a9e8e3163cd8a1d145c0334cbcb8`  
Bookmark: `codex/vb-8eocj-format-v182-source`

I compared every changed file using token comparison after whitespace/trailing-comma normalization, then manually reviewed the remaining differences. They are use-item ordering and equivalent single-expression closure/match-arm brace expansion/removal. No literal, operand, predicate, call, expected value, or binding changed.

Affected files are the compiler's `canonical_repeat.rs`, `canonical_repeat_tests.rs`, `canonical_types.rs`, `part_02.rs`, `part_04.rs`, and `source_slot_reservation_tests.rs`; workspace scalar-Do `negative.rs`, `positive.rs`, `type_identity.rs`, support `mod.rs`; and `vb_f2_source_slot_preservation.rs`. In particular, Repeat dispatch and checked private Reduce allocation preserve their prior behavior.

This is source-equivalence review, not a rustfmt run by me. The owner separately reported the formatting gate passed.

## 3. Kani migration: source oracle approved, proof unestablished

Pin: `c0d3e4ccf68f7299fb1aaddfa7bfaa865ea9284b`  
Parent: `446511bc53310f4daad640cb66cdeaf61b995eeb`  
Bookmark: `codex/vb-8eocj-repeat-kani-v187-source`

Files: `crates/vb_compile/src/kani_lower_control.rs`, new `kani_lower_repeat.rs`, and module registration in `lib.rs`.

The new harness calls the real `SlotCompiler::reserve_slots` and public `lower_repeat`; it is not a replacement implementation. Prefix, node ID, body ID, done ID, and maximum attempts are five independent full-u16 symbolic inputs. No assumptions exclude MAX, zero attempts, or aliased node IDs. The assertions correctly distinguish the RAW node-builder contract from validated workflow admission:

- Prefix <= 65533 must succeed, with count prefix+2, counter slot prefix, result/output slot prefix+1, and all fields of exactly three nodes checked.
- Prefix 65534 permits the first allocation, then fails with `SlotIndexOutOfRange { value: 65535 }`; final count is 65535. Prefix 65535 fails immediately with the same exact error/count. Whole-helper rollback is deliberately not invented.
- Start preserves supplied IDs/targets/attempts and writes the counter. Attempt has no output and uses that counter. Finish reads the counter and writes the separately allocated result slot. Common next/error fields remain absent.
- MAX/alias/zero cases are permitted only for this raw helper; the harness does not establish workflow admission or Repeat runtime progress.

The old Ask/Choose harness module was already disabled and remains disabled. Its status is not newly waived by this migration. The owner-reported 360-second timeout is **NO PROOF VERDICT**. Source review found a concrete one-page reviewability issue: the proof function occupied lines 96-204, 109 physical lines. I required bounded extraction without changing symbols, branch partition, assertions, or production calls. That issue is resolved by the next scope.

## 4. Kani extraction: approved; size finding resolved

Actual published pin: `dd823160331dfac4280289541de419563cf0521f`  
Actual commit parent: `3de0bcfb9b95eaa99c172d8827a9a957e8d67660`  
Harness comparison baseline: `c0d3e4ccf68f7299fb1aaddfa7bfaa865ea9284b`  
Bookmark: `codex/vb-8eocj-repeat-kani-extraction-v195-source`

The initially supplied spelling had an extra trailing `a`; ordinary fetch resolved the exact 40-character pin above. The extraction commit itself changes only `kani_lower_repeat.rs`. Approval does not silently extend to every intervening commit.

Independent comparison confirmed:

- All 37 normalized assertion expressions and all 12 covers are retained. Normalization accounts for whitespace/trailing commas and the explicit `reserve_result.is_ok()` to `builder.reserve_slots(prefix).is_ok()` inlining.
- Five independent `kani::any` inputs, `#[kani::unwind(12)]`, and the three field-check helper bodies are unchanged. No assume/stub was added.
- The cover helper receives the same five values. Reservation executes once before the actual production call. `(id, max_attempts, body, done)` is packed/unpacked in the same order. Only prefix <= 65533 selects success.
- Success checks read the same immutable builder, then verify the same nodes. Failure checks preserve the exact partial-allocation count/error. The existing deliberate `mem::forget` operations are retained; destructor behavior is not a proof claim.

The main proof is now 21 physical lines; all functions are at most 32 lines. The prior one-page defect is resolved. Diff-check passed. At this review, owner-reported V196 was active with an 1800-second/24-GiB bound; **no proof verdict was available**. I did not start a duplicate verifier.

## 5. Mechanical Digest Repeat split: approved

Pin: `08001dd5b8743473d29c51eab92ba6a5a485a081`  
Base: `aa3c654f4bfdd6cee5c3481c5f3ebac3a3505948`

Only `digest_repeat_unit.rs` and a comment in `vb_8mdp_7_collect_lowering_props.rs` change. I expanded the seven helper calls for comparison: all 25 assertion macros preserve normalized tokens and execution order; YAML bytes and test names are unchanged. Literal constant checks 10/20/30 and missing-node/wrong-kind errors remain. Every helper borrows the same immutable workflow and every Result is propagated. Main is 27 lines; helper lengths are 21/17/17/17/20/21/29. The Collect comment now accurately separates authored source from private collector/result slots. Diff-check passed; no Cargo/rustfmt execution by me.

## 6. Raw-MAX migration: approved

Final pin: `8d1b23b562dae48123186b3639d397089d0bb895`  
Preceding allocation-test draft: `66896b386a2361c349386f3c1ab71a0478a3d105`  
Comparison base: `aa3c654f4bfdd6cee5c3481c5f3ebac3a3505948`

`v1_primitive_lowering.rs` replaces the obsolete ID-derived slot overflow expectation with exact raw allocation tests. Prefixes 0/65533 pin exact three-node fields and counts 2/65535. Prefixes 65534/65535 pin exact slot overflow 65535 and final count 65535, retaining partial-allocation behavior. All other public-helper taxonomy assertions remain.

The public `lower_steps_to_ir` first calls shared validation, before `CompiledWorkflow::try_from_parts`. Therefore the all-MAX three-node graph must reject with exact `Validation(LoopBodyStepOutOfRange { step: 65535, node_count: 3, source_node: 0, label: "repeat body" })`, not the later NodeIdMismatch. The corrected expectation does not admit invalid IR; the separate mismatched-ID negative is untouched. Owner-reported V174: 47 PASS. Earlier wrong-precedence RED is retained, not erased. No Kani closure follows from these tests.

## 7. Depth fixture migration: approved for raw budget/policy coverage

Pin: `014d72abc10e49f3e3e88f87ce038c8acfb9d208`  
Files: `vb_qi37_2_4_integration_budget_errors.rs` and its new `depth_fixtures_tests.rs` child.

All four original names and depth/reject-limit pairs survive: 10/5 twice, 5/4, and 3/2. The old sequential owner spans are now correctly expected to have depth 1. New starts i with exits 2D-i are genuinely contained and independently expected to have depth D. Exact acceptance at D and exact `NestingDepthExceeded { actual: D, limit: D-1/reject_limit }` strengthen former broad comparisons. Reachability, dense IDs, checked arithmetic, reserved vectors, and slot bounds are coherent for D <= 10. No production change or policy relaxation occurs. These are explicitly raw budget/policy fixtures, not constructor or nested runtime activation proof. Owner-reported V177: 47 PASS.

## 8. Nested-Do migration: approved source-to-IR coverage

Final pin: `ba5724b6c860d946dd4b9ee9bc3d77381b244fcc`  
Main assertion commit: `3ea4c30138f1fd963984e615446ba777347138d2`  
Base: `014d72abc10e49f3e3e88f87ce038c8acfb9d208`  
Integrated receipt pin: `9aaa36d8ff29a9e8e3163cd8a1d145c0334cbcb8`

In `vb_xi2f_nested_do_lowering.rs`, read-only comparison confirmed all six YAML literals byte-identical, all test names retained, and the exact invalid-input-99999 negative unchanged. Five positives now compare every node (5/5/4/6/7) by complete `CompiledNode` equality, plus exact entry/counts and Reduce initial constant 10. Numeric external Do inputs remain 0/1, never rebound to private item/page/accumulator slots; action IDs remain 0/1. Together first-branch output None and later-branch result slot 3 are explicitly checked. Final correction only removes unused imports and uses the actual boxed branch API. Owner-reported V179: six PASS. No repeated-Do runtime claim follows.

## 9. Earlier primitive layout and taxonomy migration: approved, bounded

Pins: `4980066ebb11e3a07d5a3394e3ccb111d8528e2d` and `aa3c654f4bfdd6cee5c3481c5f3ebac3a3505948`  
Immediate helper base: `52436aa4e68f3e1bb5ca0449ea2544bb89330aae`

The five helpers in `v1_primitive_lowering.rs` match pinned emitters: ForEach source0/iterator1/item2/body3; Together accumulator1/result2 and body continuations3/5; Collect collector1/result2 with limit15/page-size5 and CollectNext; Reduce Copy0->1, next-item2/iterator3/accumulator4 with initial10; Repeat counter1/result2 and RepeatCheck.next1. Start.next is correctly None. Actual constant slices now reach payload assertions. No authored input slot changes.

Repeat two-Set acceptance replaces only an obsolete rejection; the other three exact body-width negatives remain. Empty Repeat remains precisely rejected. Invalid Set text still rejects with exact ExpressionUnknownIdentifier(text,index4,"an"); a max_attempts0 case retains StepFieldShape taxonomy. Choose setup changes explicitly from integer1 to boolean true, with expected EvalExpr, to satisfy the established typed-condition contract; this is not input-slot rebinding.

I qualified the initial helpers as not whole-node equality oracles, specifically noting absent Together branch-output assertions. The later daa75236 scope closes that named gap. At V170 the owner reported 46 PASS/one raw-MAX failure; it was not called green. No assertion was silently dropped to accept that failure.

## 10. Earlier diagnostic/property migrations: approved within their scope

### Repeat/Choose

Base `f8099f52f09b04930bf638718c13eef583966b44`; candidate `10288e00b4fb096288c89bf6c26c975aebaa244c`; receipt `b41ef9be4c971618fc317651b181b4f20e1dd3d5`.

Seven test files only. The three-flat-Set Repeat positive checks seven nodes, literal10/20/30, distinct initialized outputs, exact body/check/finish wiring, public API parts equality, digest sensitivity30->31, and deterministic recompilation. Empty body keeps exact `(0,"steps","at least one set or do step")`. RepeatFinish result is a kind field; its output is correctly the outer node field. Six Choose fixture changes use boolean setup while preserving branch generators, targets, and assertions. Other primitive negatives remain. Initial 127-line positive reviewability concern is resolved by scope5. No CLI/source-binding production change was included or approved here.

### Restate zero-ID and stable-kind property

Base `b41ef9be4c971618fc317651b181b4f20e1dd3d5`; candidate `6d6c0cfa91ceacb598e2ab7d595690fdae005a89`; file `restate_typed_partitioned_id_tests.rs`.

The full any-u64 run domain remains. Zero now requires exact InvalidRunId{run:0} from all four encoders; nonzero retains original byte/prefix checks. Existing sequence domain is unchanged; sequence and unknown-kind assertions remain outside the split. No filtering, assumption, or early return hides zero. The independent literal known-kind set is corrected to 1..=4 | 10..=41 | 50, with nine stable-ID checks. Regression seed blob `78c4c8da6c25288ef13583ea989e2a2d889e5c79` is preserved. This tests byte identity and typed rejection, not a key-decoder round trip. Owner reported two PASS; no execution by me.

## Final boundaries

These approvals are pinned CODE judgments for the stated deltas. They do not declare all assertions exhaustive, all F1 language surfaces implemented, runtime/durable replay proven, or canonical CI complete. In particular, the latest Kani execution was still pending when reviewed. Historical REDs and rejected drafts must remain evidence. No gate, constructor, runtime state check, or source-input ownership rule was waived by this report.

Earlier retained-guard/Reduce/budget findings were separately persisted at `velvet-f2-delivery/independent-code-review-v43-budget-v2-retained-73f0713d.md`, SHA256 `8525097113b66d4e3d36c229a5806cb5114f0b66baff85a4b374840ef7ed6a31`. That older report predates subsequent fixes and is not overwritten or represented as current acceptance.
