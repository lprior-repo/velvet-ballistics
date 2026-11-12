# Machine Gate Report: vb-qi37.4

STATUS: PASS

## Commands

- `moon run :verify-proof`: PASS; all proof checks passed.
- `tlc -config specs/admission_header_before_ack.cfg specs/admission_header_before_ack.tla`: PASS; no errors.
- `verus verification/verus/admission_artifact_model.rs`: PASS; 6 verified, 0 errors.
- `verus verification/verus/capability_artifact_model.rs`: PASS; 8 verified, 0 errors.
- `moon run :verify-deep`: PASS; all deep checks passed.
- `moon run :verify-all`: PASS; all all checks passed.
- `moon run :lint-src`: PASS.
- `moon run :fuzz-smoke`: PASS.
- `moon run :mutants-smoke`: PASS; 1 mutant tested, 1 caught.
- `cargo test -p velvet_ballistics --test admission_evidence_integration`: PASS; 8 passed.
- `cargo test -p vb_storage --test accepted_artifact_red_phase`: PASS; 29 passed.
- `cargo test -p velvet_ballistics --test admission_durability_code`: PASS; 1 passed.
- `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime journal_writer_queue`: PASS; 3 passed.
- `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime timer_fired_cancel`: PASS; 1 passed.
- `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime shutdown_drain`: PASS; 3 passed.
- `jj diff --name-only | moon ci --stdin`: PASS; 18 completed, 2 cached, including 8358 tests passed and 6 skipped.

## CI Note

- Plain `moon ci` and `moon ci --force` fail in this jj workspace before execution because Git lacks a local `main` revision. The accepted CI invocation is `jj diff --name-only | moon ci --stdin`, which avoids the missing Git ref and executed the affected CI graph successfully.
