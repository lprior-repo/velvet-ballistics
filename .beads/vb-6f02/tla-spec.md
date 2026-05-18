---- contracts-as-data.tla ----
(* ============================================================================
   Temporal Specification: Contract Discovery System
   Bead: vb-6f02
   Purpose: Model contract-discovery as a state machine with bounded versions,
            monotonicity enforcement, and gate evidence production.
   ============================================================================ *)

CONSTANTS
    ContractKind,          \* {"cli_envelope", "ui_tokens", "accepted_artifacts",
                            "  evidence_bundle", "diagnostics", "gate_output"}
    SchemaVersion,         \* set of valid version strings "N.N.N"
    FilePath,              \* set of contract file paths
    ErrorKind,             \* {"missing_version", "invalid_kind", "vet_fail",
                               "monotonicity_breach"}
    GateStatus,            \* {"Pass", "Fail", "Skipped"}

VARIABLES
    contracts,             \* map: FilePath -> [schema_version \in SchemaVersion,
                                kind \in ContractKind, vet_ok \in {TRUE, FALSE}]
    manifest,              \* map: FilePath -> SchemaVersion  (previous versions)
    report,                \* DiscoveryReport state
    gate_evidence,         \* GateEvidence state
    errors,                \* list of (FilePath, ErrorKind) tuples
    outcome                \* {"unknown", "pass", "fail"}

(* ============================================================================
   Schema: Contract file structure
   ============================================================================ *)

HasSchemaVersion(f) == \E v \in SchemaVersion : f["schema_version"] = v

HasKind(f) == \E k \in ContractKind : f["kind"] = k

IsWellFormed(f) == HasSchemaVersion(f) /\ HasKind(f) /\ f["vet_ok"] = TRUE

(* ============================================================================
   Schema: DiscoveryReport
   ============================================================================ *)

IsReport(r) ==
    \E files \in [FilePath \to [schema_version \in SchemaVersion,
                                kind \in ContractKind, vet_ok \in {TRUE, FALSE}]],
           verrs \in Subset(FilePath \* ErrorKind),
           total \in Nat,
           valid \in Nat,
           invalid \in Nat:
        r["files"] = files /\
        r["errors"] = verrs /\
        r["total"] = total /\
        r["valid"] = valid /\
        r["invalid"] = invalid

(* ============================================================================
   Schema: GateEvidence (binds to xtask/src/evidence/tooling_and_gate_types.rs)
   ============================================================================ *)

IsGateEvidence(g) ==
    \E kind \in {"contract-discovery"},
           gate_name \in {"contracts"},
           command \in {"cargo xtask contracts"},
           exit_code \in {0, 1},
           log_path \in [1..100] \to STRING,
           status \in GateStatus:
        g["kind"] = kind /\
        g["gate_name"] = gate_name /\
        g["command"] = command /\
        g["exit_code"] = exit_code /\
        g["log"] = log_path /\
        g["status"] = status

(* ============================================================================
   Initial State: No contracts loaded yet
   ============================================================================ *)

Init ==
    contracts = [f \in FilePath |-> [schema_version |-> "0.0.1",
                                      kind |-> CHOOSE k \in ContractKind : TRUE,
                                      vet_ok |-> FALSE]] /\
    manifest = [f \in FilePath |-> "0.0.0"] /\
    report = [type |-> "unknown"] /\
    gate_evidence = [type |-> "unknown"] /\
    errors = <<>> /\
    outcome = "unknown"

(* ============================================================================
   Action: Validate a single contract file
   - Checks schema_version present
   - Checks kind is valid enum member
   - Simulates cue vet result
   - Records errors if any
   ============================================================================ *)

ValidateFile(f) ==
    LET valid == IsWellFormed(contracts[f]) IN
        /\ errors' = IF valid THEN errors
                     ELSE errors \o <<f, CHOOSE e \in ErrorKind : TRUE>>
        /\ gate_evidence' = [g \in gate_evidence EXCEPT !.status = IF valid THEN "Pass" ELSE "Fail"]
        /\ outcome' = IF \E e \in errors' : e \# <<f, CHOOSE err \in ErrorKind : TRUE>>
                      THEN "fail"
                      ELSE IF outcome = "fail" THEN "fail" ELSE "pass"

(* ============================================================================
   Action: Run full discovery
   - Validates all files in contracts map
   - Builds DiscoveryReport
   - Produces GateEvidence
   - Bounded: only processes up to 100 files at a time (hardware limit)
   ============================================================================ *)

RunDiscovery ==
    /\ Len(contracts) \le 100   \* Bounded by hardware limit (MAX_U64 not needed; 100 is safe)
    /\ \A f \in DOMAIN contracts :
           ValidateFile(f)
    /\ \E total \in Nat, valid \in Nat, invalid \in Nat:
           total = Len(contracts) /\
           valid = Cardinality({f \in DOMAIN contracts : IsWellFormed(contracts[f])}) /\
           invalid = total - valid /\
           report' = [type |-> "DiscoveryReport",
                      total |-> total,
                      valid |-> valid,
                      invalid |-> invalid,
                      errors |-> errors] /\
           gate_evidence' = IF invalid = 0
                            THEN [kind |-> "contract-discovery",
                                  gate_name |-> "contracts",
                                  command |-> "cargo xtask contracts",
                                  exit_code |-> 0,
                                  log |-> <<".", "evidence", "contracts", "last_run.log">>,
                                  status |-> "Pass"]
                            ELSE [kind |-> "contract-discovery",
                                  gate_name |-> "contracts",
                                  command |-> "cargo xtask contracts",
                                  exit_code |-> 1,
                                  log |-> <<".", "evidence", "contracts", "last_run.log">>,
                                  status |-> "Fail"]
    /\ outcome' = IF invalid = 0 THEN "pass" ELSE "fail"

(* ============================================================================
   Action: Update manifest with new version (monotonicity check)
   - Before updating, verify new_version > manifest[f]
   - If breach, record error instead of updating
   ============================================================================ *)

UpdateManifest(f, newVer) ==
    LET oldVer == manifest[f] IN
        /\ IsVersion(newVer)  \* newVer \in SchemaVersion
        /\ IsVersion(oldVer)  \* oldVer \in SchemaVersion
        /\ newVer \ne oldVer  \* version must change
        /\ \E major1, minor1, patch1, major2, minor2, patch2 \in Nat:
               oldVer = Concat(Concat(IntToStr(major1), "."),
                               Concat(IntToStr(minor1),
                               Concat(".", IntToStr(patch1)))) /\
               newVer = Concat(Concat(IntToStr(major2), "."),
                               Concat(IntToStr(minor2),
                               Concat(".", IntToStr(patch2)))) /\
               (\E breach \in {TRUE, FALSE}:
                    breach == (major2 < major1 \/
                               (major2 = major1 /\ minor2 < minor1) \/
                               (major2 = major1 /\ minor2 = minor1 /\ patch2 <= patch1)))
        /\ manifest' = IF \E major1, minor1, patch1, major2, minor2, patch2 \in Nat:
                             oldVer = Concat(Concat(IntToStr(major1), "."),
                                             Concat(IntToStr(minor1),
                                             Concat(".", IntToStr(patch1)))) /\
                             newVer = Concat(Concat(IntToStr(major2), "."),
                                             Concat(IntToStr(minor2),
                                             Concat(".", IntToStr(patch2)))) /\
                             NOT (major2 > major1 \/
                                  (major2 = major1 /\ minor2 > minor1) \/
                                  (major2 = major1 /\ minor2 = minor1 /\ patch2 > patch1))
                         THEN manifest
                         ELSE [m \in manifest EXCEPT ![f] = newVer]
        /\ errors' = IF \E major1, minor1, patch1, major2, minor2, patch2 \in Nat:
                            oldVer = Concat(Concat(IntToStr(major1), "."),
                                            Concat(IntToStr(minor1),
                                            Concat(".", IntToStr(patch1)))) /\
                            newVer = Concat(Concat(IntToStr(major2), "."),
                                            Concat(IntToStr(minor2),
                                            Concat(".", IntToStr(patch2)))) /\
                            NOT (major2 > major1 \/
                                 (major2 = major1 /\ minor2 > minor1) \/
                                 (major2 = major1 /\ minor2 = minor1 /\ patch2 > patch1))
                       THEN errors \o <<f, "monotonicity_breach">>
                       ELSE errors

(* ============================================================================
   Invariants
   ============================================================================ *)

(* INV-001: Every file in report has a schema_version *)
Invariant1 ==
    \E r \in [type |-> "DiscoveryReport", files \in [FilePath \to SchemaVersion]]:
        report = r =>
            \E files \in [FilePath \to SchemaVersion] :
                report["files"] = files =>
                    \A f \in DOMAIN files : HasSchemaVersion(files[f])

(* INV-002: GateEvidence status is consistent with outcome *)
Invariant2 ==
    \E g \in [status \in GateStatus] :
        gate_evidence = g =>
            (g["status"] = "Pass" <=> outcome = "pass") /\
            (g["status"] = "Fail" <=> outcome = "fail")

(* INV-003: Report counts are consistent *)
Invariant3 ==
    \E r \in [total \in Nat, valid \in Nat, invalid \in Nat] :
        report = r =>
            r["total"] = r["valid"] + r["invalid"]

(* INV-004: Manifest versions are well-formed *)
Invariant4 ==
    \A f \in DOMAIN manifest :
        manifest[f] \in SchemaVersion

(* ============================================================================
   Fairness: All actions are eventually scheduled
   ============================================================================ *)

WF_vars <<RunDiscovery, UpdateManifest>>

(* ============================================================================
   Liveness: Eventually a valid report is produced
   ============================================================================ *)

Spec == Init /\ [][RunDiscovery \/ UpdateManifest]_vars /\
           WF_vars \/ WF_vars

(* ============================================================================
   Properties to check with TLC:
   1. Inv1..Inv4 are invariants (never violated)
   2. RunDiscovery is always enabled after Init
   3. UpdateManifest detects monotonicity breaches
   4. GateEvidence status matches outcome
   5. Report total = valid + invalid
   ============================================================================ *)
