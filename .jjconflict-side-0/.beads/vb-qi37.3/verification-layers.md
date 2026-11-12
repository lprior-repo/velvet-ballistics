# Verification Layers: vb-qi37.3

## Boundary
- Verus-owned kernel: pure/total collect-state transition model, cursor bounds, page identity classification, hydration identity validation, and collect-extra schema classifier.
- TLA+ temporal model: collect wait/suspend/replay/recovery/resume lifecycle and journal/frame/side-table coherence over time.
- Theorem projection: none; Lean/Aeneas/Hax waived unless State 4 creates a tiny codec refinement kernel beyond Verus.
- Runtime shell: shard ownership, evidence draining, Fjall append/read, wall-clock time, and value-store allocation are verified with gauntlet, exact tests, Miri/proptest/fuzz, and static scans.
- External systems excluded from formal proof: OS clock, Fjall internals, cargo/Moon execution, and postcard crate internals.

## Layer Assignment
- PRE-001 -> exact runtime test `collect_start_returns_error_when_source_is_not_list` + mutation/coverage in release gauntlet.
- PRE-002 -> temporary Verus waiver + exact runtime tests + proptest waiver.
- PRE-003 -> temporary Verus waiver + admission/bounds tests + proptest waiver.
- PRE-004 -> temporary Verus waiver + temporary TLA+ waiver + exact duplicate/stale/out-of-order scenarios.
- PRE-005 -> codec fuzz waiver + exact corrupt/identity mismatch recovery tests + static schema review + temporary TLA+ waiver.
- PRE-006 -> temporary TLA+ waiver + exact cross-crate recovery integration tests.
- PRE-007 -> exact EvidenceCollector capacity tests + mutation/coverage.
- POST-001..POST-005 -> temporary Verus waiver + exact collect lifecycle tests.
- POST-006 -> temporary TLA+ waiver + exact storage journal round-trip tests + codec fuzz waiver.
- POST-007 -> temporary TLA+ waiver + exact cross-crate recovery/resume tests.
- POST-008 -> temporary Verus waiver + exact typed-error tests + mutation waiver.
- INV-001..INV-005 -> temporary Verus waiver + temporary TLA+ waiver for over-time preservation, including source identity/item-count stability across wait/ask/replay/resume + exact recovery tests + proptest waiver.
- INV-006 -> temporary TLA+ waiver + strict/journaled durability evidence tests.
- INV-007 -> codec fuzz/proptest waiver + exact corrupt/non-collect/mismatched extra tests + temporary TLA+ waiver.
- INV-008 -> temporary TLA+ waiver + exact cross-crate recovery integration tests.
- INV-009 -> exact EvidenceCollector capacity tests + static scan for silent drop path.
- INV-010 -> temporary Verus waiver + proptest waiver + exact time-limit/value-store tests + Miri/gauntlet.
- ERR-001..ERR-003 -> exact tests + mutation.
- ERR-004..ERR-008 -> typed-error API contract tests + temporary Verus waiver for classifier + mutation waiver.

## Verus Scope
- Current status: no collect Verus proof files exist in this repo (`verification/verus/*.rs` absent).
- Required abstract model target: `CollectKey = (RunId, SlotIdx)`, `CollectState`, `PageObservation`, and `CollectTransition` over finite abstract lists.
- Required proof claims:
  - state transition preserves key isolation and bounds;
  - valid next advances monotonically and by no more than page size;
  - duplicate/stale/out-of-order classifier is total and state-preserving on rejection;
  - hydration identity validation accepts only matching `(run_id, collector_slot)`;
  - collect-extra classifier does not decode non-collect extras as collect states.
- Trusted boundary: constructors that validate `RunId`, `SlotIdx`, `ListId`, resource limits, and decoded postcard bytes before entering the Verus model.
- Shell exclusions: `RunFrame`, `ValueStore` allocation, `SystemTime`, Fjall I/O, async/shard scheduling, and postcard implementation.
- Waiver evidence command: `env VERIFY_BEAD_ID=vb-qi37.3 ALLOW_BEAD_LOCKBUD_WAIVER=1 bash scripts/rust-verification-gauntlet.sh all` plus exact nextest commands in `proof-obligations.jsonl`.
- Future non-waived command shape: exact `verus verification/verus/<collect_target>.rs` or repo-approved proof lane after proof target exists; do not invent this file in State 3.

## TLA+ Scope
- Current status: no collect-specific TLA module/config exists; existing TLA modules do not semantically cover collect pagination source stability or collect-extra hydration.
- Required model variables/actions/properties are listed in `tla-spec.md`.
- Waiver evidence command: `env VERIFY_BEAD_ID=vb-qi37.3 ALLOW_BEAD_LOCKBUD_WAIVER=1 bash scripts/rust-verification-gauntlet.sh all` plus exact recovery/resume nextest commands in `proof-obligations.jsonl`.
- Future non-waived command shape: exact `tlc -config specs/tla/CollectPagination.cfg specs/tla/CollectPagination.tla` or equivalent once the model exists.

## Concrete Existing Commands Available
- `moon run :verify-proof` -> `bash scripts/rust-verification-gauntlet.sh proof`.
- `moon run :verify-deep` -> `bash scripts/rust-verification-gauntlet.sh deep`.
- `moon run :verify-standard` -> `bash scripts/rust-verification-gauntlet.sh standard`.
- `moon ci` is canonical repository CI per `AGENTS.md`.
- Release-critical direct all-mode command for this bead: `env VERIFY_BEAD_ID=vb-qi37.3 ALLOW_BEAD_LOCKBUD_WAIVER=1 bash scripts/rust-verification-gauntlet.sh all`.
- Exact smallest-scope test commands are enumerated per obligation in `proof-obligations.jsonl`; avoid broad collect module commands when a named test exists.

## Waivers / Known Contract Gaps
- TLA-WAIVER-COLLECT-001: No collect TLA model/config exists. Owner: State 6 implementer; approval owner: State 4 reviewer. Expiry: before release-critical acceptance of `vb-qi37.3` or 2026-05-18. Limitation: no exhaustive temporal proof of wait/ask/replay/resume interleavings, including INV-005 source stability. Compensating evidence: exact recovery/resume/stale/duplicate/time-limit nextest commands plus direct all-mode gauntlet.
- VERUS-WAIVER-COLLECT-001: No collect Verus proof target exists. Owner: State 6 implementer; approval owner: State 4 reviewer. Expiry: before release-critical acceptance of `vb-qi37.3` or 2026-05-18. Limitation: tests do not prove all abstract cursor, key, classifier, and schema states. Compensating evidence: exact runtime nextest commands, proptest waiver evidence, mutation/API waiver evidence, and all-mode gauntlet.
- GAP-ERR-001: Existing `EngineError` lacks dedicated stale/duplicate/out-of-order collect page variants; contract requires typed errors and exact tests must fail until implemented.
- GAP-EXTRA-001: Existing `SlotWrittenEvent.extra` conflates collect extra and taint fallback bytes; contract requires schema separation/filtering and exact corrupt/identity tests.
- GAP-EVIDENCE-001: Existing `EvidenceCollector` comments and behavior allow silent drops at capacity; contract requires no silent loss of required collect extra.
