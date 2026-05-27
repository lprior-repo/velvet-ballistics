# Verifier Lane Matrix — vb-om21

| Proof seed | Requirement | TLA+ | Verus | Kani | Flux | Loom | Miri | Proptest | cargo-fuzz |
|---|---|---|---|---|---|---|---|---|---|
| `ps-vb-om21-prefix-bound` | `REQ-vb-om21-07` | REQ | REQ | REQ | REQ | N/A | N/A | REQ | N/A |
| `ps-vb-om21-big-endian-max` | `REQ-vb-om21-08` | N/A | REQ | REQ | REQ | N/A | N/A | REQ | N/A |
| `ps-vb-om21-tail-mismatch` | `REQ-vb-om21-03` | REQ | REQ | REQ | REQ | N/A | N/A | REQ | N/A |
| `ps-vb-om21-missing-journal` | `REQ-vb-om21-04` | REQ | REQ | REQ | REQ | N/A | N/A | REQ | N/A |
| `ps-vb-om21-zero-tail-query` | `REQ-vb-om21-05` | REQ | REQ | REQ | REQ | N/A | N/A | REQ | N/A |
| `ps-vb-om21-single-event-tail` | `REQ-vb-om21-06` | N/A | REQ | REQ | REQ | N/A | N/A | REQ | N/A |
| `ps-vb-om21-tail-overflow` | `REQ-vb-om21-08` | N/A | REQ | REQ | REQ | N/A | N/A | REQ | N/A |
| `ps-vb-om21-key-parse` | `REQ-vb-om21-07` | N/A | REQ | REQ | REQ | N/A | REQ | REQ | REQ |
| `ps-vb-om21-replay-parity` | `REQ-vb-om21-01` | REQ | REQ | REQ | REQ | N/A | N/A | REQ | N/A |
| `ps-vb-om21-bounded-scan` | `REQ-vb-om21-07` | N/A | REQ | REQ | REQ | N/A | N/A | REQ | N/A |
| `ps-vb-om21-typed-errors` | `REQ-vb-om21-02` | REQ | REQ | REQ | REQ | N/A | N/A | REQ | N/A |

Legend: `REQ` = planned required obligation in proof-obligations.planned.jsonl; `N/A` = explicit verifier-lane-decision/v1 row with evidence refs.
