#![forbid(unsafe_code)]
//! Baseline (old workflow) resolution for `diff`: a YAML file or the DB.

use std::io::Read;
use std::path::Path;

use serde_json::{Value, json};
use vb_core::action::ActionContract;
use vb_core::{CompiledWorkflow, WorkflowDigest};

use super::model::digest_hex;
use crate::file_io::io_error_kind;
use crate::ipc_serve::compiled_record_for_digest;

/// A compiled workflow as `diff` loaded it, with its YAML source when known.
pub(super) struct Loaded {
    pub(super) compiled: CompiledWorkflow,
    pub(super) source: Option<Vec<u8>>,
    /// Contracts governing the workflow's actions: the persisted admission
    /// contracts for a stored baseline, the current registry otherwise.
    pub(super) contracts: Vec<ActionContract>,
}

/// The old side of the comparison.
pub(super) struct Baseline {
    pub(super) loaded: Loaded,
    /// Machine-readable description of where the baseline came from.
    pub(super) origin: Value,
}

/// Most digests an ambiguity diagnostic lists before summarizing.
const LISTED_DIGESTS: usize = 8;

/// Reads at most the YAML profile's source limit; larger files are refused.
fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let limit = vb_compile::yaml_limits::YamlLimits::default().max_source_bytes;
    let describe = |error: std::io::Error| {
        format!(
            "error reading {}: {}: {error}",
            path.display(),
            io_error_kind(&error)
        )
    };
    let file = std::fs::File::open(path).map_err(describe)?;
    let cap = u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::new();
    file.take(cap).read_to_end(&mut bytes).map_err(describe)?;
    if bytes.len() > limit {
        return Err(format!(
            "{} exceeds the {limit}-byte workflow source limit",
            path.display()
        ));
    }
    Ok(bytes)
}

/// Reads, compiles and action-checks one workflow file; its action contracts
/// are the current registry's.
pub(super) fn load_file(path: &Path, current: &[ActionContract]) -> Result<Loaded, String> {
    let bytes = read_bounded(path)?;
    let compiled = vb_compile::compile_workflow(&bytes).map_err(|errors| {
        format!(
            "{}: {}",
            path.display(),
            crate::app_impl::compile_errors_message(&errors.0)
        )
    })?;
    if let Some(id) = crate::action_specs::unknown_action_id(&compiled) {
        return Err(format!("{}: unknown action id: {id}", path.display()));
    }
    Ok(Loaded {
        compiled,
        source: Some(bytes),
        contracts: current.to_vec(),
    })
}

/// Resolves the baseline for exactly one of `--against` / `--db`.
pub(super) fn resolve(
    against: Option<&Path>,
    db: Option<&Path>,
    current: &CompiledWorkflow,
    contracts: &[ActionContract],
) -> Result<Baseline, String> {
    match (against, db) {
        (Some(path), None) => Ok(Baseline {
            loaded: load_file(path, contracts)?,
            origin: json!({"mode": "against", "path": path.display().to_string()}),
        }),
        (None, Some(db)) => from_db(db, current),
        _ => Err("exactly one baseline is required: --against <old.yaml> or --db <path>".into()),
    }
}

fn open_journal(db: &Path) -> Result<vb_storage::FjallJournal, String> {
    if !db.exists() {
        return Err(format!(
            "journal directory does not exist: {}",
            db.display()
        ));
    }
    vb_storage::FjallJournal::open(db, None)
        .map_err(|error| format!("error opening journal at {}: {error}", db.display()))
}

/// Prefers the stored artifact with the current workflow's own digest, then
/// the single stored artifact carrying the same workflow name.
fn from_db(db: &Path, current: &CompiledWorkflow) -> Result<Baseline, String> {
    let journal = open_journal(db)?;
    let (stored, matched_by) = match stored_by_digest(&journal, current.digest())? {
        Some(found) => (found, "workflow_digest"),
        None => (stored_by_name(&journal, current.name())?, "workflow_name"),
    };
    let origin = json!({
        "mode": "db",
        "path": db.display().to_string(),
        "matched_by": matched_by,
        "stored_artifact_digest": digest_hex(stored.digest),
    });
    Ok(Baseline {
        loaded: Loaded {
            compiled: stored.compiled,
            source: None,
            contracts: stored.contracts,
        },
        origin,
    })
}

/// A stored artifact verified and reconstructed with its persisted contracts.
struct Stored {
    compiled: CompiledWorkflow,
    contracts: Vec<ActionContract>,
    digest: WorkflowDigest,
}

/// Verifies an accepted-artifact envelope exactly as the runtime does (profile
/// aware reconstruction, digest and evidence checks) and keeps its contracts.
fn decode_stored(
    record: &vb_storage::CompiledIrRecord,
    requested: WorkflowDigest,
) -> Result<Stored, String> {
    let artifact =
        postcard::from_bytes::<vb_storage::AcceptedArtifact>(&record.ir).map_err(|_| {
            format!(
                "stored artifact {} is not an accepted-artifact envelope; use --against <old.yaml>",
                digest_hex(record.digest)
            )
        })?;
    let bound = artifact.digest == record.digest
        && (record.digest == requested || artifact.source_digest == requested);
    if !bound {
        return Err(format!(
            "stored artifact {} does not match the requested digest",
            digest_hex(record.digest)
        ));
    }
    let compiled = vb_storage::admission::verify_accepted_artifact(&artifact).map_err(|error| {
        format!(
            "stored artifact {} failed verification: {error}",
            digest_hex(record.digest)
        )
    })?;
    Ok(Stored {
        compiled,
        contracts: artifact.action_contracts.into_vec(),
        digest: record.digest,
    })
}

fn stored_by_digest(
    journal: &vb_storage::FjallJournal,
    digest: WorkflowDigest,
) -> Result<Option<Stored>, String> {
    use vb_ipc::server::WorkflowResolutionError::NotFound;
    match compiled_record_for_digest(journal, digest) {
        Ok(record) => decode_stored(&record, digest).map(Some),
        Err(NotFound) => Ok(None),
        Err(error) => Err(format!("stored artifact lookup failed: {error:?}")),
    }
}

/// Name-scan state: one retained candidate, a running count and a capped
/// digest list, so ambiguity never keeps extra decoded workflows alive.
#[derive(Default)]
struct NameScan {
    first: Option<Stored>,
    count: usize,
    listed: Vec<String>,
}

impl NameScan {
    fn record(&mut self, stored: Stored) {
        self.count = self.count.saturating_add(1);
        if self.listed.len() < LISTED_DIGESTS {
            self.listed.push(digest_hex(stored.digest));
        }
        if self.first.is_none() {
            self.first = Some(stored);
        }
    }
}

fn stored_by_name(journal: &vb_storage::FjallJournal, name: &str) -> Result<Stored, String> {
    let digests = journal
        .list_artifacts()
        .map_err(|error| format!("error listing stored artifacts: {error}"))?;
    let mut scan = NameScan::default();
    for digest in digests {
        let record = journal
            .compiled_ir(digest)
            .map_err(|error| format!("error reading stored artifact: {error}"))?
            .ok_or_else(|| format!("stored artifact {} vanished", digest_hex(digest)))?;
        let stored = decode_stored(&record, digest)?;
        if stored.compiled.name() == name {
            scan.record(stored);
        }
    }
    match (scan.first, scan.count) {
        (Some(only), 1) => Ok(only),
        (None, _) => Err(format!(
            "no stored compiled artifact for workflow '{name}' in the DB; use --against <old.yaml>"
        )),
        (Some(_), count) => Err(format!(
            "{count} stored artifacts share workflow name '{name}' ({}{}); use --against <old.yaml> to pick the baseline",
            scan.listed.join(", "),
            if count > scan.listed.len() {
                ", ..."
            } else {
                ""
            }
        )),
    }
}
