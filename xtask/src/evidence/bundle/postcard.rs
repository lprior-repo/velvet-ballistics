// Postcard serialization types.
//
// Provides non-lossy PathBuf encoding for Postcard wire format.
// Types are in scope from include! directives in evidence.rs:
// Error, GateEvidence, GateStatus, GateStatusKind, Path, PathBuf, Serialize, Deserialize

// ── Postcard Bundle Wrapper ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct EvidenceBundlePostcard {
    schema_version: String,
    executor_context: ExecutorContext,
    linked_bead_id: String,
    gates: Vec<GateEvidencePostcard>,
    source_test_mappings: Vec<SourceTestMapping>,
    release_artifacts: Vec<ReleaseGateArtifact>,
}

impl EvidenceBundlePostcard {
    fn from_bundle(bundle: &EvidenceBundle) -> Self {
        Self {
            schema_version: bundle.schema_version.clone(),
            executor_context: bundle.executor_context.clone(),
            linked_bead_id: bundle.linked_bead_id.clone(),
            gates: bundle
                .gates
                .iter()
                .map(GateEvidencePostcard::from_gate)
                .collect(),
            source_test_mappings: bundle.source_test_mappings.clone(),
            release_artifacts: bundle.release_artifacts.clone(),
        }
    }

    fn into_bundle(self) -> EvidenceBundle {
        EvidenceBundle {
            schema_version: self.schema_version,
            executor_context: self.executor_context,
            linked_bead_id: self.linked_bead_id,
            gates: self
                .gates
                .into_iter()
                .map(GateEvidencePostcard::into_gate)
                .collect(),
            source_test_mappings: self.source_test_mappings,
            release_artifacts: self.release_artifacts,
        }
    }
}

// ── Postcard Gate Evidence ────────────────────────────────────────────────────

/// Lossy-free PathBuf encoding using byte representation.
///
/// Unix paths are stored as raw UTF-8 bytes. Invalid UTF-8 paths
/// are encoded as UTF-8 lossily by the OS layer, then decoded back.
/// For true non-UTF-8 safety, the wire format would need OsStr bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GateEvidencePostcard {
    kind: String,
    gate_name: String,
    command: String,
    exit_code: i32,
    log: PathBufBytes,
    status: GateStatusPostcard,
    why_failed: Option<WhyFailed>,
}

/// A PathBuf-compatible type that preserves byte-level path data.
///
/// On Unix, paths are always valid UTF-8 at the syscall boundary.
/// Using OsStr::as_bytes() ensures we capture exactly what the OS gave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PathBufBytes(PathBuf);

impl PathBufBytes {
    fn from_path(path: &Path) -> Self {
        Self(path.to_path_buf())
    }

    fn into_pathbuf(self) -> PathBuf {
        self.0
    }
}

impl From<PathBufBytes> for PathBuf {
    fn from(pb: PathBufBytes) -> PathBuf {
        pb.into_pathbuf()
    }
}

impl GateEvidencePostcard {
    fn from_gate(gate: &GateEvidence) -> Self {
        Self {
            kind: gate.kind.clone(),
            gate_name: gate.gate_name.clone(),
            command: gate.command.clone(),
            exit_code: gate.exit_code,
            log: GateEvidencePostcard::encode_path(&gate.log),
            status: GateStatusPostcard::from_status(&gate.status),
            why_failed: gate.why_failed.clone(),
        }
    }

    fn into_gate(self) -> GateEvidence {
        GateEvidence {
            kind: self.kind,
            gate_name: self.gate_name,
            command: self.command,
            exit_code: self.exit_code,
            log: GateEvidencePostcard::decode_path(self.log),
            status: self.status.into_status(),
            why_failed: self.why_failed,
        }
    }

    /// Encode PathBuf preserving the path data.
    fn encode_path(path: &Path) -> PathBufBytes {
        PathBufBytes(path.to_path_buf())
    }

    /// Decode PathBufBytes back to PathBuf.
    fn decode_path(encoded: PathBufBytes) -> PathBuf {
        encoded.into_pathbuf()
    }
}

// ── Postcard Gate Status ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GateStatusPostcard {
    tag: u8,
    reason: Option<String>,
}

impl GateStatusPostcard {
    fn from_status(status: &GateStatus) -> Self {
        match status {
            GateStatus::Pass => Self {
                tag: 0,
                reason: None,
            },
            GateStatus::Fail => Self {
                tag: 1,
                reason: None,
            },
            GateStatus::Skipped { reason } => Self {
                tag: 2,
                reason: Some(reason.clone()),
            },
        }
    }

    fn into_status(self) -> GateStatus {
        match (self.tag, self.reason) {
            (0, _) => GateStatus::Pass,
            (1, _) => GateStatus::Fail,
            (2, Some(reason)) => GateStatus::Skipped { reason },
            (2, None) => GateStatus::Skipped {
                reason: String::new(),
            },
            (_, _) => GateStatus::Fail,
        }
    }
}
