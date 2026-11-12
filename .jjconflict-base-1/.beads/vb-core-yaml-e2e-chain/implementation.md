# State 10 Implementation: vb-core-yaml-e2e-chain

STATUS: COMPLETE

## Reference Files Read

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`

## Inputs Consumed

- Approved State 9 reviews: `.beads/vb-core-yaml-e2e-chain/test-plan-review.md` and `.beads/vb-core-yaml-e2e-chain/test-suite-review.md` both contain `STATUS: APPROVED`.
- Contract/proof artifacts: `.beads/vb-core-yaml-e2e-chain/contract.md`, `proof-obligations.jsonl`, approved proof review, and approved contract verification review.
- Preserved red contract test: `tests/vb_core_yaml_e2e_chain_contract.rs::storage_produced_strict_accepted_artifact_has_runtime_required_gate_count_when_yaml_origin_run_is_submitted`.

## Code Changes Made

### `crates/vb_storage/src/admission.rs`
- Raised accepted-artifact v1 admission gate count from `2` to `15` to match runtime admission `REQUIRED_GATE_COUNT`.
- Updated warning gate bounds to `1..=15`.

### `crates/vb_compile/src/lib.rs`
- YAML-origin compile output computes its workflow digest from the serialized compiled artifact with the digest field zeroed.
- Added `compiled_artifact_digest` field and `CompileError::ArtifactEncode` so digest computation remains fallible and typed.
- Removed the YAML-source canonical digest path from the compiled artifact digest role, preserving source digest as a separate storage/test role.

### `crates/vb_storage/src/proptests.rs`, `crates/vb_storage/src/vb_2bok_durability_gate_tests.rs`, `crates/vb_storage/tests/accepted_artifact_red_phase.rs`
- Updated stale storage-side expectations from legacy two-gate artifacts to accepted-artifact v1 fifteen-gate artifacts.

## Defect Fixed

- Before State 10, strict YAML-origin `submit_artifact(&journal, &workflow, RuntimePolicy::Strict)` failed with `artifact checksum mismatch` because `vb_compile` assigned a YAML-source canonical digest while `vb_storage` validated the digest against serialized compiled IR bytes.
- After State 10, YAML-origin compiled workflows use the same artifact digest basis that storage verifies, and strict storage artifacts carry the runtime-required 15-gate proof count.

## Power-of-Ten / Zero-Panic Rules Affected

- Zero unsafe: preserved in modified production files via existing `#![forbid(unsafe_code)]`; no unsafe blocks added.
- Zero unwrap/expect/panic/todo/unimplemented/dbg in modified production paths: no new forbidden production constructs added. Grep hits are in existing test modules/comments or the word `unsafe` in diagnostics/comments.
- Checked fallible results: postcard serialization failures map to typed `CompileError::ArtifactEncode` or `JournalError::ArtifactMalformed`.
- Bounded control/resource use: digest computation serializes a bounded compiled artifact already validated by compile/resource-contract gates; no unbounded retry/loop introduced.
- Digest-role invariant: source digest remains separate in tests; artifact admission now verifies the artifact digest role against compiled artifact bytes.

## Commands Run

| Command | Status | Evidence |
|---|---|---|
| `RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo fmt --check` | PASS | no output |
| `RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo test -p vb_compile --test vb_core_yaml_e2e_chain_strict_yaml -- --nocapture` | PASS | `10 passed` |
| `RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo test -p velvet-ballistics-workspace --test vb_core_yaml_e2e_chain_contract -- --nocapture` | PASS | `35 passed` |
| `RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo test -p vb_storage -- --nocapture` | PASS | `983 passed` |
| `RUSTC_WRAPPER= TMPDIR=target/tmp TEMP=target/tmp TMP=target/tmp CFLAGS=-pipe HOST_CFLAGS=-pipe rtk cargo check -p vb_compile -p vb_storage -p velvet-ballistics-workspace --tests` | PASS | no errors |

## Failed/Retried Gates

- Initial focused tests failed with `CompileError::ArtifactEncode` missing from `CompileError::code`; repaired by adding code mapping `INVALID_COMPILED_WORKFLOW`.
- Initial `vb_storage` test run failed 10 stale legacy gate-count assertions; repaired to accepted-artifact v1 gate count.
- Second `vb_storage` test run failed 7 stale integration assertions in `accepted_artifact_red_phase`; repaired to accepted-artifact v1 gate count/bounds.

## Performance Layer Decision

- No performance claim made.
- No benchmark/profiler evidence required for this State 10 correctness repair.
- Storage placement: existing `Vec<u8>` artifact serialization is retained because accepted artifacts are persisted byte records; no new hot-path allocation strategy was claimed faster.

## Second-Ring Evidence

- No assembly/IR/vectorization/API compatibility/release-provenance claim made.
- No second-ring evidence required by this implementation state beyond existing proof artifacts and focused tests.

## Skipped Gates

- `moon ci`, full workspace clippy, Miri, mutation, coverage, and formal verifier lanes were not run in State 10. They remain State 11/formal-verifier responsibilities under the go-skill lifecycle and proof-obligation ownership.

## Residual Risks

- Full workspace release gate remains unexecuted in this state.
- Existing test-module panic/expect/assert usage remains outside production style gate; strict source lint for production should be enforced in State 11.
- State 11 has 6 FAIL_LOCAL residuals (E2E-REC-008, STATIC-BOUNDARY-009, STRICT-YAML-012, ERR-STRICT-013, MIRI-CODEC-024, GATE-RELEASE-025) already classified and routed to owner states.
