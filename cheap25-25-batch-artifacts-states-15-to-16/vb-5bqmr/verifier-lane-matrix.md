# Verifier Lane Matrix — vb-5bqmr

## Bead

`vb-5bqmr` — SlotExtra: reject unknown VBSE versions instead of legacy downgrade (P1 bug)

## Matrix legend

- **K** = Kani (bounded symbolic, CBMC)
- **V** = Verus (rust-local / pure_core invariant)
- **F** = Flux-RS (refinement type)
- **P** = Proptest (property pressure)
- **—** = not_applicable (with concrete evidence in `verifier-lane-decisions.jsonl`)
- **W** = waived (this bead has no behavior-affecting waivers; non-behavior rows only)

The user's explicit lane instruction is **Lanes: rust-local, kani, flux-rs, proptest** — that
is `{verus, kani, flux-rs, proptest}` because the `rust-local` tag is the
risk-taxonomy pointer to Verus (see `references/risk-taxonomy.md` universal profile).

## Coverage matrix

| # | Proof Seed ID | Requirement | Clause | Hazard(s) | V | K | F | P | Planned Obligations |
|---|---|---|---|---|---|---|---|---|---|
| 1 | `vb-5bqmr.ps.v1-decode-version-mismatch` | `vb-5bqmr-C-DEC-002` | C-DEC-002 | H-001, H-014 | V | K | F | P | PO-VERUS-001, PO-KANI-001, PO-FLUX-001, PO-PROP-001 |
| 2 | `vb-5bqmr.ps.v1-decode-v1` | `vb-5bqmr-C-DEC-001` | C-DEC-001 | H-006, H-016 | V | K | F | P | PO-VERUS-001, PO-KANI-002, PO-FLUX-001, PO-PROP-002 |
| 3 | `vb-5bqmr.ps.v1-decode-legacy` | `vb-5bqmr-C-DEC-003` | C-DEC-003 | H-005, H-008 | V | K | — | P | PO-VERUS-001, PO-KANI-002, PO-PROP-002 |
| 4 | `vb-5bqmr.ps.v1-decode-exclusivity` | `vb-5bqmr-C-DEC-004` | C-DEC-004 | H-001 | V | K | F | — | PO-VERUS-001, PO-KANI-002, PO-FLUX-001 |
| 5 | `vb-5bqmr.ps.v1-constant-composition` | `vb-5bqmr-C-CON-001` | C-CON-001 | H-007 | — | K | F | P | PO-KANI-002, PO-FLUX-001, PO-PROP-002 |
| 6 | `vb-5bqmr.ps.v1-err-version-mismatch-shape` | `vb-5bqmr-C-ERR-001` | C-ERR-001 | H-013 | — | — | — | P | PO-PROP-002 |
| 7 | `vb-5bqmr.ps.v1-err-found-unreachable` | `vb-5bqmr-C-ERR-002` | C-ERR-002 | H-001 | V | K | — | P | PO-VERUS-001, PO-KANI-001, PO-PROP-001 |
| 8 | `vb-5bqmr.ps.v1-rec-exhaustive` | `vb-5bqmr-C-REC-001` | C-REC-001 | H-003, H-013 | — | — | — | P | PO-PROP-003 |
| 9 | `vb-5bqmr.ps.v1-rec-version-mismatch-translation` | `vb-5bqmr-C-REC-002` | C-REC-002 | H-009 | — | — | — | P | PO-PROP-003 |
| 10 | `vb-5bqmr.ps.v1-rec-no-recoveryerror-widening` | `vb-5bqmr-C-REC-004` | C-REC-004 | H-003 | — | — | — | — | (compile-time; covered by `cargo build --all-targets -p vb_storage` in PO-PROP-003) |
| 11 | `vb-5bqmr.ps.v1-run-exhaustive` | `vb-5bqmr-C-RUN-001` | C-RUN-001 | H-004, H-013 | — | — | — | P | PO-PROP-003 |
| 12 | `vb-5bqmr.ps.v1-run-version-mismatch-translation` | `vb-5bqmr-C-RUN-002` | C-RUN-002 | H-009 | — | — | — | P | PO-PROP-003 |
| 13 | `vb-5bqmr.ps.v1-run-kind-variant-additive` | `vb-5bqmr-C-RUN-004` | C-RUN-004 | H-004 | — | — | — | — | (compile-time; covered by `cargo build --all-targets -p vb_runtime` in PO-PROP-003) |
| 14 | `vb-5bqmr.ps.v1-enc-roundtrip` | `vb-5bqmr-C-ENC-002` | C-ENC-002 | H-014 | — | K | — | P | PO-KANI-002, PO-PROP-002 |
| 15 | `vb-5bqmr.ps.v1-bdd-legacy-regression` | `vb-5bqmr-C-NEG-001` | C-NEG-001 | H-005 | — | K | — | P | PO-KANI-002, PO-PROP-002 |
| 16 | `vb-5bqmr.ps.v1-bdd-corrupt-v1-regression` | `vb-5bqmr-C-NEG-003` | C-NEG-003 | H-006 | — | K | — | P | PO-KANI-002, PO-PROP-002 |
| 17 | `vb-5bqmr.ps.v1-zero-alloc-legacy` | `vb-5bqmr-C-NEG-006` | C-NEG-006 | H-008 | — | K | — | — | PO-KANI-002 |
| 18 | `vb-5bqmr.ps.v1-no-catchall-vb-storage` | `vb-5bqmr-C-FOR-001` | C-FOR-001 | H-013 | — | — | — | P | PO-PROP-003 |
| 19 | `vb-5bqmr.ps.v1-no-catchall-collect` | `vb-5bqmr-C-FOR-002` | C-FOR-002 | H-013 | — | — | — | P | PO-PROP-003 |
| 20 | `vb-5bqmr.ps.v1-forward-compat-monotone` | `vb-5bqmr-H-014` | C-FOR-003 | H-014 | — | — | — | — | (process; tracked in `vb-1rqz7.*`) |

## Cross-lane rationale

- **Verus** owns the **for-all** discriminant proof — for arbitrary
  `bytes: &[u8]`, the three-arm classification is mutually exclusive and
  exhaustive, and `VersionMismatch { found: 0x01 }` is unreachable.
- **Kani** owns the **bounded symbolic** partition proof — for
  `len ∈ [0, KANI_BOUND_BYTES]` (256 bytes), each arm is reachable and the
  classification is exact. Kani also proves the legacy arm allocates zero
  bytes (C-NEG-006 reachability via `kani::cover!`).
- **Flux-RS** owns the **refinement** — the prefix-length relationship is a
  compile-time constant 5 = MAGIC.len() + 1 (C-CON-004), and the prefix
  constant `SLOT_WRITTEN_EXTRA_PREFIX` is compositionally derived from
  `SLOT_WRITTEN_EXTRA_MAGIC` and `SLOT_WRITTEN_EXTRA_VERSION` (C-CON-001).
- **Proptest** owns the **property pressure** — over a strategy-generated
  input space, the unknown-version rejection, the encode/decode round-trip,
  the legacy-byte and corrupt-v1 anti-invariants, and the hydrate/collect
  translation paths (including the `tracing::warn!` log capture).

## Not-applicable evidence

The following lanes are deliberately NOT emitted with `not_applicable` rows in
this plan because the user-explicit lane instruction **Lanes: rust-local,
kani, flux-rs, proptest. No loom (sync), no fuzz** already constrains the
lane set to the four listed. Loom and cargo-fuzz are excluded by
instruction (not by risk absence); Miri is excluded by `forbid(unsafe_code)`.

The four-lane set is the **complete coverage** for this bead per the user's
explicit instruction. No default-profile verifier is silently omitted.

## Risk-class → lane mapping

| Risk class | Required lanes | This plan |
|---|---|---|
| `bounded_transition` | `kani` + `verus` | covered (PO-KANI-001/002, PO-VERUS-001) |
| `rejection` | `kani` + `proptest` | covered (PO-KANI-001, PO-PROP-001) |
| `refinement` | `flux-rs` + `verus` | covered (PO-FLUX-001, PO-VERUS-001) |
| `equality` (round-trip) | `proptest` + `verus` | covered (PO-PROP-002, PO-VERUS-001) |

Defense depth per the user-stated 6-8 obligation budget: 7 obligations across
4 lanes, well-distributed.

## Out-of-scope (by user instruction)

- **loom** — sync function; no concurrency surface. Excluded by `No loom (sync)`.
- **cargo-fuzz** — fuzz target gap (RED QUEEN §M3, `vb-1rqz7.15`) is a
  separate bead. Excluded by `No fuzz (RED QUEEN §M3 separately)`.
- **miri** — `crates/vb_storage/src/slot_extra.rs:1` carries
  `#![forbid(unsafe_code)]`. No UB surface.
- **tla-plus** — temporal_workflow removed from this skill; no temporal surface
  in this bead anyway.

These are user/instruction-bounded exclusions, not risk-based `not_applicable`
rows; they do not require evidence refs in `verifier-lane-decisions.jsonl`
because the lane decision set is already constrained to four.