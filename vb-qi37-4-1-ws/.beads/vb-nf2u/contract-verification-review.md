# Contract Verification Review

STATUS: APPROVED

## Files Reviewed
- `.beads/vb-nf2u/contract.md`
- `.beads/vb-nf2u/lean-contract.md`
- `.beads/vb-nf2u/verification-layers.md`
- `.beads/vb-nf2u/proof-obligations.jsonl`
- `.beads/vb-nf2u/traceability-matrix.jsonl`
- `.beads/vb-nf2u/codebase-map.md`
- prior `.beads/vb-nf2u/contract-verification-review.md`

## Command Evidence
- `test -s .beads/vb-nf2u/contract.md && test -s .beads/vb-nf2u/lean-contract.md && test -s .beads/vb-nf2u/verification-layers.md && test -s .beads/vb-nf2u/proof-obligations.jsonl && test -s .beads/vb-nf2u/traceability-matrix.jsonl && jq -c . .beads/vb-nf2u/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-nf2u/traceability-matrix.jsonl >/dev/null` -> exit 0; required artifacts are non-empty and both JSONL files parse as one JSON object per line.

## Findings
- None blocking.

## Prior Blocker Re-check
- `PRE-004` traceability: fixed. `contract.md:22` defines the denylist precondition; `proof-obligations.jsonl:4` covers `PRE-004`; `traceability-matrix.jsonl:2` now traces `PRE-004` with denylist omission/fail-closed tests, fuzz/Bolero, `ai-release`, mutation, and evidence.
- Per-error-variant fail-closed scenarios: fixed. `contract.md:40-49` defines nine `UiReleaseGateError` variants; `proof-obligations.jsonl:22-30` contains one obligation per variant with `negative_scenario`, `expected_error`, diagnostic shape, command, and evidence; `traceability-matrix.jsonl:6-14` maps each variant separately.
- Kani/numeric coverage for layout/readability predicates: fixed. `verification-layers.md:15,29-30` requires Kani for layout predicates; `proof-obligations.jsonl:17-21` separately covers overlap, clipping, bounds, chip readability, and selected-state predicates with `cargo kani -p vb_ui_snapshot layout_predicates`.
- Structured waivers: fixed. `verification-layers.md:38-78` has structured waivers with clause IDs, waived layers, reasons, compensating evidence, owners, and expiry/follow-up. `lean-contract.md:11-35` also provides Lean waivers with owner/reason/expiry/compensating evidence.
- Release-critical command ambiguity: fixed. `contract.md:15,27` and `verification-layers.md:5-6,26-27` make `cargo xtask ai-release --bead vb-nf2u` the single required UI release entrypoint. `proof-obligations.jsonl:7` requires `moon run :verify-all` for release-critical coverage, and `verification-layers.md:36` makes the five-lane gauntlet an unconditional evidence boundary.

## Coverage Decision
- Contract clauses traced: approved for all preconditions (`PRE-001`..`PRE-004`), postconditions (`POST-001`..`POST-006`), invariants (`INV-001`..`INV-006`), and error variants.
- Lean-owned clauses covered: approved. `contract.md:62-63` and `lean-contract.md:3-10,37-38` explicitly define no Lean-owned kernel and keep Lean out of UI/I/O/runtime shell behavior.
- Proof obligations traced: approved. `proof-obligations.jsonl:1-31` and `traceability-matrix.jsonl:1-15` cover release gates, redaction, determinism, layout/readability, per-error variants, and Lean waiver.
- Lean scope valid: approved; no Lean proof is claimed over filesystem, image codecs, Makepad rendering, wall-clock time, Cargo/Moon command execution, or release aggregation (`lean-contract.md:3-10,40-42`).
- Fuzz/Bolero for parser/codec/protocol/hostile input: approved for redaction hostile artifacts via `verification-layers.md:13,31` and `proof-obligations.jsonl:13`.
- Concurrency coverage: approved by structured waiver `WAIVE-CONCURRENCY-UI-RELEASE` at `verification-layers.md:48-54`; it expires if tasks, threads, channels, shared mutable state, or cancellation are introduced.
- Release-critical coverage: approved; `verification-layers.md:36` and `proof-obligations.jsonl:7` require gauntlet/all evidence with no passing-by-absence.
- Waivers valid: approved; reviewed waivers contain required clause ID, layer, reason, compensating evidence, owner, and expiry/follow-up.
