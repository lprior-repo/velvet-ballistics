use super::*;

impl RunSubmission {
    pub(super) fn prepare_issued(
        &self,
        journal: &FjallJournal,
    ) -> Result<(u64, Self), JournalError> {
        let (next, artifact) = crate::journal::artifact_acceptance::prepare_artifact_with_sequence(
            journal,
            self.artifact.clone(),
        )?;
        let issued = Self {
            source: self.source.clone(),
            inputs: self.inputs.with_artifact_digest(artifact.digest)?,
            artifact,
            header: self.header.clone(),
            budget: self.budget,
            budget_origin_millis: self.budget_origin_millis,
            granted_capabilities: self.granted_capabilities.clone(),
        };
        issued.validate_identities()?;
        Ok((next, issued))
    }
}
