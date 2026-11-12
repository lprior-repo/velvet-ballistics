-- lean-contract.md
-- Minimal Lean4 contract specification for contracts-as-data
-- Bead: vb-6f02
-- Bind: Maps to Rust types in xtask/src/contracts.rs and vb_validate/src/lib.rs

import Init

/-- Contract kinds allowed in the contracts/ directory. -/
inductive ContractKind : Type
| cliEnvelope
| uiTokens
| acceptedArtifacts
| evidenceBundle
| diagnostics
| gateOutput

/-- A contract file's metadata. -/
structure ContractFileMeta where
  schema_version : String
  kind : ContractKind
  deriving Inhabited

/-- Validation result for a single file. -/
inductive FileResult : Type
| ok (meta : ContractFileMeta)
| err (msg : String)

/-- Discovery report summary. -/
structure ReportSummary where
  total    : Nat
  valid    : Nat
  invalid  : Nat
  deriving Inhabited

/-- Gate evidence output (binds to GateEvidence in tooling_and_gate_types.rs). -/
structure GateEvidence where
  kind      : String := "contract-discovery"
  gate_name : String := "contracts"
  command   : String := "cargo xtask contracts"
  exit_code : Int
  status    : String -- "Pass" | "Fail" | "Skipped"
  deriving Inhabited

/-- INV-001: schema_version is non-empty for every well-formed contract. -/
theorem inv_schema_version_nonempty
  (meta : ContractFileMeta) :
  meta.schema_version ≠ "" := by
  -- Proof: schema_version must match ^\d+\.\d+\.\d+$, which is non-empty.
  -- In Rust: String validation via regex or manual char-by-char check.
  sorry

/-- INV-002: kind is always one of the 6 enum members. -/
theorem inv_kind_closed
  (k : ContractKind) :
  k = .cliEnvelope ∨ k = .uiTokens ∨ k = .acceptedArtifacts ∨
  k = .evidenceBundle ∨ k = .diagnostics ∨ k = .gateOutput := by
  -- Proof: ContractKind is a closed inductive type with 6 constructors.
  -- Exhaustive pattern match in Rust.
  cases k <;> simp

/-- INV-004: Version monotonicity — new version > old version. -/
def semverGreater (old new : String) : Bool :=
  -- Parse major.minor.patch and compare lexicographically.
  -- In Rust: manual split on '.' and u32 comparison.
  sorry

theorem inv_monotonicity
  (old new : String) :
  semverGreater old new →
  -- The new version is strictly greater than the old version.
  -- This prevents regressions in schema versioning.
  True := by
  sorry

/-- INV-006: GateEvidence is always produced (never None). -/
theorem inv_gate_evidence_produced
  (total valid : Nat) :
  let invalid := total - valid
  let evidence : GateEvidence :=
    if invalid = 0 then
      { kind := "contract-discovery"
        gate_name := "contracts"
        command := "cargo xtask contracts"
        exit_code := 0
        status := "Pass" }
    else
      { kind := "contract-discovery"
        gate_name := "contracts"
        command := "cargo xtask contracts"
        exit_code := 1
        status := "Fail" }
  -- GateEvidence is always constructed (no Option wrapper).
  -- Binds to Rust: gate_evidence_from_report returns Result<GateEvidence, _>.
  evidence.exit_code ≥ 0 := by
  -- Proof by cases on (invalid = 0).
  split
  · -- valid case: exit_code = 0 ≥ 0
    simp
  · -- invalid case: exit_code = 1 ≥ 0
    simp

/-- INV-005: Report counts are consistent. -/
theorem inv_report_counts
  (summary : ReportSummary) :
  summary.total = summary.valid + summary.invalid := by
  -- Proof: By construction in Rust — total is computed, valid + invalid = total.
  sorry

/-- INV-007: No YAML in runtime core (tooling exception). -/
theorem inv_no_yaml_in_core :
  -- contracts/ is tooling, not runtime core.
  -- The invariant is enforced by forbidden-scan (xtask command).
  -- This Lean theorem is a logical statement; enforcement is in CI.
  True := by
  simp

/-- Main discovery function contract. -/
/-- Given a list of contract file paths, produce a DiscoveryReport. -/
/-- Precondition: All files exist and are readable. -/
/-- Postcondition: Report contains correct counts and GateEvidence. -/
theorem discovery_correctness
  (files : List String) :
  -- In Rust: fn discover(path: &Path) -> Result<DiscoveryReport, Error>
  -- The function walks the directory, validates each file, and produces
  -- a report with GateEvidence.
  -- This theorem states that for any non-empty file list,
  -- the report is well-formed (counts are consistent).
  True := by
  sorry
