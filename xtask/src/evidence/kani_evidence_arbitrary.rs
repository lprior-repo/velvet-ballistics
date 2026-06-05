// Kani Arbitrary implementations for evidence bundle types.
//
// These implementations allow Kani to generate arbitrary values of
// EvidenceBundle and related types for proof harnesses.
//
// Note: This file is include!()ed into xtask/src/evidence.rs when compiling with Kani.
// All types from bundle.rs and tooling_and_gate_types.rs are already in scope.
// We do NOT need use statements because include!() brings items into the current module.
//
// IMPORTANT: String and Vec do NOT implement kani::Arbitrary in this version
// of Kani. We must build strings and vectors manually using bounded loops
// with only primitive types (u8, bool) that do implement Arbitrary.

// ──────────────────────────────────────────────────────────────────────────────
// Helper: build a bounded String using only primitive kani::any() types
// ──────────────────────────────────────────────────────────────────────────────

/// Build a bounded String with length 0..max_len using only primitive Arbitrary types.
/// Uses kani::assume() to bound symbolic execution to concrete range.
fn arb_string(max_len: u8) -> String {
    let len: u8 = kani::any();
    // Bound symbolic execution: constrain len to 0..max_len (max_len is always > 0 in practice)
    if max_len > 0 {
        kani::assume(len <= max_len);
    }
    let actual_len = if max_len > 0 { (len % max_len) as usize } else { 0 };
    let mut s = String::with_capacity(actual_len);
    let mut i = 0usize;
    while i < actual_len {
        // Generate printable ASCII characters (0x21..=0x7E)
        let byte: u8 = kani::any();
        let c = (byte % 94 + 0x21) as char;
        s.push(c);
        i += 1;
    }
    s
}

// ──────────────────────────────────────────────────────────────────────────────
// EvidenceBundleFormat — simple enum
// ──────────────────────────────────────────────────────────────────────────────

impl kani::Arbitrary for EvidenceBundleFormat {
    fn any() -> Self {
        match kani::any::<u8>() % 3 {
            0 => EvidenceBundleFormat::Yaml,
            1 => EvidenceBundleFormat::Json,
            _ => EvidenceBundleFormat::Postcard,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// ArtifactType — simple enum (Benchmark, Coverage, Mutation, SupplyChain, Miri, Clippy, Fmt)
// ──────────────────────────────────────────────────────────────────────────────

impl kani::Arbitrary for ArtifactType {
    fn any() -> Self {
        match kani::any::<u8>() % 7 {
            0 => ArtifactType::Benchmark,
            1 => ArtifactType::Coverage,
            2 => ArtifactType::Mutation,
            3 => ArtifactType::SupplyChain,
            4 => ArtifactType::Miri,
            5 => ArtifactType::Clippy,
            _ => ArtifactType::Fmt,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// GateStatus — enum (Pass, Fail, Skipped { reason: String })
// ──────────────────────────────────────────────────────────────────────────────

impl kani::Arbitrary for GateStatus {
    fn any() -> Self {
        match kani::any::<u8>() % 3 {
            0 => GateStatus::Pass,
            1 => GateStatus::Fail,
            _ => GateStatus::Skipped {
                reason: arb_string(20),
            },
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// FalsePassDiagnosticVariant — simple enum
// ──────────────────────────────────────────────────────────────────────────────

impl kani::Arbitrary for FalsePassDiagnosticVariant {
    fn any() -> Self {
        match kani::any::<u8>() % 2 {
            0 => FalsePassDiagnosticVariant::Overlap,
            _ => FalsePassDiagnosticVariant::Secret,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// WhyFailed
// ──────────────────────────────────────────────────────────────────────────────

impl kani::Arbitrary for WhyFailed {
    fn any() -> Self {
        WhyFailed {
            gate_name: arb_string(20),
            hint: arb_string(30),
            repair_command: arb_string(40),
            variant: if kani::any::<bool>() {
                Some(kani::any())
            } else {
                None
            },
            fixture_id: if kani::any::<bool>() {
                Some(arb_string(15))
            } else {
                None
            },
            expected_gate: if kani::any::<bool>() {
                Some(arb_string(15))
            } else {
                None
            },
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// GateEvidence
// ──────────────────────────────────────────────────────────────────────────────

impl kani::Arbitrary for GateEvidence {
    fn any() -> Self {
        GateEvidence {
            kind: arb_string(15),
            gate_name: arb_string(20),
            command: arb_string(40),
            exit_code: kani::any(),
            log: std::path::PathBuf::from(arb_string(30)),
            status: kani::any(),
            why_failed: if kani::any::<bool>() {
                Some(kani::any())
            } else {
                None
            },
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// SourceTestMapping
// ──────────────────────────────────────────────────────────────────────────────

impl kani::Arbitrary for SourceTestMapping {
    fn any() -> Self {
        // Build tests vector manually with bounded length
        let len: u8 = kani::any();
        kani::assume(len <= 5); // bound Vec length for symbolic execution
        let actual_len = (len % 6) as usize;
        let mut tests: Vec<String> = Vec::with_capacity(actual_len);
        let mut i = 0usize;
        while i < actual_len {
            tests.push(arb_string(20));
            i += 1;
        }

        SourceTestMapping {
            source_path: arb_string(40),
            tests,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// ReleaseGateArtifact
// ──────────────────────────────────────────────────────────────────────────────

impl kani::Arbitrary for ReleaseGateArtifact {
    fn any() -> Self {
        ReleaseGateArtifact {
            name: arb_string(20),
            path: arb_string(40),
            digest: arb_string(50),
            artifact_type: kani::any(),
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// ExecutorContext
// ──────────────────────────────────────────────────────────────────────────────

impl kani::Arbitrary for ExecutorContext {
    fn any() -> Self {
        ExecutorContext {
            agent: arb_string(15),
            timestamp: arb_string(25), // ISO-8601 format
            machine: arb_string(20),
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// EvidenceBundle — top-level struct
// ──────────────────────────────────────────────────────────────────────────────

impl kani::Arbitrary for EvidenceBundle {
    fn any() -> Self {
        // Build gates vector manually with bounded length
        let len: u8 = kani::any();
        kani::assume(len <= 4); // bound Vec length for symbolic execution
        let gates_cap = (len % 5) as usize;
        let mut gates: Vec<GateEvidence> = Vec::with_capacity(gates_cap);
        let mut i = 0usize;
        while i < gates_cap {
            gates.push(kani::any());
            i += 1;
        }

        // Build source_test_mappings vector manually with bounded length
        let len: u8 = kani::any();
        kani::assume(len <= 3); // bound Vec length for symbolic execution
        let stms_cap = (len % 4) as usize;
        let mut stms: Vec<SourceTestMapping> = Vec::with_capacity(stms_cap);
        let mut j = 0usize;
        while j < stms_cap {
            stms.push(kani::any());
            j += 1;
        }

        // Build release_artifacts vector manually with bounded length
        let len: u8 = kani::any();
        kani::assume(len <= 3); // bound Vec length for symbolic execution
        let rga_cap = (len % 4) as usize;
        let mut rga: Vec<ReleaseGateArtifact> = Vec::with_capacity(rga_cap);
        let mut k = 0usize;
        while k < rga_cap {
            rga.push(kani::any());
            k += 1;
        }

        EvidenceBundle {
            schema_version: arb_string(10),
            executor_context: kani::any(),
            linked_bead_id: arb_string(20),
            gates,
            source_test_mappings: stms,
            release_artifacts: rga,
        }
    }
}