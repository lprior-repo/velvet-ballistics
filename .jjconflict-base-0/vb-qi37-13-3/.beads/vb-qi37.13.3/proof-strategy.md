# Proof Strategy: vb-qi37.13.3 — CLI Text/YAML/Postcard Emitters

## Scope
`crates/vb_ui_model/src/emitter.rs` (767 lines, `#![forbid(unsafe_code)]`)
Codec functions: `encode_yaml`, `encode_postcard`, `decode_postcard`, `validate_no_ansi`

## Risk Classification

| Risk class | Evidence | Verifier lane |
|---|---|---|
| Bounded codec/parser invariants | 52-byte header, fixed field offsets, CRC scope | **Kani** |
| Serialization validity | YAML roundtrip, deterministic postcard | **proptest** |
| Adversarial input robustness | Arbitrary bytes to decoder | **cargo-fuzz** |
| No-unsafe enforcement | `#![forbid(unsafe_code)]` + clippy | **static-scan** |
| Coverage adequacy | >90% line/branch on codec | **cargo-llvm-cov** |
| Mutation survival | >70% kill rate on emitter.rs | **cargo-mutants** |
| Infallible primitives | BLAKE3/CRC32 have no error path | **waiver** (compensated) |

## Verifier Lane Decisions

### Kani (8 obligations: KAN-EMIT-001–008)
**Why Kani over Verus:** Codec byte-layout properties are best expressed as bounded-memory struct field assertions. Kani's CBMC backend handles fixed-size header layout (52 bytes, 8 fields) with no over-approximation. Verus ghost state would add overhead without benefit for this layout-verification task.

**Scope:** `encode_postcard`, `build_cli_header`, `decode_cli_header`, `validate_no_ansi`

**CRITICAL INTEGRATION REQUIREMENT:** The Kani harnesses exist at `kani/vb-qi37.13.3/emitter_proofs.rs` but are UNREACHABLE unless explicitly integrated into `emitter.rs`. Before running any Kani verification, the following MUST be added to `crates/vb_ui_model/src/emitter.rs` (before the final closing brace, e.g., after line 765):

```rust
#[cfg(kani)]
mod emitter_proofs {
    include!("../../kani/vb-qi37.13.3/emitter_proofs.rs");
}
```

**Without this integration step, `cargo kani` will report zero harnesses found.**

**Key bounds:**
- `CLI_HEADER_BYTES = 52` (proved equal to 52 on all emit paths)
- `CLI_CRC_OFFSET = 48` (CRC covers bytes 0..47)
- `DIGEST_BYTES = 32` (BLAKE3 output width)
- Magic `0x56424C49` (proved constant on all emit paths)
- `payload_len <= max_payload_len` checked before allocation
- ANSI detection: `text.contains('\x1B')` returns `AnsiForbidden`

### proptest (3 obligations: PROP-EMIT-001–003)
**Why proptest over Kani:** Serialization validity and determinism are input-class properties, not single-execution layout properties. 1000 iterations of proptest gives better confidence than Kani symbolic execution for these statistical properties.

**Coverage:** Roundtrip validity (YAML→serde_yaml→parse), determinism (identical input → identical bytes), ANSI rejection (input class: strings containing `\x1B`).

### cargo-fuzz (1 obligation: FUZZ-EMIT-001)
**Why fuzz:** Decoder must not panic on arbitrary bytes. The decoder performs CRC validation, BLAKE3 digest validation, postcard deserialization, and bounds checks. Any of these can panic on crafted input if not guarded.

**Target:** `fuzz/fuzz_targets.rs::fuzz_emitter` (line 99) — fuzz target that exercises `fuzz_lib::fuzz_emitter`

### static-scan (1 obligation: STATIC-EMIT-001)
`#![forbid(unsafe_code)]` at emitter.rs:1 plus `cargo clippy -D warnings`.

### cargo-llvm-cov (1 obligation: COV-EMIT-001)
>90% line and branch coverage on emitter.rs. Target is the codec core, not full integration.

### cargo-mutants (1 obligation: MUT-EMIT-001)
>70% kill rate. Mutating codec logic (CRC scope, digest scope, bounds checks) is the target.

### Waivers (3: WAIVER-EMIT-002–004)
| ID | Infallible primitive | Reason |
|---|---|---|
| WAIVER-EMIT-002 | `blake3::Hasher::finalize` | Always succeeds; pure software hash |
| WAIVER-EMIT-003 | `crc32c::crc32c` | Pure CRC; returns u32 with no error path |
| WAIVER-EMIT-004 | `serde_yaml::ser::to_string` on OutputEnvelope | Serializes flat struct with primitive fields only; no self-reference, no unsupported types |

**Compensating evidence for all three:** `STATIC-EMIT-001` (no unsafe), `COV-EMIT-001` (>90% coverage) confirms the code paths are exercised.

## Snapshot Tests (3 obligations: SNAP-*)
| ID | Target | Format | Evidence |
|---|---|---|---|
| SNAP-YAML-001 | `commands_status.rs` | `--emit yaml` | YAML snapshot with schema_version, kind, command, exit_code |
| SNAP-POSTCARD-001 | `emitter.rs` | `--emit postcard` | 52-byte header snapshot with VBLI magic |
| SNAP-TEXT-001 | `commands_status.rs` | `--emit text` | Human-readable text snapshot |

## Verification Ordering
1. STATIC-EMIT-001 (gate: no unsafe)
2. SNAP-* (gate: user-visible format)
3. KAN-* (gate: formal layout proof)
4. PROP-* (gate: property-based validation)
5. COV-EMIT-001 (gate: coverage adequacy)
6. FUZZ-EMIT-001 (gate: adversarial robustness)
7. MUT-EMIT-001 (gate: mutation survival)

## Excluded Lanes
- **TLA+**: No temporal or state-machine behavior; codec is pure transformation.
- **Verus**: Kani handles the layout/bounds properties more directly; no refinement types in scope.
- **Loom**: No concurrent access; single-threaded codec.
- **Miri**: `#![forbid(unsafe_code)]` eliminates UB surface; no raw pointers or FFI.
- **Flux**: No refinement types in scope.
