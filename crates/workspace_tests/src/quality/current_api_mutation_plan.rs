//! Current API mutation-plan validation helpers for bead vb-c3k9.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanSection {
    pub id: &'static str,
    pub heading: &'static str,
    pub required_terms: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanValidationReport {
    pub section_count: usize,
    pub stale_api_mentions: usize,
    pub missing_requirements: Vec<&'static str>,
}

impl PlanValidationReport {
    pub fn is_valid(&self) -> bool {
        self.missing_requirements.is_empty() && self.stale_api_mentions == 0
    }
}

pub const REQUIRED_SECTIONS: &[PlanSection] = &[
    PlanSection {
        id: "helper-semantics",
        heading: "## Helper Semantics Mutation Targets",
        required_terms: &[
            "contains",
            "starts_with",
            "ends_with",
            "length",
            "empty",
            "has",
            "exists",
            "sum",
            "count",
            "append_if",
            "merge",
            "unique",
        ],
    },
    PlanSection {
        id: "runtime-recovery",
        heading: "## Runtime Recovery Mutation Targets",
        required_terms: &[
            "ActionCompleted before frame mutation",
            "journal sequence hydration",
            "snapshot hydration",
            "retry state",
        ],
    },
    PlanSection {
        id: "generated-parity",
        heading: "## Generated Rust Parity Mutation Targets",
        required_terms: &[
            "generated-interpreter suspension parity",
            "full final IR equivalence",
            "unsupported generated-mode rejection",
        ],
    },
    PlanSection {
        id: "cli-ipc-storage",
        heading: "## CLI, IPC, and Storage Envelope Mutation Targets",
        required_terms: &[
            "binary IPC frame length",
            "postcard envelope",
            "Fjall journal",
            "CLI accepted artifact path",
        ],
    },
    PlanSection {
        id: "ui-model",
        heading: "## UI Model Contract Mutation Targets",
        required_terms: &["vb_ui_model", "certificate", "incident", "replay"],
    },
    PlanSection {
        id: "ownership",
        heading: "## Owner Beads and Release Blockers",
        required_terms: &["owner bead", "critical survivor", "release-risk acceptance"],
    },
];

const STALE_API_MARKERS: &[&str] = &[
    "generic DAG runner",
    "Temporal clone",
    "runtime YAML interpreter",
    "HTTP runtime route",
    "JSON runtime core",
];

pub fn validate_plan(plan: &str) -> PlanValidationReport {
    let missing_requirements = REQUIRED_SECTIONS
        .iter()
        .flat_map(|section| missing_for_section(plan, section))
        .collect();
    let stale_api_mentions = STALE_API_MARKERS
        .iter()
        .filter(|marker| plan.contains(**marker))
        .count();
    PlanValidationReport {
        section_count: REQUIRED_SECTIONS.len(),
        stale_api_mentions,
        missing_requirements,
    }
}

fn missing_for_section(plan: &str, section: &PlanSection) -> Vec<&'static str> {
    section
        .required_terms
        .iter()
        .filter_map(|term| missing_term(plan, section.heading, term))
        .collect()
}

fn missing_term(plan: &str, heading: &'static str, term: &'static str) -> Option<&'static str> {
    (!plan.contains(heading) || !plan.contains(term)).then_some(term)
}
