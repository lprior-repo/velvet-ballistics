//! Incident data model: side effects and analysis results.

mod analysis;
mod checkpoint;
mod types;

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests_core;
#[cfg(test)]
mod tests_evidence;

pub use analysis::analyze_incident_events;
pub use types::{
    IncidentAnalysis, IncidentCheckpoint, IncidentEventCounts, IncidentFailureKind, SideEffect,
    SideEffectCertainty, SideEffectDisposition, SideEffectEvidence,
};
