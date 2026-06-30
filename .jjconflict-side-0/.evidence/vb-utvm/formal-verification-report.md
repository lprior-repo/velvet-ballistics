# vb-utvm formal verification report

STATUS: PASS_FOR_DISCOVERY

Scope: repair `crates/vb_validate` Kani discovery compile failure discovered from `vb-jpq7.27`. This report does not claim full Kani harness proof success; it claims truthful discovery now compiles and lists the relevant harnesses.

References read before Rust source changes:
- `/home/lewis/.agents/skills/kani/references/kani-practice.md`
- `/home/lewis/.agents/skills/kani/references/kani-patterns.md`
- `/home/lewis/.agents/skills/kani/references/kani-harness.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/latency-throughput-playbook.md`
- `/home/lewis/.agents/skills/holzman-rust/references/runtime-performance-architecture.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`
- `/home/lewis/.agents/skills/holzman-rust/references/simd-patterns.md`
- `/home/lewis/.agents/skills/holzman-rust/references/mechanical-empathy-toolchain.md`

## Repair summary

- Added `kani::Arbitrary` implementations for `ExprIdx`, `AccessorIdx`, and `ConstIdx` in `vb_core`'s Kani-only ID generator module.
- Fixed a non-exhaustive `PathSegment` match in the `vb_validate` structural Gate 8 harness.
- Removed an unused `mut` introduced in a no-mutation harness.

No `unsafe` was added. No production runtime behavior changed outside `#[cfg(kani)]` arbitrary implementations and Kani-only harness code.

## Command evidence

| cwd | command | exit | raw evidence |
| --- | --- | ---: | --- |
| `/home/lewis/src/velvet-ballistics` | `bd show vb-utvm` | 0 | terminal session |
| `/home/lewis/src/velvet-ballistics` | `bd update vb-utvm --claim` | 0 | terminal session; auto-push rejected non-fast-forward |
| `/home/lewis/src/vb-utvm-vb-validate-kani-gpt55/crates/vb_validate` | `cargo kani list` | 1 | `.evidence/vb-utvm/logs/initial-cargo-kani-list.log` |
| `/home/lewis/src/vb-utvm-vb-validate-kani-gpt55` | `cargo --version; rustc --version --verbose; rustup show active-toolchain; cargo kani --version; if command -v kani >/dev/null; then kani --version; fi` | 0 | `.evidence/vb-utvm/logs/tool-versions.log` |
| `/home/lewis/src/vb-utvm-vb-validate-kani-gpt55` | `rustfmt --check crates/vb_core/src/ids/kani_id_arbitrary.rs crates/vb_validate/src/kani_gate_08_structural.rs` | 0 | `.evidence/vb-utvm/logs/rustfmt-touched-check-final.log` |
| `/home/lewis/src/vb-utvm-vb-validate-kani-gpt55` | `cargo check -p vb_validate` | 0 | `.evidence/vb-utvm/logs/cargo-check-vb-validate-final.log` |
| `/home/lewis/src/vb-utvm-vb-validate-kani-gpt55` | `cargo test -p vb_validate --all-features` | 0 | `.evidence/vb-utvm/logs/cargo-test-vb-validate-final.log` |
| `/home/lewis/src/vb-utvm-vb-validate-kani-gpt55/crates/vb_validate` | `cargo kani list` | 0 | `.evidence/vb-utvm/logs/final-cargo-kani-list-success.log` |
| `/home/lewis/src/vb-utvm-vb-validate-kani-gpt55/crates/vb_validate` | `cargo kani list --format json` | 0 | `.evidence/vb-utvm/kani-list.actual.json` copied from Kani's generated `crates/vb_validate/kani-list.json` |
| `/home/lewis/src/vb-utvm-vb-validate-kani-gpt55` | `ruby -rjson -e 'JSON.parse(File.read(ARGV.fetch(0)))' .evidence/vb-utvm/kani-list.actual.json` | 0 | `.evidence/vb-utvm/logs/kani-list-actual-json-validate.log` |

Attempted scoped proof execution:
- `cargo kani --harness kani_gate_08_accessor::kani_gate_08_no_panic_bounded_inputs` in `/home/lewis/src/vb-utvm-vb-validate-kani-gpt55/crates/vb_validate` exceeded the 180s command timeout. Partial raw output is `.evidence/vb-utvm/logs/cargo-kani-scoped-no-panic.log`. No proof PASS is claimed from this timed-out command.

## Harness/discovery summary

Kani discovery now lists 26 standard `vb_validate` harnesses and 0 contract harnesses. Relevant Gate 8 harnesses include:
- `kani_gate_08_accessor::*` (7 harnesses)
- `kani_gate_08_structural::*` (14 harnesses)
- `kani_idempotency_contract::*` (5 harnesses)

Kani emitted unsupported-construct warnings during discovery: `caller_location (1)` and `foreign function (2)`. Discovery succeeded; any future full proof run must treat reachable unsupported constructs as proof blockers if encountered.

## Proof-review checklist outcome

- Harness inventory exists and is command-backed: `.evidence/vb-utvm/kani-list.actual.json`.
- Initial failure is reproduced with raw log and exit status.
- Missing structural generators were repaired via `kani::Arbitrary` implementations for the ID newtypes that harnesses request symbolically.
- Non-exhaustive `PathSegment` handling was made explicit with wildcard rejection in the Kani assumption predicate.
- No hardcoded single-shape PASS was laundered: discovery is PASS; scoped proof execution timed out and remains non-PASS.

## Residual blockers / limits

- Full Kani proof execution for the scoped harness timed out at 180s and is not claimed as successful.
- Repo-wide `cargo fmt --check` has pre-existing unrelated formatting drift outside touched files; touched-file rustfmt passed.
- `cargo kani list --format json` writes a status line to stdout and the actual JSON to `crates/vb_validate/kani-list.json`; the validated artifact is copied to `.evidence/vb-utvm/kani-list.actual.json`.
