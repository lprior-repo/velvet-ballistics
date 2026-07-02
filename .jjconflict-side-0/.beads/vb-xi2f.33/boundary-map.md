# Boundary Map — vb-xi2f.33: Digest Covers Ask Semantics

## Architecture: Pure Core / Imperative Shell

```
┌──────────────────────────────────────────────────────────────────┐
│                      IMPERATIVE SHELL                            │
│                                                                  │
│  ┌──────────────────┐    ┌──────────────────────────────────┐   │
│  │  vb_yaml (parser)│    │  vb_compile (compiler pipeline)   │   │
│  │                  │    │                                    │   │
│  │  parse_ask()     │    │  compile_workflow()                │   │
│  │  StepPrimitive   │    │  compile_source()                  │   │
│  │  ::Ask { prompt, │    │  canonical_compile_core()          │   │
│  │   timeout }      │    │  lower_canonical_ask()             │   │
│  │                  │    │  lower_ask()                       │   │
│  └────────┬─────────┘    └──────────────┬───────────────────┘   │
│           │                             │                        │
└───────────┼─────────────────────────────┼────────────────────────┘
            │                             │
            │    ┌─────────────────────┐  │
            │    │   PURE CORE         │  │
            │    │                     │  │
            │    │ canonical_digest()  │◄─┤  (reads &WorkflowSource)
            │    │ digest_step_        │  │
            │    │   primitive()       │  │
            │    │                     │  │
            │    │ compute_compiled_   │  │
            │    │   digest()          │  │
            │    │                     │  │
            │    └─────────┬───────────┘  │
            │              │              │
            │              ▼              │
            │    ┌─────────────────────┐  │
            │    │ WorkflowDigest      │──┤──▶ Embedded in
            │    │ (value object)      │  │    WorkflowParts
            │    └─────────────────────┘  │
            │                             │
            ▼                             ▼
   ┌────────────────┐          ┌─────────────────────┐
   │  vb_yaml        │          │  vb_compile output   │
   │  YAML AST       │          │  CompiledWorkflow    │
   │  (parsed)       │          │  (embedded digest)   │
   └────────────────┘          └─────────────────────┘
```

## Pure Core

| Function | Location | Purity Justification |
|----------|----------|---------------------|
| `canonical_digest(source)` | `part_05.rs` line 116 | Takes `&WorkflowSource`. Produces `WorkflowDigest`. No I/O, no time, no randomness. Pure blake3 computation. |
| `digest_step_primitive(hasher, primitive)` | `part_05.rs` line 140 | Takes `&mut Hasher, &StepPrimitive`. Side-effect: mutates hasher state (this is local, deterministic, and thread-confined). No external I/O. |
| `canonical_primitive_name(primitive)` | `part_05.rs` line 98 | Pure match → `&'static str`. No side effects. |
| `compute_compiled_digest(source)` | `mod_compile_core.rs` line 114 | Pure. Takes `&[u8]`, returns `WorkflowDigest`. |

**Rule**: The pure core MUST remain free of YAML re-parsing, file I/O, network access, time, random number generation, and async operations.

## Imperative Shell

| Component | Location | Role |
|-----------|----------|------|
| `compile_workflow(source: &[u8])` | `compile/mod.rs` line 21 | I/O-facing: takes raw bytes, delegates to `YamlCompiler`, calls `compile_source()` |
| `compile_source(source: &WorkflowSource)` | `compile/mod.rs` line 25 | Orchestrates: validates scope, lowers steps, calls `canonical_digest()` |
| `YamlCompiler` | `compile/` | Parses YAML, orchestrates slot resolution, invokes core functions |
| Canonical compilation pipeline | `mod_compile_lowering/part_01.rs` | Calls `canonical_digest()` at line 46 as part of pipeline |
| `lower_ask()` | `mod_compile_lowering/part_07.rs` | Compiles Ask IR nodes (post-digest, not in digest scope) |

## Async Shell

- **Not applicable to this bead.** `canonical_digest` is synchronous. The compilation pipeline may be invoked from async contexts but the digest computation itself has no await points.

## Storage Boundary

- **`vb_storage` crate**: Stores `CompiledWorkflow` artifacts including the embedded digest. The storage layer trusts the digest produced by the compiler — no re-computation at storage time.
- **Impact of bug**: Incorrect digests stored in compiled artifacts are trusted (and used for admission/idempotency) without detection.

## Network Boundary

- **Not applicable.** No HTTP, IPC, or network operations in digest computation.

## Time Boundary

- **Not applicable.** No timestamp, deadline, or duration operations in digest computation. (`timeout` in `Ask` is a string expression, not a time system call.)

## FFI / Unsafe Boundary

- **blake3 crate**: External dependency providing the hasher. Treated as a trusted cryptographic primitive.
  - **Risk**: If `blake3` produces non-deterministic output for identical input, the digest determinism contract is violated.
  - **Mitigation**: blake3 is a well-known, audited cryptographic hash. Determinism is a fundamental design property.
  - **Proof requirement**: None for this bead — blake3 determinism is assumed.

## Parser Boundaries

### YAML → StepPrimitive::Ask (vb_yaml)
```
User YAML text
    │
    ▼
parse_ask(sub: &Yaml) → YamlResult<StepPrimitive>
    │
    ├── prompt: validated as required string
    ├── timeout: validated as optional string
    └── reject_unknown_fields(["prompt", "timeout"])
    │
    ▼
StepPrimitive::Ask { prompt: String, timeout: Option<String> }
```

**Contract**: The parser guarantees that `prompt` is a present, non-`None` string and `timeout` is either a string or absent. The digest computation can assume these invariants without re-validating.

### Digest Bytes → WorkflowDigest (vb_core)
```
blake3::Hash (32 bytes)
    │
    ▼
WorkflowDigest::from_bytes([u8; 32])
    │
    ▼
Embedded in WorkflowParts.digest
```

**Contract**: `WorkflowDigest` accepts any `[u8; 32]` without validation. The 32-byte invariant is enforced by the type system (`[u8; 32]`), not by value checking.

## Boundary Crossing Rules

| Boundary | Direction | Validation |
|----------|-----------|------------|
| YAML text → StepPrimitive | Inbound | `parse_ask()` validates format |
| StepPrimitive → canonical_digest | Internal (pure) | No re-validation needed; types encode invariants |
| canonical_digest → WorkflowDigest | Internal (pure) | `Hasher::finalize()` produces 32 bytes by construction |
| WorkflowDigest → WorkflowParts | Internal | No extra validation at this level |
| WorkflowParts → CompiledWorkflow | Outbound | `vb_validate::shared::validate()` checks structural invariants |

## Open Boundary Questions

1. Should `canonical_digest` be extracted to a shared location (e.g., `vb_core::digest`) to eliminate the duplicate implementation across `compile/mod.rs` and `mod_compile_lowering/part_05.rs`?
2. Should the `blake3::Hasher` be wrapped in a domain type (e.g., `DigestHasher`) to prevent misuse (e.g., hashing the wrong fields)?
