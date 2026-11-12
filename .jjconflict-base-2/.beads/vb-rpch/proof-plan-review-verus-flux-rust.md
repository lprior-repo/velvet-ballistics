# Proof Plan Review — vb-rpch Verus/Flux/Rust

reviewer_skill: proof-plan-reviewer
reviewer_invocation_id: proof-plan-reviewer-vb-rpch-p4-verus-flux-rust-20260524
planner_invocation_id: ses_1a353247fffef4JiPxKFf9Hlv3
review_state: 4 review / proof-plan-reviewer
workdir: /home/lewis/src/vb-jpq7-jj-fix
bead: vb-rpch

## Reviewed artifacts and hashes

- `.beads/vb-rpch/contract.md` sha256 `80208500fc9e5e237f5551a60bf0d1e8e858601102b63ea7a71bd702ce7f797d`
- `.beads/vb-rpch/verification-layers.md` sha256 `070ac16fff746469ce0b7dd66bf24df0090528520fc29b81e39bfc040d5e7c7c`
- `.beads/vb-rpch/proof-obligations.jsonl` sha256 `54b8c94c14c5aa9ed955a4aad27846317c9b1878bbbf62cad7153639af8bb1ac`
- `.beads/vb-rpch/proof-strategy-verus-flux-rust.md` sha256 `d361d703ea9554c63dfdd9650c1d195981deacc5bd4b99a3595ce1d65d31ebbd`
- `.beads/vb-rpch/verifier-lane-decisions.verus-flux-rust.jsonl` sha256 `b071707a07dcd322ecc8f1f74283276e1efeb7ec1f1610fb2b09b0c89394d907`
- `.beads/vb-rpch/proof-obligations.verus-flux-rust.planned.jsonl` sha256 `a253fedcfd8bd0b85c1b340523cbbcac6d8260b5b795b8be601bd39de4b43b32`
- `.beads/vb-rpch/proof-coverage-matrix-verus-flux-rust.md` sha256 `2cc1c9433015c806d7b80288972ef27731127c6020feb691fc865ebf61bfa7b0`
- `.beads/vb-rpch/trusted-base-plan-verus-flux-rust.md` sha256 `3d42b6adcd513610ecf8e05e33b84adc8668ebfe1639f9c0f89438265e406fd5`
- `.beads/vb-rpch/waiver-candidates-verus-flux-rust.md` sha256 `4099cb6516bca7e86d10960886e36b09fb3d0fe9fb2fbfdb0722f4cb801f5c4a`
- `.beads/vb-rpch/proof-to-implementation-input-verus-flux-rust.md` sha256 `e5e469fa505f621a53606900de7562b689f8e8c557b4292fc833a0f33ed999a8`
- `.beads/vb-rpch/proof-review-tlc-fix-round3.md` sha256 `a76deae173e16b312b2c9c591b5a18c8bfb5e73657809862b6b52b8c2bbd3502`

## Commands run

1. `python - <<'PY' ...` from `/home/lewis/src/vb-jpq7-jj-fix` validated JSONL syntax for:
   - `verifier-lane-decisions.verus-flux-rust.jsonl`: JSONL_OK, rows=5.
   - `proof-obligations.verus-flux-rust.planned.jsonl`: JSONL_OK, rows=21.
   - `proof-obligations.jsonl`: JSONL_OK, rows=36.
2. `python - <<'PY' ...` from `/home/lewis/src/vb-jpq7-jj-fix` computed SHA-256 hashes listed above.

## Review verdict

Rejected. The files parse as JSONL, but the planner artifacts are not in the required machine-readable schemas and are not safe to hand to proof-writer.

Blocking findings:

1. `verifier-lane-decisions.verus-flux-rust.jsonl` is not `verifier-lane-decision/v1`. Rows lack `schema_version`, `id`, `requirement_id`, `contract_clause`, `proof_seed_id`, canonical `verifier`, canonical `applicability`, `decision_reason`, `required_obligation_ids`, `non_applicability_evidence_refs`, `limitation_kind`, and canonical `status`. The file uses ad-hoc `lane`/`decision`/`clauses` fields, so downstream lane review cannot precisely bind a lane to a `(requirement_id, contract_clause, proof_seed_id, verifier)` tuple.
2. `proof-obligations.verus-flux-rust.planned.jsonl` is not `proof-obligation/v1`. Rows lack mandatory `schema_version`, `domain_claim`, `risk_tags`, `model_bounds`, `tool_metadata`, `trusted_base_refs`, and `behavior_affecting`, and use noncanonical `bounds` instead of `model_bounds`.
3. Core lane coverage is incomplete at the machine-readable level. For the remaining clauses (`INV-002`, `INV-003`, `INV-004`, `INV-005`, `PRE-001`, `PRE-002`, `POST-009`), the canonical core verifier set requires explicit decisions for `tla-plus`, `verus`, `kani`, `flux-rs`, `loom`, `miri`, `proptest`, and `cargo-fuzz`. The suffix has only aggregate ad-hoc rows for `verus`, `flux-rs`, `production-rust-holzman`, `tla-plus`, and `kani`; it has no canonical decisions for `loom`, `miri`, `proptest`, or `cargo-fuzz`, and the aggregate `tla-plus`/`kani` rows are not per-clause schema rows.
4. The `kani` non-applicability rationale is too weak. It cites existing closed/blocked findings and user suffix scope, but `verification-layers.md` assigns Kani as secondary for `PRE-001`, `PRE-002`, `POST-009`, `INV-003`, `INV-004`, and related risk. Scope preference is not concrete non-applicability evidence.

Non-blocking positive checks:

- Flux is classified honestly as `blocked_tooling`, not as proof success. The plan states `cargo flux --version` failed and that Flux proves nothing until rechecked.
- Production Rust proof-attachment obligations are routed to State 11 / Holzman rather than proof-writer.
- The existing TLC round-3 approval is preserved as bounded TLA/TLC evidence only and is not overclaimed as Rust/Flux evidence.

Nearest rerun state: State 3 / proof-planner repair for schema-normalized lane decisions and planned obligations, then rerun State 4 proof-plan-reviewer.

Proof-writer may proceed: no.

STATUS: REJECTED
