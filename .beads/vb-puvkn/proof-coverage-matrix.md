# Proof Coverage Matrix — vb-puvkn

| requirement_id | contract_clause | proof_seed | verus | kani | flux | loom | proptest | fuzz | coverage_status |
|---|---|---|---|---|---|---|---|---|---|
| REQ-RUNTIME-SHARD-INDEX | vb-puvkn/shard_index | vb-puvkn | YES | YES | N/A | N/A | YES | N/A | 3-of-3 applicable |
| REQ-RUNTIME-SHARD-INDEX-ZERO | vb-puvkn/shard_index_zero | vb-puvkn | YES | YES | N/A | N/A | YES | N/A | 3-of-3 applicable |
| REQ-RUNTIME-SHARD-INDEX-NONZERO | vb-puvkn/shard_index_nonzero | vb-puvkn | — | YES | N/A | N/A | YES | N/A | 2-of-2 applicable |
| REQ-RUNTIME-SHARD-INDEX-IDEMPOTENT | vb-puvkn/shard_index_idempotent | vb-puvkn | — | YES | N/A | N/A | YES | N/A | 2-of-2 applicable |
| REQ-RUNTIME-SHARD-INDEX-BOUNDED | vb-puvkn/shard_index_bounded | vb-puvkn | YES | YES | N/A | N/A | YES | N/A | 3-of-3 applicable |

**Summary:**
- Required: Verus ×2 lemmas, Kani ×4 harnesses, proptest ×5 properties.
- Non-applicable: Flux (pure modulo), Loom (sync), fuzz (no parser).
- **Master §40 + §44 satisfied:** production Runtime::shard_index is now
  bound to spec_shard_index via the new production-binding lemma.