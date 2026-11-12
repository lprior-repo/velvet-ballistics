# Assurance Bundle: vb-m5gp

STATUS: APPROVED

## Scope

- Bead: `vb-m5gp` — split `crates/vb_compile/src/lib.rs` into private modules.
- Evidence state: State 13 packaging after State 12 black-hat approval.
- Authoritative contract: `.beads/vb-m5gp/contract.md`.
- Truth Serum startup read: `/home/lewis/.claude/skills/truth-serum/SKILL.md` and `/home/lewis/.agents/skills/truth-serum/SKILL.md`; files matched, and `.agents` wins on conflict.

## Requirement-to-Evidence Map

| Requirement | Evidence | Decision |
| --- | --- | --- |
| PRE-001 isolated workspace | `pwd -P` returned `/home/lewis/src/go-skill-vb-m5gp`; `.beads/vb-m5gp/STATE.md` records isolated workspace. | PASS |
| PRE-002 no dependency/feature/config scope expansion | `jj status` shows scoped production/test/evidence changes; formal `STATIC-001` passed; regression report records no new local/regression failures. | PASS |
| PRE-003 active implementation moved; stale scaffolding not blindly reused | `formal-verification-report.md` STRUCT-003 and targeted scan show no `include!`, no `compile_core_impl`, and no blind stale scaffold wiring. | PASS |
| PRE-004 / POST-002 / INV-001 public crate-root API parity | `api-compat-report.md` PASS; `cargo +nightly check -p vb_compile --all-targets --all-features` reran locally and passed; workspace API tests recorded PASS. | PASS |
| POST-001 private split facade | `lib.rs` declares private `mod_compile_core`, `mod_compile_errors`, `mod_compile_validation`, `mod_compile_lowering`; public `pub mod mod_compile_*` scan returned 0. | PASS |
| POST-003 accepted behavior parity | `test-report.md` PASS; `moon ci` recorded PASS with `10771 passed, 44 skipped`; Kani idempotency parity PASS. | PASS |
| POST-004 / ERR-001..003 diagnostics parity | `test-report.md` records diagnostics integration PASS: `21 passed, 4 ignored`; formal ERR-001 PASS. | PASS |
| POST-005 no new public internal module paths | Targeted scan: `pub_mod_compile=0 []`; split contract API/privacy test passed. | PASS |
| POST-006 source-length governance | Direct `bash scripts/check-source-length.sh` emitted only DEFERRED_GLOBAL unrelated files and exited 0; direct recursive count found `bead_local_files=28`, max `collection.rs` 286, `oversized=[]`. | PASS |
| INV-002 dependency direction | Targeted scan returned `errors_to_validation=0`, `validation_to_core=0`, `validation_to_lowering=0`; executable dependency-edge test passed. | PASS |
| INV-003 validation remains validation / INV-004 lowering deterministic | Formal STRUCT-003 and behavior gates approved; no forbidden validation-to-core/lowering edge remains. | PASS |
| INV-005 typed diagnostics | Formal ERR-001 PASS; `CompileError`/`CompileErrors` preserved per contract and diagnostic tests. | PASS |
| INV-006 zero runtime panic-surface for touched source | Strict source clippy reran with `-D unsafe_code`, unwrap/expect/panic/todo/dbg/indexing/arithmetic/as-conversions gates and passed. | PASS |
| INV-007 minimal visibility leakage | Targeted scan found no public split modules; black-hat approved. | PASS |
| TLA / theorem / Verus waivers | `contract-verification-review.md` approved concrete waivers; pure structural refactor evidence supports waiver conditions. | WAIVED |

## Formal and Machine Evidence Summary

- `formal-verification-report.md`: `STATUS: APPROVED`.
- `verification-ledger.jsonl`: 15 rows parsed; all `required:true` rows are `PASS` or `WAIVED`; only `MIRI-001` is `DEFERRED_GLOBAL` and `required:false`.
- `machine-gate-report.md`: `STATUS: PASS`.
- `test-report.md`: `STATUS: PASS`.
- `api-compat-report.md`: `STATUS: PASS`.
- `source-length-report.md`: `STATUS: PASS`.
- `static-scan-report.md`: `STATUS: PASS`.
- `kani-report.md`: `STATUS: PASS`.
- `regression-diff.md`: `STATUS: PASS`; no `FAIL_LOCAL` and no `FAIL_REGRESSION`.

## Black-Hat Approval

- `.beads/vb-m5gp/black-hat-review.md`: `STATUS: APPROVED`.
- Black-hat verified prior dependency-edge blocker repair, executable edge gate, real split/no hidden include, recursive source-length governance, and updated formal evidence.

## DEFERRED_GLOBAL Debt

- `MIRI-001`: direct `cargo +nightly miri test -p vb_compile` is `required:false` and `DEFERRED_GLOBAL` due missing local nightly rust-src path; canonical `moon ci` Miri lane passed selected checks.
- Pre-existing unrelated oversized files remain `DEFERRED_GLOBAL`: `expression_bytecode.rs`, `expression.rs`, `references.rs`, `schema.rs`, and `type_taint.rs`.

## Decision

All bead-local required gates are approved or waived by prior review, and State 13 direct checks reject missing/laundered required evidence. Proceed to landing.
