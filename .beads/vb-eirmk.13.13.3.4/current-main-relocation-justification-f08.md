# Approved relocation review: guarded97d metadata transition

Same user-approved R163 five-case intent and guarded metadata/context carry-forward, with explicit current authorization. Ordinary source parent 357503572b6df6fcd8cb7eafcf4b278c37019795, human source 6a32e0b090503217991978baec76e2f42d5751a2, current integrity base 97d50ee85f17e95a5f4ea8842bb5216dfbe7e9ab, prior base c44d595fdb34838297e27a6463a40f7909c4fdfd. Source capsule SHA256393a355904a8ed25446ed2cdf6898a578dcac88c3eb6135e282b6975ff7c162b. The frozen transition receipt binds final source/root/command/exit; current transition CODE/TEST and final canonicalCI remain separate requirements. Origin/main is retained at c44 during parent CI; this operation performs no fetch or push.

Only the five existing records' base and review-blob fields change. All case scopes, old/current/replacement blobs,22 destination hashes,70 historical records,75-row count, review path and original tail remain exact. Checker root/wrapper/13 modules,17 formal AFTER files,25 production files and four formatting AFTER files retain their guards. The renamed-test/membership body already corrected in human6a32 is a separate ordinary repair: it is not classified as metadata carry-forward and is left byte-identical to that human baseline. The97d helper-test split and automatic exception deletion are preserved. No new waiver, assertion, config, membership fixture, RootCargo, formal, gate or source helper change is introduced by this two-file transition.

The following181002-byte original portable review document remains verbatim. Earlier verdicts/provenance remain historical; this header does not claim new independent review, proof or fullCI acceptance.

---

# Current-main test relocation justification

Proposal only for vb-eirmk.13.13.3.4. Human approval and final independent packet review are pending. This artifact binds exactly five findings against base f633dfa7921d55aa4379d723afa02fdb6527a057 and candidate92471a2f5da3ea5b597ddad0f995515ee374a51b. It neither changes the checker nor adds generic exceptions. All existing ledger rows remain byte-identical.

The three old source files in the reviewed882-base packet have identical full canonical Rust ASTs at this current base. Every one of all22 current destination bindings is byte-identical to the independently reviewed packet: no new behavior or oracle delta is inferred from formatting. The replay map contains90 case identities and29 original assertion sites; record map33 and72; roundtrip map1 and15. Existing explicit normative corrections, full-u16 record traversal, exact ordered acceptance events and actual Fjall close/reopen remain as independently reviewed below. Current exact before/after source and replacement blobs are guarded by the proposed TSV rows.

This is a refreshed source/base binding of the previously reviewed five-row packet, not a claim that its old source is current. CODE report SHA256 b267e361090c273249698f441bafd0a45a8b098e41a3701f3157824604bdb102 and TEST report SHA256 349df94beb30d83f7d8408e64184fb16c23fe9699036acdb0395b6ffe397bac1 are included verbatim below for durable repository-contained review provenance. Their historical scope and limitations remain; the fresh packet reviews separately examine the current base equivalence and all unchanged destination hashes. The full native r48 run remains incomplete:15470passed,two unrelated stale member-inventory cases failed,1735unrun,oneexistingignored. No feature/master/full-CI acceptance is claimed.

Current baseline equivalence:

```json
[
  {
    "path": "crates/vb_core/src/replay/ops/tests.rs",
    "old_base": "88296a54441f49806fdc3fa21410042476d5297d",
    "new_base": "f633dfa7921d55aa4379d723afa02fdb6527a057",
    "canonical_ast_equal": true
  },
  {
    "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests.rs",
    "old_base": "88296a54441f49806fdc3fa21410042476d5297d",
    "new_base": "f633dfa7921d55aa4379d723afa02fdb6527a057",
    "canonical_ast_equal": true
  },
  {
    "path": "crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs",
    "old_base": "88296a54441f49806fdc3fa21410042476d5297d",
    "new_base": "f633dfa7921d55aa4379d723afa02fdb6527a057",
    "canonical_ast_equal": true
  }
]
```

Current destination guards:

```json
[
  {
    "group": "replay",
    "path": "crates/vb_core/src/replay/ops/contains_replay_parity_tests.rs",
    "expected_sha256": "cb00cb52b5d948cc394a6b75e04907dee86505323df160621ec24f3db4a91b0e",
    "actual_sha256": "cb00cb52b5d948cc394a6b75e04907dee86505323df160621ec24f3db4a91b0e",
    "same": true
  },
  {
    "group": "replay",
    "path": "crates/vb_core/src/replay/ops/contains_replay_workflow_tests.rs",
    "expected_sha256": "a6c9563bcd71d1164951257143d133c3c638ff321301c948ecd5c2eef00514f0",
    "actual_sha256": "a6c9563bcd71d1164951257143d133c3c638ff321301c948ecd5c2eef00514f0",
    "same": true
  },
  {
    "group": "replay",
    "path": "crates/vb_core/src/replay/ops/context_tests.rs",
    "expected_sha256": "140204b004929a1555437ce3fbf93793fbda38a06611f30ca765cae9f2facaf2",
    "actual_sha256": "140204b004929a1555437ce3fbf93793fbda38a06611f30ca765cae9f2facaf2",
    "same": true
  },
  {
    "group": "replay",
    "path": "crates/vb_core/src/replay/ops/has_bytecode_boundary_hot_tests.rs",
    "expected_sha256": "9072c8a855161abb1aaf38f36618c440605ff6204014db353e74a0ee057ddd40",
    "actual_sha256": "9072c8a855161abb1aaf38f36618c440605ff6204014db353e74a0ee057ddd40",
    "same": true
  },
  {
    "group": "replay",
    "path": "crates/vb_core/src/replay/ops/has_bytecode_boundary_replay_tests.rs",
    "expected_sha256": "aac1ab306d4f23cf3387134e9d1ee69dbe53fed8e310fd1aad1aace7e1344365",
    "actual_sha256": "aac1ab306d4f23cf3387134e9d1ee69dbe53fed8e310fd1aad1aace7e1344365",
    "same": true
  },
  {
    "group": "replay",
    "path": "crates/vb_core/src/replay/ops/has_error_replay_tests.rs",
    "expected_sha256": "e126da8a3859126b87372dde6946a6263fbcf55f38b9729928b9adab9c230c04",
    "actual_sha256": "e126da8a3859126b87372dde6946a6263fbcf55f38b9729928b9adab9c230c04",
    "same": true
  },
  {
    "group": "replay",
    "path": "crates/vb_core/src/replay/ops/has_parity_replay_tests.rs",
    "expected_sha256": "b151af706293357ccfb2843f2bfd36668c4511d238d719f069582ce7d7119ad3",
    "actual_sha256": "b151af706293357ccfb2843f2bfd36668c4511d238d719f069582ce7d7119ad3",
    "same": true
  },
  {
    "group": "replay",
    "path": "crates/vb_core/src/replay/ops/tests.rs",
    "expected_sha256": "f5320277b5d6f50d2f9b99451b035313a2d38b3edd7ad6a3e8888807e05afa5e",
    "actual_sha256": "f5320277b5d6f50d2f9b99451b035313a2d38b3edd7ad6a3e8888807e05afa5e",
    "same": true
  },
  {
    "group": "replay",
    "path": "crates/vb_core/src/replay/ops/tests_ce003.rs",
    "expected_sha256": "5be3f48147df4a0656565de1cedc1d084e0135e2f222fc21651de81693bf398d",
    "actual_sha256": "5be3f48147df4a0656565de1cedc1d084e0135e2f222fc21651de81693bf398d",
    "same": true
  },
  {
    "group": "replay",
    "path": "crates/vb_core/src/replay/ops/tests_expr.rs",
    "expected_sha256": "7ffd5bb79c5ae3b1b52f1587091d2c410f30159f4bf1afd91e1e7fa712125d50",
    "actual_sha256": "7ffd5bb79c5ae3b1b52f1587091d2c410f30159f4bf1afd91e1e7fa712125d50",
    "same": true
  },
  {
    "group": "replay",
    "path": "crates/vb_core/src/replay/ops/tests_stack.rs",
    "expected_sha256": "13fef49b1e97669bc6840170d9b62c9cd4655ba891abdeb02f60bce085eef751",
    "actual_sha256": "13fef49b1e97669bc6840170d9b62c9cd4655ba891abdeb02f60bce085eef751",
    "same": true
  },
  {
    "group": "record",
    "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests.rs",
    "expected_sha256": "3fc7d3590dd4ceca96f3b149779a305e8e440578e4f016475816e47279833a75",
    "actual_sha256": "3fc7d3590dd4ceca96f3b149779a305e8e440578e4f016475816e47279833a75",
    "same": true
  },
  {
    "group": "record",
    "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs",
    "expected_sha256": "26b09be59ecadc88881d3245fd3d24d780e26de8a9320c4b5cf127f0e4534dd2",
    "actual_sha256": "26b09be59ecadc88881d3245fd3d24d780e26de8a9320c4b5cf127f0e4534dd2",
    "same": true
  },
  {
    "group": "record",
    "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs",
    "expected_sha256": "8cc51c3230684436946cd34dec9c248b0eb9fcf3b822fc030a779eab181be58d",
    "actual_sha256": "8cc51c3230684436946cd34dec9c248b0eb9fcf3b822fc030a779eab181be58d",
    "same": true
  },
  {
    "group": "record",
    "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs",
    "expected_sha256": "615990e727e009c66d51df00c8ca5690a14064d464dc58ba5ca2412ed322ed06",
    "actual_sha256": "615990e727e009c66d51df00c8ca5690a14064d464dc58ba5ca2412ed322ed06",
    "same": true
  },
  {
    "group": "record",
    "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests/unknown_record_kind_rejection_tests.rs",
    "expected_sha256": "8f5bd94c89a976402089a09968856a95dc085b950ba7cd7edf587dce56855725",
    "actual_sha256": "8f5bd94c89a976402089a09968856a95dc085b950ba7cd7edf587dce56855725",
    "same": true
  },
  {
    "group": "record",
    "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs",
    "expected_sha256": "f384bd6d605cb9106a62e993e03f0e55368bf0685103cc224242fdf89c145479",
    "actual_sha256": "f384bd6d605cb9106a62e993e03f0e55368bf0685103cc224242fdf89c145479",
    "same": true
  },
  {
    "group": "record",
    "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests/storage_key_family_separation_tests.rs",
    "expected_sha256": "08fccd7fcd82c8b5be89c74234ad405271aef2c1ab9bcb8101427f69a42410e2",
    "actual_sha256": "08fccd7fcd82c8b5be89c74234ad405271aef2c1ab9bcb8101427f69a42410e2",
    "same": true
  },
  {
    "group": "record",
    "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests/min_max_numeric_id_roundtrip_tests.rs",
    "expected_sha256": "5dc836ad811ef5413580091db7d6a66bffcf571cb7436ce464bab3f7dd9d6f6a",
    "actual_sha256": "5dc836ad811ef5413580091db7d6a66bffcf571cb7436ce464bab3f7dd9d6f6a",
    "same": true
  },
  {
    "group": "roundtrip",
    "path": "crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs",
    "expected_sha256": "67a456b3cbeb221ad83098a01b0d46dd0c2f08d2d5621a7822d5ea8e147959c7",
    "actual_sha256": "67a456b3cbeb221ad83098a01b0d46dd0c2f08d2d5621a7822d5ea8e147959c7",
    "same": true
  },
  {
    "group": "roundtrip",
    "path": "crates/workspace_tests/tests/vb_f1_owned_inputs/event_assertions.rs",
    "expected_sha256": "11c075462b285d20c17ed284452e16f4ea999ac15f73870f260bc150dc8fbef4",
    "actual_sha256": "11c075462b285d20c17ed284452e16f4ea999ac15f73870f260bc150dc8fbef4",
    "same": true
  },
  {
    "group": "roundtrip",
    "path": "crates/workspace_tests/tests/vb_f1_owned_inputs/graph_assertions.rs",
    "expected_sha256": "e478f977f93af526772916c4071c5eaa2142f8325a7b9758a507631dbc4320d5",
    "actual_sha256": "e478f977f93af526772916c4071c5eaa2142f8325a7b9758a507631dbc4320d5",
    "same": true
  }
]
```

Historical independent CODE review (verbatim):

```json
{
  "schema_version": 1,
  "review_role": "independent CODE",
  "bead": "vb-eirmk.13.13.2.8",
  "verdict": "PASS",
  "verdict_scope": "Exact unapplied five-row protected relocation justification proposal only; ordinary CLI source b0d3 remains separately REJECTED.",
  "artifact_sha256": "0fe5d31a535a80d8cf00614816fbfe400c3d7f03291bdc477d8ad50f431d70c0",
  "patch_sha256": "18726a7a583bdb327b79007e72fcea4ea64c3586b6702b702b64e20b4602e354",
  "canonical_base": "88296a54441f49806fdc3fa21410042476d5297d",
  "source_pin": "20f585411c5df77a489b1805f30176af7ce8bf07",
  "ordinary_source_referenced": "b0d3bc118343aff5ce2da077b6b46aa045d0a0b0",
  "workspace": "/home/lewis/src/isoloated/velvet-ballistics-four-repairs-code-sol",
  "findings": [],
  "source_audit": {
    "source": "20f585411c5df77a489b1805f30176af7ce8bf07",
    "canonical_base": "88296a54441f49806fdc3fa21410042476d5297d",
    "old_ledger_sha256": "09d3eedd7f3bb7024ae19a48c460b7a39e60bd98aadfc1494fb1cdeab2b10b9e",
    "proposed_ledger_sha256": "04739b0a52a92e4317c424b342eae2240a2b3c3c96a176c51d3ce3213551da4d",
    "old_ledger_prefix_byte_preserved": true,
    "new_rows": [
      {
        "path": "crates/vb_core/src/replay/ops/tests.rs",
        "kind": "DeletedTestDeclaration",
        "detail": "removed_declarations=90 added_declarations=0",
        "old_blob": "779da03dea97aa148d559adcf247c868be13fa64",
        "new_blob": "6df0d2c4ac19489e7cf168eb5cb758b808beb7d0",
        "replacement_count": 10
      },
      {
        "path": "crates/vb_core/src/replay/ops/tests.rs",
        "kind": "WeakenedAssertion",
        "detail": "removed_exact=4 added_exact=0 added_weak=0",
        "old_blob": "779da03dea97aa148d559adcf247c868be13fa64",
        "new_blob": "6df0d2c4ac19489e7cf168eb5cb758b808beb7d0",
        "replacement_count": 10
      },
      {
        "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests.rs",
        "kind": "DeletedTestDeclaration",
        "detail": "removed_declarations=22 added_declarations=0",
        "old_blob": "30ddc9bf8b272c6577d711c4f87fb87f728ca33a",
        "new_blob": "34db544b1bea3c12dda0b2865a80e5bafedc6dae",
        "replacement_count": 7
      },
      {
        "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests.rs",
        "kind": "WeakenedAssertion",
        "detail": "removed_exact=54 added_exact=0 added_weak=0",
        "old_blob": "30ddc9bf8b272c6577d711c4f87fb87f728ca33a",
        "new_blob": "34db544b1bea3c12dda0b2865a80e5bafedc6dae",
        "replacement_count": 7
      },
      {
        "path": "crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs",
        "kind": "WeakenedAssertion",
        "detail": "removed_exact=7 added_exact=1 added_weak=0",
        "old_blob": "156cb1e5bac16fc7a0e3321b457ae1263ab801b9",
        "new_blob": "07d2adfdd84e3517d53e6941833bd259e0634c8c",
        "replacement_count": 2
      }
    ],
    "justification_git_blob": "b6c64ce2aa58df4f94fc887ea66a5972a17a59fa",
    "patch_exact_reconstruction": true,
    "groups": [
      {
        "group": "replay",
        "old_cases": 90,
        "exact_body_cases": 88,
        "changed_cases": [
          "blackhat_helper_contains_matches_engine",
          "blackhat_helper_has_matches_engine_list_membership"
        ],
        "old_assertions": 29,
        "exact_assertions": 29,
        "nonidentical_replacements": [],
        "all_case_fqns_and_assertion_source_spans_verified": true,
        "destination_files": [
          {
            "path": "crates/vb_core/src/replay/ops/contains_replay_parity_tests.rs",
            "git_blob": "400b70fd5a4b8a8fbb3143cd9dfa289dc7e01f0a",
            "sha256": "cb00cb52b5d948cc394a6b75e04907dee86505323df160621ec24f3db4a91b0e"
          },
          {
            "path": "crates/vb_core/src/replay/ops/contains_replay_workflow_tests.rs",
            "git_blob": "182d32a92ac0ef4a57e90c9bf5870d21f458dfdb",
            "sha256": "a6c9563bcd71d1164951257143d133c3c638ff321301c948ecd5c2eef00514f0"
          },
          {
            "path": "crates/vb_core/src/replay/ops/context_tests.rs",
            "git_blob": "707d2b4b9ff4f087eb2b46ef21bbf7f112d784c0",
            "sha256": "140204b004929a1555437ce3fbf93793fbda38a06611f30ca765cae9f2facaf2"
          },
          {
            "path": "crates/vb_core/src/replay/ops/has_bytecode_boundary_hot_tests.rs",
            "git_blob": "10998f607190e68af3f75f2b6a20cd91876e96f8",
            "sha256": "9072c8a855161abb1aaf38f36618c440605ff6204014db353e74a0ee057ddd40"
          },
          {
            "path": "crates/vb_core/src/replay/ops/has_bytecode_boundary_replay_tests.rs",
            "git_blob": "1dfccdd718305b3c67465c99754c190e6f743b4a",
            "sha256": "aac1ab306d4f23cf3387134e9d1ee69dbe53fed8e310fd1aad1aace7e1344365"
          },
          {
            "path": "crates/vb_core/src/replay/ops/has_error_replay_tests.rs",
            "git_blob": "1d2c167cc9fce7cbde18a11f6c3ed2666b4376c2",
            "sha256": "e126da8a3859126b87372dde6946a6263fbcf55f38b9729928b9adab9c230c04"
          },
          {
            "path": "crates/vb_core/src/replay/ops/has_parity_replay_tests.rs",
            "git_blob": "16db22d408a4e36fe47c66a6c905a53f5e3d208e",
            "sha256": "b151af706293357ccfb2843f2bfd36668c4511d238d719f069582ce7d7119ad3"
          },
          {
            "path": "crates/vb_core/src/replay/ops/tests.rs",
            "git_blob": "6df0d2c4ac19489e7cf168eb5cb758b808beb7d0",
            "sha256": "f5320277b5d6f50d2f9b99451b035313a2d38b3edd7ad6a3e8888807e05afa5e"
          },
          {
            "path": "crates/vb_core/src/replay/ops/tests_ce003.rs",
            "git_blob": "2cf49f4a867a3e8c811a5c25b8d99372743ba8a0",
            "sha256": "5be3f48147df4a0656565de1cedc1d084e0135e2f222fc21651de81693bf398d"
          },
          {
            "path": "crates/vb_core/src/replay/ops/tests_expr.rs",
            "git_blob": "890b15d29d710514a7b58e01803a11eda19b21cc",
            "sha256": "7ffd5bb79c5ae3b1b52f1587091d2c410f30159f4bf1afd91e1e7fa712125d50"
          },
          {
            "path": "crates/vb_core/src/replay/ops/tests_stack.rs",
            "git_blob": "7ba58725c5fc1c4f48ab7441107848c5d1da8498",
            "sha256": "13fef49b1e97669bc6840170d9b62c9cd4655ba891abdeb02f60bce085eef751"
          }
        ]
      },
      {
        "group": "record",
        "old_cases": 33,
        "exact_body_cases": 30,
        "changed_cases": [
          "decode_accepts_all_known_journal_event_kinds",
          "is_known_record_kind_returns_true_for_valid_kinds",
          "is_known_record_kind_returns_false_for_invalid_kinds"
        ],
        "old_assertions": 72,
        "exact_assertions": 68,
        "nonidentical_replacements": [
          {
            "line": 450,
            "rationale": "Current independent45-ID and8-nonjournal tables require exact RecordKindFamilyMismatch magic/kind or exact decoded record_kind. This strengthens the old is_ok/not-Unknown checks; it is not byte-identical relocation."
          },
          {
            "line": 468,
            "rationale": "Generic exact decoded-kind equality is mandatory for literal [38,48,49], retaining38 and adding48/49; original literal38 message is generalized."
          },
          {
            "line": 500,
            "rationale": "Current independent45-ID and8-nonjournal tables require exact RecordKindFamilyMismatch magic/kind or exact decoded record_kind. This strengthens the old is_ok/not-Unknown checks; it is not byte-identical relocation."
          },
          {
            "line": 532,
            "rationale": "Same UnknownRecordKind echoed-ID predicate with explicit expanded error message is called from complete u16 traversal; current known48/49 are separated from the obsolete old unknown48/49 samples. This is an explicit contract adaptation, not identical old inputs/messages."
          }
        ],
        "all_case_fqns_and_assertion_source_spans_verified": true,
        "destination_files": [
          {
            "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests.rs",
            "git_blob": "34db544b1bea3c12dda0b2865a80e5bafedc6dae",
            "sha256": "3fc7d3590dd4ceca96f3b149779a305e8e440578e4f016475816e47279833a75"
          },
          {
            "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs",
            "git_blob": "6cf208709abe7a783e3878b6bc19e16eeada41ee",
            "sha256": "26b09be59ecadc88881d3245fd3d24d780e26de8a9320c4b5cf127f0e4534dd2"
          },
          {
            "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs",
            "git_blob": "2966bfa94a849ea7bfde6ab2761be74429acfc77",
            "sha256": "8cc51c3230684436946cd34dec9c248b0eb9fcf3b822fc030a779eab181be58d"
          },
          {
            "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs",
            "git_blob": "0bcf14f4224e74195e1f33b81447f314c468aaca",
            "sha256": "615990e727e009c66d51df00c8ca5690a14064d464dc58ba5ca2412ed322ed06"
          },
          {
            "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests/unknown_record_kind_rejection_tests.rs",
            "git_blob": "c05531ea704c28bef557cf21f8ccee69aca9aea3",
            "sha256": "8f5bd94c89a976402089a09968856a95dc085b950ba7cd7edf587dce56855725"
          },
          {
            "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs",
            "git_blob": "079d45b3d5928e685ff3e0bb63ae55535c087f3d",
            "sha256": "f384bd6d605cb9106a62e993e03f0e55368bf0685103cc224242fdf89c145479"
          },
          {
            "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests/storage_key_family_separation_tests.rs",
            "git_blob": "cad111e13010d0d9d6684ce30e40d5246dd06bae",
            "sha256": "08fccd7fcd82c8b5be89c74234ad405271aef2c1ab9bcb8101427f69a42410e2"
          },
          {
            "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests/min_max_numeric_id_roundtrip_tests.rs",
            "git_blob": "cc60e2beb7ab821c365efdaaab7cd79a5ee567d0",
            "sha256": "5dc836ad811ef5413580091db7d6a66bffcf571cb7436ce464bab3f7dd9d6f6a"
          }
        ]
      },
      {
        "group": "roundtrip",
        "old_cases": 1,
        "exact_body_cases": 0,
        "changed_cases": [
          "strict_owned_submission_reopens_with_exact_arenas_roots_taints_and_receipt"
        ],
        "old_assertions": 15,
        "exact_assertions": 14,
        "nonidentical_replacements": [
          {
            "line": 61,
            "rationale": "Old two-event length becomes exact3 plus mandatory full RunAccepted/RunAdmission/CompatibilityDecisionEvent shape/order and run/seq0,1,2/pin/capability/policy/decision assertions. Existing RunSubmission::stage_acceptance writes the third compatibility event; no production/schema change occurs here."
          }
        ],
        "all_case_fqns_and_assertion_source_spans_verified": true,
        "destination_files": [
          {
            "path": "crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs",
            "git_blob": "07d2adfdd84e3517d53e6941833bd259e0634c8c",
            "sha256": "67a456b3cbeb221ad83098a01b0d46dd0c2f08d2d5621a7822d5ea8e147959c7"
          },
          {
            "path": "crates/workspace_tests/tests/vb_f1_owned_inputs/event_assertions.rs",
            "git_blob": "0fc2a8a3b018339b6056bf2afe9b3d5521204b4d",
            "sha256": "11c075462b285d20c17ed284452e16f4ea999ac15f73870f260bc150dc8fbef4"
          },
          {
            "path": "crates/workspace_tests/tests/vb_f1_owned_inputs/graph_assertions.rs",
            "git_blob": "bf9dcf3e832c13436750115bba64154521a00afd",
            "sha256": "e478f977f93af526772916c4071c5eaa2142f8325a7b9758a507631dbc4320d5"
          }
        ]
      }
    ]
  },
  "independent_checks": [
    "Exactly two patch paths: existing strict ledger plus portable .beads/vb-eirmk.13.13.2.8/relocation-justification-v2.md. Independently reconstructed both patch outputs in memory; no patch applied.",
    "All62 existing ledger lines remain a byte-identical prefix. Exactly5 new rows, each11 fields/full40hex base, exact before/current/replacement blobs and same relative justification Git blob.",
    "Verified every case-body equality classification independently: replay90 cases88 exact/2 disclosed normative changes; record33 cases30 exact/3 changed; roundtrip1 changed. Every mapped name/FQN and every old/replacement assertion source span was checked against immutable Git source, not map booleans.",
    "Actual same-lexical replay include registrations and seven record #[path] module identities preserve old qualified names. Roundtrip helpers remain mandatory in the registered strict close/reopen test.",
    "All29 mapped replay assertions have exact destinations, including all4 per-file removed exact oracles. Record68/72 old assertion spans match exactly; four explicit nonidentical substitutions independently adjudicated. Roundtrip14/15 old spans match; two-event length correction explicitly adjudicated.",
    "Unmodified checker still requires exact finding identity, resolved full base, current parent/replacement/review blobs and rejects active stale rows. No matcher, allowlist, source scanning domain, finding kind or checker configuration was changed."
  ],
  "semantic_adjudications": {
    "replay": "Two existing normative Contains/Has fixture bodies are not token-equivalent. Current tests_expr bytes equal independently covered source922c; old90 all remain. Contains requires List receiver/raw tagged membership; Has requires Object/Symbol key membership. No Contains rename or claim that old List-Has/string-substring inputs remain normative.",
    "record": "Current destination files equal independently reviewed0c6ade6087986e27697d9dfe3fe13548698d958a. Existing45-ID/8-nonjournal independent literals and complete u16 traversal preserved; exact family mismatch/decoded kind replaces old presence predicates, separate38/48/49 positives retained and final kind4 exact family check mandatory. Changed diagnostic messages and old obsolete48/49 unknown samples are explicitly disclosed, not labeled identical.",
    "roundtrip": "Current3 destination files equal independently reviewed90964b2775cd07d23416ee48299684e5144adb91. Same real Strict close/reopen, loaded source/roots/checkpoint/graph/taints and retained-input controls remain mandatory. Source20f585 inputs/submission.rs stage_acceptance appends accepted, admission, compatibility events in order for Strict, so obsolete expected2 becomes exact3 plus full event shapes/run/seq0,1,2/pin/capabilities/policy/decision. Expected compatibility classifier is production decide_boundary: qualifies transport/wiring, not independent classifier correctness."
  },
  "prior_independent_code": {
    "record": {
      "source": "0c6ade6087986e27697d9dfe3fe13548698d958a",
      "report_sha256": "4dd9a2609cb27d4e166da01a02431f9e24504e42fefc68fdfd334c6a847068eb",
      "all_current_destination_bytes_equal": true
    },
    "roundtrip": {
      "source": "90964b2775cd07d23416ee48299684e5144adb91",
      "report_sha256": "11386a448d62e035c3066dc7009d9203b008a5a78c6a3c253da86fd8f6fc0183",
      "all_current_destination_bytes_equal": true
    },
    "replay": {
      "source": "922c21cb58a93bc8946162f828d82557c123437f",
      "all_current_destination_bytes_equal": true,
      "limit": "Carry prior scoped CODE coverage, not a new full replay/Has/Contains qualification."
    }
  },
  "actual_raw_receipts": {
    "checker_sha256": "461cf1a5bab360748745cfad9a59a6a9ea5f7d3dcfb696aad91e956a187b0c1e",
    "checker_binary_sha256": "ceb6533feb21612b59d476007b2a57691c234d3d8c04a15a6a6da76715e64926",
    "baseline_exit": 1,
    "baseline_log_sha256": "a9ab7445e69f93d5110f995e694ab5519778f42ede465d579d77c3079e99cf94",
    "baseline_findings": [
      "DeletedTestDeclaration|crates/vb_core/src/replay/ops/tests.rs|removed_declarations=90 added_declarations=0",
      "WeakenedAssertion|crates/vb_core/src/replay/ops/tests.rs|removed_exact=4 added_exact=0 added_weak=0",
      "DeletedTestDeclaration|crates/workspace_tests/tests/vb_eepg_bdd_tests.rs|removed_declarations=22 added_declarations=0",
      "WeakenedAssertion|crates/workspace_tests/tests/vb_eepg_bdd_tests.rs|removed_exact=54 added_exact=0 added_weak=0",
      "WeakenedAssertion|crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs|removed_exact=7 added_exact=1 added_weak=0"
    ],
    "controls": [
      {
        "name": "positive",
        "exit": 0,
        "command_sha256": "e37624781045acc9838193c73943e317a9346228393a8eaa0e4cdee4b7c975a9",
        "log_sha256": "5a3dd6974b9fe66c2ae7cc861b45e30e45de0e5aae2043b51c8c382004cd079d",
        "justified": 5,
        "remaining": []
      },
      {
        "name": "removed-replacement-case",
        "exit": 1,
        "command_sha256": "e37624781045acc9838193c73943e317a9346228393a8eaa0e4cdee4b7c975a9",
        "log_sha256": "68b5a5465138a8ddb0b7a437f74486362eea303be39e5ec84af8f127e3c4e5dc",
        "justified": 3,
        "remaining": [
          "DeletedTestDeclaration|crates/workspace_tests/tests/vb_eepg_bdd_tests.rs|removed_declarations=22 added_declarations=0",
          "WeakenedAssertion|crates/workspace_tests/tests/vb_eepg_bdd_tests.rs|removed_exact=54 added_exact=0 added_weak=0"
        ]
      },
      {
        "name": "changed-current-parent",
        "exit": 1,
        "command_sha256": "e37624781045acc9838193c73943e317a9346228393a8eaa0e4cdee4b7c975a9",
        "log_sha256": "68b5a5465138a8ddb0b7a437f74486362eea303be39e5ec84af8f127e3c4e5dc",
        "justified": 3,
        "remaining": [
          "DeletedTestDeclaration|crates/workspace_tests/tests/vb_eepg_bdd_tests.rs|removed_declarations=22 added_declarations=0",
          "WeakenedAssertion|crates/workspace_tests/tests/vb_eepg_bdd_tests.rs|removed_exact=54 added_exact=0 added_weak=0"
        ]
      },
      {
        "name": "changed-review-binding",
        "exit": 1,
        "command_sha256": "e37624781045acc9838193c73943e317a9346228393a8eaa0e4cdee4b7c975a9",
        "log_sha256": "a9ab7445e69f93d5110f995e694ab5519778f42ede465d579d77c3079e99cf94",
        "justified": 0,
        "remaining": [
          "DeletedTestDeclaration|crates/vb_core/src/replay/ops/tests.rs|removed_declarations=90 added_declarations=0",
          "WeakenedAssertion|crates/vb_core/src/replay/ops/tests.rs|removed_exact=4 added_exact=0 added_weak=0",
          "DeletedTestDeclaration|crates/workspace_tests/tests/vb_eepg_bdd_tests.rs|removed_declarations=22 added_declarations=0",
          "WeakenedAssertion|crates/workspace_tests/tests/vb_eepg_bdd_tests.rs|removed_exact=54 added_exact=0 added_weak=0",
          "WeakenedAssertion|crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs|removed_exact=7 added_exact=1 added_weak=0"
        ]
      },
      {
        "name": "changed-finding-identity",
        "exit": 1,
        "command_sha256": "e37624781045acc9838193c73943e317a9346228393a8eaa0e4cdee4b7c975a9",
        "log_sha256": "7e8eb8adc39e8ef3b11105e19d354f44b94511bdc4a58e7a9c22b02d405d3fa9",
        "justified": 4,
        "remaining": [
          "DeletedTestDeclaration|crates/vb_core/src/replay/ops/tests.rs|removed_declarations=90 added_declarations=0"
        ]
      },
      {
        "name": "positive-restored",
        "exit": 0,
        "command_sha256": "e37624781045acc9838193c73943e317a9346228393a8eaa0e4cdee4b7c975a9",
        "log_sha256": "5a3dd6974b9fe66c2ae7cc861b45e30e45de0e5aae2043b51c8c382004cd079d",
        "justified": 5,
        "remaining": []
      }
    ],
    "live_ledger_sha256": "09d3eedd7f3bb7024ae19a48c460b7a39e60bd98aadfc1494fb1cdeab2b10b9e"
  },
  "authorization": {
    "applied": false,
    "independent_test_and_exact_human_approval_required": true,
    "checker_edits_authorized": false,
    "user_question": "Parent coordination only after final stable packet; reviewer did not ask/apply."
  },
  "limits": [
    "Canonical unmodified live ledger still has5 findings/exit1 at the inspected source/base. Copied overlay positive has5 JUSTIFIED/0 findings/exit0; this is not live canonical gate or wholeCI success.",
    "Four negative controls each actually exit1 with the specific rows becoming unqualified; positive-restored returns0. Reviewer independently read hashes, argv, raw exits and findings, but executed no checker/native/build/BD command.",
    "Ordinary CLI source is rejected for changed function over25 logical lines. This protected proposal does not waive that bound, approve its native family, or depend on importing b0d3.",
    "Future changes to any bound base/current/replacement/justification byte require an updated exact packet and applicability review. No suppressed finding, removed test, broad blob exemption, full master acceptance or issue closure."
  ]
}
```

Historical independent TEST review (verbatim):

```json
{
  "verdict": "PASS",
  "verdict_scope": "Conditional readiness of exact five882-base hash-bound ledger rows and durable relative justification only. Protected application still requires independent CODE and exact human permission; no ordinary b0d3 acceptance.",
  "proposal_sha256": "0fe5d31a535a80d8cf00614816fbfe400c3d7f03291bdc477d8ad50f431d70c0",
  "patch_sha256": "18726a7a583bdb327b79007e72fcea4ea64c3586b6702b702b64e20b4602e354",
  "base": "88296a54441f49806fdc3fa21410042476d5297d",
  "composed_source": "20f585411c5df77a489b1805f30176af7ce8bf07",
  "ordinary_provenance_source": "b0d3bc118343aff5ce2da077b6b46aa045d0a0b0",
  "findings": [],
  "bindings": {
    "source_map_sha256": "be0d584d22e95908bc2d802e832714305ea30cb4a9eb6206e3df0307d91e4076",
    "doc_relative_path": ".beads/vb-eirmk.13.13.2.8/relocation-justification-v2.md",
    "doc_git_blob": "b6c64ce2aa58df4f94fc887ea66a5972a17a59fa",
    "doc_sha256": "72c3cb8b8055aa8d9e74d82185cf602334bb97981cb0b1505fa32dc467875921",
    "old_ledger_sha256": "09d3eedd7f3bb7024ae19a48c460b7a39e60bd98aadfc1494fb1cdeab2b10b9e",
    "proposed_ledger_sha256": "04739b0a52a92e4317c424b342eae2240a2b3c3c96a176c51d3ce3213551da4d",
    "new_rows": 5,
    "columns_per_row": 11,
    "all_existing_rows_byte_preserved": true
  },
  "semantic_dispositions": [
    "Replay: all90 identities/FQNs preserved,88 bodies token-identical and every old helper body retained. All29 old assertion token streams\u2014including four exact findings\u2014have registered destinations. Two historical nonnormative fixtures explicitly change: Contains substring->List[7,11] membership7true/13false and Has List membership->Object present/absent Symboltrue/false through real fallible helpers; not called identical relocation.",
    "Record: all33 identities remain;30 bodies identical, three changed use independent45known/8nonjournal literals, fullu16 traversal, exact echoed UnknownRecordKind or exact family/decodedkind. Of72oldassertions,68identical and four explicitly stronger/adapted. Current production882already knows GateOpened48/TenantAdmission49, so excluding them from obsolete unknown samples is a documented contract correction; literal38positive and finalkind4family retained.",
    "Roundtrip: actual Strict Fjall commit/drop/reopen and full source/parts/owned-root/arena/taint assertions mandatory. Fourteen of15oldassertions exact; obsoleteevents.len2 becomes exact3 plus orderedRunAccepted/RunAdmission/CompatibilityDecision seq0/1/2 and full identities/pin/caps/policy/decision. Actual stage_acceptance writes those three events; 28currentassertions preserved through mandatory helpers. Compatibility expected uses production decide_boundary: no independent compatibility-algorithm proof claimed.",
    "Exact5findings only: detail strings/currentparent/oldblob/destinationblob/docblob all bound. No checker/config/suppression change or generic weakening; modified/future blobs invalidate rows."
  ],
  "controls": [
    {
      "name": "positive",
      "actual_exit": 0,
      "log_sha256": "5a3dd6974b9fe66c2ae7cc861b45e30e45de0e5aae2043b51c8c382004cd079d",
      "command_sha256": "e37624781045acc9838193c73943e317a9346228393a8eaa0e4cdee4b7c975a9",
      "command": {
        "argv": [
          "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/gate-tools/check-test-integrity",
          "--base",
          "88296a54441f49806fdc3fa21410042476d5297d"
        ],
        "cwd": "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/core-build-20261110/evidence/cli-integrity-repair/copied-overlay-v2",
        "environment": {
          "TMPDIR": "/run/user/1000/vbs",
          "GIT_DIR": "/home/lewis/src/velvet-ballistics/.git/worktrees/velvet-ballistics-cli-integrity-repair-local5090",
          "GIT_WORK_TREE": "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/core-build-20261110/evidence/cli-integrity-repair/copied-overlay-v2",
          "GIT_OPTIONAL_LOCKS": "0"
        }
      },
      "mutation": null
    },
    {
      "name": "removed-replacement-case",
      "actual_exit": 1,
      "log_sha256": "68b5a5465138a8ddb0b7a437f74486362eea303be39e5ec84af8f127e3c4e5dc",
      "command_sha256": "e37624781045acc9838193c73943e317a9346228393a8eaa0e4cdee4b7c975a9",
      "command": {
        "argv": [
          "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/gate-tools/check-test-integrity",
          "--base",
          "88296a54441f49806fdc3fa21410042476d5297d"
        ],
        "cwd": "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/core-build-20261110/evidence/cli-integrity-repair/copied-overlay-v2",
        "environment": {
          "TMPDIR": "/run/user/1000/vbs",
          "GIT_DIR": "/home/lewis/src/velvet-ballistics/.git/worktrees/velvet-ballistics-cli-integrity-repair-local5090",
          "GIT_WORK_TREE": "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/core-build-20261110/evidence/cli-integrity-repair/copied-overlay-v2",
          "GIT_OPTIONAL_LOCKS": "0"
        }
      },
      "mutation": {
        "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs",
        "before_sha256": "26b09be59ecadc88881d3245fd3d24d780e26de8a9320c4b5cf127f0e4534dd2",
        "after_sha256": "6a9d28166ece973b0df183816bae179e25fa1a420f2f40a6e651481793b9ca66"
      }
    },
    {
      "name": "changed-current-parent",
      "actual_exit": 1,
      "log_sha256": "68b5a5465138a8ddb0b7a437f74486362eea303be39e5ec84af8f127e3c4e5dc",
      "command_sha256": "e37624781045acc9838193c73943e317a9346228393a8eaa0e4cdee4b7c975a9",
      "command": {
        "argv": [
          "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/gate-tools/check-test-integrity",
          "--base",
          "88296a54441f49806fdc3fa21410042476d5297d"
        ],
        "cwd": "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/core-build-20261110/evidence/cli-integrity-repair/copied-overlay-v2",
        "environment": {
          "TMPDIR": "/run/user/1000/vbs",
          "GIT_DIR": "/home/lewis/src/velvet-ballistics/.git/worktrees/velvet-ballistics-cli-integrity-repair-local5090",
          "GIT_WORK_TREE": "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/core-build-20261110/evidence/cli-integrity-repair/copied-overlay-v2",
          "GIT_OPTIONAL_LOCKS": "0"
        }
      },
      "mutation": {
        "path": "crates/workspace_tests/tests/vb_eepg_bdd_tests.rs",
        "before_sha256": "3fc7d3590dd4ceca96f3b149779a305e8e440578e4f016475816e47279833a75",
        "after_sha256": "0f8f43a6f313f7c21373551a7b27bee9db1f6e6dea5cbd1ce44eb24a455ea9a9"
      }
    },
    {
      "name": "changed-review-binding",
      "actual_exit": 1,
      "log_sha256": "a9ab7445e69f93d5110f995e694ab5519778f42ede465d579d77c3079e99cf94",
      "command_sha256": "e37624781045acc9838193c73943e317a9346228393a8eaa0e4cdee4b7c975a9",
      "command": {
        "argv": [
          "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/gate-tools/check-test-integrity",
          "--base",
          "88296a54441f49806fdc3fa21410042476d5297d"
        ],
        "cwd": "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/core-build-20261110/evidence/cli-integrity-repair/copied-overlay-v2",
        "environment": {
          "TMPDIR": "/run/user/1000/vbs",
          "GIT_DIR": "/home/lewis/src/velvet-ballistics/.git/worktrees/velvet-ballistics-cli-integrity-repair-local5090",
          "GIT_WORK_TREE": "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/core-build-20261110/evidence/cli-integrity-repair/copied-overlay-v2",
          "GIT_OPTIONAL_LOCKS": "0"
        }
      },
      "mutation": {
        "path": ".beads/vb-eirmk.13.13.2.8/relocation-justification-v2.md",
        "before_sha256": "72c3cb8b8055aa8d9e74d82185cf602334bb97981cb0b1505fa32dc467875921",
        "after_sha256": "ab3370a150a02cc3b7b6709807dcb58df88f080e4b62919521e969bb4cd122fc"
      }
    },
    {
      "name": "changed-finding-identity",
      "actual_exit": 1,
      "log_sha256": "7e8eb8adc39e8ef3b11105e19d354f44b94511bdc4a58e7a9c22b02d405d3fa9",
      "command_sha256": "e37624781045acc9838193c73943e317a9346228393a8eaa0e4cdee4b7c975a9",
      "command": {
        "argv": [
          "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/gate-tools/check-test-integrity",
          "--base",
          "88296a54441f49806fdc3fa21410042476d5297d"
        ],
        "cwd": "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/core-build-20261110/evidence/cli-integrity-repair/copied-overlay-v2",
        "environment": {
          "TMPDIR": "/run/user/1000/vbs",
          "GIT_DIR": "/home/lewis/src/velvet-ballistics/.git/worktrees/velvet-ballistics-cli-integrity-repair-local5090",
          "GIT_WORK_TREE": "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/core-build-20261110/evidence/cli-integrity-repair/copied-overlay-v2",
          "GIT_OPTIONAL_LOCKS": "0"
        }
      },
      "mutation": {
        "path": "scripts/test-integrity-justifications.tsv",
        "before_sha256": "04739b0a52a92e4317c424b342eae2240a2b3c3c96a176c51d3ce3213551da4d",
        "after_sha256": "d0228c33804492a34b93511d22c0cfb4c818ea76a8815694441f4bd2fef1ac6b"
      }
    },
    {
      "name": "positive-restored",
      "actual_exit": 0,
      "log_sha256": "5a3dd6974b9fe66c2ae7cc861b45e30e45de0e5aae2043b51c8c382004cd079d",
      "command_sha256": "e37624781045acc9838193c73943e317a9346228393a8eaa0e4cdee4b7c975a9",
      "command": {
        "argv": [
          "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/gate-tools/check-test-integrity",
          "--base",
          "88296a54441f49806fdc3fa21410042476d5297d"
        ],
        "cwd": "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/core-build-20261110/evidence/cli-integrity-repair/copied-overlay-v2",
        "environment": {
          "TMPDIR": "/run/user/1000/vbs",
          "GIT_DIR": "/home/lewis/src/velvet-ballistics/.git/worktrees/velvet-ballistics-cli-integrity-repair-local5090",
          "GIT_WORK_TREE": "/home/lewis/src/isoloated/velvet-ballistics-cli-integrity-repair-local5090/target/core-build-20261110/evidence/cli-integrity-repair/copied-overlay-v2",
          "GIT_OPTIONAL_LOCKS": "0"
        }
      },
      "mutation": null
    }
  ],
  "bounded_family_execution": {
    "source": "54e3e67dc4a6ca8c9c900e22cc8e4a7af293918a",
    "summary": [
      "Summary [  28.120s] 17172 tests run: 17172 passed, 1 skipped"
    ],
    "actual_exit": 0,
    "log_sha256": "9e7886b86d80b0d1436df326764ae39c6c9ae1d26a4d336bd20600dd9e918a37",
    "all124_old_registered_cases_observed_PASS": true,
    "independent_execution": false
  },
  "limits": [
    "Copiedchecker positive/restored exit0 with fiveJUSTIFIED; removal of one #[test], wrong current parent blob, wrong review document blob and wrong finding detail each realexit1. Controls test hashes/finding identity, not semantic theorem strength.",
    "b0d3 ordinary CLI is REJECT in separate report. Its successor must refresh provenance and actual landing before/current blobs before protected application; unchanged family hashes can be reused after guard.",
    "No live checker/gate/ledger/source/DB changes, dynamic execution or broad verification here. Existing legacy panic/expect/index debt and fullsource behavior are not waived by these rows.",
    "No complete feature/CI/master190/proof acceptance. Future base/blob changes require new proposal/review; prior conditional approvals are not substituted."
  ],
  "measurements": {
    "path": "/home/lewis/src/isoloated/velvet-ballistics-four-repairs-test-sol/target/evidence/test-integrity-current882-ledger-test-review-v2-measurements.json",
    "sha256": "df5fcb95e56eecfde03b7150295c8b4015f775426212429d3671406d1aedcd0b"
  }
}
```

Historical reviewed detailed relocation map (verbatim; all base/source/line references below remain explicitly historical):

````markdown
# Proposed current882 relocation justification

UNAPPLIED author proposal for vb-eirmk.13.13.2.8. Independent CODE and TEST plus exact human approval are pending. Existing ledger rows remain byte-identical; no checker change or exemption is proposed.

Canonical base 88296a54441f49806fdc3fa21410042476d5297d; composed source 20f585411c5df77a489b1805f30176af7ce8bf07; ordinary CLI repair b0d3bc118343aff5ce2da077b6b46aa045d0a0b0. Findings are source-bound to the unchanged replay/record/roundtrip blobs, not to an alternate base. Native CLI and unchanged Do controls do not qualify these relocation families.

Existing replay semantic corrections are explicit: historical Contains substring fixtures became List membership and historical Has List membership became Object/Symbol membership under MASTER. This proposal does not label those changes token-equivalent or extend their qualification. Existing .beads/vb-eirmk.13.13.1/replay-relocation-review.md retains the historical rationale and prior source bindings; the actual current882 maps below expose every changed body and assertion.

Record extraction preserves complete u16 traversal, all45 known/8 nonjournal IDs, separate38/48/49 positives, final kind4 family check and original test identities. The two bounded record tests invoke meaningful extracted helpers; body-token difference is expected and requires call/order review. Roundtrip extraction must retain actual strict close/reopen and ordered28 assertions through mandatory helpers; it is not justified by unrelated test counts.

## replay source binding

Old crates/vb_core/src/replay/ops/tests.rs blob 779da03dea97aa148d559adcf247c868be13fa64; current blob 6df0d2c4ac19489e7cf168eb5cb758b808beb7d0. Destination bindings: crates/vb_core/src/replay/ops/contains_replay_parity_tests.rs:400b70fd5a4b8a8fbb3143cd9dfa289dc7e01f0a; crates/vb_core/src/replay/ops/contains_replay_workflow_tests.rs:182d32a92ac0ef4a57e90c9bf5870d21f458dfdb; crates/vb_core/src/replay/ops/context_tests.rs:707d2b4b9ff4f087eb2b46ef21bbf7f112d784c0; crates/vb_core/src/replay/ops/has_bytecode_boundary_hot_tests.rs:10998f607190e68af3f75f2b6a20cd91876e96f8; crates/vb_core/src/replay/ops/has_bytecode_boundary_replay_tests.rs:1dfccdd718305b3c67465c99754c190e6f743b4a; crates/vb_core/src/replay/ops/has_error_replay_tests.rs:1d2c167cc9fce7cbde18a11f6c3ed2666b4376c2; crates/vb_core/src/replay/ops/has_parity_replay_tests.rs:16db22d408a4e36fe47c66a6c905a53f5e3d208e; crates/vb_core/src/replay/ops/tests.rs:6df0d2c4ac19489e7cf168eb5cb758b808beb7d0; crates/vb_core/src/replay/ops/tests_ce003.rs:2cf49f4a867a3e8c811a5c25b8d99372743ba8a0; crates/vb_core/src/replay/ops/tests_expr.rs:890b15d29d710514a7b58e01803a11eda19b21cc; crates/vb_core/src/replay/ops/tests_stack.rs:7ba58725c5fc1c4f48ab7441107848c5d1da8498.
- Case vb_core::replay::ops::tests::expr_stack_push_pop_roundtrip at crates/vb_core/src/replay/ops/tests.rs:93 tokens=b2a8605c2dd6f4a736f1b09d2a8dd7962136eea4cddf226d92a7e905e95e1028 -> vb_core::replay::ops::tests::expr_stack_push_pop_roundtrip at crates/vb_core/src/replay/ops/tests_stack.rs:2 tokens=b2a8605c2dd6f4a736f1b09d2a8dd7962136eea4cddf226d92a7e905e95e1028; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::expr_stack_pop_empty_returns_error at crates/vb_core/src/replay/ops/tests.rs:117 tokens=3c0df5f2d8e6c3f4240eef2e00820df574f963b07cc05b4de70df5f9afd357b0 -> vb_core::replay::ops::tests::expr_stack_pop_empty_returns_error at crates/vb_core/src/replay/ops/tests_stack.rs:26 tokens=3c0df5f2d8e6c3f4240eef2e00820df574f963b07cc05b4de70df5f9afd357b0; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::expr_stack_push_overflow_returns_error at crates/vb_core/src/replay/ops/tests.rs:128 tokens=b2055a309e7126e522bb90895b7608cf6d7420917e30eada5288e535751eef10 -> vb_core::replay::ops::tests::expr_stack_push_overflow_returns_error at crates/vb_core/src/replay/ops/tests_stack.rs:37 tokens=b2055a309e7126e522bb90895b7608cf6d7420917e30eada5288e535751eef10; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::expr_stack_max_capacity_boundary at crates/vb_core/src/replay/ops/tests.rs:140 tokens=25554b012f94ee42f2a6b7cc71808969182c03f5d95d64f231c46380d06b5224 -> vb_core::replay::ops::tests::expr_stack_max_capacity_boundary at crates/vb_core/src/replay/ops/tests_stack.rs:49 tokens=25554b012f94ee42f2a6b7cc71808969182c03f5d95d64f231c46380d06b5224; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::pop_pair_returns_left_then_right at crates/vb_core/src/replay/ops/tests.rs:161 tokens=9c1c9390db5b26666e2e794a9d26355eeeff874c4906860ad0bd5e8de7b8dfd9 -> vb_core::replay::ops::tests::pop_pair_returns_left_then_right at crates/vb_core/src/replay/ops/tests_stack.rs:70 tokens=9c1c9390db5b26666e2e794a9d26355eeeff874c4906860ad0bd5e8de7b8dfd9; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::pop_i64_pair_succeeds_for_i64_values at crates/vb_core/src/replay/ops/tests.rs:176 tokens=e9f975eacea4d846483455546096ba3ec28cc7bb6b2cbdd094aaee17482397cb -> vb_core::replay::ops::tests::pop_i64_pair_succeeds_for_i64_values at crates/vb_core/src/replay/ops/tests_stack.rs:85 tokens=e9f975eacea4d846483455546096ba3ec28cc7bb6b2cbdd094aaee17482397cb; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::pop_i64_pair_rejects_non_i64_left at crates/vb_core/src/replay/ops/tests.rs:191 tokens=72f5e1c9d27d27efbabfe3d54da603adbb1aa1371a6882d55d3d924a0b63f841 -> vb_core::replay::ops::tests::pop_i64_pair_rejects_non_i64_left at crates/vb_core/src/replay/ops/tests_stack.rs:100 tokens=72f5e1c9d27d27efbabfe3d54da603adbb1aa1371a6882d55d3d924a0b63f841; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::pop_i64_pair_rejects_non_i64_right at crates/vb_core/src/replay/ops/tests.rs:207 tokens=4d9e8e92e069067468dd00395eec5ff5cf71fb6186e09e33b0e50c72a096cafe -> vb_core::replay::ops::tests::pop_i64_pair_rejects_non_i64_right at crates/vb_core/src/replay/ops/tests_stack.rs:116 tokens=4d9e8e92e069067468dd00395eec5ff5cf71fb6186e09e33b0e50c72a096cafe; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::pop_pair_underflow_returns_error at crates/vb_core/src/replay/ops/tests.rs:223 tokens=53a62619e77afdb52c0fa1d15af0e63910210635d0617fed9059982f05490ebe -> vb_core::replay::ops::tests::pop_pair_underflow_returns_error at crates/vb_core/src/replay/ops/tests_stack.rs:132 tokens=53a62619e77afdb52c0fa1d15af0e63910210635d0617fed9059982f05490ebe; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_add_succeeds at crates/vb_core/src/replay/ops/tests.rs:239 tokens=28de9a60f0943b034635a890bfe0cfab3b5189f8a79e8121b8741b4aa33fd6fe -> vb_core::replay::ops::tests::eval_add_succeeds at crates/vb_core/src/replay/ops/tests_stack.rs:148 tokens=28de9a60f0943b034635a890bfe0cfab3b5189f8a79e8121b8741b4aa33fd6fe; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_add_overflow_returns_error at crates/vb_core/src/replay/ops/tests.rs:261 tokens=a704d416535d4c81ff32e65d917b3a652be51dd03e52ec2c4cd4ea4f235c0d3e -> vb_core::replay::ops::tests::eval_add_overflow_returns_error at crates/vb_core/src/replay/ops/tests_stack.rs:170 tokens=a704d416535d4c81ff32e65d917b3a652be51dd03e52ec2c4cd4ea4f235c0d3e; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_sub_succeeds at crates/vb_core/src/replay/ops/tests.rs:283 tokens=ce9cc0c145b6dad75facb7d13f14cadc1ccdfd588897d5954c828239b92d5178 -> vb_core::replay::ops::tests::eval_sub_succeeds at crates/vb_core/src/replay/ops/tests_stack.rs:192 tokens=ce9cc0c145b6dad75facb7d13f14cadc1ccdfd588897d5954c828239b92d5178; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_sub_underflow_returns_error at crates/vb_core/src/replay/ops/tests.rs:307 tokens=6fe6edccdfcb102837299c7cdf45486e9188f373542aec67b312357d3fda945c -> vb_core::replay::ops::tests::eval_sub_underflow_returns_error at crates/vb_core/src/replay/ops/tests_stack.rs:216 tokens=6fe6edccdfcb102837299c7cdf45486e9188f373542aec67b312357d3fda945c; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_mul_succeeds at crates/vb_core/src/replay/ops/tests.rs:327 tokens=23eb792d9c6e09c270c82049e74f5dca40bb941faa0750937f5fc3ef816502bf -> vb_core::replay::ops::tests::eval_mul_succeeds at crates/vb_core/src/replay/ops/tests_stack.rs:236 tokens=23eb792d9c6e09c270c82049e74f5dca40bb941faa0750937f5fc3ef816502bf; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_mul_overflow_returns_error at crates/vb_core/src/replay/ops/tests.rs:349 tokens=8f0385725779596755e11983b7dac15604758cd4d16772a80892e39d038f48b3 -> vb_core::replay::ops::tests::eval_mul_overflow_returns_error at crates/vb_core/src/replay/ops/tests_stack.rs:258 tokens=8f0385725779596755e11983b7dac15604758cd4d16772a80892e39d038f48b3; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_div_succeeds at crates/vb_core/src/replay/ops/tests.rs:369 tokens=192e3c2acd115932c65bec8cc03ab7503e9b493538cce1094d357bcbc64437c1 -> vb_core::replay::ops::tests::eval_div_succeeds at crates/vb_core/src/replay/ops/tests_stack.rs:278 tokens=192e3c2acd115932c65bec8cc03ab7503e9b493538cce1094d357bcbc64437c1; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_div_by_zero_returns_error at crates/vb_core/src/replay/ops/tests.rs:391 tokens=41380e98dfb35b23468de263b9b5491862a78f50817e99c2d3e46eae4be6a110 -> vb_core::replay::ops::tests::eval_div_by_zero_returns_error at crates/vb_core/src/replay/ops/tests_stack.rs:300 tokens=41380e98dfb35b23468de263b9b5491862a78f50817e99c2d3e46eae4be6a110; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_add_underflow_returns_error at crates/vb_core/src/replay/ops/tests.rs:409 tokens=00e12f3d2771b7b0c75babf37e6ae3bb45b8a1ed5d522286ab7786172796b294 -> vb_core::replay::ops::tests::eval_add_underflow_returns_error at crates/vb_core/src/replay/ops/tests_stack.rs:318 tokens=00e12f3d2771b7b0c75babf37e6ae3bb45b8a1ed5d522286ab7786172796b294; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_sub_negative_result at crates/vb_core/src/replay/ops/tests.rs:429 tokens=fc1363e7bcc1574d457a1bfdfdab980457a2f0ff99d14df1c81f5ec63eded3f7 -> vb_core::replay::ops::tests::eval_sub_negative_result at crates/vb_core/src/replay/ops/tests_stack.rs:338 tokens=fc1363e7bcc1574d457a1bfdfdab980457a2f0ff99d14df1c81f5ec63eded3f7; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_mul_zero at crates/vb_core/src/replay/ops/tests.rs:451 tokens=a49a97fec12e32bb389bc5f43da80c0e94767608d779669b34363385c12fe433 -> vb_core::replay::ops::tests::eval_mul_zero at crates/vb_core/src/replay/ops/tests_stack.rs:360 tokens=a49a97fec12e32bb389bc5f43da80c0e94767608d779669b34363385c12fe433; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_div_negative at crates/vb_core/src/replay/ops/tests.rs:475 tokens=58d2eccca7bb6c73afd947945c2bab0c7f7e04f2b0df546d6737490366872542 -> vb_core::replay::ops::tests::eval_div_negative at crates/vb_core/src/replay/ops/tests_stack.rs:384 tokens=58d2eccca7bb6c73afd947945c2bab0c7f7e04f2b0df546d6737490366872542; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_gt_true at crates/vb_core/src/replay/ops/tests.rs:501 tokens=429d6c5cc3a2ca86c98e4328d728fce6fb1ac3e4069515ffb96524474039df40 -> vb_core::replay::ops::tests::eval_gt_true at crates/vb_core/src/replay/ops/tests_stack.rs:410 tokens=429d6c5cc3a2ca86c98e4328d728fce6fb1ac3e4069515ffb96524474039df40; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_gt_false_when_equal at crates/vb_core/src/replay/ops/tests.rs:523 tokens=aefb3bc799953bcfa23483eb8997437de33cfac2d663f1bf9e0e671aacd529ff -> vb_core::replay::ops::tests::eval_gt_false_when_equal at crates/vb_core/src/replay/ops/tests_stack.rs:432 tokens=aefb3bc799953bcfa23483eb8997437de33cfac2d663f1bf9e0e671aacd529ff; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_lt_true at crates/vb_core/src/replay/ops/tests.rs:545 tokens=51dafab76e125493ab6a0a9631739ca35907c67c18826fcdc0a1d3b527b5ad0b -> vb_core::replay::ops::tests::eval_lt_true at crates/vb_core/src/replay/ops/tests_stack.rs:454 tokens=51dafab76e125493ab6a0a9631739ca35907c67c18826fcdc0a1d3b527b5ad0b; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_lt_false_when_greater at crates/vb_core/src/replay/ops/tests.rs:567 tokens=2ac48646acb123f973d6b7daa65c3f690017ecb43763e0c23c8f72139696d213 -> vb_core::replay::ops::tests::eval_lt_false_when_greater at crates/vb_core/src/replay/ops/tests_stack.rs:476 tokens=2ac48646acb123f973d6b7daa65c3f690017ecb43763e0c23c8f72139696d213; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_gte_true_when_equal at crates/vb_core/src/replay/ops/tests.rs:589 tokens=88c4bbd321364b25efc23e5130ffeb601f2bbe814b895348be9437ee342a24dc -> vb_core::replay::ops::tests::eval_gte_true_when_equal at crates/vb_core/src/replay/ops/tests_stack.rs:498 tokens=88c4bbd321364b25efc23e5130ffeb601f2bbe814b895348be9437ee342a24dc; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_gte_false_when_less at crates/vb_core/src/replay/ops/tests.rs:611 tokens=94e080d28d585e0b0d7f70f266e24edcbd8c9a78ccf814a5d285166eb1cb5cfb -> vb_core::replay::ops::tests::eval_gte_false_when_less at crates/vb_core/src/replay/ops/tests_stack.rs:520 tokens=94e080d28d585e0b0d7f70f266e24edcbd8c9a78ccf814a5d285166eb1cb5cfb; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_lte_true_when_equal at crates/vb_core/src/replay/ops/tests.rs:633 tokens=4b0f0d7129a062b547742a209bfbda0e79ae3309136cc2f46e0140a64a4f591a -> vb_core::replay::ops::tests::eval_lte_true_when_equal at crates/vb_core/src/replay/ops/tests_stack.rs:542 tokens=4b0f0d7129a062b547742a209bfbda0e79ae3309136cc2f46e0140a64a4f591a; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_lte_false_when_greater at crates/vb_core/src/replay/ops/tests.rs:655 tokens=4b9c600762bd788177262c3d3decb6a16e0e1efa2c5ad818fcda976baa2e7948 -> vb_core::replay::ops::tests::eval_lte_false_when_greater at crates/vb_core/src/replay/ops/tests_stack.rs:564 tokens=4b9c600762bd788177262c3d3decb6a16e0e1efa2c5ad818fcda976baa2e7948; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_eq_same_i64 at crates/vb_core/src/replay/ops/tests.rs:679 tokens=e9f6f747bde95f10ebebade4c29b2bb2234da54cf37e05d5f9906ff35b1eeb49 -> vb_core::replay::ops::tests::eval_eq_same_i64 at crates/vb_core/src/replay/ops/tests_stack.rs:588 tokens=e9f6f747bde95f10ebebade4c29b2bb2234da54cf37e05d5f9906ff35b1eeb49; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_eq_different_types_returns_false at crates/vb_core/src/replay/ops/tests.rs:701 tokens=2853b793f05ad596220ae3cc976c0864405992fd63fc7130fcc16d2cdab584ce -> vb_core::replay::ops::tests::eval_eq_different_types_returns_false at crates/vb_core/src/replay/ops/tests_stack.rs:610 tokens=2853b793f05ad596220ae3cc976c0864405992fd63fc7130fcc16d2cdab584ce; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_eq_null_null_returns_true at crates/vb_core/src/replay/ops/tests.rs:725 tokens=5a3d123aa7b949aaef7b6c97f9539745a6589030be0c9629d21eeaf535cebb48 -> vb_core::replay::ops::tests::eval_eq_null_null_returns_true at crates/vb_core/src/replay/ops/tests_stack.rs:634 tokens=5a3d123aa7b949aaef7b6c97f9539745a6589030be0c9629d21eeaf535cebb48; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_not_eq_true at crates/vb_core/src/replay/ops/tests.rs:747 tokens=1318b5467a9b54f27b152b93e401489963e3b5339cafd0f60beff1140c06111a -> vb_core::replay::ops::tests::eval_not_eq_true at crates/vb_core/src/replay/ops/tests_stack.rs:656 tokens=1318b5467a9b54f27b152b93e401489963e3b5339cafd0f60beff1140c06111a; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_and_true_true at crates/vb_core/src/replay/ops/tests.rs:776 tokens=4873492ae79acc545c113f1ed7f643f22ea1695cd3e7025acc877e712e5589b0 -> vb_core::replay::ops::tests::eval_and_true_true at crates/vb_core/src/replay/ops/tests_stack.rs:685 tokens=4873492ae79acc545c113f1ed7f643f22ea1695cd3e7025acc877e712e5589b0; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_and_true_false at crates/vb_core/src/replay/ops/tests.rs:802 tokens=c5886cd2ec4c313af2beae283d7156a1021de9a44573ddc2c699de51331a6f30 -> vb_core::replay::ops::tests::eval_and_true_false at crates/vb_core/src/replay/ops/tests_stack.rs:711 tokens=c5886cd2ec4c313af2beae283d7156a1021de9a44573ddc2c699de51331a6f30; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_or_false_true at crates/vb_core/src/replay/ops/tests.rs:828 tokens=f0db22b66614a53710f8018a53b97d082150fd8eb2adc6cf457788ff4ddd9fb1 -> vb_core::replay::ops::tests::eval_or_false_true at crates/vb_core/src/replay/ops/tests_stack.rs:737 tokens=f0db22b66614a53710f8018a53b97d082150fd8eb2adc6cf457788ff4ddd9fb1; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_or_false_false at crates/vb_core/src/replay/ops/tests.rs:854 tokens=0a55f9c328e29f0912f74b733e9f985664da521d776835fbbc0879eea64d7999 -> vb_core::replay::ops::tests::eval_or_false_false at crates/vb_core/src/replay/ops/tests_stack.rs:763 tokens=0a55f9c328e29f0912f74b733e9f985664da521d776835fbbc0879eea64d7999; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_not_true at crates/vb_core/src/replay/ops/tests.rs:880 tokens=30f87f627ccce877d3e4c2dcc82c93b14c5d8d0123a526e876bbbb3d2f988364 -> vb_core::replay::ops::tests::eval_not_true at crates/vb_core/src/replay/ops/tests_stack.rs:789 tokens=30f87f627ccce877d3e4c2dcc82c93b14c5d8d0123a526e876bbbb3d2f988364; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_not_false at crates/vb_core/src/replay/ops/tests.rs:903 tokens=c55eff8e1fea3458020046ada118d2b87df9f42646e3c3bdc1f31b7310506470 -> vb_core::replay::ops::tests::eval_not_false at crates/vb_core/src/replay/ops/tests_stack.rs:812 tokens=c55eff8e1fea3458020046ada118d2b87df9f42646e3c3bdc1f31b7310506470; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_and_rejects_non_bool at crates/vb_core/src/replay/ops/tests.rs:926 tokens=fe310a10c007d35171e91860582d52b147c6888c23d665bcda431d9eb539bc42 -> vb_core::replay::ops::tests::eval_and_rejects_non_bool at crates/vb_core/src/replay/ops/tests_stack.rs:835 tokens=fe310a10c007d35171e91860582d52b147c6888c23d665bcda431d9eb539bc42; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_or_rejects_non_bool at crates/vb_core/src/replay/ops/tests.rs:946 tokens=0531d6db051fce0024e6fac2d67ac9d8fba8bd421d405da07999540819957aa0 -> vb_core::replay::ops::tests::eval_or_rejects_non_bool at crates/vb_core/src/replay/ops/tests_stack.rs:855 tokens=0531d6db051fce0024e6fac2d67ac9d8fba8bd421d405da07999540819957aa0; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_not_rejects_non_bool at crates/vb_core/src/replay/ops/tests.rs:966 tokens=5d95d23356db751fb63f586d01efae7c81f59d2203345cb7823ec304cad27818 -> vb_core::replay::ops::tests::eval_not_rejects_non_bool at crates/vb_core/src/replay/ops/tests_stack.rs:875 tokens=5d95d23356db751fb63f586d01efae7c81f59d2203345cb7823ec304cad27818; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_load_slot_succeeds at crates/vb_core/src/replay/ops/tests.rs:985 tokens=ca68aed8312fd83225fb4d65104a1545127428ea92e1ea8a9c5849e1766c449d -> vb_core::replay::ops::tests::eval_load_slot_succeeds at crates/vb_core/src/replay/ops/tests_expr.rs:2 tokens=ca68aed8312fd83225fb4d65104a1545127428ea92e1ea8a9c5849e1766c449d; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_load_slot_out_of_bounds_returns_error at crates/vb_core/src/replay/ops/tests.rs:1013 tokens=13adc4f331983c30bf4d23453448906eb6929464a5a83a71c7e7fb22a049b18b -> vb_core::replay::ops::tests::eval_load_slot_out_of_bounds_returns_error at crates/vb_core/src/replay/ops/tests_expr.rs:30 tokens=13adc4f331983c30bf4d23453448906eb6929464a5a83a71c7e7fb22a049b18b; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_load_slot_propagates_taint at crates/vb_core/src/replay/ops/tests.rs:1036 tokens=1b22f2b969e1e288e3133a512184e918ea8517569a8ed7728ed8faca3ace1176 -> vb_core::replay::ops::tests::eval_load_slot_propagates_taint at crates/vb_core/src/replay/ops/tests_expr.rs:53 tokens=1b22f2b969e1e288e3133a512184e918ea8517569a8ed7728ed8faca3ace1176; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_load_const_i64 at crates/vb_core/src/replay/ops/tests.rs:1063 tokens=7e497ac134c887631bb4e997856a4d1f3efc946dc5a0b16768d2f5d7d1f9979b -> vb_core::replay::ops::tests::eval_load_const_i64 at crates/vb_core/src/replay/ops/tests_expr.rs:80 tokens=7e497ac134c887631bb4e997856a4d1f3efc946dc5a0b16768d2f5d7d1f9979b; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_load_const_bool at crates/vb_core/src/replay/ops/tests.rs:1090 tokens=f15575a3be58b6424bd53c16d8f70c77f61a664d87c302b3754b9d5766f71dfd -> vb_core::replay::ops::tests::eval_load_const_bool at crates/vb_core/src/replay/ops/tests_expr.rs:107 tokens=f15575a3be58b6424bd53c16d8f70c77f61a664d87c302b3754b9d5766f71dfd; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_load_const_null at crates/vb_core/src/replay/ops/tests.rs:1117 tokens=b4346a066cc9378477ee584de681675117dce7173c7717623aa87b20538382b5 -> vb_core::replay::ops::tests::eval_load_const_null at crates/vb_core/src/replay/ops/tests_expr.rs:134 tokens=b4346a066cc9378477ee584de681675117dce7173c7717623aa87b20538382b5; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_load_const_out_of_bounds_returns_error at crates/vb_core/src/replay/ops/tests.rs:1144 tokens=a672a7fd605f3a5e8b376aa91b9f0f44c07697899075a89f0cb7c5c2feec2c6a -> vb_core::replay::ops::tests::eval_load_const_out_of_bounds_returns_error at crates/vb_core/src/replay/ops/tests_expr.rs:161 tokens=a672a7fd605f3a5e8b376aa91b9f0f44c07697899075a89f0cb7c5c2feec2c6a; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_load_accessor_object_field at crates/vb_core/src/replay/ops/tests.rs:1169 tokens=17aa2e82a03cb76132de56e97c8036ddb23109179bc9169b9ecf5063a84ebd54 -> vb_core::replay::ops::tests::eval_load_accessor_object_field at crates/vb_core/src/replay/ops/tests_expr.rs:186 tokens=17aa2e82a03cb76132de56e97c8036ddb23109179bc9169b9ecf5063a84ebd54; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_load_accessor_list_index at crates/vb_core/src/replay/ops/tests.rs:1228 tokens=d4eaec0619cf1459a169da5efb2ee03c0e33796542c8444fd42adbd680a8fddf -> vb_core::replay::ops::tests::eval_load_accessor_list_index at crates/vb_core/src/replay/ops/tests_expr.rs:245 tokens=d4eaec0619cf1459a169da5efb2ee03c0e33796542c8444fd42adbd680a8fddf; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_load_accessor_empty_path_returns_root at crates/vb_core/src/replay/ops/tests.rs:1283 tokens=bb6c0896b55ba9a9afeeadf4769089cc299ed58c745b74cc65f09a46b71d264f -> vb_core::replay::ops::tests::eval_load_accessor_empty_path_returns_root at crates/vb_core/src/replay/ops/tests_expr.rs:300 tokens=bb6c0896b55ba9a9afeeadf4769089cc299ed58c745b74cc65f09a46b71d264f; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_load_accessor_out_of_bounds_returns_error at crates/vb_core/src/replay/ops/tests.rs:1335 tokens=04a0759bd5ff7ea2514218e6f3518d76e1bc91691564e1c7d0b4bb2c0cd0d3e8 -> vb_core::replay::ops::tests::eval_load_accessor_out_of_bounds_returns_error at crates/vb_core/src/replay/ops/tests_expr.rs:352 tokens=04a0759bd5ff7ea2514218e6f3518d76e1bc91691564e1c7d0b4bb2c0cd0d3e8; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_helper_op_with_wrong_types_returns_eval_failed at crates/vb_core/src/replay/ops/tests.rs:1360 tokens=cf0ca9c73920403b4ae33f92fabe79b642f72cf9049bb16f984d90fe0e5bab00 -> vb_core::replay::ops::tests::eval_helper_op_with_wrong_types_returns_eval_failed at crates/vb_core/src/replay/ops/tests_expr.rs:377 tokens=cf0ca9c73920403b4ae33f92fabe79b642f72cf9049bb16f984d90fe0e5bab00; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_expr_for_replay_addition at crates/vb_core/src/replay/ops/tests.rs:1387 tokens=9cd53c0194f9e3b181edabe5e492472c39f84a57fe21d675ab4f0d70631e29ce -> vb_core::replay::ops::tests::eval_expr_for_replay_addition at crates/vb_core/src/replay/ops/tests_expr.rs:404 tokens=9cd53c0194f9e3b181edabe5e492472c39f84a57fe21d675ab4f0d70631e29ce; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_expr_for_replay_comparison_chain at crates/vb_core/src/replay/ops/tests.rs:1433 tokens=1d325dfe29a4ca90bd9524ed4cabc6748b3ca44911048c4e27b404157c8f8e8f -> vb_core::replay::ops::tests::eval_expr_for_replay_comparison_chain at crates/vb_core/src/replay/ops/tests_expr.rs:450 tokens=1d325dfe29a4ca90bd9524ed4cabc6748b3ca44911048c4e27b404157c8f8e8f; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_expr_for_replay_with_uninitialized_slot at crates/vb_core/src/replay/ops/tests.rs:1486 tokens=1fe0447bdfb231121d5b79e3bbcff177a82a1406028b2c1cbd8a7ca906f43365 -> vb_core::replay::ops::tests::eval_expr_for_replay_with_uninitialized_slot at crates/vb_core/src/replay/ops/tests_expr.rs:503 tokens=1fe0447bdfb231121d5b79e3bbcff177a82a1406028b2c1cbd8a7ca906f43365; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::eval_expr_for_replay_out_of_bounds_expr at crates/vb_core/src/replay/ops/tests.rs:1525 tokens=a07dcfae0fbe238181b44a3cb5670cd2c7e3b16fffe1d0b779aadb7345f928c9 -> vb_core::replay::ops::tests::eval_expr_for_replay_out_of_bounds_expr at crates/vb_core/src/replay/ops/tests_expr.rs:542 tokens=a07dcfae0fbe238181b44a3cb5670cd2c7e3b16fffe1d0b779aadb7345f928c9; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_eval_div_min_by_neg_one_returns_error at crates/vb_core/src/replay/ops/tests.rs:1548 tokens=093b632dc8eb2570434707167b8bb1335d64dec22b3f4d8fa564711d83841861 -> vb_core::replay::ops::tests::blackhat_eval_div_min_by_neg_one_returns_error at crates/vb_core/src/replay/ops/tests_expr.rs:565 tokens=093b632dc8eb2570434707167b8bb1335d64dec22b3f4d8fa564711d83841861; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_eval_sub_min_by_one_returns_error at crates/vb_core/src/replay/ops/tests.rs:1570 tokens=ad6bdadced4218e365dfab546b831b7386078b38680703c4810378d2bfad7faa -> vb_core::replay::ops::tests::blackhat_eval_sub_min_by_one_returns_error at crates/vb_core/src/replay/ops/tests_expr.rs:587 tokens=ad6bdadced4218e365dfab546b831b7386078b38680703c4810378d2bfad7faa; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_eval_add_max_plus_one_returns_error at crates/vb_core/src/replay/ops/tests.rs:1592 tokens=fec32de96e591473f3c071b5019fb2e1ec430caace74fa9c5de858b867929e8b -> vb_core::replay::ops::tests::blackhat_eval_add_max_plus_one_returns_error at crates/vb_core/src/replay/ops/tests_expr.rs:609 tokens=fec32de96e591473f3c071b5019fb2e1ec430caace74fa9c5de858b867929e8b; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_eval_mul_boundary_overflow at crates/vb_core/src/replay/ops/tests.rs:1614 tokens=090e75b8786493f60c1d6d4846772028b2f4fb8021dc5d7d1b543e90971a3943 -> vb_core::replay::ops::tests::blackhat_eval_mul_boundary_overflow at crates/vb_core/src/replay/ops/tests_expr.rs:631 tokens=090e75b8786493f60c1d6d4846772028b2f4fb8021dc5d7d1b543e90971a3943; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_taint_joins_across_expression at crates/vb_core/src/replay/ops/tests.rs:1639 tokens=22d4ac002549cca064bfb839014fee1d66a0d6a1c3bef59013bde7b0fa9665eb -> vb_core::replay::ops::tests::blackhat_taint_joins_across_expression at crates/vb_core/src/replay/ops/tests_expr.rs:656 tokens=22d4ac002549cca064bfb839014fee1d66a0d6a1c3bef59013bde7b0fa9665eb; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_expression_stack_overflow_is_safe at crates/vb_core/src/replay/ops/tests.rs:1694 tokens=aa729f93216528aa8d2a5d1f77a2440416c3ca95ceda044ba8f1b311777e3659 -> vb_core::replay::ops::tests::blackhat_expression_stack_overflow_is_safe at crates/vb_core/src/replay/ops/tests_expr.rs:711 tokens=aa729f93216528aa8d2a5d1f77a2440416c3ca95ceda044ba8f1b311777e3659; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_helper_contains_matches_engine at crates/vb_core/src/replay/ops/tests.rs:1764 tokens=616a5024fa9d66d87946a626de6290f166257e64af5c9abb80a8351388b1bddd -> vb_core::replay::ops::tests::blackhat_helper_contains_matches_engine at crates/vb_core/src/replay/ops/tests_expr.rs:781 tokens=28703f73dfbf79616e625d8779e9646b43f2c8d165ab6cc9389ae63f06604ef2; same_fqn=true; exact_function_tokens=false.
- Case vb_core::replay::ops::tests::blackhat_helper_starts_with_matches_engine at crates/vb_core/src/replay/ops/tests.rs:1793 tokens=b77f48526a3cbc872e8346a8fa217452310715a884a3851888f978cd6753f89e -> vb_core::replay::ops::tests::blackhat_helper_starts_with_matches_engine at crates/vb_core/src/replay/ops/tests_expr.rs:810 tokens=b77f48526a3cbc872e8346a8fa217452310715a884a3851888f978cd6753f89e; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_helper_ends_with_matches_engine at crates/vb_core/src/replay/ops/tests.rs:1811 tokens=14b8191a99b1134ab7f178bec7671f46e371f0dad48917b59e9ccdcc08ddb8c4 -> vb_core::replay::ops::tests::blackhat_helper_ends_with_matches_engine at crates/vb_core/src/replay/ops/tests_expr.rs:828 tokens=14b8191a99b1134ab7f178bec7671f46e371f0dad48917b59e9ccdcc08ddb8c4; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_helper_has_matches_engine_list_membership at crates/vb_core/src/replay/ops/tests.rs:1829 tokens=2666d8fc81f83b60be3c184ad3d048252dd45f16346c2f3ebf76adc9edd895d8 -> vb_core::replay::ops::tests::blackhat_helper_has_matches_engine_list_membership at crates/vb_core/src/replay/ops/tests_expr.rs:876 tokens=a4646c5f212111348d28cce97b79e70fbb283ad1513ed6a1069d4f11cbe16cee; same_fqn=true; exact_function_tokens=false.
- Case vb_core::replay::ops::tests::blackhat_helper_exists_matches_engine at crates/vb_core/src/replay/ops/tests.rs:1856 tokens=7cbdb654c6641ba89fa7b2cbaa48033cc9cee2d9b3c3b8e60036acb837a73dca -> vb_core::replay::ops::tests::blackhat_helper_exists_matches_engine at crates/vb_core/src/replay/ops/tests_expr.rs:887 tokens=7cbdb654c6641ba89fa7b2cbaa48033cc9cee2d9b3c3b8e60036acb837a73dca; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_helper_length_matches_engine at crates/vb_core/src/replay/ops/tests.rs:1886 tokens=6874c4eaca4611a23da3934a3d4fc027acef0d63bf65ccf059471b6aefc62a9a -> vb_core::replay::ops::tests::blackhat_helper_length_matches_engine at crates/vb_core/src/replay/ops/tests_expr.rs:917 tokens=6874c4eaca4611a23da3934a3d4fc027acef0d63bf65ccf059471b6aefc62a9a; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_helper_empty_matches_engine at crates/vb_core/src/replay/ops/tests.rs:1906 tokens=aee82e7f0b0e36bc248e529cf1b033dc459a8499a6804705478de5ba4d5359c7 -> vb_core::replay::ops::tests::blackhat_helper_empty_matches_engine at crates/vb_core/src/replay/ops/tests_expr.rs:937 tokens=aee82e7f0b0e36bc248e529cf1b033dc459a8499a6804705478de5ba4d5359c7; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_helper_append_matches_engine at crates/vb_core/src/replay/ops/tests.rs:1932 tokens=1c5b9ea4bd3141d3e489a7163b96edab149d94de6303b915653ce07e9aa8a6d4 -> vb_core::replay::ops::tests::blackhat_helper_append_matches_engine at crates/vb_core/src/replay/ops/tests_expr.rs:963 tokens=1c5b9ea4bd3141d3e489a7163b96edab149d94de6303b915653ce07e9aa8a6d4; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_helper_append_if_matches_engine at crates/vb_core/src/replay/ops/tests.rs:1962 tokens=6c97901c4c331525a876fd282b1e5153e8b0f0a1ccc997b3d48d65eebabd5aed -> vb_core::replay::ops::tests::blackhat_helper_append_if_matches_engine at crates/vb_core/src/replay/ops/tests_expr.rs:993 tokens=6c97901c4c331525a876fd282b1e5153e8b0f0a1ccc997b3d48d65eebabd5aed; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_helper_merge_matches_engine at crates/vb_core/src/replay/ops/tests.rs:2023 tokens=83617e85bbf5e0c40ca53bc9f971b3b2c956c31ca807f3c5483ba550834a51e4 -> vb_core::replay::ops::tests::blackhat_helper_merge_matches_engine at crates/vb_core/src/replay/ops/tests_expr.rs:1054 tokens=83617e85bbf5e0c40ca53bc9f971b3b2c956c31ca807f3c5483ba550834a51e4; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_helper_sum_matches_engine at crates/vb_core/src/replay/ops/tests.rs:2076 tokens=e5acac694c4c7fe8bd17ffa86263081531c7e0ea86727c7011bcd67cf1a16776 -> vb_core::replay::ops::tests::blackhat_helper_sum_matches_engine at crates/vb_core/src/replay/ops/tests_expr.rs:1107 tokens=e5acac694c4c7fe8bd17ffa86263081531c7e0ea86727c7011bcd67cf1a16776; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_helper_count_matches_engine at crates/vb_core/src/replay/ops/tests.rs:2089 tokens=b6cf873385afd6044887ab024c20ad47a1f4f14dcb60b9e428f046bed33eb4f7 -> vb_core::replay::ops::tests::blackhat_helper_count_matches_engine at crates/vb_core/src/replay/ops/tests_expr.rs:1120 tokens=b6cf873385afd6044887ab024c20ad47a1f4f14dcb60b9e428f046bed33eb4f7; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_helper_unique_matches_engine at crates/vb_core/src/replay/ops/tests.rs:2102 tokens=9be5d3e90a136f01956c5c388a69eb002bc29c9032f07ebf9fc523020aabec23 -> vb_core::replay::ops::tests::blackhat_helper_unique_matches_engine at crates/vb_core/src/replay/ops/tests_expr.rs:1133 tokens=9be5d3e90a136f01956c5c388a69eb002bc29c9032f07ebf9fc523020aabec23; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::replay_mixed_add_promotes_both_directions at crates/vb_core/src/replay/ops/tests.rs:2134 tokens=ca1bf22dc01ea2d5bdc3a50bfc9bdf3a4a8b3d190a52463d85d61289605bade4 -> vb_core::replay::ops::tests::replay_mixed_add_promotes_both_directions at crates/vb_core/src/replay/ops/tests_ce003.rs:6 tokens=ca1bf22dc01ea2d5bdc3a50bfc9bdf3a4a8b3d190a52463d85d61289605bade4; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::replay_mixed_sub_mul_div_promote at crates/vb_core/src/replay/ops/tests.rs:2160 tokens=6e2860395a77e450fdf1651c38860bd0a8f3e1ac1b9666c07dd7d8a2c0095583 -> vb_core::replay::ops::tests::replay_mixed_sub_mul_div_promote at crates/vb_core/src/replay/ops/tests_ce003.rs:32 tokens=6e2860395a77e450fdf1651c38860bd0a8f3e1ac1b9666c07dd7d8a2c0095583; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::replay_mixed_div_by_zero_fails at crates/vb_core/src/replay/ops/tests.rs:2196 tokens=301343042b6da7b00493303c8aede0ef6146f8d693b29c19ed745487b3b2990f -> vb_core::replay::ops::tests::replay_mixed_div_by_zero_fails at crates/vb_core/src/replay/ops/tests_ce003.rs:68 tokens=301343042b6da7b00493303c8aede0ef6146f8d693b29c19ed745487b3b2990f; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::replay_f64_add_and_div_by_zero at crates/vb_core/src/replay/ops/tests.rs:2202 tokens=83808e8d380745c08c9889940028cbc1858dea36f08d6cd373260f6a860e6243 -> vb_core::replay::ops::tests::replay_f64_add_and_div_by_zero at crates/vb_core/src/replay/ops/tests_ce003.rs:74 tokens=83808e8d380745c08c9889940028cbc1858dea36f08d6cd373260f6a860e6243; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::replay_mixed_comparisons_promote_operands at crates/vb_core/src/replay/ops/tests.rs:2214 tokens=b398f3dc54af706657b5247377d33e13d72895d8d921d87de179fadaee8f2d26 -> vb_core::replay::ops::tests::replay_mixed_comparisons_promote_operands at crates/vb_core/src/replay/ops/tests_ce003.rs:86 tokens=b398f3dc54af706657b5247377d33e13d72895d8d921d87de179fadaee8f2d26; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::replay_integer_error_contract_preserved at crates/vb_core/src/replay/ops/tests.rs:2250 tokens=189d0035115093c848139208956919f5f83df00aaca010051c0ba2b2ddf881e9 -> vb_core::replay::ops::tests::replay_integer_error_contract_preserved at crates/vb_core/src/replay/ops/tests_ce003.rs:122 tokens=189d0035115093c848139208956919f5f83df00aaca010051c0ba2b2ddf881e9; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::replay_mixed_add_via_expr_program at crates/vb_core/src/replay/ops/tests.rs:2259 tokens=c1d81d58f8c0fb0b92198a625b51f3a0aa67daab0136509b06ba67b4b15fd96a -> vb_core::replay::ops::tests::replay_mixed_add_via_expr_program at crates/vb_core/src/replay/ops/tests_ce003.rs:131 tokens=c1d81d58f8c0fb0b92198a625b51f3a0aa67daab0136509b06ba67b4b15fd96a; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::ce003_replay_object_field_taint_joins_segment at crates/vb_core/src/replay/ops/tests.rs:2313 tokens=a6ef7ff697d281f92aa39d750fcf54c4390d3306b370c47c9af19d7d127f7f1f -> vb_core::replay::ops::tests::ce003_replay_object_field_taint_joins_segment at crates/vb_core/src/replay/ops/tests_ce003.rs:185 tokens=a6ef7ff697d281f92aa39d750fcf54c4390d3306b370c47c9af19d7d127f7f1f; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::ce003_replay_list_item_taint_joins_segment at crates/vb_core/src/replay/ops/tests.rs:2380 tokens=e28506bbc7081aa568b2fa6aadfe2d356f3c751dc663ccb43a646d44089230f3 -> vb_core::replay::ops::tests::ce003_replay_list_item_taint_joins_segment at crates/vb_core/src/replay/ops/tests_ce003.rs:252 tokens=e28506bbc7081aa568b2fa6aadfe2d356f3c751dc663ccb43a646d44089230f3; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::ce003_replay_multi_segment_path_joins_every_segment_taint at crates/vb_core/src/replay/ops/tests.rs:2442 tokens=90385a8be82c4eb13c1e502076b296a939c2cc46260dd7a752c8661d5b0541ef -> vb_core::replay::ops::tests::ce003_replay_multi_segment_path_joins_every_segment_taint at crates/vb_core/src/replay/ops/tests_ce003.rs:314 tokens=90385a8be82c4eb13c1e502076b296a939c2cc46260dd7a752c8661d5b0541ef; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::ce003_replay_root_taint_joins_into_accumulator at crates/vb_core/src/replay/ops/tests.rs:2525 tokens=a4e767bb768d1b3e896c1160a26c46082368e6488a00bfd1818e2dcd173df254 -> vb_core::replay::ops::tests::ce003_replay_root_taint_joins_into_accumulator at crates/vb_core/src/replay/ops/tests_ce003.rs:397 tokens=a4e767bb768d1b3e896c1160a26c46082368e6488a00bfd1818e2dcd173df254; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::ce003_replay_lattice_random_respected at crates/vb_core/src/replay/ops/tests.rs:2605 tokens=ad5ac7ae0d4b6ecee4c03c2657ed61055e2282591601162adcd3aabbddb04bba -> vb_core::replay::ops::tests::ce003_replay_lattice_random_respected at crates/vb_core/src/replay/ops/tests_ce003.rs:477 tokens=ad5ac7ae0d4b6ecee4c03c2657ed61055e2282591601162adcd3aabbddb04bba; same_fqn=true; exact_function_tokens=true.
- Case vb_core::replay::ops::tests::blackhat_helper_unique_rejects_non_list at crates/vb_core/src/replay/ops/tests.rs:2698 tokens=9cba606a9d04982f1358721f0afd7f3eeab2a8e63899fc1081d581edde51948d -> vb_core::replay::ops::tests::blackhat_helper_unique_rejects_non_list at crates/vb_core/src/replay/ops/tests_ce003.rs:570 tokens=9cba606a9d04982f1358721f0afd7f3eeab2a8e63899fc1081d581edde51948d; same_fqn=true; exact_function_tokens=true.
- Assertion 1 vb_core::replay::ops::tests::expr_stack_pop_empty_returns_error at crates/vb_core/src/replay/ops/tests.rs:120 hash=f67d97f8be1f7a9fba4952eabee983381b1cec653b1ba59bac5a91b2ca67040f tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","step","}",")","if","step","==","StepIdx","::","ZERO",")",",","\"popping from empty stack must fail with ExpressionEvalFailed\"",")"]; exact destinations=vb_core::replay::ops::tests::expr_stack_pop_empty_returns_error at crates/vb_core/src/replay/ops/tests_stack.rs:29.
- Assertion 2 vb_core::replay::ops::tests::expr_stack_push_overflow_returns_error at crates/vb_core/src/replay/ops/tests.rs:132 hash=5cf18952b71f1ff3c6f76616837e0266260ca859e60f85565da7fa46e37eedfa tokens=["assert","!","(","matches","!","(","overflow_result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","step","}",")","if","step","==","StepIdx","::","ZERO",")",",","\"push overflow must fail with ExpressionEvalFailed\"",")"]; exact destinations=vb_core::replay::ops::tests::expr_stack_push_overflow_returns_error at crates/vb_core/src/replay/ops/tests_stack.rs:41.
- Assertion 3 vb_core::replay::ops::tests::expr_stack_max_capacity_boundary at crates/vb_core/src/replay/ops/tests.rs:151 hash=8915c62b105ebef39a5045d982dfd9816b91283f3f00c67931ebc23dbfb7a2de tokens=["assert","!","(","matches","!","(","overflow_result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","step","}",")","if","step","==","StepIdx","::","ZERO",")",",","\"push beyond capacity must fail with ExpressionEvalFailed\"",")"]; exact destinations=vb_core::replay::ops::tests::expr_stack_max_capacity_boundary at crates/vb_core/src/replay/ops/tests_stack.rs:60.
- Assertion 4 vb_core::replay::ops::tests::pop_i64_pair_rejects_non_i64_left at crates/vb_core/src/replay/ops/tests.rs:199 hash=c7cc01ecc0521cbafd31e386d949d53a28aefcab9b571986e3835f7102f42c6a tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","..","}",")",")",",","\"non-i64 left operand must fail\"",")"]; exact destinations=vb_core::replay::ops::tests::pop_i64_pair_rejects_non_i64_left at crates/vb_core/src/replay/ops/tests_stack.rs:108.
- Assertion 5 vb_core::replay::ops::tests::pop_i64_pair_rejects_non_i64_right at crates/vb_core/src/replay/ops/tests.rs:215 hash=2db6ac5ff4eefe28cfdefd9a1abfee1bf6b3f7840218068ca2ea8a50aa902160 tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","..","}",")",")",",","\"non-i64 right operand must fail\"",")"]; exact destinations=vb_core::replay::ops::tests::pop_i64_pair_rejects_non_i64_right at crates/vb_core/src/replay/ops/tests_stack.rs:124.
- Assertion 6 vb_core::replay::ops::tests::pop_pair_underflow_returns_error at crates/vb_core/src/replay/ops/tests.rs:227 hash=25686f86986641558d322966c316f9f2cd6b0fd4d620e478e87e288b42b23aa9 tokens=["assert_eq","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","step",":","StepIdx","::","ZERO","}",")",")"]; exact destinations=vb_core::replay::ops::tests::pop_pair_underflow_returns_error at crates/vb_core/src/replay/ops/tests_stack.rs:136.
- Assertion 7 vb_core::replay::ops::tests::eval_add_overflow_returns_error at crates/vb_core/src/replay/ops/tests.rs:275 hash=013a36a9e8bc0826783a5dc6e74cd51a562e12fa6a0cce10e351cc92c0d0f971 tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","..","}",")",")",",","\"i64::MAX + i64::MAX must overflow\"",")"]; exact destinations=vb_core::replay::ops::tests::eval_add_overflow_returns_error at crates/vb_core/src/replay/ops/tests_stack.rs:184.
- Assertion 8 vb_core::replay::ops::tests::eval_sub_underflow_returns_error at crates/vb_core/src/replay/ops/tests.rs:319 hash=314cfd1f8e53de2c48a3d1afb32d35bab4c51f3a0afe89f85e3706d1242def0b tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","..","}",")",")",",","\"i64::MIN - 1 must underflow\"",")"]; exact destinations=vb_core::replay::ops::tests::eval_sub_underflow_returns_error at crates/vb_core/src/replay/ops/tests_stack.rs:228.
- Assertion 9 vb_core::replay::ops::tests::eval_mul_overflow_returns_error at crates/vb_core/src/replay/ops/tests.rs:361 hash=c1ec4e6661dfea805fc89a5d2d9ceacbe6c40520afeb031b098bc416a806e0b8 tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","..","}",")",")",",","\"i64::MAX * 2 must overflow\"",")"]; exact destinations=vb_core::replay::ops::tests::eval_mul_overflow_returns_error at crates/vb_core/src/replay/ops/tests_stack.rs:270.
- Assertion 10 vb_core::replay::ops::tests::eval_div_by_zero_returns_error at crates/vb_core/src/replay/ops/tests.rs:401 hash=98d3f3b9e829449a7dd15a3b72c89a8b6abc2125e6528704397a750497f3fc69 tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","..","}",")",")",",","\"division by zero must fail\"",")"]; exact destinations=vb_core::replay::ops::tests::eval_div_by_zero_returns_error at crates/vb_core/src/replay/ops/tests_stack.rs:310.
- Assertion 11 vb_core::replay::ops::tests::eval_add_underflow_returns_error at crates/vb_core/src/replay/ops/tests.rs:421 hash=f33b40b207d5d1a688fb51a6be3fe2c3986ca040df1024a9c48ddd0453cc3b1c tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","..","}",")",")",",","\"i64::MIN + (-1) must overflow\"",")"]; exact destinations=vb_core::replay::ops::tests::eval_add_underflow_returns_error at crates/vb_core/src/replay/ops/tests_stack.rs:330.
- Assertion 12 vb_core::replay::ops::tests::eval_and_rejects_non_bool at crates/vb_core/src/replay/ops/tests.rs:938 hash=c15078c8060e773b786afdbe533cab9f1cc43a9af24a924c3d67af155f210f73 tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","..","}",")",")",",","\"And with non-bool left must fail\"",")"]; exact destinations=vb_core::replay::ops::tests::eval_and_rejects_non_bool at crates/vb_core/src/replay/ops/tests_stack.rs:847.
- Assertion 13 vb_core::replay::ops::tests::eval_or_rejects_non_bool at crates/vb_core/src/replay/ops/tests.rs:958 hash=166d7294bd5a2a963631beda37f405ae735aeccf77bc7af8d4e76c907fa2d3da tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","..","}",")",")",",","\"Or with non-bool right must fail\"",")"]; exact destinations=vb_core::replay::ops::tests::eval_or_rejects_non_bool at crates/vb_core/src/replay/ops/tests_stack.rs:867.
- Assertion 14 vb_core::replay::ops::tests::eval_not_rejects_non_bool at crates/vb_core/src/replay/ops/tests.rs:975 hash=098f5319d1750a8c04f332086ff8ccbab96daa5fe0bca5c026a3a8859e961b9d tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","..","}",")",")",",","\"Not on non-bool must fail\"",")"]; exact destinations=vb_core::replay::ops::tests::eval_not_rejects_non_bool at crates/vb_core/src/replay/ops/tests_stack.rs:884.
- Assertion 15 vb_core::replay::ops::tests::eval_load_slot_out_of_bounds_returns_error at crates/vb_core/src/replay/ops/tests.rs:1028 hash=6ad23a9f7bd60dd4854f32493cfe71bbb4f0fb7d8689874f1e4d33df462b32c4 tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","SlotNotAvailable","{","..","}",")",")",",","\"loading out-of-bounds slot must fail\"",")"]; exact destinations=vb_core::replay::ops::tests::eval_load_slot_out_of_bounds_returns_error at crates/vb_core/src/replay/ops/tests_expr.rs:45.
- Assertion 16 vb_core::replay::ops::tests::eval_load_const_out_of_bounds_returns_error at crates/vb_core/src/replay/ops/tests.rs:1159 hash=4f4bdb6edc7384b5dd803cf6cb758336463f43b2bd7f3cf843be5cde364f702e tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","Internal","{","..","}",")",")",",","\"loading out-of-bounds constant must fail\"",")"]; exact destinations=vb_core::replay::ops::tests::eval_load_const_out_of_bounds_returns_error at crates/vb_core/src/replay/ops/tests_expr.rs:176.
- Assertion 17 vb_core::replay::ops::tests::eval_load_accessor_out_of_bounds_returns_error at crates/vb_core/src/replay/ops/tests.rs:1350 hash=c2ebe87ec600246fcb2e5de5651dc6ea20dae7d3686cfcc30cff5e1eae44cc2c tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","Internal","{","..","}",")",")",",","\"out-of-bounds accessor must fail\"",")"]; exact destinations=vb_core::replay::ops::tests::eval_load_accessor_out_of_bounds_returns_error at crates/vb_core/src/replay/ops/tests_expr.rs:367.
- Assertion 18 vb_core::replay::ops::tests::eval_helper_op_with_wrong_types_returns_eval_failed at crates/vb_core/src/replay/ops/tests.rs:1377 hash=f35d9893ed51d47451391ab10f1b23fcd000347926066f030a8c84489bc1534e tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","..","}",")",")",",","\"Contains on non-symbols must fail evaluation, not panic\"",")"]; exact destinations=vb_core::replay::ops::tests::eval_helper_op_with_wrong_types_returns_eval_failed at crates/vb_core/src/replay/ops/tests_expr.rs:394.
- Assertion 19 vb_core::replay::ops::tests::eval_expr_for_replay_with_uninitialized_slot at crates/vb_core/src/replay/ops/tests.rs:1517 hash=79a758414b15598d11d944155ba12e7b200a4c4a6a9d8b639e4ccc03ff7d2690 tokens=["assert","!","(","result",".","is_err","(",")",",","\"expression loading uninitialized slot must fail\"",")"]; exact destinations=vb_core::replay::ops::tests::eval_expr_for_replay_with_uninitialized_slot at crates/vb_core/src/replay/ops/tests_expr.rs:534.
- Assertion 20 vb_core::replay::ops::tests::eval_expr_for_replay_out_of_bounds_expr at crates/vb_core/src/replay/ops/tests.rs:1531 hash=ad1056bf11b7b813fbd53a26b1d402da0f852fc6bc7bf248f117f358633b0d43 tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","Internal","{","..","}",")",")",",","\"out-of-bounds expression must fail\"",")"]; exact destinations=vb_core::replay::ops::tests::eval_expr_for_replay_out_of_bounds_expr at crates/vb_core/src/replay/ops/tests_expr.rs:548.
- Assertion 21 vb_core::replay::ops::tests::blackhat_eval_div_min_by_neg_one_returns_error at crates/vb_core/src/replay/ops/tests.rs:1560 hash=8f90bb3b6c1373585c2e2fc9e7bde33753598c062e1dd3828b561a7e236c653d tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","..","}",")",")",",","\"BLACKHAT BH-OPS-01: i64::MIN / -1 must overflow\"",")"]; exact destinations=vb_core::replay::ops::tests::blackhat_eval_div_min_by_neg_one_returns_error at crates/vb_core/src/replay/ops/tests_expr.rs:577.
- Assertion 22 vb_core::replay::ops::tests::blackhat_eval_sub_min_by_one_returns_error at crates/vb_core/src/replay/ops/tests.rs:1582 hash=efb84cf188fd948345cc87fa333abe1604c3b7b74b0732a55189de8677c2d2a5 tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","..","}",")",")",",","\"BLACKHAT BH-OPS-02: i64::MIN - 1 must underflow\"",")"]; exact destinations=vb_core::replay::ops::tests::blackhat_eval_sub_min_by_one_returns_error at crates/vb_core/src/replay/ops/tests_expr.rs:599.
- Assertion 23 vb_core::replay::ops::tests::blackhat_eval_add_max_plus_one_returns_error at crates/vb_core/src/replay/ops/tests.rs:1604 hash=5aca4aec40d562d05f979c558e7a0f8db20afe2123bf4e4aa141cd5552638dcd tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","..","}",")",")",",","\"BLACKHAT BH-OPS-03: i64::MAX + 1 must overflow\"",")"]; exact destinations=vb_core::replay::ops::tests::blackhat_eval_add_max_plus_one_returns_error at crates/vb_core/src/replay/ops/tests_expr.rs:621.
- Assertion 24 vb_core::replay::ops::tests::blackhat_eval_mul_boundary_overflow at crates/vb_core/src/replay/ops/tests.rs:1626 hash=33074e6fbca623b6b46eae4ce32d9b2fc9d2d9a5c2a1a7ccb3f8c2bfb21a7627 tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","..","}",")",")",",","\"BLACKHAT BH-OPS-04: i64::MIN * 2 must overflow\"",")"]; exact destinations=vb_core::replay::ops::tests::blackhat_eval_mul_boundary_overflow at crates/vb_core/src/replay/ops/tests_expr.rs:643.
- Assertion 25 vb_core::replay::ops::tests::blackhat_taint_joins_across_expression at crates/vb_core/src/replay/ops/tests.rs:1676 hash=a4abaf91326d8197d0affe55c37b7d1d941c3ecc0743408bf652438f99eef50e tokens=["assert_eq","!","(","value",",","SlotValue","::","I64","(","30",")",",","\"BLACKHAT BH-OPS-05: value must be 30\"",")"]; exact destinations=vb_core::replay::ops::tests::blackhat_taint_joins_across_expression at crates/vb_core/src/replay/ops/tests_expr.rs:693.
- Assertion 26 vb_core::replay::ops::tests::blackhat_taint_joins_across_expression at crates/vb_core/src/replay/ops/tests.rs:1681 hash=eedc372180e121045fb24951206d57aec1c1f930f92d0ebda0854fd5d8338041 tokens=["assert_eq","!","(","taint",",","Taint","::","Secret",",","\"BLACKHAT BH-OPS-05: taint must be Secret when one operand is Secret\"",")"]; exact destinations=vb_core::replay::ops::tests::blackhat_taint_joins_across_expression at crates/vb_core/src/replay/ops/tests_expr.rs:698.
- Assertion 27 vb_core::replay::ops::tests::blackhat_expression_stack_overflow_is_safe at crates/vb_core/src/replay/ops/tests.rs:1699 hash=65756abfb42a4875f4f807daf4f3e5d6a0705fc555b3d221d4acefa9824be600 tokens=["assert","!","(","result",".","is_err","(",")",",","\"BLACKHAT BH-OPS-06: stack overflow must return error, not corrupt\"",")"]; exact destinations=vb_core::replay::ops::tests::blackhat_expression_stack_overflow_is_safe at crates/vb_core/src/replay/ops/tests_expr.rs:716.
- Assertion 28 vb_core::replay::ops::tests::blackhat_expression_stack_overflow_is_safe at crates/vb_core/src/replay/ops/tests.rs:1706 hash=995dee1709f62263e6de38a5b4a49e2c33c4097761bdeb6dcf687444f1c61b49 tokens=["assert_eq","!","(","first",",","SlotValue","::","I64","(","2",")",",","\"BLACKHAT BH-OPS-06: stack must remain consistent after overflow\"",")"]; exact destinations=vb_core::replay::ops::tests::blackhat_expression_stack_overflow_is_safe at crates/vb_core/src/replay/ops/tests_expr.rs:723.
- Assertion 29 vb_core::replay::ops::tests::check_helper_type_error at crates/vb_core/src/replay/ops/tests.rs:1747 hash=c5b5752bb5d49595816913856ad5947b711d10492aa1de46b31515a5a0728b0a tokens=["assert","!","(","matches","!","(","result",",","Err","(","ReplayError","::","ExpressionEvalFailed","{","..","}",")",")",",","\"BLACKHAT BH-OPS-07: {op:?} on wrong types must fail evaluation, not panic\"",")"]; exact destinations=vb_core::replay::ops::tests::check_helper_type_error at crates/vb_core/src/replay/ops/tests_expr.rs:764.

## record source binding

Old crates/workspace_tests/tests/vb_eepg_bdd_tests.rs blob 30ddc9bf8b272c6577d711c4f87fb87f728ca33a; current blob 34db544b1bea3c12dda0b2865a80e5bafedc6dae. Destination bindings: crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:34db544b1bea3c12dda0b2865a80e5bafedc6dae; crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs:6cf208709abe7a783e3878b6bc19e16eeada41ee; crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:2966bfa94a849ea7bfde6ab2761be74429acfc77; crates/workspace_tests/tests/vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs:0bcf14f4224e74195e1f33b81447f314c468aaca; crates/workspace_tests/tests/vb_eepg_bdd_tests/unknown_record_kind_rejection_tests.rs:c05531ea704c28bef557cf21f8ccee69aca9aea3; crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs:079d45b3d5928e685ff3e0bb63ae55535c087f3d; crates/workspace_tests/tests/vb_eepg_bdd_tests/storage_key_family_separation_tests.rs:cad111e13010d0d9d6684ce30e40d5246dd06bae; crates/workspace_tests/tests/vb_eepg_bdd_tests/min_max_numeric_id_roundtrip_tests.rs:cc60e2beb7ab821c365efdaaab7cd79a5ee567d0.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_preserves_run_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:66 tokens=6eedc84ab0d3d154e36a40e0c7bf63dced72a72d1893ed0d88387d66017c9051 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_preserves_run_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs:4 tokens=6eedc84ab0d3d154e36a40e0c7bf63dced72a72d1893ed0d88387d66017c9051; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_prefix_is_0x10 at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:92 tokens=e648d7d0facdd0cb2ca244a3b135599ce05ce59d049a94ec13063f22e78cd6b6 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_prefix_is_0x10 at crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs:30 tokens=e648d7d0facdd0cb2ca244a3b135599ce05ce59d049a94ec13063f22e78cd6b6; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_zero_run_id at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:110 tokens=942f2ece6ef345b4a366b0d6c94fd716e8faadd5a0fd6a8baf3d5b119b36731e -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_zero_run_id at crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs:48 tokens=942f2ece6ef345b4a366b0d6c94fd716e8faadd5a0fd6a8baf3d5b119b36731e; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_max_run_id at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:133 tokens=3fa6383d5244f0e4db3217c9c0c386dcb9b441d0167e1ce944b328f78770a925 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_max_run_id at crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs:71 tokens=3fa6383d5244f0e4db3217c9c0c386dcb9b441d0167e1ce944b328f78770a925; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_preserves_workflow_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:158 tokens=eb56f039fc4c36621534650fc5adfa5be878fd89e8362a09716091929ed46c71 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_preserves_workflow_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:4 tokens=eb56f039fc4c36621534650fc5adfa5be878fd89e8362a09716091929ed46c71; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_lexicographic_ordering at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:192 tokens=7ee0312645a87c05d91a3b5f9c5ea165431780fff8468d3d90e3e0b194966e1f -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_lexicographic_ordering at crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:38 tokens=7ee0312645a87c05d91a3b5f9c5ea165431780fff8468d3d90e3e0b194966e1f; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_zero_values at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:211 tokens=5039ee048050f675083318987c25b6dbdc62e03742ed1ac484a611eace3105a5 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_zero_values at crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:57 tokens=5039ee048050f675083318987c25b6dbdc62e03742ed1ac484a611eace3105a5; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_zero_workflow_id at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:238 tokens=50e4fed365bd51bf1a1798ef0357a7353535b93086fe55d2fcbc07320be6b3ec -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_zero_workflow_id at crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:84 tokens=50e4fed365bd51bf1a1798ef0357a7353535b93086fe55d2fcbc07320be6b3ec; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:267 tokens=c0c1b949c55d18274471fc27993401c062a2e4b3557dca1c4d25073bf65d77af -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs:4 tokens=c0c1b949c55d18274471fc27993401c062a2e4b3557dca1c4d25073bf65d77af; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_max_values at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:309 tokens=729f8d3f351d651741742d650b341f091bc294f8430827ed3beca19d8d013957 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_max_values at crates/workspace_tests/tests/vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs:46 tokens=729f8d3f351d651741742d650b341f091bc294f8430827ed3beca19d8d013957; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::seqno_overflow_rejection::seqno_overflow_returns_none at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:346 tokens=e1db8a72e2b9cfcaa1ccb7c6e25fdc5ff91c40d365d018e59e0bfd7639f0efd2 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::seqno_overflow_rejection::seqno_overflow_returns_none at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:87 tokens=e1db8a72e2b9cfcaa1ccb7c6e25fdc5ff91c40d365d018e59e0bfd7639f0efd2; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::seqno_overflow_rejection::seqno_add_exact_max_succeeds at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:358 tokens=742a63f6ac0a4957f4a1ffb6c2b0f62046336676eb85047a94c44e8c2427b21b -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::seqno_overflow_rejection::seqno_add_exact_max_succeeds at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:99 tokens=742a63f6ac0a4957f4a1ffb6c2b0f62046336676eb85047a94c44e8c2427b21b; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::seqno_overflow_rejection::seqno_normal_addition_succeeds at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:374 tokens=aac4af4f9f37a1f107355819e78a3c3dc866589be6db50326d77067af0d99412 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::seqno_overflow_rejection::seqno_normal_addition_succeeds at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:115 tokens=aac4af4f9f37a1f107355819e78a3c3dc866589be6db50326d77067af0d99412; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::seqno_overflow_rejection::seqno_checked_add_associativity at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:386 tokens=f427e3bb53ab6b45421a84e6f42b4a359624adb5f05d6afc47e342c23a4e1833 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::seqno_overflow_rejection::seqno_checked_add_associativity at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:127 tokens=f427e3bb53ab6b45421a84e6f42b4a359624adb5f05d6afc47e342c23a4e1833; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::decode_rejects_unknown_record_kind at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:410 tokens=1048cae6e8080ead06080211323741cf7646fc5e29e204fed132a071596caf8a -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::decode_rejects_unknown_record_kind at crates/workspace_tests/tests/vb_eepg_bdd_tests/unknown_record_kind_rejection_tests.rs:32 tokens=1048cae6e8080ead06080211323741cf7646fc5e29e204fed132a071596caf8a; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::decode_accepts_all_known_journal_event_kinds at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:433 tokens=cd2f7df8e814960ab8e3a2c2309096e733a0a4941e889cf325953a7e8aeaf145 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::decode_accepts_all_known_journal_event_kinds at crates/workspace_tests/tests/vb_eepg_bdd_tests/unknown_record_kind_rejection_tests.rs:55 tokens=2033c31f7adf68d329b6a4f32b6c472d80aae72b9453e6ba6a5712865dbf85da; same_fqn=true; exact_function_tokens=false.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::is_known_record_kind_returns_true_for_valid_kinds at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:478 tokens=d5d65769fb61649e16a90c5a30c3e93d41cdbf27b28d671316564c74e912d1b0 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::is_known_record_kind_returns_true_for_valid_kinds at crates/workspace_tests/tests/vb_eepg_bdd_tests/unknown_record_kind_rejection_tests.rs:66 tokens=036541be377a8ecc0554836683240480ce0dd0f06a59e87ca78dc43a549015cb; same_fqn=true; exact_function_tokens=false.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::is_known_record_kind_returns_false_for_invalid_kinds at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:510 tokens=da95011fcfae8bf32480e6dfb976e299ed677c8107133925cf24d45ab811f20d -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::is_known_record_kind_returns_false_for_invalid_kinds at crates/workspace_tests/tests/vb_eepg_bdd_tests/unknown_record_kind_rejection_tests.rs:87 tokens=17d4d301d052f3b6cf34d8d22bb590224a36f687b864bed7f711682e1b0fa5dc; same_fqn=true; exact_function_tokens=false.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_is_stable at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:575 tokens=fa27c01535a5f5f0c0260655cca412e0d1371268c20c40a6d267eb3b8bebbdef -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_is_stable at crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs:4 tokens=fa27c01535a5f5f0c0260655cca412e0d1371268c20c40a6d267eb3b8bebbdef; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:631 tokens=e9593b13e5b650e8efcc7353954bfbf47f286946a0c87a0fd3b6f2d9f316610e -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs:60 tokens=e9593b13e5b650e8efcc7353954bfbf47f286946a0c87a0fd3b6f2d9f316610e; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::all_key_prefixes_are_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:694 tokens=97ff91119aba062f9a3101b565638cf0e3c736d4103ed5951a47b7f3b7965a78 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::all_key_prefixes_are_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests/storage_key_family_separation_tests.rs:4 tokens=97ff91119aba062f9a3101b565638cf0e3c736d4103ed5951a47b7f3b7965a78; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::run_family_prefixes_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:730 tokens=c13259f87acc00f52deab9642f978f4562ec826a720b6a8499f4e63423031ba2 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::run_family_prefixes_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests/storage_key_family_separation_tests.rs:40 tokens=c13259f87acc00f52deab9642f978f4562ec826a720b6a8499f4e63423031ba2; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::run_id_zero_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:769 tokens=7b0510bec075583ece69531f448b0e97302b4f72117f94328edb0e0e46406759 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::run_id_zero_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests/min_max_numeric_id_roundtrip_tests.rs:4 tokens=7b0510bec075583ece69531f448b0e97302b4f72117f94328edb0e0e46406759; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::run_id_nonzero_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:793 tokens=002321f925f39d2080b0c1db527a1e1fc13ae746ee532e530ddbfa56a8b6cfab -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::run_id_nonzero_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests/min_max_numeric_id_roundtrip_tests.rs:28 tokens=002321f925f39d2080b0c1db527a1e1fc13ae746ee532e530ddbfa56a8b6cfab; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::run_id_max_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:804 tokens=3b313763ff3eee0e704960caa7acb3db324b254460cbbf71d72b308a3504c837 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::run_id_max_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests/min_max_numeric_id_roundtrip_tests.rs:39 tokens=3b313763ff3eee0e704960caa7acb3db324b254460cbbf71d72b308a3504c837; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::workflow_id_extents_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:818 tokens=f10b0622bdddacd2f0e1eec8c9f959a3cf97adc2f43d5f051d1f671af5bf5042 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::workflow_id_extents_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests/min_max_numeric_id_roundtrip_tests.rs:53 tokens=f10b0622bdddacd2f0e1eec8c9f959a3cf97adc2f43d5f051d1f671af5bf5042; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::integration_full_roundtrip::full_run_id_to_key_to_decode_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:852 tokens=713cb19036827354c8d05ac174eeb5b2ba26a453cf8492a04b81eaf939c865f4 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::integration_full_roundtrip::full_run_id_to_key_to_decode_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:179 tokens=713cb19036827354c8d05ac174eeb5b2ba26a453cf8492a04b81eaf939c865f4; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::integration_full_roundtrip::full_workflow_id_index_key_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:866 tokens=871b94b6c2cec98de50dab7165a08d165d10a078a2cfb2d4788dfe2552914349 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::integration_full_roundtrip::full_workflow_id_index_key_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:193 tokens=871b94b6c2cec98de50dab7165a08d165d10a078a2cfb2d4788dfe2552914349; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_max_u64_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:892 tokens=d12b8575656a73887763c0c5cb69a56946e148e03d58caa14cabc26b217948f0 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_max_u64_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:219 tokens=d12b8575656a73887763c0c5cb69a56946e148e03d58caa14cabc26b217948f0; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_zero_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:917 tokens=96011149e03c13b3edc0c43c611773e802a4d9fb078eb74c5331337d46b883c3 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_zero_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:244 tokens=96011149e03c13b3edc0c43c611773e802a4d9fb078eb74c5331337d46b883c3; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_rejects_empty at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:930 tokens=e54ea5addd3e2eccf5084259bfe84081e75ca5a82e4318312e2eb5c0c6d8efc3 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_rejects_empty at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:257 tokens=e54ea5addd3e2eccf5084259bfe84081e75ca5a82e4318312e2eb5c0c6d8efc3; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_rejects_negative at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:942 tokens=84f66e01d818e2e94858752ac20bc8615fa78ef60c4283f8caf487733083b568 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_rejects_negative at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:269 tokens=84f66e01d818e2e94858752ac20bc8615fa78ef60c4283f8caf487733083b568; same_fqn=true; exact_function_tokens=true.
- Case velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_rejects_overflow at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:954 tokens=639927867b72d53d618e8635261fc88103d0c3a022f72c36576a2659668e3e02 -> velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_rejects_overflow at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:281 tokens=639927867b72d53d618e8635261fc88103d0c3a022f72c36576a2659668e3e02; same_fqn=true; exact_function_tokens=true.
- Assertion 1 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_preserves_run_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:74 hash=b8ff1106ef9efc79aebbfc58266432e894ff981fa01318a6da8d0e855b900bf1 tokens=["assert_eq","!","(","key","[","0","]",",","vb_storage","::","constants","::","PREFIX_RUN_HEADER",",","\"prefix must be 0x10\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_preserves_run_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs:12.
- Assertion 2 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_preserves_run_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:79 hash=3dca025afecfc85515ee338d089b8f245a3ae42dc46319dfb28cbe6659dfd09f tokens=["assert_eq","!","(","&","key","[","1","..","9","]",",","run_id",".","get","(",")",".","to_be_bytes","(",")",",","\"run_id bytes must be preserved in big-endian\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_preserves_run_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs:17.
- Assertion 3 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_preserves_run_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:87 hash=944248ccd4a9d4f45205153f1043c5246de943cfe4ac2c3fd5e5c4ddcabac09f tokens=["assert_eq","!","(","recovered_run_id",",","run_id",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_preserves_run_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs:25; velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_preserves_workflow_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:33; velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs:40.
- Assertion 4 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_prefix_is_0x10 at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:100 hash=fe5117f7fd1e2354457ccf7083c8558f6dca8215a8d3af09d57635a79969c695 tokens=["assert_eq","!","(","key","[","0","]",",","0x10",",","\"prefix byte must be 0x10\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_prefix_is_0x10 at crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs:38.
- Assertion 5 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_prefix_is_0x10 at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:101 hash=74a33bf9cb1fde0d705a81fdcd1faef6b31b0149176632711f263e0d6ab397ba tokens=["assert_eq","!","(","key","[","0","]",",","vb_storage","::","constants","::","PREFIX_RUN_HEADER",",","\"prefix must match constant\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_prefix_is_0x10 at crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs:39.
- Assertion 6 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_zero_run_id at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:122 hash=d44a797882b500da0b7c7c7da729412b7a994de90f532785439b945fb0aeaf93 tokens=["assert_eq","!","(","run",",","RunId","::","new","(","0",")",",","\"the run must round-trip back as zero\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_zero_run_id at crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs:60; velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_zero_values at crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:73.
- Assertion 7 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_max_run_id at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:141 hash=948e753c9304f063af430fd1b0c20b3d04b2899e2b6cb6a263243b8947555531 tokens=["assert_eq","!","(","&","key","[","1","..","9","]",",","u64","::","MAX",".","to_be_bytes","(",")",",","\"max run_id must encode as all-0xFF bytes\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_max_run_id at crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs:79.
- Assertion 8 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_preserves_workflow_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:167 hash=c9e457ea3f889bf2000d434340a6473a694f5d47a10bd22cb506e1b57d637f5c tokens=["assert_eq","!","(","key","[","0","]",",","vb_storage","::","constants","::","PREFIX_INDEX_WORKFLOW",",","\"prefix must be 0x31\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_preserves_workflow_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:13.
- Assertion 9 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_preserves_workflow_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:172 hash=731d7532bf4cd3ca8e039eb74379dba1bcf5e15d79271010cea7df18d8cd2160 tokens=["assert_eq","!","(","&","key","[","1","..","5","]",",","workflow_id",".","get","(",")",".","to_be_bytes","(",")",",","\"workflow_id bytes must be preserved in big-endian\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_preserves_workflow_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:18.
- Assertion 10 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_preserves_workflow_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:177 hash=dfb0a7731e3c9541c42e57ed77b93c10410057151c7501b6196eef6b971de199 tokens=["assert_eq","!","(","&","key","[","5","..","13","]",",","run_id",".","get","(",")",".","to_be_bytes","(",")",",","\"run_id bytes must be preserved in big-endian\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_preserves_workflow_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:23.
- Assertion 11 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_preserves_workflow_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:186 hash=8795184d6a4399c69bddc9e4704d112554e29d631563432b889487db0a1748b9 tokens=["assert_eq","!","(","recovered_workflow",",","workflow_id",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_preserves_workflow_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:32.
- Assertion 12 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_preserves_workflow_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:187 hash=944248ccd4a9d4f45205153f1043c5246de943cfe4ac2c3fd5e5c4ddcabac09f tokens=["assert_eq","!","(","recovered_run_id",",","run_id",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_preserves_run_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs:25; velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_preserves_workflow_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:33; velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs:40.
- Assertion 13 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_lexicographic_ordering at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:203 hash=7bf03264234f833e304966f23b9415f2425f9dfcdfe1f6ec28faf58c79a06c67 tokens=["assert","!","(","key1","<","key2",",","\"key(w1) must be less than key(w2) when w1 < w2\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_lexicographic_ordering at crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:49.
- Assertion 14 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_zero_values at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:227 hash=d44a797882b500da0b7c7c7da729412b7a994de90f532785439b945fb0aeaf93 tokens=["assert_eq","!","(","run",",","RunId","::","new","(","0",")",",","\"the run must round-trip back as zero\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_zero_run_id at crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs:60; velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_zero_values at crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:73.
- Assertion 15 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_zero_workflow_id at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:244 hash=e10d601ba264b6781866e6f86b3c5dee0cf54c0f97865f253f5c8016077584f6 tokens=["assert_eq","!","(","&","key","[","1","..","5","]",",","0u32",".","to_be_bytes","(",")",",","\"zero workflow_id must encode as all-zero bytes\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_zero_workflow_id at crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:90.
- Assertion 16 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_zero_workflow_id at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:249 hash=a79ea5105875c651b4bbc400664fa6e521f5718b70ac4efcf59c43f5f13e2e7f tokens=["assert_eq","!","(","&","key","[","5","..","13","]",",","1u64",".","to_be_bytes","(",")",",","\"non-zero run_id bytes must encode as big-endian u64\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_zero_workflow_id at crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:95.
- Assertion 17 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_zero_workflow_id at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:254 hash=a17212bd8a55af65ba3dd0a52f8e7fb0ab6345f8651d1fa202e02296466353d9 tokens=["assert_eq","!","(","key",".","len","(",")",",","13",",","\"index_workflow_key must be 13 bytes\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_zero_workflow_id at crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:100.
- Assertion 18 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:277 hash=a56647c118248a99130299e13580283035fbb13825c8a4833dffe3fbe1a17133 tokens=["assert_eq","!","(","key","[","0","]",",","vb_storage","::","constants","::","PREFIX_INDEX_ACTION",",","\"prefix must be 0x32\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs:14.
- Assertion 19 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:282 hash=f039a257edb8eee516cb87a1d258c51630003b46ac34880b851e68f977cffc7b tokens=["assert_eq","!","(","&","key","[","1","..","3","]",",","action_id",".","get","(",")",".","to_be_bytes","(",")",",","\"action_id bytes must be preserved\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs:19.
- Assertion 20 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:287 hash=36fad67a31e39a4632c75495016f22ff0e7bd7a89e9636b2ebc2b47aabf3fc24 tokens=["assert_eq","!","(","&","key","[","3","..","11","]",",","run_id",".","get","(",")",".","to_be_bytes","(",")",",","\"run_id bytes must be preserved\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs:24.
- Assertion 21 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:292 hash=b29b4bd6efd42a0d18c8117098c0318602c328cbd29b0aa0575e406089eafa05 tokens=["assert_eq","!","(","&","key","[","11","..","13","]",",","step_idx",".","get","(",")",".","to_be_bytes","(",")",",","\"step_idx bytes must be preserved\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs:29.
- Assertion 22 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:302 hash=a4c06cbf7c30e9d95147f947a4564453ef8409d91c45fe36bdbbacaf13e9f31b tokens=["assert_eq","!","(","recovered_action",",","action_id",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs:39.
- Assertion 23 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:303 hash=944248ccd4a9d4f45205153f1043c5246de943cfe4ac2c3fd5e5c4ddcabac09f tokens=["assert_eq","!","(","recovered_run_id",",","run_id",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::run_id_roundtrip::run_header_key_preserves_run_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests/run_id_roundtrip_tests.rs:25; velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::workflow_id_roundtrip::index_workflow_key_preserves_workflow_id_bytes at crates/workspace_tests/tests/vb_eepg_bdd_tests/workflow_id_roundtrip_tests.rs:33; velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs:40.
- Assertion 24 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:304 hash=b581255e3d3ac9e056f78345aca24989b829f2b41faf2f147f16114746f1d0f8 tokens=["assert_eq","!","(","recovered_step",",","step_idx",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_preserves_all_fields at crates/workspace_tests/tests/vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs:41.
- Assertion 25 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_max_values at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:319 hash=aa5f6553721da351bbafd743363c3bd73523e5b48b01c39cb847e2b83bed0f88 tokens=["assert_eq","!","(","&","key","[","1","..","3","]",",","u16","::","MAX",".","to_be_bytes","(",")",",","\"max action_id must encode as 0xFFFF\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_max_values at crates/workspace_tests/tests/vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs:56.
- Assertion 26 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_max_values at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:324 hash=8f9ca262320d1416d1a749ea370fd5d6d71d2836aaf2af0c00f0890019b139e3 tokens=["assert_eq","!","(","&","key","[","3","..","11","]",",","u64","::","MAX",".","to_be_bytes","(",")",",","\"max run_id must encode as all 0xFF\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_max_values at crates/workspace_tests/tests/vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs:61.
- Assertion 27 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_max_values at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:329 hash=0a4a510c3e160fec2bd4fb35eec37a71cb28c04e8e57547bef9611267761a578 tokens=["assert_eq","!","(","&","key","[","11","..","13","]",",","u16","::","MAX",".","to_be_bytes","(",")",",","\"max step_idx must encode as 0xFFFF\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::action_ticket_key_roundtrip::index_action_key_max_values at crates/workspace_tests/tests/vb_eepg_bdd_tests/action_ticket_key_roundtrip_tests.rs:66.
- Assertion 28 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::seqno_overflow_rejection::seqno_overflow_returns_none at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:354 hash=a7f99cfa18ea17a5d001f06fa0d0784bd5123e99ee91ebcaa7fc2e5c60636ab6 tokens=["assert","!","(","result",".","is_none","(",")",",","\"u64::MAX + 1 must return None (overflow)\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::seqno_overflow_rejection::seqno_overflow_returns_none at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:95.
- Assertion 29 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::seqno_overflow_rejection::seqno_add_exact_max_succeeds at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:366 hash=46c556caae7103e7aa3d399d0e1eb0bf7f7993dcbc7495f4e3d1564508cad986 tokens=["assert_eq","!","(","result",",","Some","(","SeqNo","::","new","(","u64","::","MAX",")",")",",","\"0 + u64::MAX must equal u64::MAX\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::seqno_overflow_rejection::seqno_add_exact_max_succeeds at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:107.
- Assertion 30 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::seqno_overflow_rejection::seqno_normal_addition_succeeds at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:382 hash=6e590e540d0ed4dc013aac7db8d53265deafa1e35810ac921464e13a69db8087 tokens=["assert_eq","!","(","result",",","Some","(","SeqNo","::","new","(","150",")",")",",","\"100 + 50 must equal 150\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::seqno_overflow_rejection::seqno_normal_addition_succeeds at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:123.
- Assertion 31 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::seqno_overflow_rejection::seqno_checked_add_associativity at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:395 hash=43567241075d6ccf11b488cab54fa198467fd8a115f08d21f42e161e12ef3fc6 tokens=["assert_eq","!","(","ab",",","ac",",","\"checked_add must be associative for non-overflowing values\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::seqno_overflow_rejection::seqno_checked_add_associativity at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:136.
- Assertion 32 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::decode_rejects_unknown_record_kind at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:425 hash=67485cd2a98a640beb846e000f947402d3b58d87ace832d42da137a53d9fd5db tokens=["assert_eq","!","(","kind",",","255",",","\"unknown kind must be 255\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::decode_rejects_unknown_record_kind at crates/workspace_tests/tests/vb_eepg_bdd_tests/unknown_record_kind_rejection_tests.rs:47.
- Assertion 33 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::decode_accepts_all_known_journal_event_kinds at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:450 hash=b7489519785c4493214202871cb02b827c811a8902b641901c8894e85839cac5 tokens=["assert","!","(","result",".","is_ok","(",")",",","\"known journal event kind {} must not be rejected, got {:?}\"",",","kind",",","result",")"]; EXPLICIT NONIDENTICAL REPLACEMENT: velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::exact_assert_family at crates/workspace_tests/tests/vb_eepg_bdd_tests/unknown_record_kind_rejection_tests.rs:19 hash=d7ec306057c457616bf8ab56ae3abe1830cfe0bfb7d09eccdd84a78a961f0d1c tokens=["assert","!","(","matches","!","(","result",",","Err","(","JournalError","::","RecordKindFamilyMismatch","{","magic",":","vb_storage","::","constants","::","MAGIC_JOURNAL_EVENT",",","kind",":","k","}",")","if","k","==","kind",")",")"]; velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::exact_assert_family at crates/workspace_tests/tests/vb_eepg_bdd_tests/unknown_record_kind_rejection_tests.rs:27 hash=ddbe3352cc085272e8fd6913cb41b474c810631e58f17c74dfc4e2de34ced556 tokens=["assert","!","(","matches","!","(","result",",","Ok","(","decoded",")","if","decoded",".","record_kind","==","kind",")",")"]. Rationale requiring CODE/TEST: Current independent45-ID and8-nonjournal tables require exact RecordKindFamilyMismatch magic/kind or exact decoded record_kind. This strengthens the old is_ok/not-Unknown checks; it is not byte-identical relocation.
- Assertion 34 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::decode_accepts_all_known_journal_event_kinds at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:468 hash=05374eeb70581f42f8c09a438bafa9950f46d8b01317d308838b738a69805be1 tokens=["assert_eq","!","(","decoded",".","record_kind",",","38",",","\"kind 38 (PolicyDecision) must decode with record_kind 38, got {}\"",",","decoded",".","record_kind",")"]; EXPLICIT NONIDENTICAL REPLACEMENT: velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::assert_positive_kind at crates/workspace_tests/tests/vb_eepg_bdd_tests/unknown_record_kind_rejection_tests.rs:106 hash=82abea97ddc15c8d4fe9585748f2cc553b310ed1400b9b94a9eb92f890928c76 tokens=["assert_eq","!","(","decoded",".","record_kind",",","kind",",","\"kind {} must decode with record_kind {}, got {}\"",",","kind",",","kind",",","decoded",".","record_kind",")"]. Rationale requiring CODE/TEST: Generic exact decoded-kind equality is mandatory for literal [38,48,49], retaining38 and adding48/49; original literal38 message is generalized.
- Assertion 35 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::is_known_record_kind_returns_true_for_valid_kinds at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:500 hash=10ee8dd37a2d3be35e7de52dd75d0e68e73643f219bf4e584ec3adddb32f1d02 tokens=["assert","!","(","!","matches","!","(","result",",","Err","(","JournalError","::","UnknownRecordKind","{","..","}",")",")",",","\"kind {} must be known (is_known_record_kind returned true), got {:?}\"",",","kind",",","result",")"]; EXPLICIT NONIDENTICAL REPLACEMENT: velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::exact_assert_family at crates/workspace_tests/tests/vb_eepg_bdd_tests/unknown_record_kind_rejection_tests.rs:19 hash=d7ec306057c457616bf8ab56ae3abe1830cfe0bfb7d09eccdd84a78a961f0d1c tokens=["assert","!","(","matches","!","(","result",",","Err","(","JournalError","::","RecordKindFamilyMismatch","{","magic",":","vb_storage","::","constants","::","MAGIC_JOURNAL_EVENT",",","kind",":","k","}",")","if","k","==","kind",")",")"]; velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::exact_assert_family at crates/workspace_tests/tests/vb_eepg_bdd_tests/unknown_record_kind_rejection_tests.rs:27 hash=ddbe3352cc085272e8fd6913cb41b474c810631e58f17c74dfc4e2de34ced556 tokens=["assert","!","(","matches","!","(","result",",","Ok","(","decoded",")","if","decoded",".","record_kind","==","kind",")",")"]. Rationale requiring CODE/TEST: Current independent45-ID and8-nonjournal tables require exact RecordKindFamilyMismatch magic/kind or exact decoded record_kind. This strengthens the old is_ok/not-Unknown checks; it is not byte-identical relocation.
- Assertion 36 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::is_known_record_kind_returns_false_for_invalid_kinds at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:532 hash=a9f7a17f2e82d842bee4e5b14d679bc7b31707cbfbc214d2e041ddce35571bee tokens=["assert","!","(","matches","!","(","result",",","Err","(","JournalError","::","UnknownRecordKind","{","kind",":","k","}",")","if","k","==","kind",")",",","\"kind {} must be unknown, got {:?}\"",",","kind",",","result",")"]; EXPLICIT NONIDENTICAL REPLACEMENT: velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::assert_exact_unknown at crates/workspace_tests/tests/vb_eepg_bdd_tests/unknown_record_kind_rejection_tests.rs:125 hash=d5a572c72fa82821ceaafb3428508b1da87bb3fb22682e9c059abee55c8a0c3f tokens=["assert","!","(","matches","!","(","result",",","Err","(","JournalError","::","UnknownRecordKind","{","kind",":","k","}",")","if","k","==","kind",")",",","\"unknown kind {} must produce UnknownRecordKind with exact echoed ID, got {:?}\"",",","kind",",","result",")"]. Rationale requiring CODE/TEST: Same UnknownRecordKind echoed-ID predicate with explicit expanded error message is called from complete u16 traversal; current known48/49 are separated from the obsolete old unknown48/49 samples. This is an explicit contract adaptation, not identical old inputs/messages.
- Assertion 37 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::is_known_record_kind_returns_false_for_invalid_kinds at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:553 hash=2f6825008864cf197b775de49e7d1ee3a79da991b820ce05466505fd47313433 tokens=["assert","!","(","matches","!","(","result",",","Err","(","JournalError","::","RecordKindFamilyMismatch","{","magic",":","vb_storage","::","constants","::","MAGIC_JOURNAL_EVENT",",","kind",":","4","}",")",")",",","\"kind 4 with journal magic must produce RecordKindFamilyMismatch, got {:?}\"",",","result",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::unknown_record_kind_rejection::assert_kind_four_family at crates/workspace_tests/tests/vb_eepg_bdd_tests/unknown_record_kind_rejection_tests.rs:157.
- Assertion 38 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_is_stable at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:614 hash=a10a9b492b0e9f498abec722fb158b55c9a2d4a8abba7a03458625ab7d27d0da tokens=["assert_eq","!","(","ids",",","ids_again",",","\"RecordKind.id() must be stable across invocations\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_is_stable at crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs:43.
- Assertion 39 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_is_stable at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:623 hash=a3b49e538db9591f322dda6ad0de9dc009ddcd43c1cb31179d29f380da764c87 tokens=["assert_eq","!","(","sorted_ids",".","len","(",")",",","ids",".","len","(",")",",","\"all RecordKind IDs must be unique\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_is_stable at crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs:52.
- Assertion 40 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:639 hash=e0f23ada6c410368225f98d35284a3d9cd2e9df7d3ff61f7670183660fe678a3 tokens=["assert_eq","!","(","id",",","1",",","\"WorkflowSource.id() must equal 1\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs:68.
- Assertion 41 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:640 hash=c4410af624f24f6b2f878a4af7d35422fe52771cb70fd2128136bb91b303265f tokens=["assert_eq","!","(","RecordKind","::","CompiledIr",".","id","(",")",",","2",",","\"CompiledIr.id() must equal 2\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs:69.
- Assertion 42 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:645 hash=725ec1980191f9d094728a2fe4db153517aa1a3fcc072701da244c488e312baa tokens=["assert_eq","!","(","RecordKind","::","RunHeader",".","id","(",")",",","3",",","\"RunHeader.id() must equal 3\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs:74.
- Assertion 43 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:646 hash=7d240ecd2e92729a5e223aa3d0ada9e5a928bcafe2a3da9658572513fb286da2 tokens=["assert_eq","!","(","RecordKind","::","RunAccepted",".","id","(",")",",","10",",","\"RunAccepted.id() must equal 10\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs:75.
- Assertion 44 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:651 hash=9143b8a93f5c676a0e4914674f434a140f2100ee6101c1a1e6e234218cda50b5 tokens=["assert_eq","!","(","RecordKind","::","RunAnswered",".","id","(",")",",","27",",","\"RunAnswered.id() must equal 27\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs:80.
- Assertion 45 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:656 hash=e08af752729da7bf0d8a8ea834d58012a7fd59db70b2623277860b4a124df11e tokens=["assert_eq","!","(","RecordKind","::","RunKilled",".","id","(",")",",","28",",","\"RunKilled.id() must equal 28\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs:85.
- Assertion 46 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:661 hash=cc09c9b953247d037c1865de9d3a7170c554e4fe635a0b6be9ad7e0f33b481d7 tokens=["assert_eq","!","(","RecordKind","::","AskTimedOut",".","id","(",")",",","29",",","\"AskTimedOut.id() must equal 29\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs:90.
- Assertion 47 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:666 hash=7401a5b5aa7842ee95682b097022efb8a9ba2eeb0fde9304bd9d2678d1906ba9 tokens=["assert_eq","!","(","RecordKind","::","Snapshot",".","id","(",")",",","30",",","\"Snapshot.id() must equal 30\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs:95.
- Assertion 48 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:667 hash=0418ece401914ecb75a42c7fb7e261874110311af13e670fb5ca3c131cc5e326 tokens=["assert_eq","!","(","RecordKind","::","WaitResolved",".","id","(",")",",","31",",","\"WaitResolved.id() must equal 31\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs:96.
- Assertion 49 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:672 hash=074d79d7f45c21ea3ca45dd363d8a52b21757ff4b743396e836770bb8d810a65 tokens=["assert_eq","!","(","RecordKind","::","ActionAbandoned",".","id","(",")",",","32",",","\"ActionAbandoned.id() must equal 32\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs:101.
- Assertion 50 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:677 hash=39e8e7d93186b51d158de4470ecd1f333b04e8bfd3010c6011a9f09cbe20f19b tokens=["assert_eq","!","(","RecordKind","::","Blob",".","id","(",")",",","40",",","\"Blob.id() must equal 40\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs:106.
- Assertion 51 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:678 hash=b16fd77f4fdb04a043c80fe6b9b1867ab17adaaf54efd394f4ee31574a286132 tokens=["assert_eq","!","(","RecordKind","::","IndexUpdate",".","id","(",")",",","50",",","\"IndexUpdate.id() must equal 50\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::record_kind_stable_ids::record_kind_id_values_match_constants at crates/workspace_tests/tests/vb_eepg_bdd_tests/record_kind_stable_ids_tests.rs:107.
- Assertion 52 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::all_key_prefixes_are_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:719 hash=d380c4a6262e67ee3cf8349db2736d009d479afc6face2051e1ec87e19a741c3 tokens=["assert_ne","!","(","prefixes","[","i","]",",","prefixes","[","j","]",",","\"prefix at index {} ({:#04x}) must differ from prefix at index {} ({:#04x})\"",",","i",",","prefixes","[","i","]",",","j",",","prefixes","[","j","]",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::all_key_prefixes_are_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests/storage_key_family_separation_tests.rs:29.
- Assertion 53 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::run_family_prefixes_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:742 hash=419649bdddfeac14a261e1c28fb42d94438bf8f20fea9090618edd81c3ac1e48 tokens=["assert_eq","!","(","header_prefix",",","0x10",",","\"run_header prefix must be 0x10\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::run_family_prefixes_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests/storage_key_family_separation_tests.rs:52.
- Assertion 54 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::run_family_prefixes_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:743 hash=9905ed073fe1e31f363e9665da0720947a335afb00200ebc5303e2117caaaa12 tokens=["assert_eq","!","(","event_prefix",",","0x11",",","\"run_event prefix must be 0x11\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::run_family_prefixes_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests/storage_key_family_separation_tests.rs:53.
- Assertion 55 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::run_family_prefixes_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:744 hash=03aacbfcd324531ee9449b5045779b525da94d97222551b8c9972669d4ebc7fb tokens=["assert_eq","!","(","snapshot_prefix",",","0x12",",","\"run_snapshot prefix must be 0x12\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::run_family_prefixes_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests/storage_key_family_separation_tests.rs:54.
- Assertion 56 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::run_family_prefixes_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:745 hash=8e7698c19b41ecc8b18f389d8bac380ff780c2b9caae54affd7fab0c64126a01 tokens=["assert_ne","!","(","header_prefix",",","event_prefix",",","\"header and event prefixes must differ\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::run_family_prefixes_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests/storage_key_family_separation_tests.rs:55.
- Assertion 57 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::run_family_prefixes_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:749 hash=a687f70866a479c8f0ce0eb384a5908ce960172402ceebe359032e160137d355 tokens=["assert_ne","!","(","header_prefix",",","snapshot_prefix",",","\"header and snapshot prefixes must differ\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::run_family_prefixes_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests/storage_key_family_separation_tests.rs:59.
- Assertion 58 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::run_family_prefixes_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:753 hash=9cefa1d11129ab7a1dd146b7a5bd3f2285c63897b5bf48fa18fd9b07a51ef8e1 tokens=["assert_ne","!","(","event_prefix",",","snapshot_prefix",",","\"event and snapshot prefixes must differ\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::storage_key_family_separation::run_family_prefixes_distinct at crates/workspace_tests/tests/vb_eepg_bdd_tests/storage_key_family_separation_tests.rs:63.
- Assertion 59 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::run_id_zero_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:782 hash=1e08fc1ebad6c2faf03185fc1aed0e50c970293a426baf3a248cf4be67deac9b tokens=["assert_eq","!","(","run",",","RunId","::","new","(","0",")",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::run_id_zero_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests/min_max_numeric_id_roundtrip_tests.rs:17.
- Assertion 60 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::run_id_nonzero_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:799 hash=5fc7d328a3ded13a68f0252686d8da83702c813f3ef7e93f68222a983799d328 tokens=["assert_eq","!","(","decoded",",","run_id",",","\"non-zero RunId must roundtrip correctly\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::run_id_nonzero_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests/min_max_numeric_id_roundtrip_tests.rs:34.
- Assertion 61 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::run_id_max_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:813 hash=c3696aef0b692f35f5376462f43fe0e479ed50b1b7cfb21ffc28dc40e29f3bd1 tokens=["assert_eq","!","(","decoded",",","run_id",",","\"max RunId must roundtrip correctly\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::run_id_max_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests/min_max_numeric_id_roundtrip_tests.rs:48.
- Assertion 62 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::workflow_id_extents_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:832 hash=a0df611b914e22be36cdd07bc9cfc6c5d312dc916a1b8b481c448671fcf81200 tokens=["assert_eq","!","(","decoded_min",",","min_workflow",",","\"min WorkflowId must roundtrip correctly\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::workflow_id_extents_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests/min_max_numeric_id_roundtrip_tests.rs:67.
- Assertion 63 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::workflow_id_extents_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:836 hash=f224937e3e9a41cd26a2cc12608de078d448080183778f41db14e8c66a089266 tokens=["assert_eq","!","(","decoded_max",",","max_workflow",",","\"max WorkflowId must roundtrip correctly\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::min_max_numeric_id_roundtrip::workflow_id_extents_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests/min_max_numeric_id_roundtrip_tests.rs:71.
- Assertion 64 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::integration_full_roundtrip::full_run_id_to_key_to_decode_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:861 hash=6fbc033cabe6fe16b223682da89dd04c714e59bf0490ccbb1c2aa2b43476d520 tokens=["assert_eq","!","(","recovered",",","original",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::integration_full_roundtrip::full_run_id_to_key_to_decode_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:188.
- Assertion 65 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::integration_full_roundtrip::full_workflow_id_index_key_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:878 hash=9ddf33c060db008a31f45cb909ebbf1ed92d9b4f42bd8153eb64ce72aebbfca5 tokens=["assert_eq","!","(","extracted_workflow",",","workflow_id",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::integration_full_roundtrip::full_workflow_id_index_key_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:205.
- Assertion 66 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::integration_full_roundtrip::full_workflow_id_index_key_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:879 hash=52aa9630f38449eb230f8c3f01b0013717cc8447b53e31c9e932ceebe7fa9fe9 tokens=["assert_eq","!","(","extracted_run_id",",","run_id",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::integration_full_roundtrip::full_workflow_id_index_key_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:206.
- Assertion 67 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_max_u64_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:900 hash=69b2fff4d73f7320637a4ff71da8f95245d1ebf2493ed0926fb5b6ada6786169 tokens=["assert_eq","!","(","run_id",".","get","(",")",",","18446744073709551615u64",",","\"parsed max u64 must equal original value\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_max_u64_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:227.
- Assertion 68 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_max_u64_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:908 hash=bc8d86f3431a3a1c513b36f49ce12ad26b17bc1f711e84c495508c3517d23677 tokens=["assert_eq","!","(","&","key","[","1","..","9","]",",","18446744073709551615u64",".","to_be_bytes","(",")",",","\"key bytes must match max u64\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_max_u64_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:235.
- Assertion 69 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_zero_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:925 hash=0f194433aefef734120905b33c928342411d3e8c0b8fea9bf97eae35e47d4537 tokens=["assert_eq","!","(","run_id",".","get","(",")",",","0u64",",","\"parsed zero must equal 0\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_zero_roundtrip at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:252.
- Assertion 70 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_rejects_empty at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:938 hash=925d6e276c3a4987c77e08433167a7fc1481f4dfc867334941f8c71119b3391d tokens=["assert","!","(","result",".","is_err","(",")",",","\"empty string must fail to parse\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_rejects_empty at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:265.
- Assertion 71 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_rejects_negative at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:950 hash=c7e2c7d2de0c7244db689e22ebfcb17eb974e181987691a5fb73096067b3de02 tokens=["assert","!","(","result",".","is_err","(",")",",","\"negative string must fail to parse\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_rejects_negative at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:277.
- Assertion 72 velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_rejects_overflow at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:962 hash=944e2c631db74dae231c3990734fe0607dedfe7f282fbae3f092b8b6987d1d95 tokens=["assert","!","(","result",".","is_err","(",")",",","\"overflow string must fail to parse\"",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_eepg_bdd_tests::e2e_fromstr_cold_path::fromstr_rejects_overflow at crates/workspace_tests/tests/vb_eepg_bdd_tests.rs:289.

## roundtrip source binding

Old crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs blob 156cb1e5bac16fc7a0e3321b457ae1263ab801b9; current blob 07d2adfdd84e3517d53e6941833bd259e0634c8c. Destination bindings: crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:07d2adfdd84e3517d53e6941833bd259e0634c8c; crates/workspace_tests/tests/vb_f1_owned_inputs/event_assertions.rs:0fc2a8a3b018339b6056bf2afe9b3d5521204b4d; crates/workspace_tests/tests/vb_f1_owned_inputs/graph_assertions.rs:bf9dcf3e832c13436750115bba64154521a00afd.
- Case velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::strict_owned_submission_reopens_with_exact_arenas_roots_taints_and_receipt at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:35 tokens=06799505cd9c8ef8e1694d86169a42b5a2156d3a0db0799c963276f3bab8237f -> velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::strict_owned_submission_reopens_with_exact_arenas_roots_taints_and_receipt at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:68 tokens=2d9bff478adf70fe99aa12bde207f2402ba7b52f61a1dafda2c89cd2449fd7cf; same_fqn=true; exact_function_tokens=false.
- Assertion 1 velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::assert_owned_graph at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:10 hash=bcaf58e044cb2a6354281fe8d94579fcdc898ea366d5b63006cb606c67ec66a7 tokens=["assert","!","(","bundle",".","matches_workflow","(","&","workflow",")",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::graph_assertions::assert_owned_graph at crates/workspace_tests/tests/vb_f1_owned_inputs/graph_assertions.rs:10.
- Assertion 2 velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::assert_owned_graph at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:11 hash=c6adcd761fadcb26264419c17883fb10a18624650bc70fdd7f42a6e0724dab9c tokens=["assert_eq","!","(","bundle",".","input_slots","(",")",",","workflow",".","input_slots","(",")",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::graph_assertions::assert_owned_graph at crates/workspace_tests/tests/vb_f1_owned_inputs/graph_assertions.rs:11.
- Assertion 3 velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::assert_owned_graph at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:12 hash=1982d28e3239eaa080f067c16ebcb7a03e0f31fa7049020dfa0d4bccacd0bd17 tokens=["assert_eq","!","(","*","bundle",".","resource_contract","(",")",",","workflow",".","resource_contract","(",")",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::graph_assertions::assert_owned_graph at crates/workspace_tests/tests/vb_f1_owned_inputs/graph_assertions.rs:12.
- Assertion 4 velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::assert_owned_graph at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:14 hash=60e7c693cb0e7bd71c94dfb851175f5ed2fd904cbdc4682c4ac38e919d0b1534 tokens=["assert_eq","!","(","bundle",".","root","(","entries",")",",","Some","(","(","SlotValue","::","List","(","ListId","::","new","(","2",")",")",",","Taint","::","Secret",")",")",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::graph_assertions::assert_owned_graph at crates/workspace_tests/tests/vb_f1_owned_inputs/graph_assertions.rs:14.
- Assertion 5 velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::assert_owned_graph at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:19 hash=079391a358c24140b8e0eea2ef6783f3348802d5b148e02aa2570b573217f09b tokens=["assert_eq","!","(","store",".","list","(","ListId","::","new","(","0",")",")","?",",","&","[","SlotValue","::","I64","(","7",")",",","SlotValue","::","I64","(","1",")",",","SlotValue","::","I64","(","7",")","]",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::graph_assertions::assert_list_values_and_taints at crates/workspace_tests/tests/vb_f1_owned_inputs/graph_assertions.rs:24.
- Assertion 6 velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::assert_owned_graph at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:23 hash=8566ace3d824227e7fdf070be198df2c207a975ac5432ee65d05e328a49d44a9 tokens=["assert_eq","!","(","store",".","list_item_with_taint","(","ListId","::","new","(","0",")",",","1",")","?",",","(","SlotValue","::","I64","(","1",")",",","Taint","::","DerivedFromSecret",")",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::graph_assertions::assert_list_values_and_taints at crates/workspace_tests/tests/vb_f1_owned_inputs/graph_assertions.rs:28.
- Assertion 7 velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::assert_owned_graph at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:27 hash=b5c31416ed6d720bf817c9ba41cb2953be55f4293c15a4733111107565e6dadc tokens=["assert_eq","!","(","store",".","list_item_with_taint","(","ListId","::","new","(","0",")",",","2",")","?",",","(","SlotValue","::","I64","(","7",")",",","Taint","::","Secret",")",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::graph_assertions::assert_list_values_and_taints at crates/workspace_tests/tests/vb_f1_owned_inputs/graph_assertions.rs:32.
- Assertion 8 velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::strict_owned_submission_reopens_with_exact_arenas_roots_taints_and_receipt at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:52 hash=4f9524351b2d9db3ed3842975f3ada3ccfd18177727a35750bf2526820cd5b55 tokens=["assert_eq","!","(","source",",","f",".","source",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::load_reopened_state at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:28.
- Assertion 9 velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::strict_owned_submission_reopens_with_exact_arenas_roots_taints_and_receipt at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:56 hash=07d01efe68dbf0ef9b7da8fdf5285f5dad59b468d8a0a7acd6900360577534f0 tokens=["assert_eq","!","(","loaded",".","input_digest","(",")",",","expected",".","input_digest","(",")",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::assert_reopened_state at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:44.
- Assertion 10 velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::strict_owned_submission_reopens_with_exact_arenas_roots_taints_and_receipt at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:57 hash=f73ba27fb5b2ab506546e614d1f895e27fdea9ab9baa2d5f713eb5929adb6a23 tokens=["assert_eq","!","(","loaded",".","roots","(",")",",","expected",".","roots","(",")",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::assert_reopened_state at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:45.
- Assertion 11 velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::strict_owned_submission_reopens_with_exact_arenas_roots_taints_and_receipt at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:58 hash=c45d1100137b470e9d70f817675367de261000d39e233a19b74477516403735f tokens=["assert_eq","!","(","loaded",".","checkpoint","(",")",",","expected",".","checkpoint","(",")",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::assert_reopened_state at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:46.
- Assertion 12 velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::strict_owned_submission_reopens_with_exact_arenas_roots_taints_and_receipt at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:61 hash=a63db2ad61ce5b60c90d7f625730ef4f0525e33e1975cac63b7735357e0ab7b3 tokens=["assert_eq","!","(","events",".","len","(",")",",","2",")"]; EXPLICIT NONIDENTICAL REPLACEMENT: velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::strict_owned_submission_reopens_with_exact_arenas_roots_taints_and_receipt at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:80 hash=7e12faaad65b9f2e4b771d354808c1d5883ef033640e6174157a66d32b87ebd1 tokens=["assert_eq","!","(","events",".","len","(",")",",","3",")"]; velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::event_assertions::assert_accepted at crates/workspace_tests/tests/vb_f1_owned_inputs/event_assertions.rs:34 hash=e1eb698cabc1dd82d4d3a8e64b4222aa62298c875e858e78019d1fba7310d24c tokens=["assert_eq","!","(","*","run",",","f",".","run",")"]; velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::event_assertions::assert_accepted at crates/workspace_tests/tests/vb_f1_owned_inputs/event_assertions.rs:35 hash=85d5305fcea3f09d7f91b627764e280c26a2e11055cc99535de9bb686f513e19 tokens=["assert_eq","!","(","seq",".","get","(",")",",","0",")"]; velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::event_assertions::assert_accepted at crates/workspace_tests/tests/vb_f1_owned_inputs/event_assertions.rs:36 hash=c2c83c202ddb561a4ca56377dca7b5105769a21b8dc3b9041dd7b7ecfcb40e31 tokens=["assert_eq","!","(","*","workflow",",","f",".","workflow",".","digest","(",")",")"]; velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::event_assertions::assert_admission_identity at crates/workspace_tests/tests/vb_f1_owned_inputs/event_assertions.rs:55 hash=d09468f7d0fbc22f374c2d938912eec6ec501433906cb80e383cf5844bad3abc tokens=["assert_eq","!","(","*","admission_run",",","f",".","run",")"]; velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::event_assertions::assert_admission_identity at crates/workspace_tests/tests/vb_f1_owned_inputs/event_assertions.rs:56 hash=40bec89c2310d398b280e3b34137a8c3a657bb3b3abe3011458e1058b6487c79 tokens=["assert_eq","!","(","admission_seq",".","get","(",")",",","1",")"]; velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::event_assertions::assert_admission_identity at crates/workspace_tests/tests/vb_f1_owned_inputs/event_assertions.rs:57 hash=f67f0a2fdd23a0654e2a86ab5eb344190cb4082c7217f567fff72ea67c31e455 tokens=["assert_eq","!","(","*","artifact_digest",",","f",".","artifact",".","digest",")"]; velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::event_assertions::assert_admission_identity at crates/workspace_tests/tests/vb_f1_owned_inputs/event_assertions.rs:58 hash=4e2acadeaf40e7a3f9ba917eb5efef6a01ca5975c28b795a6bcfeef48f2a72ba tokens=["assert_eq","!","(","*","input_digest",",","vb_storage","::","RunInputBinding","::","Digest","(","expected",".","input_digest","(",")",")",")"]; velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::event_assertions::assert_admission_authority at crates/workspace_tests/tests/vb_f1_owned_inputs/event_assertions.rs:75 hash=84a9c058f877780cea670187419079fde1f90e2b78637e5cb472b91d5ae1acd7 tokens=["assert_eq","!","(","*","policy_pin",",","vb_storage","::","RunPolicyPin","::","Pinned","(","f",".","artifact",".","policy_digest",")",")"]; velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::event_assertions::assert_admission_authority at crates/workspace_tests/tests/vb_f1_owned_inputs/event_assertions.rs:79 hash=a92e7435436f39d0afde434963c5ecc061f7c7918f0c69fe4d41fb9e581c1c31 tokens=["assert_eq","!","(","*","granted_capabilities",",","vb_core","::","CapabilitySet","::","empty","(",")",")"]; velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::event_assertions::assert_admission_authority at crates/workspace_tests/tests/vb_f1_owned_inputs/event_assertions.rs:80 hash=341e479b37ee3e86dcbdaec0fe3b433cd18b288544b845cb96e0e60e49df0269 tokens=["assert_eq","!","(","*","policy",",","RuntimePolicy","::","Strict",")"]; velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::event_assertions::assert_compatibility at crates/workspace_tests/tests/vb_f1_owned_inputs/event_assertions.rs:93 hash=b71eb7f3aeccdc239584ea068a54347e6fd0a98eb3d8e91be5647f496b177b10 tokens=["assert_eq","!","(","*","compat_run",",","f",".","run",")"]; velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::event_assertions::assert_compatibility at crates/workspace_tests/tests/vb_f1_owned_inputs/event_assertions.rs:94 hash=308eef527f0d824b4c7dfcb8e35d7a1a28fbbceff9d3afa2994fbdb6bea2e824 tokens=["assert_eq","!","(","compat_seq",".","get","(",")",",","2",")"]; velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::event_assertions::assert_compatibility at crates/workspace_tests/tests/vb_f1_owned_inputs/event_assertions.rs:100 hash=ffcef527dca497dc3737b7600d05a6c9ab47cbfe83af4a0c0df7282c2a3e4d44 tokens=["assert_eq","!","(","*","decision",",","expected",")"]. Rationale requiring CODE/TEST: Old two-event length becomes exact3 plus mandatory full RunAccepted/RunAdmission/CompatibilityDecisionEvent shape/order and run/seq0,1,2/pin/capability/policy/decision assertions. Existing RunSubmission::stage_acceptance writes the third compatibility event; no production/schema change occurs here.
- Assertion 13 velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::strict_owned_submission_reopens_with_exact_arenas_roots_taints_and_receipt at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:67 hash=4ba1716f4725449b149bd42285aaea89873f1d7f7fbf4eee33327aedb0520468 tokens=["assert","!","(","journal",".","run_header","(","f",".","run",")","?",".","is_some","(",")",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::assert_retained_state at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:58.
- Assertion 14 velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::strict_owned_submission_reopens_with_exact_arenas_roots_taints_and_receipt at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:68 hash=985e1d4041838c28d2b1d39d797358c19d50d52c29447179307e28c30203e48a tokens=["assert","!","(","matches","!","(","journal",".","retained_run_inputs","(","f",".","run",",","artifact",".","digest",",","source",".","digest",",","&","workflow",")","?",",","Some","(","RetainedRunInputs","::","Owned","(","_",")",")",")",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::assert_retained_state at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:59.
- Assertion 15 velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::strict_owned_submission_reopens_with_exact_arenas_roots_taints_and_receipt at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:72 hash=8ad3aca939c79f6f6f09cf94c2222ab5dd7873e893ede490fd026619d52f38ea tokens=["assert_eq","!","(","workflow",".","to_parts","(",")",",","f",".","workflow",".","to_parts","(",")",")"]; exact destinations=velvet-ballistics-workspace-tests::vb_f1_owned_inputs::roundtrip::assert_retained_state at crates/workspace_tests/tests/vb_f1_owned_inputs/roundtrip.rs:63.

Limits: maps are source observations. No independent CODE/TEST disposition, exhaustive semantic equivalence, full feature, master190 or canonical CI acceptance is claimed. Native receipts and copied-overlay gate controls are separate evidence; an overlay checker pass only validates these exact hash-bound justification records.
````
