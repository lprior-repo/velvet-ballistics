# Verifier Lane Matrix — vb-svvr7

## Bead: vb-svvr7 — IPC: reject trailing bytes in CLI postcard frame decoder (P1 bug)

Maps each proof seed to its assigned verifier lanes, with the (req, cc, seed, verifier) tuple, the limitation_kind for non-applicable lanes, and the paired proof-obligation ID.

## Lane Profile (Default)

| Verifier | Required? | Rationale |
|---|---|---|
| `proptest` | yes | Primary lane for the property claim (PS-TB-01). Property-based pressure over arbitrary `[0, MAX_PAYLOAD] x [1, 4096]` with shrink. |
| `cargo-test` | yes | Primary lane for unit-test claims (PS-TB-02, PS-TB-03, PS-TB-04, PS-TB-07). Discriminant + format! + exact equality + json-propagation. |
| `cargo-clippy` | yes | Required by contract.md AC-9; cross-crate parity lock against `vb_ipc/src/frame.rs:44`. |
| `source-lint` | yes | Required by contract.md AC-10 and AGENTS.md canonical CI gate (`moon run :source-lint`). |
| `verus` | no | Vacuum Verus is rejected per AGENTS.md GOD RULE 2. |
| `kani` | no | Optional per contract.md PS coverage targets line 100; proptest is the primary evidence. |
| `flux-rs` | no | No refinement surface; single integer compare. |
| `loom` | no | No concurrency surface; pure single-threaded function. |
| `miri` | no | `unsafe_code = forbid` at workspace level. |
| `cargo-fuzz` | no | No fuzz target exists; proptest covers arbitrary trailing lengths. |

## (Requirement, Contract Clause, Proof Seed) → Lane Decisions

| Proof Seed | Req ID | CC | Proptest | Cargo-test | Cargo-clippy | Source-lint | Verus | Kani | Flux | Loom | Miri | Fuzz |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| PS-TB-01 | REQ-TB-STRICT-LENGTH | CC-TB-1 | ✅ PO-TB-PROP-01 | — | — | — | ❌ NA | ❌ NA | ❌ NA | ❌ NA | ❌ NA | ❌ NA |
| PS-TB-02 | REQ-TB-VARIANT-SHAPE | CC-TB-4 | — | ✅ PO-TB-UNIT-01 | — | — | ❌ NA | — | — | — | — | — |
| PS-TB-03 | REQ-TB-EXACT-LENGTH | CC-TB-1 | (covered by PS-TB-01) | (covered by PS-TB-02) | — | ✅ PO-TB-LINT-01 | — | — | — | — | — | — |
| PS-TB-04 | REQ-TB-JSON-PROPAGATION | CC-TB-6 | (covered by PS-TB-01) | (covered by PS-TB-02) | — | — | — | — | — | — | — | — |
| PS-TB-05 | REQ-TB-ENCODER-EXACT-LENGTH | CC-TB-7 | — | — | ✅ PO-TB-CLIPPY-01 | (covered by PS-TB-03) | — | — | — | — | — | — |
| PS-TB-06 | REQ-TB-HOSTILE-INPUT | CC-TB-1 | (covered by PS-TB-01) | — | — | — | — | — | — | — | — | — |
| PS-TB-07 | REQ-TB-DISPLAY-EXHAUSTIVE | CC-TB-5 | — | (covered by PS-TB-02) | — | — | — | — | — | — | — | — |
| PS-TB-08 | REQ-TB-CROSS-CRATE-PARITY | CC-TB-9 | — | — | (covered by PS-TB-05) | — | — | — | — | — | — | — |

### Legend

- ✅ = required lane; pairs with a `proof-obligation/v1` ID.
- ❌ NA = not_applicable; the cell in the `verifier-lane-decisions.jsonl` carries `non_applicability_evidence_refs` and a typed `limitation_kind`.
- (covered by …) = the seed is discharged by a paired seed's primary obligation; the cell is informational, not a separate obligation.
- — = not part of the default profile for this seed (verifier does not apply to the seed's risk class).

## Not-Applicable Detail

| Verifier | ID | limitation_kind | Decision reason (truncated) |
|---|---|---|---|
| `verus` | VLD-TB-05 | `surface_absent` | Vacuum Verus is rejected per GOD RULE 2; the cli_postcard module has no production-bound spec; the property is a single integer compare fully discharged by proptest + cargo-test. |
| `kani` | VLD-TB-06 | `superseded_by_other_lane_with_evidence` | Optional per contract.md PS coverage targets; proptest over 10000 cases of `[0, MAX_PAYLOAD] x [1, 4096]` is stronger than a Kani bounded proof for a 32-line single-compare function. |
| `flux-rs` | VLD-TB-07 | `surface_absent` | Single `data.len() != payload_end` compare; no refinement surface; cargo-flux is not pinned. |
| `loom` | VLD-TB-08 | `surface_absent` | Pure single-threaded function over `&[u8]`; no Mutex/RwLock/Arc/thread/spawn/channel/async. |
| `miri` | VLD-TB-09 | `surface_absent` | `unsafe_code = forbid` at workspace level; no `unsafe` block in cli_postcard. |
| `cargo-fuzz` | VLD-TB-10 | `superseded_by_other_lane_with_evidence` | No fuzz target exists for `vb_cli::cli_postcard`; proptest covers arbitrary trailing lengths. |

## Required-Lane Detail

| ID | Verifier | Req ID / CC | Seed | Obligation | Tool / version |
|---|---|---|---|---|---|
| VLD-TB-01 | proptest | REQ-TB-STRICT-LENGTH / CC-TB-1 | PS-TB-01 | PO-TB-PROP-01 | proptest@1.5, PROPTEST_CASES=10000, --release |
| VLD-TB-02 | cargo-test | REQ-TB-VARIANT-SHAPE / CC-TB-4 | PS-TB-02 | PO-TB-UNIT-01 | cargo-test@nightly-2026-04-28 |
| VLD-TB-03 | cargo-clippy | REQ-TB-CROSS-CRATE-PARITY / CC-TB-9 | PS-TB-08 | PO-TB-CLIPPY-01 | clippy@nightly-2026-04-28, --all-targets, -D warnings |
| VLD-TB-04 | source-lint | REQ-TB-PRESERVE-MAGIC-HEADER-VALIDATION / CC-TB-10 | PS-TB-03 | PO-TB-LINT-01 | moon@2.2.4, run :lint-src |

## Cross-Reference With Risk Taxonomy

| Tag | Triggered Verifier | Status |
|---|---|---|
| `rust_local` | proptest (primary) | VLD-TB-01 ✅ |
| `parser` | cargo-test + proptest | VLD-TB-01 + VLD-TB-02 ✅ |
| `codec` | cargo-test + proptest | VLD-TB-01 + VLD-TB-02 ✅ |
| `rejection` | proptest (primary) | VLD-TB-01 ✅ |
| `public_api` | cargo-clippy + source-lint | VLD-TB-03 + VLD-TB-04 ✅ |
| `hostile_input` | proptest (primary); cargo-fuzz (optional) | VLD-TB-01 ✅; cargo-fuzz not_applicable with evidence |
| `user_visible_behavior` | n/a (downgraded to `behavior_affecting: false` on obligations per bead-scope policy) | not_applicable for proof purposes; behavior is locked by the proptest + unit-test obligations |

## Existing Verification Coverage (Context Only — No Edits)

| Artifact | Description | Coverage |
|---|---|---|
| `crates/vb_cli/src/cli_postcard/tests.rs:1-197` | 17 existing unit tests covering magic, header length, payload too large, kind, version, CRC, digest, truncation, encoder shape, roundtrip | PS-TB-02, PS-TB-03, PS-TB-05, PS-TB-07 (partial; pre-fix baseline) |
| `verification/proptest/properties.rs:1-369` | 11 proptest groups covering roundtrip, header layout, CRC rejection, digest rejection, bad magic, oversized payload, wrong kind, invalid schema version, schema version roundtrip, content type, all kinds | PS-TB-01 (none; new property) |
| `crates/vb_ipc/src/frame.rs:35-51` | IPC sibling decoder using `!=` length check | PS-TB-08 (parity reference; not edited) |
| `fuzz/fuzz_targets/` | No cli_postcard target exists | n/a (cargo-fuzz not_applicable) |
| `verification/verus/` | No TrailingBytes spec; no cli_postcard spec | n/a (verus not_applicable) |
