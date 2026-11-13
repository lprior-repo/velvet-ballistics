use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use vb_core::{CompiledWorkflow, StepIdx};

use super::support::TestResult;

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

pub(super) struct Scratch(PathBuf);

impl Scratch {
    pub(super) fn new(label: &str) -> TestResult<Self> {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.evidence/omp/scratch");
        std::fs::create_dir_all(&base)?;
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = base.join(format!("active_{}_{}_{}", std::process::id(), id, label));
        std::fs::create_dir(&path)?;
        Ok(Self(path))
    }

    pub(super) fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(super) fn step(workflow: &CompiledWorkflow, name: &str) -> TestResult<StepIdx> {
    let index = workflow
        .to_parts()
        .step_names
        .iter()
        .position(|candidate| candidate.as_ref() == name)
        .ok_or_else(|| format!("compiled workflow has no step named {name}"))?;
    let raw = u16::try_from(index)?;
    Ok(StepIdx::new(raw))
}
