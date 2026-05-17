# Domain Model Review: CLI Text/YAML/Postcard Emitters

## Domain Model Summary

### Emitter Architecture

The codebase contains two distinct postcard implementations:

| Module | Magic | Purpose | File |
|--------|-------|---------|------|
| vb_ui_model/emitter.rs | VBLI (0x56424C49) | CLI output envelopes (YAML/text/postcard) | crates/vb_ui_model/src/emitter.rs |
| velvet_ballastics/cli_postcard.rs | VCLA (0x564C4141) | Compile command IR serialization | crates/velvet_ballastics/src/cli_postcard.rs |

### Envelope Type Hierarchy

```
OutputEnvelope (vb_ui_model)
├── schema_version: SchemaVersion (u16)
├── kind: EnvelopeKind
│   ├── Success = 0
│   ├── Error = 1
│   ├── DiagnosticReport = 2
│   ├── Status = 3
│   ├── Event = 4
│   └── Workflow = 5
├── metadata: MetadataEnvelope
│   ├── run_id: RunId
│   ├── command: String
│   └── timestamp: i64
├── data: Option<PayloadEnvelope>  (for Success, Error, Status, Event, Workflow)
└── diagnostics: Vec<DiagnosticEntry>  (for DiagnosticReport only)

YamlEnvelope (emitter.rs)
├── schema_version: String ("velvet-ballastics/cli-output/v1")
├── kind: String
├── command: String
├── exit_code: u8
├── data: Option<serde_json::Value>
└── diagnostics: Option<Vec<DiagnosticEntry>>
```

### Postcard Binary Format (52-byte header)

```
Offset  Size  Field
0       4     magic (0x56424C49 = "VBLI")
4       2     schema_version (u16 LE)
6       2     kind (u16 LE)
8       4     header_len (u32 LE = 52)
12      4     payload_len (u32 LE)
16      32    payload_digest (BLAKE3)
48      4     header_crc (CRC32C over bytes 0..47)
```

### EmitMode Enum (proposed extension to args.rs)

```
EmitMode {
    Text,    // human-readable (default)
    Yaml,    // YAML text output
    Postcard // binary output
}
```

## Risk Assessment

### Critical: Magic Constant Divergence
- **Risk:** Two different postcard implementations use different magic bytes
- **Mitigation:** These are intentionally separate domains (CLI output vs IR serialization)
- **Action:** Document clearly in code comments

### High: ANSI Escape Sequences in Machine Output
- **Risk:** Text/YAML output containing ANSI codes breaks machine consumers
- **Current State:** `validate_no_ansi` function exists but is not called for text output
- **Mitigation:** INV-EMIT-007 requires ANSI-free text output

### Medium: Bounded Allocation Enforcement
- **Risk:** Unbounded payload could cause OOM
- **Current State:** INV-005 in cli_postcard.rs, INV-EMIT-004/INV-EMIT-005 in emitter.rs
- **Mitigation:** MAX_CLI_PAYLOAD_BYTES = 16,777,216 enforced before allocation

## Open Questions Resolution

### Q1: Which commands should support --emit text|yaml|postcard?
**Decision pending**: Start with status command only per bead scope. Future beads may extend.

### Q2: What is the schema_version for text output?
**Resolution**: Text output is human-readable and does not use envelope format. Only YAML and postcard outputs use the structured envelope discipline.

### Q3: Should postcard emit use vb_ui_model emitter or cli_postcard?
**Resolution**: Use vb_ui_model/emitter.rs for CLI output envelopes (VBLI magic). cli_postcard.rs (VCLA magic) remains for compile IR serialization only.

### Q4: Error handling for unsupported emit modes?
**Resolution**: ParseError::InvalidStatusArgument is correct for status command. This should be extended to ParseError::UnsupportedEmitMode for other commands if needed.

## Contract Alignment

| Contract Clause | Code Location | Status |
|-----------------|---------------|--------|
| INV-002 (schema_version never empty) | cli_envelope.rs:18, envelope.rs:18 | ✓ Satisfied |
| INV-003 (kind matches registered constants) | cli_envelope.rs:42-61 | ✓ Satisfied |
| INV-005 (bounded allocation) | emitter.rs:55, cli_postcard.rs:17-18 | ✓ Satisfied |
| POST-003 (YAML has schema_version) | emitter.rs:144-167 | ✓ Satisfied |
| POST-007 (magic + header_len validation) | emitter.rs:270-303, cli_postcard.rs:60-71 | ✓ Satisfied |
