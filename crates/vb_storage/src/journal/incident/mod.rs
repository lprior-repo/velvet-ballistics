//! Incident analysis and lifecycle state derivation for workflow runs.
//!
//! Three sub-modules isolate distinct domain responsibilities:
//! - **model** — side-effect tracking and incident analysis
//! - **repair** — repair-hint generation from analysis results
//! - **lifecycle** — state derivation from journal events

pub mod lifecycle;
pub mod model;
pub mod repair;

pub use self::lifecycle::{derive_lifecycle_state_from_events, lifecycle_state_to_inspect_status};
pub use self::model::{IncidentAnalysis, SideEffect, SideEffectCertainty, analyze_incident_events};
pub use self::repair::build_repair_hints;
