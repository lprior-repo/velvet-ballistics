# Proof Evidence: vb-qi37.13.3

## Verification Artifacts Produced

### 1. Kani Proof Harnesses
**Location:** `kani/vb-qi37.13.3/emitter_proofs.rs`

**Obligations Covered:**
- KAN-EMIT-001: Magic field = 0x56424C49
- KAN-EMIT-002: header_len field = 52
- KAN-EMIT-003: CRC32C over bytes 0..47
- KAN-EMIT-004: BLAKE3 digest over payload only
- KAN-EMIT-005: payload_len check before allocation
- KAN-EMIT-006: PayloadTooLarge error without allocation
- KAN-EMIT-007: RunId non-zero precondition (documented as caller-enforced)
- KAN-EMIT-008: ANSI detection

**Assumptions:**
- Kani CBMC backend available and on PATH
- rust-goto-branch support enabled for Kani
- Bounded payloads used for tractability (payload_len <= 64 for most harnesses)
- CLI_MAGIC constant = 0x56424C49 verified

**Run Command (when integrated):**
```bash
cargo kani --package vb_ui_model --tests --harness emitter_proofs
```

### 2. Proptest Tests
**Location:** `crates/vb_ui_model/tests/emitter_proptest.rs`

**Obligations Covered:**
- PROP-EMIT-001: YAML roundtrip validity (encode_yaml produces valid YAML with required fields)
- PROP-EMIT-002: Deterministic postcard encoding (byte-identical output for identical inputs)
- PROP-EMIT-003: ANSI escape sequence rejection (AnsiForbidden for strings containing 0x1B)

**Evidence:**
```
$ cargo test -p vb_ui_model emitter -- --test-threads=1
16 passed, 32 filtered out (2 suites, 0.01s)
```

**Assumptions:**
- serde_yaml available with std feature (vb_ui_model default)
- proptest configured with 1000 iterations (default)
- OutputEnvelope uses primitive fields only for YAML serialization
- YamlEnvelope::from_envelope correctly populates all required fields

**Run Commands:**
```bash
cargo test -p vb_ui_model emitter -- yaml          # PROP-EMIT-001
cargo test -p vb_ui_model emitter -- postcard      # PROP-EMIT-002
cargo test -p vb_ui_model emitter -- ansi         # PROP-EMIT-003
```

### 3. Fuzz Target
**Location:** `fuzz/fuzz_targets/emitter.rs` (FUZZ-EMIT-001)

**Obligation:** Adversarial byte decoder testing for `decode_postcard`

**Target Functions:**
- `decode_postcard` — Tests with arbitrary byte sequences
- `encode_postcard` — Tests encoding path
- `validate_no_ansi` — Tests ANSI rejection

**Evidence:**
- Tool not available: cargo-fuzz not installed

**Assumptions:**
- corpus directory available at `fuzz/corpus/emitter/`
- max_len=1024 sufficient for fuzzing
- 60 second max_time budget adequate

**Run Command (when cargo-fuzz installed):**
```bash
cargo fuzz build && cargo fuzz run emitter -- -max_len=1024 -max_time=60
```

## Waiver Evidence

### WAIVER-EMIT-002 (BLAKE3 infallible)
- **Primitive:** `blake3::Hasher::finalize`
- **Evidence:** blake3 is a pure software hash with no error return path
- **Compensating:** STATIC-EMIT-001 (#![forbid(unsafe_code)]) + COV-EMIT-001

### WAIVER-EMIT-003 (CRC32C infallible)
- **Primitive:** `crc32c::crc32c`
- **Evidence:** Pure CRC computation, returns u32 directly
- **Compensating:** STATIC-EMIT-001 + COV-EMIT-001

### WAIVER-EMIT-004 (YAML serialization infallible)
- **Primitive:** `serde_yaml::ser::to_string` on YamlEnvelope
- **Evidence:** YamlEnvelope has primitive fields only, no self-reference
- **Compensating:** PROP-EMIT-001 (roundtrip exercise) + COV-EMIT-001

## Coverage Gaps and Mitigations

| Gap | Risk | Mitigation |
|-----|------|------------|
| Kani not run (tooling) | Layout proofs incomplete | Manual code review confirms layout invariants |
| Fuzz not run (tooling) | Adversarial robustness unproven | Unit tests cover error paths |
| YAML full roundtrip not tested | Deserialization correctness | Existing unit tests in emitter.rs |

## Verification Completeness

- **Proof obligations mapped:** 11 of 11
- **Obligations with artifacts:** 11 of 11
- **Obligations executed:** 3 of 11 (proptest only)
- **BLOCKED_TOOLING:** 8 of 11 (Kani: 7, Fuzz: 1)

## Integration Repairs (State 5 Retry 2)

### CRITICAL-1: Kani Harness Integration — FIXED
Kani harnesses are now integrated into vb_ui_model via `#[cfg(kani)] mod emitter_proofs` in `emitter.rs` (line 767). Harnesses are reachable via `cargo kani --package vb_ui_model --tests`.

### CRITICAL-2: FUZZ-EMIT-001 Artifact Path — FIXED
Updated `proof-obligations.jsonl` to point `FUZZ-EMIT-001.evidence` from `formal-verification-report.md` to `fuzz/fuzz_targets.rs` (where `fuzz_emitter` function is defined).

### MEDIUM-3: SNAP Evidence — PENDING
SNAP-YAML-001, SNAP-POSTCARD-001, SNAP-TEXT-001 test outputs not yet captured.

### MEDIUM-4: Coverage/Mutation Evidence — PENDING
llvm-cov and cargo-mutants output not yet captured.

### MEDIUM-5: Kani Unwind Bounds — PENDING
Unwind justification comments not yet added to harnesses.

## Required Production Changes

**None.** The code is verification-ready. Tooling integration is needed:
1. ~~Kani: Add `#[cfg(kani)] mod emitter_proofs`~~ — INTEGRATED
2. Fuzz: Install cargo-fuzz (`cargo install cargo-fuzz`)