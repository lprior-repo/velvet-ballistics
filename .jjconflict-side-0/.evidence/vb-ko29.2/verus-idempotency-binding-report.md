# vb-ko29.2 Verus idempotency binding report

## Scope

Repaired idempotency Verus artifacts so they are no longer presented as standalone toy models. Each artifact now carries a binding ledger with concrete production source references and a machine-readable claim map in `verus-idempotency-binding-map.jsonl`.

## Binding outcomes

- `verification/verus/idempotency_decision.rs` binds the finite Verus decision table to `vb_core::action::{SideEffect, RetrySafety, Idempotency}`, `vb_validate::idempotency_contract::is_statically_idempotent_contract`, and exported `vb_compile::check_idempotency_gates`.
- `verification/verus/idempotency_replay_tracker.rs` binds set-algebra proofs to `vb_storage::recovery::types::ActionReplayTracker` and `vb_storage::recovery::replay::core::replay_events` action scheduling/completion/failure branches.
- `verification/verus/idempotency_certificate_summary.rs` binds identifier-local certificate membership proofs to `vb_storage::admission::VerificationProof`, `submit_artifact_with_contracts`, idempotency evidence construction, and `vb_runtime::admission::first_missing_idempotency_attestation`.

## Production parity finding

No production Rust was changed. The exported `vb_compile::check_idempotency_gates` surface is `crates/vb_compile/src/mod_compile_core.rs:177-229` via `crates/vb_compile/src/lib.rs:53-58`; that path already rejects side-effecting `Idempotency::DeterministicPure` and matches `vb_validate::is_statically_idempotent_contract`. The older `crates/vb_compile/src/compile/mod.rs` file is not the exported `vb_compile` surface used by `lib.rs`, so it is not used as proof authority in this binding report.

## Verifier evidence

- `verus verification/verus/idempotency_decision.rs` → PASS, 8 verified, 0 errors. Raw log: `.evidence/vb-ko29.2/verus-idempotency-decision.log`.
- `verus verification/verus/idempotency_replay_tracker.rs` → PASS, 8 verified, 0 errors. Raw log: `.evidence/vb-ko29.2/verus-idempotency-replay-tracker.log`.
- `verus verification/verus/idempotency_certificate_summary.rs` → PASS, 9 verified, 0 errors. Raw log: `.evidence/vb-ko29.2/verus-idempotency-certificate-summary.log`.
- Modified-artifact trust scan found no `assume`, `external_body`, `external`, or `axiom` in the three modified Verus files. Raw log: `.evidence/vb-ko29.2/verus-trust-scan.log`.
- `rtk cargo check -p vb_compile` → PASS. Raw log: `.evidence/vb-ko29.2/cargo-check-vb-compile.log`.

## Trusted boundaries

- Manual Verus mirrors of production enum variants and branch tables are trusted to stay in sync with the cited source refs; future production enum/function changes require updating this binding evidence.
- Rust `HashSet` correctness and `ActionId`/`StepIdx` equality/hash coherence are trusted for the replay tracker projection.
- Runtime/storage certificate proofs are identifier-local membership projections; Rust slice `contains` semantics and postcard/Fjall persistence are outside the Verus set proof.

## Blockers

No verifier blockers remain for the modified Verus artifacts. Full proof-reviewer approval is not claimed here.
