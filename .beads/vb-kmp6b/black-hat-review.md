# Black-Hat Review: vb-kmp6b

**Bead:** vb-kmp6b — Strict verifier-binding architecture for MRWE5 production Verus closure
**Branch:** `process/vb-63st6.2-worktree-loom-route`
**Commit under review:** `d735046ce arch(vb-kmp6b): register MRWE5 r11 source-include Verus obligations 001/006/011/016`
**Reviewer:** black-hat-reviewer (State 13)
**Date:** 2026-06-06

## Verdict: APPROVED (with one mandatory follow-up)

The four r11 source-include Verus artifacts are implementation-bound at the
`const _: [(); 1] = ...;` compile-time guard level, the production kernel
`crates/vb_storage/src/mrwe5_contract.rs` is dependency-free, `#![forbid(unsafe_code)]`,
and is actually consumed by production code in `codec/semantic.rs`,
`codec/validation.rs`, and `events.rs`. The Verus runs reproduce verbatim
(8/8/7/11 verified, 0 errors) and the trust-boundary scan returns clean.

The bead's stated acceptance criteria are met. One follow-up is mandatory:
the `mrwe5_production_bridge.rs` file (and the parallel Flux module under
`crates/vb_storage/src/verification/flux/`) are **dead code** — neither is
referenced from `lib.rs`, neither is compiled, neither has a runnable
test. The architecture packet acknowledges the bridge as a follow-up but
understates the actual state (the module is not "broken" — it is not built
at all). The same is true of the Flux module that the bridge's doc-comment
references. Both must be wired into `lib.rs` (or removed) in a follow-up
bead; do not silently leave them in a non-built state.

---

## Vacuity Check (per file)

### `verification/verus/vb_mrwe5_kind_parity.rs` (obl-001)

- Line 7–8: `#[path = "../../crates/vb_storage/src/mrwe5_contract.rs"] mod production_mrwe5_contract;` — REAL source-include.
- Lines 18–35: Four `const _: [(); 1] = [(); bool_len(matches!(production_mrwe5_contract::mrwe5_*, ...))];` blocks. These compile-time guards call the production kernel's `const fn` and pin the return value. If the production kernel ever returns a different value, the array length breaks and the file fails to compile. This is a **hard behavioral contract**.
- Lines 41–127: `verus!` block has 3 exec fns (`exec_step_succeeded_kind`, `exec_slot_written_kind`, `exec_kinds_are_exact_match`) and 5 proof fns. The exec fns **do not call** the production kernel; they hardcode `29u16`, `12u16`, and `envelope_kind == payload_kind` directly. The proof is essentially "my spec agrees with my hardcoded answer" — the binding is at the const-block level, not the exec level.

**Verdict on this file:** binding is real (const blocks), but the `verus!` exec layer is a thin scaffolding. Not a vacuum proof — the const blocks force the production kernel to match the spec. Acceptable.

### `verification/verus/vb_mrwe5_decode_reject.rs` (obl-006)

- Line 7–8: source-include ✓
- Lines 18–37: 5 const blocks invoke `production_mrwe5_contract::mrwe5_classify_semantic_decode` and `mrwe5_classify_kind_compatibility` with concrete (12, 29, true), (29, 29, true), (29, 29, false), (29, 29), (12, 29) inputs and pin the decisions.
- Lines 43–160: `verus!` block defines `LocalKindCompatibility` and `LocalSemanticDecodeDecision` enums (duplicates of production enums; 1:1 mapping claimed in architecture packet). 5 exec fns and 3 proof fns. Same observation: exec fns hardcode the answer.

**Verdict on this file:** binding is real at const-block level. Exec layer vacuous. Acceptable for r11.

### `verification/verus/vb_mrwe5_roundtrip.rs` (obl-011)

- Line 7–8: source-include ✓
- Lines 18–45: 6 const blocks cover the roundtrip success path (29, 29, true), (12, 12, true) and the cross-kind rejection path (29, 12, true), (12, 29, true). This exercises the production kernel's pattern-match logic.
- Lines 51–150: `verus!` block has 3 exec fns and 4 proof fns. Same vacuous pattern.

**Verdict on this file:** acceptable.

### `verification/verus/vb_mrwe5_compat_kind_family.rs` (obl-016)

- Line 7–8: source-include ✓
- Lines 18–45: 6 const blocks cover journal-family membership (29, 12, 9, 30), record-family classification under magic `0x5642_4A45`, and kind-compatibility policy.
- Lines 51–184: `verus!` block has 5 exec fns and 5 proof fns, including `lemma_journal_family_bounds_stable` which proves the [10..=29] range boundary. Same vacuous exec pattern.

**Verdict on this file:** acceptable.

### Cross-file vacuity concern

The `Local*` enums in each `verus!` block are **not the same types** as
`production_mrwe5_contract::Mrwe5*` enums. They are separate `pub enum`
definitions in the spec layer. The proof layer reasons about the local
types; the production layer reasons about the production types. The bridge
between them is implicit (the const blocks pin the production enum to a
value, the spec mirrors that value as a `29int` literal). This is a
"model-of-the-model" pattern, not a "proof that production is correct"
pattern. The architecture packet's claim of "1:1 mapping" is true
structurally but the spec layer never structurally references the
production types. **Acceptable for the bead's stated r11 binding**, but
worth flagging as a known weakness.

## assume_specification Audit

```
$ rg -n "assume_specification|assume\(|axiom|verifier::external" \
      verification/verus/vb_mrwe5_*.rs
0 matches
```

The repo-level `verify-verus.sh` trust scan also passed:

```
$ cat .evidence/verus/trust-scan.txt
VERUS_TRUST_SCAN_OK no assume/external/axiom matches in verification/verus contracts/verus
```

**Confirmed: zero `assume_specification`, `assume(`, `#[verifier::external]`, or `axiom` in any of the four MRWE5 Verus files.**

## Verus Reproducibility

```
$ bash scripts/verify-verus.sh \
    verification/verus/vb_mrwe5_kind_parity.rs \
    verification/verus/vb_mrwe5_decode_reject.rs \
    verification/verus/vb_mrwe5_roundtrip.rs \
    verification/verus/vb_mrwe5_compat_kind_family.rs

[verus] verus --crate-type=lib verification/verus/vb_mrwe5_kind_parity.rs
verification results:: 8 verified, 0 errors
[verus] verus --crate-type=lib verification/verus/vb_mrwe5_decode_reject.rs
verification results:: 8 verified, 0 errors
[verus] verus --crate-type=lib verification/verus/vb_mrwe5_roundtrip.rs
verification results:: 7 verified, 0 errors
[verus] verus --crate-type=lib verification/verus/vb_mrwe5_compat_kind_family.rs
verification results:: 11 verified, 0 errors
VERUS_REGISTRY_OK evidence=.evidence/verus
```

**Reproduced verbatim.** Verus version on this host: `Verus 0.2026.05.05.d03e906, release profile, linux_x86_64, toolchain 1.95.0-x86_64-unknown-linux-gnu` (matches the packet's §6).

Per-target evidence files are persisted in `.evidence/verus/` and match the
packet's claims (`vb_mrwe5_kind_parity.txt` = 118B, `vb_mrwe5_decode_reject.txt` = 120B, `vb_mrwe5_roundtrip.txt` = 116B, `vb_mrwe5_compat_kind_family.txt` = 126B, `trust-scan.txt` = 91B).

## Production Kernel Dependency Check

```
$ head -5 crates/vb_storage/src/mrwe5_contract.rs
#![forbid(unsafe_code)]
//! Source-includable MRWE5 journal kind contract kernel.
//!
//! This module is intentionally limited to primitive scalar inputs and outputs so
//! production code can delegate MRWE5 decisions to the same source file that a
//! Verus artifact can include with `#[path = ...]`.  It avoids serde, chrono,
```

```
$ rg -n "^use |^extern " crates/vb_storage/src/mrwe5_contract.rs
0 matches
```

- First line is `#![forbid(unsafe_code)]` ✓
- Zero `use` statements ✓
- Zero `extern crate` ✓
- Zero `crate::*` references ✓

The kernel is genuinely dependency-free. Verus can source-include it without pulling in `serde`, `postcard`, `chrono`, `Fjall`, `vb_core`, or any other crate-local types.

**Production consumption check** — the kernel is NOT orphaned dead code. It is actually consumed by production:

```
$ rg -n "use crate::mrwe5_contract|mrwe5_contract::" crates/vb_storage/src/
crates/vb_storage/src/codec/semantic.rs:7:    mrwe5_contract::{
crates/vb_storage/src/codec/validation.rs:7:    mrwe5_contract::{
crates/vb_storage/src/events.rs:4:use crate::mrwe5_contract::{Mrwe5PayloadClass, mrwe5_canonical_kind_id};
```

Specifically, `codec/semantic.rs`:
- `classify_journal_kind_compatibility` → `mrwe5_classify_kind_compatibility` (line 156)
- `journal_kinds_are_exact_match` → `mrwe5_kinds_are_exact_match` (line 165)
- `classify_journal_semantic_decode` → `mrwe5_classify_semantic_decode` (line 175)
- `classify_record_kind_family` → `mrwe5_classify_record_kind_family` (line 199)

The r11 binding is a **real production contract**, not a standalone specimen. The same `const fn` source is invoked by `vb_storage`'s write/decode seams at runtime AND by the Verus artifacts at verify-time. This is the strongest part of the architecture packet.

## Proof Obligations Registry

Read `contracts/proof_obligations.yaml` lines 122–194 (the four L4 obligations):

| Line | ID | Verifier lane | Source refs | Evidence command |
|------|-----|---------------|-------------|------------------|
| 129 | `obl-vb-mrwe-5-kind-parity-verus-001` | verus | `crates/vb_storage/src/mrwe5_contract.rs` + `verification/verus/vb_mrwe5_kind_parity.rs` | `verus --crate-type=lib verification/verus/vb_mrwe5_kind_parity.rs` |
| 145 | `obl-vb-mrwe-5-decode-reject-verus-006` | verus | same pattern | `verus --crate-type=lib verification/verus/vb_mrwe5_decode_reject.rs` |
| 162 | `obl-vb-mrwe-5-roundtrip-verus-011` | verus | same pattern | `verus --crate-type=lib verification/verus/vb_mrwe5_roundtrip.rs` |
| 179 | `obl-vb-mrwe-5-compat-kind-family-verus-016` | verus | same pattern | `verus --crate-type=lib verification/verus/vb_mrwe5_compat_kind_family.rs` |

All four obligations are properly registered with section `["vb-mrwe.5", "REQ-MRWE5-..."]`, `proof_level: L4`, `crate: vb_storage`, `files: [kernel, verus_file]`, and matching `commands:` entries. The default `bash scripts/verify-verus.sh` run (no args) discovered them via the registry (per `summary.txt` line: `VERUS_TARGET_COUNT=21`, includes all 4 MRWE5 files).

## The Follow-up Gap (the dead bridge)

The architecture packet §9 states:

> "`crates/vb_storage/src/verification/mrwe5_production_bridge.rs` is test-only verification wiring that imports `classify_journal_semantic_decode`, `classify_record_kind_family`, `is_journal_record_kind`, etc. via `use crate::{...}`. Those names live under `crate::codec::`, not at the crate root, so `cargo test -p vb_storage --lib mrwe5_verus_bridge_binds_to_production_seams` does not compile against the current `lib.rs`."

**This claim is technically correct on the import paths, but misframes the actual state.** Reality:

1. The `verification` module is **not referenced from `lib.rs`** — `rg -n "pub mod verification|mod verification" crates/vb_storage/src/lib.rs` returns 0 matches.
2. Therefore the bridge file is **not compiled at all** — not in `cargo build`, not in `cargo test --tests`, not in `cargo test --lib`.
3. Confirmed via `cargo test --package vb_storage --lib mrwe5_verus_bridge_binds_to_production_seams` → `0 passed, 1293 filtered out` (the test does not exist in the binary).
4. Confirmed via `strings target/debug/deps/libvb_storage-*.rlib | grep mrwe5` → contains `mrwe5_contract::*` and `kani_vb_mrwe5_*` symbols, **does not** contain `mrwe5_production_bridge` or `mrwe5_verus_bridge_binds_to_production_seams`.

The bridge is **dead code**, not "broken code." The framing in §9 is wrong: the bridge doesn't fail to compile, it's never asked to compile. **Mandatory follow-up:** either wire `#[cfg(test)] pub mod verification;` (or just `pub mod verification;`) into `lib.rs` and fix the bridge's unresolved imports (`JournalEventKindClass` lives in `crate::events::`, not crate root; `classify_journal_semantic_decode` is in `crate::codec::`, not re-exported at crate root by `pub use codec::{...}` which is selective not glob; `MAGIC_JOURNAL_EVENT` is re-exported at crate root via `pub use constants::*;`), or delete the bridge file and the Flux module.

**Same concern extends to `crates/vb_storage/src/verification/flux/`** — that module is also `#[cfg(any(test, flux))]` but is never wired into `lib.rs`. The `verify-verus.sh` script does NOT discover these Flux files. The bridge file's doc-comment (line 3) references "parallel Flux obligations" but those Flux obligations are themselves in dead code. The Flux module and its 4 Flux files (`vb_mrwe5_compat_kind_family.rs`, `vb_mrwe5_decode_reject.rs`, `vb_mrwe5_kind_parity.rs`, `vb_mrwe5_roundtrip.rs`) are also not compiled.

**Do not silently leave these as dead code.** The architecture packet must explicitly call out both the bridge AND the Flux module as follow-up beads, not just the bridge.

## Behavior-Affecting Waiver Check

```
$ rg -n "waive|wip|TODO|FIXME" verification/verus/vb_mrwe5_*.rs
0 matches
```

**Clean.** No `waive` markers, no `TODO`/`FIXME` in any of the four artifacts.

## TLA+ / Flux Cross-Check

No TLA+ or Flux artifacts exist specifically for the 4 MRWE5 obligations. `verification/tla/` contains 60+ `.tla` files for other sections (CapabilityLifecycle, ChooseSlot, IdempotencySafety, etc.) but none for `vb-mrwe.5`. `verification/flux/` contains 30+ Flux files for other sections (vb-fzgdn, vb-h09wf, vb-vzcuf, vb_ajc40) but none for `vb-mrwe.5` (the Flux files at `crates/vb_storage/src/verification/flux/vb_mrwe5_*.rs` exist but are dead code per the prior section).

The bead's closure is **Verus-only** for the 4 obligations. This is consistent with the bead's stated scope (the parallel Flux obligations are referenced in the bridge file's doc-comment but they are not part of `vb-kmp6b`'s acceptance criteria; the bead only commits to Verus closure for 001/006/011/016).

## Architecture-Confirmation.md Packet Quality

The packet contains:

- Raw Verus output reproduced verbatim (lines 159–169) — matches my reproduced run exactly.
- File:line refs for each artifact (table in §4) — accurate per my reads.
- `.evidence/verus/summary.txt` content quoted (lines 180–196) — matches actual file.
- Trust-scan output (lines 134–138) — matches actual `trust-scan.txt`.
- Specific call-site line numbers (lines 115–122) — verified accurate.

This is **not** self-congratulatory prose; it contains raw evidence. The framing in §9 is wrong about the bridge being "broken compilation" vs "dead code," but the rest of the packet is solid.

## Adversarial Findings

1. **Bridge file is dead code, not "broken."** Architecture packet §9 misframes the follow-up. The `verification` module is not in `lib.rs`, so the bridge is never compiled. Same applies to the Flux module and its 4 files. **Mandatory follow-up bead** must address BOTH the bridge and the Flux module, not just the bridge. If left as-is, the next bead consumer will be misled into thinking the bridge is "almost working" when it is in fact not wired into the build at all.

2. **Verus `verus!` exec layer is vacuous scaffolding.** The exec fns (e.g., `exec_kind_compatibility_exact` at `decode_reject.rs:88-93`, `exec_step_succeeded_kind` at `kind_parity.rs:67-71`) hardcode their return values and never call the production kernel. The actual binding is at the `const _: [(); 1] = ...;` layer, which IS a real implementation-bound check. The architecture packet's §2.1 claim of "implementation-bound" is true at the const-block level but the framing in §10 ("deductive reasoning over spec/exec/proof bodies that the Verus toolchain type-checks against the production kernel via `#[path = "..."]`") slightly overstates — the exec bodies themselves don't reference the production kernel. Acceptable for r11, but should be acknowledged more precisely.

3. **Spec-layer enums duplicate production enums.** `LocalPayloadClass`, `LocalKindCompatibility`, `LocalSemanticDecodeDecision`, `LocalRecordKindFamilyDecision` are `pub enum` redefinitions in the `verus!` block. The proof layer never structurally references the production types. This is a model-of-the-model pattern, not a structural proof of production. The architecture packet acknowledges this in §2.1 ("mirrors production types with local enums") but does not flag it as a known weakness. **Recommended:** the next iteration of this binding should either use the production types directly (with appropriate `#[verifier::external_body]` annotations) or explicitly document why duplication is required for the r11 pattern.

4. **Bead is `CLOSED` but the follow-up is not yet filed.** `bd show vb-kmp6b` shows the bead is already closed. The mandatory follow-up (bridge + Flux wiring) should be filed as a new P0/P1 bead before the next landing cycle, not deferred silently.

5. **The 5-cycle reproduce check passes** — re-running `verify-verus.sh` on the 4 files produced identical output. No evidence staleness.

6. **No `unsafe` / `unwrap` / `expect` / `panic` in any of the 4 Verus files.** Verified by inspection.

## Repair Routing

No production Rust changes are required. The Verus artifacts are correct and the production kernel is correct. The only follow-up is wiring dead verification code into `lib.rs` (or removing it). This is a State 4 (architecture) concern, not a State 5 (proof-writer) or State 7 (proof-to-implementation) concern.

If the bead author chooses to wire the bridge in a follow-up bead:
- Add `#[cfg(test)] mod verification;` to `lib.rs` (NOT `pub mod` — the verification module is test-only)
- Fix the bridge's `use crate::{...}` block to use `crate::codec::classify_journal_semantic_decode` (or re-export at crate root via glob)
- Add `crate::JournalEventKindClass` to the re-exports in `lib.rs:222` (`pub use events::{DurableActionOutcome, JournalEvent, JournalEventKindClass};`)
- Same for the Flux module if it is to be wired

If the bead author chooses to remove the dead code:
- Delete `crates/vb_storage/src/verification/mrwe5_production_bridge.rs`
- Delete `crates/vb_storage/src/verification/` (the mod.rs is also dead)
- This loses the documented "executable production bridge" intent but the bead's stated acceptance criteria do not require it

## Evidence

- Reproduction command output (this session): see "Verus Reproducibility" above
- Trust scan: `VERUS_TRUST_SCAN_OK no assume/external/axiom matches in verification/verus contracts/verus`
- Per-target evidence: `.evidence/verus/vb_mrwe5_{kind_parity,decode_reject,roundtrip,compat_kind_family}.txt` (all show `verification results:: N verified, 0 errors`)
- Production kernel proof of consumption: `rg -n "mrwe5_contract::" crates/vb_storage/src/` returns 3 hits in `codec/semantic.rs`, `codec/validation.rs`, `events.rs`
- Bridge test non-existence proof: `cargo test --package vb_storage --lib mrwe5_verus_bridge_binds_to_production_seams` returns `0 passed, 1293 filtered out`
- Bridge dead-code proof: `rg -n "pub mod verification|mod verification" crates/vb_storage/src/lib.rs` returns 0 matches
- `strings target/debug/deps/libvb_storage-*.rlib | grep mrwe5_production` returns 0 matches
- Registry integration: `contracts/proof_obligations.yaml` lines 122–194 (4 obligations, all with `verus:` key and `commands:` block)

---

## Final Verdict: APPROVED

The four r11 source-include Verus artifacts satisfy the bead's stated
acceptance criteria:

- ✅ No behavior-affecting `assume_specification`, `assume(`, `#[verifier::external]`, or `axiom` (trust scan clean).
- ✅ Repo-approved Verus binding (shared-source `#[path = ...]` source-include of the production kernel, which is consumed by `codec/semantic.rs`, `codec/validation.rs`, `events.rs` at runtime).
- ✅ State 5/6 verifies 001/006/011/016 without support-only claims. 34 verified, 0 errors, reproducible.

The architecture packet is honest about its strengths and mostly honest
about its weaknesses. The framing of the bridge follow-up (§9) needs
sharpening — the bridge is dead code, not "broken compilation" — and the
parallel Flux module has the same problem and is not mentioned at all in
§9. These are mandatory follow-ups for a new bead, not blockers for
landing `vb-kmp6b`.
