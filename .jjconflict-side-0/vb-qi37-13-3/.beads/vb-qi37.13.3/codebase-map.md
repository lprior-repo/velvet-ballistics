# Codebase Map: vb-qi37.13.3

**Bead:** cli: Implement text yaml and postcard emitters
**Scope:** Implement --emit text|yaml|postcard output modes for supported CLI commands
**Source checkout:** /home/lewis/src/Velvet-ballistics
**Parent commit:** 336dbd58bfb5d17ccacb75dfb2713e17ac002e46

---

## 1. Relevant Crates and Files

### Primary Crate
| Crate | Path | Purpose |
|-------|------|---------|
| velvet_ballastics | `crates/velvet_ballastics/` | Main CLI binary, all command implementations |
| vb_ui_model | `crates/vb_ui_model/` | Emitter implementations (YAML, Postcard encode/decode) |
| vb_yaml | `crates/vb_yaml/` | YAML parsing/validation for workflow sources |
| vb_compile | `crates/vb_compile/` | Workflow compilation pipeline |
| vb_core | `crates/vb_core/` | Core types (CompiledWorkflow, RunId, etc.) |
| vb_storage | `crates/vb_storage/` | Journal storage (Fjall) |

### Key Source Files

| File | Purpose |
|------|---------|
| `crates/velvet_ballastics/src/main.rs` | CLI entry point, command dispatch, `cmd_status`, `cmd_compile` |
| `crates/velvet_ballastics/src/args.rs` | Argument parsing, `EmitTarget`, `OutputFormat`, `StatusOptions`, `ParseError` |
| `crates/velvet_ballastics/src/commands_status.rs` | `print_status`, `print_status_yaml` |
| `crates/velvet_ballastics/src/cli_envelope.rs` | `build_envelope`, `serialize_with_version`, `Kind` enum, SCHEMA_VERSION |
| `crates/velvet_ballastics/src/cli_postcard.rs` | Postcard header encode/decode, bounded allocation (INV-005) |
| `crates/vb_ui_model/src/emitter.rs` | `encode_yaml`, `encode_postcard`, `decode_postcard`, `EmitterError`, `YamlEnvelope` |
| `crates/vb_ui_model/src/envelope.rs` | `OutputEnvelope`, `EnvelopeKind`, `EnvelopeError` (separate from cli_envelope) |

---

## 2. Emitter Architecture

### Existing Emitter Types

#### OutputFormat (args.rs:9-17)
```rust
pub(crate) enum OutputFormat {
    Text,   // human-readable (default)
    Json,   // JSON object
    Jsonl,  // JSON Lines
}
```

#### EmitTarget (args.rs:204-209) - for `compile` command only
```rust
pub(crate) enum EmitTarget {
    Ir,
    Rust,
    Yaml,
    Postcard,
}
```

#### StatusOptions (args.rs:188-193)
```rust
pub(crate) struct StatusOptions {
    pub(crate) active_runs: Option<usize>,
    pub(crate) queue_depth: Option<usize>,
    pub(crate) trace_dropped: Option<u64>,
    pub(crate) emit_yaml: bool,  // --emit yaml for status command
}
```

### CLI Envelope Contract (cli_envelope.rs)
- `SCHEMA_VERSION = "velvet-ballastics/cli-output/v1"` (never empty - INV-002)
- `Kind` enum: VerificationReport, DiagnosticReport, WorkflowExplanation, SimulationReport, SubmitRunResult, RunInspection, RunEvents, ReplayReport, IncidentReport, ActionList, ActionDescription, DoctorReport, AiContextPacket, CliStatus, AgentContext
- `build_envelope(data, Kind)` returns JSON Value with schema_version, kind, data fields
- `serialize_with_version(data, Kind)` merges data into envelope

### Existing Emitter Implementation (vb_ui_model/src/emitter.rs)
- `encode_yaml<T: Serialize>(payload: &T) -> Result<String, EmitterError>` - YAML text output
- `encode_postcard<T: Serialize>(payload: &T, kind: EnvelopeKind, max_payload_len: u32) -> Result<Vec<u8>, EmitterError>` - binary with 52-byte header
- `decode_postcard<'a, T: Deserialize<'a>>(bytes: &'a [u8], expected_kind: EnvelopeKind, max_payload_len: u32) -> Result<T, EmitterError>`
- `YamlEnvelope` struct for YAML output shape

### Postcard Binary Format (vb_ui_model/src/emitter.rs:8-24)
- 52-byte header: magic(4) + schema_version(2) + kind(2) + header_len(4) + payload_len(4) + payload_digest(32) + header_crc(4)
- Magic: 0x56424C49 ("VBLI")
- CRC32C over bytes 0..47
- BLAKE3 digest of payload

### Alternative Postcard Implementation (crates/velvet_ballastics/src/cli_postcard.rs)
- Magic: "VCLA" (0x564C4141)
- `PostcardHeader`, `encode_postcard`, `decode_postcard`
- Used for IR serialization in compile command
- INV-005: bounded allocation validation

---

## 3. Command Emit Mode Status

| Command | --emit text | --emit yaml | --emit postcard | Notes |
|---------|-------------|-------------|-----------------|-------|
| status | existing (default) | existing (emit_yaml bool) | NOT SUPPORTED (rejected in parse_status_options) | Text output at commands_status.rs:114 |
| compile | N/A | N/A | N/A | Uses EmitTarget enum with Ir/Rust/Yaml/Postcard |
| verify | OutputFormat only | OutputFormat only | NOT SUPPORTED | |
| validate | OutputFormat only | OutputFormat only | NOT SUPPORTED | |
| doctor | OutputFormat only | OutputFormat only | NOT SUPPORTED | |
| inspect | OutputFormat only | OutputFormat only | NOT SUPPORTED | |
| events | OutputFormat only | OutputFormat only | NOT SUPPORTED | |
| simulate | OutputFormat only | OutputFormat only | NOT SUPPORTED | |

---

## 4. Existing Tests

### CLI Integration Tests (crates/velvet_ballastics/tests/cli_integration.rs)
- `cli_emit_yaml_contract_is_not_silent_when_master_emit_mode_is_requested` (line 2655): Tests `status --emit yaml` produces YAML with schema_version and kind fields
- `cli_action_inspect_text_output_has_contract_details` (line 571): Text output format test
- `cli_doctor_text_reports_trim_eligibility` (line 2374): Doctor text output test

### Emitter Tests (crates/vb_ui_model/src/emitter.rs:486-767)
- `cli_magic_is_vbli`, `cli_header_length_is_52`
- `build_cli_header_produces_correct_length`, `cli_header_roundtrip`
- `encode_decode_postcard_roundtrip`, `postcard_rejects_wrong_kind`, `postcard_rejects_bad_magic`, `postcard_rejects_bad_crc`, `postcard_rejects_bad_payload_digest`
- `yaml_envelope_from_envelope`

---

## 5. Parse Error Handling

### Status emit mode errors (args.rs:322-343)
```rust
Some("--emit") => match rest.split_first() {
    Some((emit, remaining)) => match emit.to_str() {
        Some("yaml") => ... emit_yaml: true ...
        Some("text") => ... // no change
        Some("postcard") => Err(ParseError::InvalidStatusArgument(
            "postcard emit is not supported for status".into(),
        )),
        Some(other) => Err(ParseError::InvalidStatusArgument(format!(
            "unknown emit mode {other}"
        ))),
        ...
    },
    None => Err(ParseError::MissingArgument("--emit")),
}
```

### UnknownEmitTarget (args.rs:229)
```rust
UnknownEmitTarget(String),  // used when compile --emit has invalid value
```

---

## 6. Risk Tags

| Risk | Category | Description |
|------|----------|-------------|
| user-visible behavior | CLI_OUTPUT_CONTRACT | Emitter output directly affects CLI UX and downstream consumers |
| parser/codec | YAML_EMIT | YAML serialization must produce valid YAML |
| parser/codec | POSTCARD_EMIT | Postcard encoding must respect bounded allocation |
| dependency | vb_yaml | YAML emitter depends on vb_yaml for serialization |
| performance | EMITTER_ALLOC | Postcard header validation prevents OOM (INV-005) |
| public API | CLI_ENVELOPE | schema_version and kind fields are part of stable contract |

---

## 7. Open Questions

1. **Which commands should support --emit text|yaml|postcard?** Currently only `status` has emit modes. Need to determine scope.
2. **What is the schema_version for text output?** Text output doesn't use envelope format - is this intentional?
3. **Should postcard emit use vb_ui_model emitter or cli_postcard?** Two different postcard implementations exist.
4. **Error handling for unsupported emit modes?** Currently returns `ParseError::InvalidStatusArgument` - is this the right error type?

---

## 8. Recommended Downstream Owners

| Lane | Owner |
|------|-------|
| Contract | rust-contract skill for output schema validation |
| Proof | kani for bounded allocation proofs, miri for codec safety |
| Test | test-writer skill for snapshot tests and format validation |
| Implementation | functional-rust skill for zero-panic emitter code |
| Black Hat | black-hat-reviewer for CLI output contract enforcement |
