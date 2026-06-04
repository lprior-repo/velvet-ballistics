# Proof Coverage Matrix — vb-jpq7.21

STATUS: PLANNED. No proof closure is claimed.

| Requirement | Seed | Required behavior lanes/obligations | Non-applicable lanes recorded | Behavior tests cited |
|---|---|---|---|---|
| `ipc-answerask-shape` | `vb-jpq7-21-seed-ipc-answerask-shape` | proptest `obl-vb-jpq7-21-proptest-ipc-roundtrip-001`; cargo-fuzz `obl-vb-jpq7-21-fuzz-ipc-shape-002` | kani, verus, flux-rs, loom, miri | `handle_answer_ask_accepts_valid_postcard_slot_value_and_default_clean_taint`; deterministic IPC roundtrips |
| `runtime-derives-ask-ticket` | `vb-jpq7-21-seed-runtime-ticket-derivation` | kani `obl-vb-jpq7-21-kani-ticket-derivation-003`; proptest `obl-vb-jpq7-21-proptest-ticket-derivation-004` | verus, flux-rs, cargo-fuzz, loom, miri | `answer_pending_ask_slot_accepts_matching_answer_slot_and_completes_run`; absent/non-ask state tests |
| `answer-slot-equality` | `vb-jpq7-21-seed-answer-slot-equality` | kani `obl-vb-jpq7-21-kani-slot-equality-005`; proptest `obl-vb-jpq7-21-proptest-slot-equality-006` | verus, flux-rs, cargo-fuzz, loom, miri | mismatch-without-advance tests |
| `ipc-handler-runtime-bridge` | `vb-jpq7-21-seed-ipc-handler-runtime-bridge` | cargo-test `007`-`010`, `013`-`019`; kani `012`; proptest `020`; cargo-fuzz `011`; moon-ci `021` | verus, flux-rs, loom, miri | taint defaulting, malformed SlotValue rejection-before-mutation, handler routing, absent ask, runtime matching/mismatch/non-Ask states, CLI happy path, CLI malformed local rejection |

Every lane decision is one row per `(requirement_id, contract_clause, proof_seed_id, verifier)`. Behavior-affecting semantic rows are marked `behavior_affecting: true`; waiver candidates are non-behavior proof-infrastructure exceptions only.
