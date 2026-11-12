# State 12: Diagnostic Parser Repair

## Bead: vb-nf2u

## STATUS: PASS

## Problem

`verify-deep` failed because `XtaskCommandDiagnostic::parse_output()` in `xtask/src/evidence.rs` uses `#[serde(deny_unknown_fields)]` on `RawCommandDiagnostic`, but the xtask diagnostic output now includes a `variant: OverlapFalsePass` or `variant: SecretFalsePass` field.

## Fix Applied

**File:** `xtask/src/evidence.rs`

### Change 1 — `RawCommandDiagnostic` (line 748-757)

Added `variant: Option<String>` with `#[serde(default)]` to accept the new field without breaking the `deny_unknown_fields` contract:

```rust
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCommandDiagnostic {
    error_code: String,
    fixture_id: String,
    expected_gate: String,
    actual_status: String,
    #[serde(default)]
    variant: Option<String>,
}
```

### Change 2 — `XtaskCommandDiagnostic` (line 735-741)

Added `pub variant: Option<String>` to the public struct:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XtaskCommandDiagnostic {
    pub error_code: DiagnosticCode,
    pub fixture_id: FixtureId,
    pub expected_gate: FixtureGate,
    pub actual_status: FixtureStatus,
    pub variant: Option<String>,
}
```

### Change 3 — `TryFrom<RawCommandDiagnostic>` impl (line 768-777)

Wired `variant` through the conversion:

```rust
fn try_from(raw: RawCommandDiagnostic) -> std::result::Result<Self, Self::Error> {
    Ok(Self {
        error_code: parse_diagnostic_code_value(raw.error_code)?,
        fixture_id: FixtureId::parse(raw.fixture_id, "diagnostic fixture_id")?,
        expected_gate: parse_gate_value(raw.expected_gate)?,
        actual_status: parse_status_value(raw.actual_status)?,
        variant: raw.variant,
    })
}
```

## Verification

```bash
cargo nextest run -p velvet-ballistics-workspace --test vb_nf2u_ui_release_acceptance
```

**Result:** 8 tests run, 8 passed, 0 skipped — all in 8.590s

## Engineering Rules Compliance

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, or `dbg` introduced.
- `#[serde(default)]` on `variant` means the field is optional and backwards-compatible with existing evidence files that lack the field.
- The fix preserves `deny_unknown_fields` so future unexpected fields will still be caught.
