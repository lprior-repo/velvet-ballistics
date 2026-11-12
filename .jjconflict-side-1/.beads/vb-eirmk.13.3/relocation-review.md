# Replay test relocation review

## Pins and scope

Original base: `origin/main` commit `1dbcbe96cacdc0fad91edbba641f7ad3ad542fb7`. Target split commit: `743f93433ed92f6d400c7ac2f008fd47e4be2636` (JJ change `qqklyzpu`). This artifact reviews test relocation and catalog evidence only; it makes no source or gate changes.

## Exact Git blob IDs

| Role | Path | Git blob |
|---|---|---|
| old source | `crates/vb_core/src/replay/ops/tests.rs` | `6178f772bbca4f510f386f71172165ce430531b1` |
| new same-module include | `crates/vb_core/src/replay/ops/tests.rs` | `c72d1036e72d16afc56947d8d83f8313328e515d` |
| replacement include | `crates/vb_core/src/replay/ops/tests_stack.rs` | `99cc11f167ddbe5e0b424d3bdce36281c54a051f` |
| replacement include | `crates/vb_core/src/replay/ops/tests_expr.rs` | `5ec62370dc7635812b480c5132bbb11c96ca6fc3` |
| replacement include | `crates/vb_core/src/replay/ops/tests_ce003.rs` | `acb8a177acf3e0c04e34f8c0da9a64053d942165` |
| replacement include | `crates/vb_core/src/replay/ops/context_tests.rs` | `707d2b4b9ff4f087eb2b46ef21bbf7f112d784c0` |
| module owner at original base | `crates/vb_core/src/replay/ops.rs` | `80bb685c82bc801d399bbbe2501e8821065fefb9` |
| module owner at target | `crates/vb_core/src/replay/ops.rs` | `08832d1fbefbb5674370694550e8f494e3b84688` |


The `ops.rs` blobs differ because the target commit also contains an `eval_replay_op` production change. The `#[cfg(test)] #[path = "ops/tests.rs"] mod tests;` registration is byte-identical at both pins. That production change is inherited from the requested target pin and is outside this relocation review.

## Module context and actual split

The original 90 test function names remain under `vb_core::replay::ops::tests` through the unchanged `ops.rs` registration and the same `tests` lexical module. The target `tests.rs` is the short shell: it retains imports and shared helpers, then `include!`s `tests_stack.rs`, `tests_expr.rs`, and `tests_ce003.rs`; `tests_ce003.rs` in turn includes `context_tests.rs`, which contains the two additional cases.

## Registered catalog and native evidence

The available strict catalog is `/home/lewis/src/isoloated/velvet-ballistics-core-current-review/target/core-build-20261110/evidence/replay-current-catalog.json` (SHA-256 `9f0f8b4a21ee9c58b4b4e7510b5ce785732a34a604edf885fefc1d703173227c). Enumerating only `replay::ops::tests::*` entries whose `filter-match.status` is `matches` yields 92 cases: the 90 baseline cases mapped below plus `eval_replay_op_preserves_load_slot_error` and `eval_replay_op_uses_actual_step_index`. The complete filtered mapping and old/new body hashes are in `target/core-build-20261110/evidence/replay-test-body-map.tsv`. Raw execution evidence is `/home/lewis/src/isoloated/velvet-ballistics-core-current-review/target/core-build-20261110/evidence/trace-assertion-integrated-moon.log`, with receipt `MOON_EXIT=0` and Nextest run ID `4fb3cd61-799f-40ce-a823-bff3fa105ebc`; the run log has exactly 92 passing `replay::ops::tests::*` cases, matching the catalog filter set. The log's Git commit `2f3f82b4ad48de6c2b696d26ab8be4c8086c5600` is raw Git HEAD and the first parent only, not the merged working-tree identity. The actual JJ snapshot used for the run was short pin `nozunwpv84d7edd0`, change `nozunwpvlpmysqsoqxrqxppkqxulozyx` / commit `84d7edd0e4c7540d165782510d58407e10a7ee10`; its parents are JJ change `kotmxuuz` / commit `2f3f82b4ad48de6c2b696d26ab8be4c8086c5600` and JJ change `nrxryxxz` / commit `01b56891a6cf8442ddb030afc65df2bdd35eb61f` (trace 01b5). The parent observed `jj new noz84` immediately followed by the Moon command, with no source edits during the run. All five replay test source blobs listed above are identical at that JJ snapshot and reviewed source pin `qqklyzpu`.

## Preserved exact oracles

All four `assert_eq!` oracles in the original file are byte-preserved inside identical function bodies: `pop_pair_underflow_returns_error` checks `ExpressionEvalFailed { step: StepIdx::ZERO }` (original `tests.rs:227`, target `tests_stack.rs:136`); `blackhat_taint_joins_across_expression` checks value `I64(30)` and taint `Secret` (original `tests.rs:1676,1681`, target `tests_expr.rs:693,698`); `blackhat_expression_stack_overflow_is_safe` checks the remaining stack value `I64(2)` (original `tests.rs:1706`, target `tests_expr.rs:723`). The 90-case mapping contains the body SHA-256 for every original registered test; all 100 original function bodies, including shared helpers, compare byte-identical across the target chunks.

## Preservation map

| Baseline test function | Original source line | Target source line | Body SHA-256 (old = new) | Result |
|---|---|---|---|---|
| `blackhat_eval_add_max_plus_one_returns_error` | `crates/vb_core/src/replay/ops/tests.rs:1592` | `crates/vb_core/src/replay/ops/tests_expr.rs:609` | `254b5af30fd18189f817b6cf4cdc59aa604cfd7e7434f574445d0feba10fb262` | identical |
| `blackhat_eval_div_min_by_neg_one_returns_error` | `crates/vb_core/src/replay/ops/tests.rs:1548` | `crates/vb_core/src/replay/ops/tests_expr.rs:565` | `4379efc1e88c3601595c492abc93fb4a66653da4add2e5b025529ac3789c3f41` | identical |
| `blackhat_eval_mul_boundary_overflow` | `crates/vb_core/src/replay/ops/tests.rs:1614` | `crates/vb_core/src/replay/ops/tests_expr.rs:631` | `28302dff814142557a94a290e49f3a716e9e2da87d5cffe3d0b62a65bca808de` | identical |
| `blackhat_eval_sub_min_by_one_returns_error` | `crates/vb_core/src/replay/ops/tests.rs:1570` | `crates/vb_core/src/replay/ops/tests_expr.rs:587` | `30d8da3788bbe41b5d0a7662f17a66c3d60e1fcd6aab477a7486f70eeb8df300` | identical |
| `blackhat_expression_stack_overflow_is_safe` | `crates/vb_core/src/replay/ops/tests.rs:1694` | `crates/vb_core/src/replay/ops/tests_expr.rs:711` | `311a83c33a1bdb4d47714b450a5cbba9c104c235471bfe76f70e7dcf62edc923` | identical |
| `blackhat_helper_append_if_matches_engine` | `crates/vb_core/src/replay/ops/tests.rs:1962` | `crates/vb_core/src/replay/ops/tests_expr.rs:979` | `ba17a3e08ff9891b10c6f993fabceb813e20ace77dbf157b98016aa9a838f9f9` | identical |
| `blackhat_helper_append_matches_engine` | `crates/vb_core/src/replay/ops/tests.rs:1932` | `crates/vb_core/src/replay/ops/tests_expr.rs:949` | `cb5204d7a379f6b46456c9f58604e2d24e3ced2d0152abe666f7681e674b952b` | identical |
| `blackhat_helper_contains_matches_engine` | `crates/vb_core/src/replay/ops/tests.rs:1764` | `crates/vb_core/src/replay/ops/tests_expr.rs:781` | `8f5516112ccbca13a407020be9c8c55f8f718e8b203dc7b501091116386059be` | identical |
| `blackhat_helper_count_matches_engine` | `crates/vb_core/src/replay/ops/tests.rs:2089` | `crates/vb_core/src/replay/ops/tests_expr.rs:1106` | `50d97639a0bca47b5c1182ef75cbcd8e7c5ff33a555784d71a3ec52cca71cc61` | identical |
| `blackhat_helper_empty_matches_engine` | `crates/vb_core/src/replay/ops/tests.rs:1906` | `crates/vb_core/src/replay/ops/tests_expr.rs:923` | `598b16561c53e7c89427ddf4b430fb3f171bad2a02c778dca41d413bdf255227` | identical |
| `blackhat_helper_ends_with_matches_engine` | `crates/vb_core/src/replay/ops/tests.rs:1811` | `crates/vb_core/src/replay/ops/tests_expr.rs:828` | `b9827835e59e4f1c4fc188f5b73d9155093682af4524bc6ceb3c7e9086f86a97` | identical |
| `blackhat_helper_exists_matches_engine` | `crates/vb_core/src/replay/ops/tests.rs:1856` | `crates/vb_core/src/replay/ops/tests_expr.rs:873` | `1c879424c2d63d4367d333fd123e2dd117ac4d53252de316db883bfee5376d52` | identical |
| `blackhat_helper_has_matches_engine_list_membership` | `crates/vb_core/src/replay/ops/tests.rs:1829` | `crates/vb_core/src/replay/ops/tests_expr.rs:846` | `2dd51c5fe7cc3b273fcfd0f3be2d24583389e396e9d8cf6268f15591862b0452` | identical |
| `blackhat_helper_length_matches_engine` | `crates/vb_core/src/replay/ops/tests.rs:1886` | `crates/vb_core/src/replay/ops/tests_expr.rs:903` | `1090f522cb4f5bca6c81f32bfeaf0b3b2c451e075bc012af37e03428bcb5e3a3` | identical |
| `blackhat_helper_merge_matches_engine` | `crates/vb_core/src/replay/ops/tests.rs:2023` | `crates/vb_core/src/replay/ops/tests_expr.rs:1040` | `c1bcc5c6ca60e2ed426bd03f82587e21e020e8fe59ba70f79d6479971e3f076e` | identical |
| `blackhat_helper_starts_with_matches_engine` | `crates/vb_core/src/replay/ops/tests.rs:1793` | `crates/vb_core/src/replay/ops/tests_expr.rs:810` | `29b6c5e718b19806a49018c07f4bfe3834cd4e3338a28f0d7d7b3d0b23881284` | identical |
| `blackhat_helper_sum_matches_engine` | `crates/vb_core/src/replay/ops/tests.rs:2076` | `crates/vb_core/src/replay/ops/tests_expr.rs:1093` | `7dbfaf4b40bfb4c4b6c985acc2fd6091ff0faa9d18bfc14a636ccf4743a3c1c5` | identical |
| `blackhat_helper_unique_matches_engine` | `crates/vb_core/src/replay/ops/tests.rs:2102` | `crates/vb_core/src/replay/ops/tests_expr.rs:1119` | `a1a9f65855a2dd1fb522748e3e8f4c6c0bd793416ef1500c03288a830920ad41` | identical |
| `blackhat_helper_unique_rejects_non_list` | `crates/vb_core/src/replay/ops/tests.rs:2698` | `crates/vb_core/src/replay/ops/tests_ce003.rs:570` | `5a7e10a6a9505dc9de78524b34f1f3a1883c06003737404dceb8d5c2a21e3f59` | identical |
| `blackhat_taint_joins_across_expression` | `crates/vb_core/src/replay/ops/tests.rs:1639` | `crates/vb_core/src/replay/ops/tests_expr.rs:656` | `445751b2fd7c6fb4bbb257f2307d428ea4d4e2cffba08211fe87abe4f17774d1` | identical |
| `ce003_replay_lattice_random_respected` | `crates/vb_core/src/replay/ops/tests.rs:2605` | `crates/vb_core/src/replay/ops/tests_ce003.rs:477` | `783b34f79a8586a4cfc59d52a8cb349d5b770b3160e198e95769f0b7515049ce` | identical |
| `ce003_replay_list_item_taint_joins_segment` | `crates/vb_core/src/replay/ops/tests.rs:2380` | `crates/vb_core/src/replay/ops/tests_ce003.rs:252` | `89da17d54c508cf1e88eb9351ac5e4e560a5783bfb980ab229741ee68eaefa57` | identical |
| `ce003_replay_multi_segment_path_joins_every_segment_taint` | `crates/vb_core/src/replay/ops/tests.rs:2442` | `crates/vb_core/src/replay/ops/tests_ce003.rs:314` | `8518bba002a0dfe39e2c236259c3299d32cd4313fdbd11bd41771c89cb9f22a4` | identical |
| `ce003_replay_object_field_taint_joins_segment` | `crates/vb_core/src/replay/ops/tests.rs:2313` | `crates/vb_core/src/replay/ops/tests_ce003.rs:185` | `8321f0816e6293aa712908b2342050c993e5723fd35f8dd3d2173f6c391115ca` | identical |
| `ce003_replay_root_taint_joins_into_accumulator` | `crates/vb_core/src/replay/ops/tests.rs:2525` | `crates/vb_core/src/replay/ops/tests_ce003.rs:397` | `c7e53adbdb851beb3a990b64dfb364d82e9fc29c99121b9051dce892cba97917` | identical |
| `eval_add_overflow_returns_error` | `crates/vb_core/src/replay/ops/tests.rs:261` | `crates/vb_core/src/replay/ops/tests_stack.rs:170` | `92c3eadcc5b67ce75effd9ce8aa7580309d3c47808347c77755e22788a8101da` | identical |
| `eval_add_succeeds` | `crates/vb_core/src/replay/ops/tests.rs:239` | `crates/vb_core/src/replay/ops/tests_stack.rs:148` | `0576cce2fb287d240570598079f1a439f8ebd93c19354b880fc129e33eeb4cd0` | identical |
| `eval_add_underflow_returns_error` | `crates/vb_core/src/replay/ops/tests.rs:409` | `crates/vb_core/src/replay/ops/tests_stack.rs:318` | `53a0f4f9b5c3bb5b9b77a2333007106db9a9d80b2123a392b7d47e300d801da2` | identical |
| `eval_and_rejects_non_bool` | `crates/vb_core/src/replay/ops/tests.rs:926` | `crates/vb_core/src/replay/ops/tests_stack.rs:835` | `f29cb99a73ee9122e9a63956fed216529827a1eb292323ddb0ddca95ca3d15b1` | identical |
| `eval_and_true_false` | `crates/vb_core/src/replay/ops/tests.rs:802` | `crates/vb_core/src/replay/ops/tests_stack.rs:711` | `ea1b786dc6ba5856330b6ed8ee6290cc784e8b734c10bbc662e08c7b0556b9dc` | identical |
| `eval_and_true_true` | `crates/vb_core/src/replay/ops/tests.rs:776` | `crates/vb_core/src/replay/ops/tests_stack.rs:685` | `5442c0110df0abbbd0b3785994ea613dcb23c4bec5c26f43eaa4befd987944fd` | identical |
| `eval_div_by_zero_returns_error` | `crates/vb_core/src/replay/ops/tests.rs:391` | `crates/vb_core/src/replay/ops/tests_stack.rs:300` | `1d74c30227454b18669d7f8188b8b733c42e1ff002aa20ea77224c3fb0859672` | identical |
| `eval_div_negative` | `crates/vb_core/src/replay/ops/tests.rs:475` | `crates/vb_core/src/replay/ops/tests_stack.rs:384` | `5c166664ce95606093ef9ad5b905af890091f565e1e7cd4dfa66f6695080acf3` | identical |
| `eval_div_succeeds` | `crates/vb_core/src/replay/ops/tests.rs:369` | `crates/vb_core/src/replay/ops/tests_stack.rs:278` | `980ab303df2686b181613613ecacd9a07e7b06a544576eac8eec90055c47aa63` | identical |
| `eval_eq_different_types_returns_false` | `crates/vb_core/src/replay/ops/tests.rs:701` | `crates/vb_core/src/replay/ops/tests_stack.rs:610` | `72b1e9df54a63a71caa0b6b6bde580bc1f8d583d082826feffe7e8505eb59918` | identical |
| `eval_eq_null_null_returns_true` | `crates/vb_core/src/replay/ops/tests.rs:725` | `crates/vb_core/src/replay/ops/tests_stack.rs:634` | `a6376c2000ed789051f55b616ea13edf97b9d5830b9a7c8a4d1afa9930c3ae00` | identical |
| `eval_eq_same_i64` | `crates/vb_core/src/replay/ops/tests.rs:679` | `crates/vb_core/src/replay/ops/tests_stack.rs:588` | `ce15471ca14ae0a8d80e2ea8b4d01f4685be6a2e658f05ae0e55b12d6ebe4101` | identical |
| `eval_expr_for_replay_addition` | `crates/vb_core/src/replay/ops/tests.rs:1387` | `crates/vb_core/src/replay/ops/tests_expr.rs:404` | `0b1497f98d4d06df01065fd27572c3beaafd13fe9e7abe211f9569ce1fdc9715` | identical |
| `eval_expr_for_replay_comparison_chain` | `crates/vb_core/src/replay/ops/tests.rs:1433` | `crates/vb_core/src/replay/ops/tests_expr.rs:450` | `b8c94557e9d9006c5426f0074764bccdf0c0e6802fc6df83ea43ca80200fba02` | identical |
| `eval_expr_for_replay_out_of_bounds_expr` | `crates/vb_core/src/replay/ops/tests.rs:1525` | `crates/vb_core/src/replay/ops/tests_expr.rs:542` | `c25aaef6876f5f03e376fc19b4bcab3d8b6c3ee8b08d5fb9ac6864df99cf0fbf` | identical |
| `eval_expr_for_replay_with_uninitialized_slot` | `crates/vb_core/src/replay/ops/tests.rs:1486` | `crates/vb_core/src/replay/ops/tests_expr.rs:503` | `ff78c8428b002d5ebbffdc09f310d70a7c786c66b9b02304586e311a5530d188` | identical |
| `eval_gt_false_when_equal` | `crates/vb_core/src/replay/ops/tests.rs:523` | `crates/vb_core/src/replay/ops/tests_stack.rs:432` | `0c777132dffca60c2248320fec1017527847d2242a42fb879298b8dc0180d01e` | identical |
| `eval_gt_true` | `crates/vb_core/src/replay/ops/tests.rs:501` | `crates/vb_core/src/replay/ops/tests_stack.rs:410` | `b0fa4df9bb1ae15ac6daa1e033e5e9becc7b33823e27797a08ee1ecf40fb35e4` | identical |
| `eval_gte_false_when_less` | `crates/vb_core/src/replay/ops/tests.rs:611` | `crates/vb_core/src/replay/ops/tests_stack.rs:520` | `76c415918c677ae92fa3776b83410c9ef0c3eeaaa28ab986ddefc9707cfbf691` | identical |
| `eval_gte_true_when_equal` | `crates/vb_core/src/replay/ops/tests.rs:589` | `crates/vb_core/src/replay/ops/tests_stack.rs:498` | `1cb0a5ad6330be8328f9e2d264508f34782a689dca719433a0cdd45b7c2fd535` | identical |
| `eval_helper_op_with_wrong_types_returns_eval_failed` | `crates/vb_core/src/replay/ops/tests.rs:1360` | `crates/vb_core/src/replay/ops/tests_expr.rs:377` | `a1ffb855d76efb72ef4a4dcce24dc786fb9df3b3a11cffdc256f7a649ac777bf` | identical |
| `eval_load_accessor_empty_path_returns_root` | `crates/vb_core/src/replay/ops/tests.rs:1283` | `crates/vb_core/src/replay/ops/tests_expr.rs:300` | `ed1480e5f577f2465f2add1b05d836faee7412fb0ff25b9abbe35b9b2c2e8030` | identical |
| `eval_load_accessor_list_index` | `crates/vb_core/src/replay/ops/tests.rs:1228` | `crates/vb_core/src/replay/ops/tests_expr.rs:245` | `73ec439b5817b0fd1cecec56203b28c4aaed5d0eff9d12de4596419a4998f2aa` | identical |
| `eval_load_accessor_object_field` | `crates/vb_core/src/replay/ops/tests.rs:1169` | `crates/vb_core/src/replay/ops/tests_expr.rs:186` | `2634c716af4157bbe9531fe5e89a1d702d725ebf3e7379635379b51033478e39` | identical |
| `eval_load_accessor_out_of_bounds_returns_error` | `crates/vb_core/src/replay/ops/tests.rs:1335` | `crates/vb_core/src/replay/ops/tests_expr.rs:352` | `6f124f946cbd28f77739bdb3930b3fed7b7ee4e55f63d45537c3f7488712bb13` | identical |
| `eval_load_const_bool` | `crates/vb_core/src/replay/ops/tests.rs:1090` | `crates/vb_core/src/replay/ops/tests_expr.rs:107` | `621b7b8c5cae842a08b645baeebd42aae5785d96bf67a13e138039ceb4555d80` | identical |
| `eval_load_const_i64` | `crates/vb_core/src/replay/ops/tests.rs:1063` | `crates/vb_core/src/replay/ops/tests_expr.rs:80` | `5e049794f0647026dc64df59867851196226a38765d13ea8a9b62238b3aec074` | identical |
| `eval_load_const_null` | `crates/vb_core/src/replay/ops/tests.rs:1117` | `crates/vb_core/src/replay/ops/tests_expr.rs:134` | `76c4faee8887761e0be4e0813e5e9ab189b0751fb7e2529e519adc3b8d1dfc14` | identical |
| `eval_load_const_out_of_bounds_returns_error` | `crates/vb_core/src/replay/ops/tests.rs:1144` | `crates/vb_core/src/replay/ops/tests_expr.rs:161` | `9ec4bbbf8c64c0d73840c6974792a496d822fd470bc7d2d83fa5b871475917d7` | identical |
| `eval_load_slot_out_of_bounds_returns_error` | `crates/vb_core/src/replay/ops/tests.rs:1013` | `crates/vb_core/src/replay/ops/tests_expr.rs:30` | `93aa4ac96e8c1addb48208b852252fcff698988f68518b9fe0bba9e604b3d5d5` | identical |
| `eval_load_slot_propagates_taint` | `crates/vb_core/src/replay/ops/tests.rs:1036` | `crates/vb_core/src/replay/ops/tests_expr.rs:53` | `897be531326c43b20d073b5ba741cff10f0b8ab794c94e6b8621cf982803c3c7` | identical |
| `eval_load_slot_succeeds` | `crates/vb_core/src/replay/ops/tests.rs:985` | `crates/vb_core/src/replay/ops/tests_expr.rs:2` | `3cce72f3ef0af3880e0f699cec8a156077dbd5223976beb0809011e551192055` | identical |
| `eval_lt_false_when_greater` | `crates/vb_core/src/replay/ops/tests.rs:567` | `crates/vb_core/src/replay/ops/tests_stack.rs:476` | `fa20f162470202f25de44e25f560849e22d9592b6dc885592e40d47e3f84aa88` | identical |
| `eval_lt_true` | `crates/vb_core/src/replay/ops/tests.rs:545` | `crates/vb_core/src/replay/ops/tests_stack.rs:454` | `b5e324464bf25a7bf784048b392d5b263ccbdf99b9e17eee271342b8c256ccf0` | identical |
| `eval_lte_false_when_greater` | `crates/vb_core/src/replay/ops/tests.rs:655` | `crates/vb_core/src/replay/ops/tests_stack.rs:564` | `3f38efd3dea8aaab4b4d76ec4100dc088fd9eb0505a119a62ddeee717d08da5c` | identical |
| `eval_lte_true_when_equal` | `crates/vb_core/src/replay/ops/tests.rs:633` | `crates/vb_core/src/replay/ops/tests_stack.rs:542` | `6ca7ce673eafa238ec76acec6733da34796e38f030c7e877bc50e3c6bfadd01a` | identical |
| `eval_mul_overflow_returns_error` | `crates/vb_core/src/replay/ops/tests.rs:349` | `crates/vb_core/src/replay/ops/tests_stack.rs:258` | `003790753f99f0d61ec57cffc7136d79be77121f81833a29834dda3ef2fb4e7b` | identical |
| `eval_mul_succeeds` | `crates/vb_core/src/replay/ops/tests.rs:327` | `crates/vb_core/src/replay/ops/tests_stack.rs:236` | `302f7c507df96a3ae268214c79591b994c84b9521b570110eaad01eb8c97ea5d` | identical |
| `eval_mul_zero` | `crates/vb_core/src/replay/ops/tests.rs:451` | `crates/vb_core/src/replay/ops/tests_stack.rs:360` | `d3b54e5725ab7e45c3e72092bf0a41b0888f84a0df3a6e662bb8df987655889f` | identical |
| `eval_not_eq_true` | `crates/vb_core/src/replay/ops/tests.rs:747` | `crates/vb_core/src/replay/ops/tests_stack.rs:656` | `096e1c1ba101cc432745232731e6d9d9706258c2e510c00b930b04b881f80b4a` | identical |
| `eval_not_false` | `crates/vb_core/src/replay/ops/tests.rs:903` | `crates/vb_core/src/replay/ops/tests_stack.rs:812` | `b42a85b2cd4b65da2be8e09b41d648cd4c9a2ae3ee980d0e6d719f9943278a11` | identical |
| `eval_not_rejects_non_bool` | `crates/vb_core/src/replay/ops/tests.rs:966` | `crates/vb_core/src/replay/ops/tests_stack.rs:875` | `9aa36800c8b54159bc04c9fd7bc90535eade72293010cbd5da6a00f171410689` | identical |
| `eval_not_true` | `crates/vb_core/src/replay/ops/tests.rs:880` | `crates/vb_core/src/replay/ops/tests_stack.rs:789` | `63a7ab924f5dd05c09b054bb14236fd056ecdc364d1a6f79e6fd66563276d6aa` | identical |
| `eval_or_false_false` | `crates/vb_core/src/replay/ops/tests.rs:854` | `crates/vb_core/src/replay/ops/tests_stack.rs:763` | `0a61d2aa70f819611d930a1ccf01da7171e3956d09bc414c592da36da2a46816` | identical |
| `eval_or_false_true` | `crates/vb_core/src/replay/ops/tests.rs:828` | `crates/vb_core/src/replay/ops/tests_stack.rs:737` | `59633a51eb3cc7bf3e80ad11efc40e4405a93ede3425984454d03a759f3a5a3c` | identical |
| `eval_or_rejects_non_bool` | `crates/vb_core/src/replay/ops/tests.rs:946` | `crates/vb_core/src/replay/ops/tests_stack.rs:855` | `27e1424cd73b276c4af903c3f0d000ab6e36a914daff8c628e6c74fa2826be5d` | identical |
| `eval_sub_negative_result` | `crates/vb_core/src/replay/ops/tests.rs:429` | `crates/vb_core/src/replay/ops/tests_stack.rs:338` | `c9b672a00f3e6ca8c100c3a9cf0b627b651b2c09e9723301ad4150d3a10d2071` | identical |
| `eval_sub_succeeds` | `crates/vb_core/src/replay/ops/tests.rs:283` | `crates/vb_core/src/replay/ops/tests_stack.rs:192` | `f399cc5a6854ceeb28e3f3b093beba7d80a654f65c8386d4a6c351bc30ea66e8` | identical |
| `eval_sub_underflow_returns_error` | `crates/vb_core/src/replay/ops/tests.rs:307` | `crates/vb_core/src/replay/ops/tests_stack.rs:216` | `608d5555515367ce1b9c4f6c25ed4de995d3b4b2dfa8595e28a1d837088c6750` | identical |
| `expr_stack_max_capacity_boundary` | `crates/vb_core/src/replay/ops/tests.rs:140` | `crates/vb_core/src/replay/ops/tests_stack.rs:49` | `130aebf8b5ff35f52dbe86eac7b4ed8535cac6cd69409622d2792cd6b0d56f41` | identical |
| `expr_stack_pop_empty_returns_error` | `crates/vb_core/src/replay/ops/tests.rs:117` | `crates/vb_core/src/replay/ops/tests_stack.rs:26` | `04ecf1879e2714b080a39abd603e43a7a2601dd8fad1eac1bfde3b3f3c102bbe` | identical |
| `expr_stack_push_overflow_returns_error` | `crates/vb_core/src/replay/ops/tests.rs:128` | `crates/vb_core/src/replay/ops/tests_stack.rs:37` | `609e13730d018ba0d9acf7cfa3ed6a60ae90b35342cda38584c082680b6d8a90` | identical |
| `expr_stack_push_pop_roundtrip` | `crates/vb_core/src/replay/ops/tests.rs:93` | `crates/vb_core/src/replay/ops/tests_stack.rs:2` | `28d11531362878a23943acc3e15b3e41513fdfc31fb56fd33732e21b31999314` | identical |
| `pop_i64_pair_rejects_non_i64_left` | `crates/vb_core/src/replay/ops/tests.rs:191` | `crates/vb_core/src/replay/ops/tests_stack.rs:100` | `9c5496e713ab89fea70951ce70cfa19930fede333b16c000b047b5a426a31914` | identical |
| `pop_i64_pair_rejects_non_i64_right` | `crates/vb_core/src/replay/ops/tests.rs:207` | `crates/vb_core/src/replay/ops/tests_stack.rs:116` | `d5166b073073907d6bec3480825a3be827503d040926939baf44619960192db1` | identical |
| `pop_i64_pair_succeeds_for_i64_values` | `crates/vb_core/src/replay/ops/tests.rs:176` | `crates/vb_core/src/replay/ops/tests_stack.rs:85` | `4eeb08933b3811f7b2a102dee174385a070854c324e51da86ef94dce6a37dc69` | identical |
| `pop_pair_returns_left_then_right` | `crates/vb_core/src/replay/ops/tests.rs:161` | `crates/vb_core/src/replay/ops/tests_stack.rs:70` | `395d1cbccb89a6c59481bc6538863c04031b71fed7d68a07f9d05297e30d655f` | identical |
| `pop_pair_underflow_returns_error` | `crates/vb_core/src/replay/ops/tests.rs:223` | `crates/vb_core/src/replay/ops/tests_stack.rs:132` | `a19ef0886b6353ae669e72e86b21428ba2d30fdb5def0be0abbe7b848df2dafd` | identical |
| `replay_f64_add_and_div_by_zero` | `crates/vb_core/src/replay/ops/tests.rs:2202` | `crates/vb_core/src/replay/ops/tests_ce003.rs:74` | `e4cae80f185449b3d9d6c5d51eff6cc070a7f5b244b4a573da71fb810b9ab44d` | identical |
| `replay_integer_error_contract_preserved` | `crates/vb_core/src/replay/ops/tests.rs:2250` | `crates/vb_core/src/replay/ops/tests_ce003.rs:122` | `5fc35cf8b55cedb3f1228395df93207b702e8cb7c38d79e482040cbf0c0e3feb` | identical |
| `replay_mixed_add_promotes_both_directions` | `crates/vb_core/src/replay/ops/tests.rs:2134` | `crates/vb_core/src/replay/ops/tests_ce003.rs:6` | `d4d0e3549493013e1227d104cfcab54d45550d7a982dd24a7d65fa8fbab6c723` | identical |
| `replay_mixed_add_via_expr_program` | `crates/vb_core/src/replay/ops/tests.rs:2259` | `crates/vb_core/src/replay/ops/tests_ce003.rs:131` | `6f7c4072979ab28c720362bd26b4c4cc6dd426fb5d9cdde4e2f0e43e83c4ae03` | identical |
| `replay_mixed_comparisons_promote_operands` | `crates/vb_core/src/replay/ops/tests.rs:2214` | `crates/vb_core/src/replay/ops/tests_ce003.rs:86` | `1877a311ac0e86f7f97569718c2c40794bb5b481f162dd2dce14a0eba9e64922` | identical |
| `replay_mixed_div_by_zero_fails` | `crates/vb_core/src/replay/ops/tests.rs:2196` | `crates/vb_core/src/replay/ops/tests_ce003.rs:68` | `ba1b7fbfb80280a4e70cfecdf1bb0c597c2ff3037fea75a37cd0b5898f3b1350` | identical |
| `replay_mixed_sub_mul_div_promote` | `crates/vb_core/src/replay/ops/tests.rs:2160` | `crates/vb_core/src/replay/ops/tests_ce003.rs:32` | `1afa1e54cefee1d4a71ecfc87b1e881a78d00859ec5c87d5ff09fa86cf63ea6e` | identical |

## Proposed integrity entries

The two-row, hash-bound proposal is `target/core-build-20261110/evidence/test-integrity-justifications-proposal.tsv`. It binds the original `tests.rs` blob, its current include-shell blob, the four replacement include blobs, and this review artifact blob. It is not applied to `scripts/test-integrity-justifications.tsv`; independent Sol review and user approval remain prerequisites.

## Limits and approval

The protected `scripts/test-integrity-justifications.tsv` was not edited. FIFO and relaxed-followup parent fixes are separate. This is a source-bound proposal for independent Sol CODE and TEST review followed by user approval; it does not claim that the per-file checker has been fixed or that native execution passed.
