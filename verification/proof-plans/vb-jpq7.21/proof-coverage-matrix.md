# Proof Coverage Matrix — vb-jpq7.21

| Requirement | Seeds | Required lanes | Planned obligations | Existing bridge | Gaps before closure |
|---|---|---|---|---|---|
| `ipc-answerask-shape` | `vb-jpq7-21-seed-ipc-answerask-shape` | proptest, cargo-fuzz, cargo-test | `obl-vb-jpq7-21-proptest-ipc-roundtrip-003`, `obl-vb-jpq7-21-fuzz-ipc-hostile-boundary-005`, `obl-vb-jpq7-21-existing-behavior-bridge-006` | vb_ipc deterministic roundtrips; CLI AnswerAsk emission | Add generated roundtrip property and hostile fuzz evidence for repaired shape. |
| `runtime-derives-ask-ticket` | `vb-jpq7-21-seed-runtime-ticket-derivation` | Kani, proptest, cargo-test | `obl-vb-jpq7-21-kani-ticket-derivation-002`, `obl-vb-jpq7-21-proptest-slot-valid-invalid-004`, `obl-vb-jpq7-21-existing-behavior-bridge-006` | Direct legacy `answer_ask` resume test | Add production-seam harnesses for `answer_pending_ask_slot` invalid shard/timer/resume states. |
| `answer-slot-equality` | `vb-jpq7-21-seed-answer-slot-equality` | Kani, proptest | `obl-vb-jpq7-21-kani-slot-equality-001`, `obl-vb-jpq7-21-proptest-slot-valid-invalid-004` | CLI invalid slot behavior context | Add generated valid/mismatch answer slot checks and no-AskAnswered-on-mismatch assertions. |
| `ipc-handler-runtime-bridge` | `vb-jpq7-21-seed-ipc-handler-runtime-bridge` | cargo-fuzz, cargo-test, moon-ci | `obl-vb-jpq7-21-fuzz-ipc-hostile-boundary-005`, `obl-vb-jpq7-21-existing-behavior-bridge-006`, `obl-vb-jpq7-21-moon-ci-007` | CLI IPC mock verifies emitted payload | Add focused handler tests or proof bridge showing decoded SlotValue/taint/encoded_len reaches `answer_pending_ask_slot`. |

No row is marked executed. Evidence columns are intentionally empty until formal-verifier runs commands.
