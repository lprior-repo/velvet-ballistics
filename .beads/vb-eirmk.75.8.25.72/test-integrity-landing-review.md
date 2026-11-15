# Intentional retirement of answer Step encoder assertions

## Scope

The answer command's legacy `--step`/`--value-file` selector was explicitly retired. The production CLI now rejects those flags during `validate_known_flags`; it accepts only required `--slot` and `--value`. This record concerns only two direct `AnswerSelector::Step(...).wire_ticket()` test assertions removed from `crates/velvet_ballistics/src/args/answer_selector.rs`.

## Removed checks

The removed exact checks asserted that the private CLI selector encoded `StepIdx::ZERO` as wire ticket `0` and `StepIdx::new(u16::MAX)` as `65_535`. `AnswerSelector::Step` was the CLI-only constructor for the selector being retired. Retaining it solely to preserve these assertions would retain dead private code that can no longer be produced by the public parser.

## Replacement coverage and preserved protocol

The actual IPC answer decoder remains unchanged in `crates/vb_ipc/src/server/handlers/answer.rs`. Its existing test `server::handlers::answer::tests::answer_ticket_decoder_preserves_step_and_slot_boundaries` in `crates/vb_ipc/src/server/handlers/answer_tests.rs` asserts the full boundary matrix: `0 -> Step(0)`, `65_535 -> Step(u16::MAX)`, `65_536 -> Slot(0)`, `131_071 -> Slot(u16::MAX)`, and rejection of `131_072` and `u32::MAX`. This exact test passed in the focused `vb_ipc` run. Separate low-range decoder tests remain in `crates/vb_ipc/src/server/ticket.rs`; slot encoding tests remain in `answer_selector.rs`, asserting slot zero maps to `65_536` and `u16::MAX` maps to `131_071`.

The changed CLI tests replace successful legacy-selector parsing with exact `UnknownFlag` results for `--step` and `--value-file`. The public integration test attempts the retired flags against a genuine pending Ask in text, YAML, and Postcard modes; each invocation must exit 2, emit no stdout, report the exact unknown flag and canonical usage, and leave the complete observed run state, pending ticket, journal events, trace, counters, and queue depth unchanged. The same Ask then succeeds via its actual pending answer slot. These tests prove removal of the legacy CLI execution path while keeping the existing wire decoder and canonical slot protocol behavior.

## Exact integrity record proposal

The proposed row is scoped to the final composition base `91472dd5624f79c73b1376236a3ba232f2a44039`, old blob `343d0bb2563ac6dc3646b28d187e3ae8509d47b6`, and current blob `c89700cf1beb8398d60d9e12dd91622c4f3cb150`. The 13 owned source/test preimages in this change were checked byte-identical between the earlier test base `cff99b1c38b264d4e8a57a29a036882daf02cda6` and `91472dd5`. It records `WeakenedAssertion|removed_exact=2 added_exact=0 added_weak=0` as an intentional retirement under bead `vb-eirmk.75.8.25.72`, with replacement paths bound to their current blobs and this R3 review artifact hash-bound. No gate configuration has been modified.

## Observed verification

Baseline RED (source blobs shared with R3 base): `.evidence/runtime/answer-legacy-red/cli-red.log`, SHA256 `2e6cc255c2c0cc4f558f4f75bfaa44cec2a9a8f2e0b625de5ffc27d6c8cacb8a`; four old-behavior tests failed because the parser/metadata still accepted or described the retired flags.

After the implementation on the source-identical cff99 base, all 460 binary unit tests passed and the complete `velvet-ballistics` package suite passed 1,266 tests with zero skipped, including the updated agent-context snapshot. Public answer CLI E2E, workspace lifecycle refusal, BDD canonical-usage, and the exact IPC answer decoder boundary test passed. `moon run :lint-src`, `cargo fmt --all -- --check`, and `git diff --check` passed. The test-integrity gate on the final 91472 base currently fails only on the two intentionally retired Step encoding assertions and remains red until this R3 proposal is independently reviewed and an authorized integrity record is applied. The exact proposed hash-bound row is in `.evidence/runtime/answer-legacy-green/test-integrity-justification-proposal-r3.tsv`; it has not been applied to the protected ledger.

## Landing-base binding

The same reviewed retirement is bound to incoming main 52d1e7cfb48ccf2a30e6303f0d42de6eeca2688d. All thirteen CLI production/test preimages are byte-identical between the original reviewed base 91472dd5624f79c73b1376236a3ba232f2a44039 and this incoming main; incoming changes are preserved. Old/new selector and replacement-test blob identities remain unchanged. The only gate input addition is one exact hash-bound retirement record; scanner logic and existing records remain untouched. The package receipts above refer to the preserved source context; final joined canonical verification is required before landing.
