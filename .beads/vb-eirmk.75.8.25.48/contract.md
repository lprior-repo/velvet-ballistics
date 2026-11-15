# Recover canonical answer slot selector through pending Ask authority

## Contract

Implement the existing MASTER §69 `answer <run-id> --slot <slot-id> --value <file> --db <path>` behavior without changing persisted formats, public IPC variants, admission, or effect authority. The current `--step`/`--value-file` path remains compatible and preserves its byte encoding, value-file decoding, taint and size checks, error precedence, output formats, and rejection side effects.

Use a private typed Step/Slot selector and checked encoding in the existing `AnswerAsk.ticket` scalar only. Step values `0..=65535` keep their existing wire values. Slot values `0..=65535` map to the disjoint range `65536..=131071`; all other encoded values are rejected. Mixed, duplicate, malformed, or overflowing CLI selectors are rejected through existing typed errors. Do not change the shared action-ticket decoder.

The server resolves a slot selector only against actual pending Ask authority belonging to the addressed run. Reuse the actor-owned run state/workflow and `lookup_pending_ask_ticket`; require exactly one pending ticket whose `answer_slot` equals the selected slot, after existing step/frame/compiled-resume/timer checks. Do not infer a step from the slot, assume root PC identifies every nested Ask, construct a ticket, add a public API, or use an unchecked/default mapping.

## Acceptance evidence

- Registered public CLI tests exercise the genuine issued step-3/slot-2 fixture: `--slot 2 --value <Postcard file>` succeeds, the retained `AskAnswered` is step 3 / slot 2, and existing `--step` positive behavior remains unchanged.
- Rejections for wrong slot, unknown/stale/foreign run, duplicate/mixed selectors, and out-of-range encodings remain read-only: no answer event, slot write, or queue mutation.
- Handler tests cover exact slot selection for real owned ForEach/nested Ask call paths and bounds `0`, `65535`, `65536`, `131071`, `131072`, and `u32::MAX`.
- Existing three-format, value-file, taint, payload-size, and error-path tests remain intact. Tests assert public behavior and actual journal/state effects, not a copied projection.
- New/modified Rust functions remain at most 25 raw lines and five parameters; changed Rust files remain at most 300 lines. No lint allowances, public API/schema/dependency/gate edits, benchmark claims, snapshot updates, or GPU work.

## Non-goals

Do not repair or reinterpret the separately retained second cold-start visitor refusal. Do not alter snapshot proposals or the integrity ledger. Independent CODE and TEST reviews are required before landing.
