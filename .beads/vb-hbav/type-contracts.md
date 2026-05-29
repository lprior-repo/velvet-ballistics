# Type Contracts — Fuzz Hardening (vb-hbav)

## Newtypes (replace primitives)

### FuzzTargetName
```rust
/// A validated fuzz target name. Must be kebab-case, non-empty, ≤ 100 chars,
/// and match exactly one `[[bin]] name` in `fuzz/Cargo.toml`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FuzzTargetName(String);
```
- **Smart constructor**: `FuzzTargetName::new(s: &str) -> Result<Self, InvalidTargetName>`
- **Invariants**: `!s.is_empty()`, `s.len() <= 100`, `s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')`
- **Replaces**: `String` used as bin name in scripts and Cargo.toml

### SeedInput
```rust
/// A seed corpus file's content. Validated for non-zero length before admission.
#[derive(Clone, Debug)]
pub struct SeedInput(Box<[u8]>);
```
- **Smart constructor**: `SeedInput::new(bytes: &[u8]) -> Option<Self>` — returns `None` for empty
- **Invariants**: `!bytes.is_empty()`

### MaxFuzzPayload
```rust
/// Maximum bytes a fuzz harness will accept before early-return.
/// Bounded to [1, 65536] to prevent OOM from length-targeting mutations.
#[derive(Clone, Copy, Debug)]
pub struct MaxFuzzPayload(u32);
```
- **Smart constructor**: `MaxFuzzPayload::new(n: u32) -> Option<Self>` — clamps to range
- **Replaces**: raw `MAX_FUZZ_PAYLOAD: u32 = 4096` constant

### CampaignDurationSecs
```rust
/// Duration of a fuzz campaign in seconds. Must be in [10, 86400].
#[derive(Clone, Copy, Debug)]
pub struct CampaignDurationSecs(u32);
```
- **Smart constructor**: `CampaignDurationSecs::new(n: u32) -> Result<Self, InvalidDuration>`
- **Invariants**: `n >= 10`, `n <= 86400`

## Enums (replace boolean flags)

### InstrumentationKind
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstrumentationKind {
    /// libfuzzer: #![no_main] + fuzz_target!(), coverage-guided, ASAN-compatible
    Libfuzzer,
    /// Stdin: feature-gated main(), reads from pipe, no coverage feedback
    Stdin,
}
```

### AssertionStrength
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssertionStrength {
    /// Only verifies panic-freedom: let _ = result; drop(result);
    CoverageOnly,
    /// Matches error result exhaustively against known error variants
    TypedError,
    /// On Ok: asserts field-level structural invariants
    Structural,
    /// Two independent code paths MUST produce identical results
    Equivalence,
    /// decode(encode(x)) == x for all round-trippable x
    Roundtrip,
}
```
Constraints: `CoverageOnly < TypedError < Structural < Equivalence`. `Roundtrip` is orthogonal to the linear scale.

### HarnessCategory
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HarnessCategory {
    /// Decodes bytes into a typed structure: parser, codec, deserializer
    Parser,
    /// decode → encode → decode roundtrip
    Roundtrip,
    /// Asserts an algebraic property holds across randomized input
    Property,
    /// Feeds hostile/corrupted bytes to format-aware boundaries
    Hostile,
    /// Two equivalent paths produce identical output
    Differential,
    /// Generates structured inputs programmatically from bytes
    StructureAware,
}
```

### Sanitizer
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sanitizer {
    Address,     // ASAN
    Undefined,   // UBSAN
    Leak,        // LSan
}
```

### FuzzCampaignStatus
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FuzzCampaignStatus {
    /// Not yet run
    Pending,
    /// Currently executing
    Running,
    /// Passed: zero crashes, zero leaks, corpus growth > 0
    Passed,
    /// Failed: one or more crashes or leaks
    Failed { crash_count: usize, leak_count: usize },
    /// Timed out before completion
    TimedOut,
}
```

### TargetBuildStatus
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetBuildStatus {
    /// No `[[bin]]` entry in Cargo.toml
    Undeclared,
    /// Declared but doesn't compile
    Uncompilable { error_count: usize },
    /// Compiles but missing libfuzzer instrumentation
    Uninstrumented,
    /// Compiles and is instrumented (or is a valid Stdin target)
    Buildable,
}
```

### SeedCategory
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeedCategory {
    /// Zero-length input
    Empty,
    /// Single byte: 0x00, 0xFF, 0x7F
    SingleByte,
    /// Magic bytes correct for the format
    MagicBytes,
    /// Valid happy-path input from integration tests
    ValidHappyPath,
    /// Structurally valid but at boundary values
    EdgeCase,
    /// Known-valid with one bit flipped
    OneBitFlipped,
}
```

## Type Aliases (enforce semantic meaning)

```rust
/// A harness body function pointer: takes &[u8], returns nothing, must not panic.
pub type HarnessBodyFn = fn(&[u8]);

/// A reference to a specific function in fuzz/src/lib.rs.
pub type HarnessBodyRef = &'static str;  // e.g. "fuzz_lib::fuzz_ipc_frame"

/// Number of assertions in a harness.
pub type AssertionCount = usize;

/// Total executions in a fuzz campaign.
pub type Executions = u64;

/// SHA-256 hash of a crashing input.
pub type CrashHash = [u8; 32];

/// Path relative to fuzz/ directory.
pub type RelativeHarnessPath = std::path::PathBuf;

/// Number of seed files in a corpus.
pub type SeedCount = usize;

/// Crate name exercised by a fuzz target.
pub type CrateName = &'static str;
```

## Parser Functions at Boundaries

Every `fuzz_lib::fuzz_*` function that accepts `&[u8]` SHOULD return early (not panic) when:
- Input is empty and the function requires at least N bytes
- `std::str::from_utf8(data)` fails for text-based functions
- `postcard::from_bytes::<T>(data)` fails for structured decode functions

No fuzz harness body (in `fuzz/src/lib.rs`, `fuzz_targets/*.rs`, or `src/bin/*.rs`) shall:
- Use `.unwrap()`, `.expect()`, `panic!()`, `todo!()`, `unimplemented!()`
- Use unchecked indexing (e.g., `data[0]` without bounds check)
- Perform unchecked arithmetic that may overflow
- Allocate based on attacker-controlled length without a reasonable upper bound

## Error Types That Must Be Matched

Every fuzz harness exercising a production crate API MUST match the crate's error enum exhaustively:

- **vb_storage::JournalError**: 38 variants — all must be matched
- **vb_ipc::IpcError**: 14 variants — all must be matched
- **vb_boundary_inventory::BoundaryInventoryError**: 13 variants — all must be matched
- **vb_validate::ValidationError**: ~30 variants — representative set must be matched

The wildcard `_ => {}` arm is permitted ONLY as a forward-compat guard. It MUST NOT match any currently-defined variant. When a new variant is added to the production error enum, the wildcard arm temporarily "catches" it gracefully; the fuzz harness must be updated before bead closure.

## Forbidden Type Patterns

- `String` for fuzz target names → use `FuzzTargetName`
- `bool` for assertion strength checks → use `AssertionStrength` enum
- `u32` for max payload bytes without bounds validation → use `MaxFuzzPayload`
- `Option<Vec<u8>>` for seed corpus → use explicit `SeedInput` with non-empty invariant
- Raw `fn(&[u8])` without type alias → use `HarnessBodyFn`
- Multiple `[[bin]]` entries targeting the same `name` field → rejected at declaration time
