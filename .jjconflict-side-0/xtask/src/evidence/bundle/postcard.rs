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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GateEvidencePostcard {
    kind: String,
    gate_name: String,
    command: String,
    exit_code: i32,
    log: String,
    status: GateStatusPostcard,
    why_failed: Option<WhyFailed>,
}

impl GateEvidencePostcard {
    fn from_gate(gate: &GateEvidence) -> Self {
        Self {
            kind: gate.kind.clone(),
            gate_name: gate.gate_name.clone(),
            command: gate.command.clone(),
            exit_code: gate.exit_code,
            log: gate.log.to_string_lossy().into_owned(),
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
            log: PathBuf::from(self.log),
            status: self.status.into_status(),
            why_failed: self.why_failed,
        }
    }
}

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

/// Convert format to a display string.
fn format_to_string(format: EvidenceBundleFormat) -> String {
    match format {
        EvidenceBundleFormat::Yaml => "yaml".to_string(),
        EvidenceBundleFormat::Json => "json".to_string(),
        EvidenceBundleFormat::Postcard => "postcard".to_string(),
    }
}
