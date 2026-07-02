//! Current API mutation-plan validation helpers for bead vb-c3k9.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanSection {
    pub id: &'static str,
    pub heading: &'static str,
    pub required_terms: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissingRequirement {
    pub section_id: &'static str,
    pub term: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanValidationReport {
    pub required_section_count: usize,
    pub covered_required_sections: usize,
    pub stale_api_mentions: usize,
    pub missing_sections: Vec<&'static str>,
    pub duplicate_sections: Vec<&'static str>,
    pub missing_requirements: Vec<MissingRequirement>,
}

impl PlanValidationReport {
    pub fn is_valid(&self) -> bool {
        self.missing_sections.is_empty()
            && self.duplicate_sections.is_empty()
            && self.missing_requirements.is_empty()
            && self.stale_api_mentions == 0
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
        required_terms: &[
            "owner bead",
            "critical survivor",
            "release-risk acceptance",
            "cargo mutants --package velvet-ballistics-workspace-tests --test vb_c3k9_current_api_mutation_plan",
            "90% mutation kill rate",
            "exclusion policy",
        ],
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
    let section_reports: Vec<SectionReport> = REQUIRED_SECTIONS
        .iter()
        .map(|section| validate_section(plan, section))
        .collect();
    let covered_required_sections = section_reports
        .iter()
        .filter(|report| report.heading_count > 0)
        .count();
    let missing_sections = section_reports
        .iter()
        .filter_map(|report| (report.heading_count == 0).then_some(report.section_id))
        .collect();
    let duplicate_sections = section_reports
        .iter()
        .filter_map(|report| (report.heading_count > 1).then_some(report.section_id))
        .collect();
    let missing_requirements = section_reports
        .into_iter()
        .flat_map(|report| report.missing_requirements)
        .collect();
    let stale_api_mentions = STALE_API_MARKERS
        .iter()
        .filter(|marker| plan.contains(**marker))
        .count();
    PlanValidationReport {
        required_section_count: REQUIRED_SECTIONS.len(),
        covered_required_sections,
        stale_api_mentions,
        missing_sections,
        duplicate_sections,
        missing_requirements,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SectionReport {
    section_id: &'static str,
    heading_count: usize,
    missing_requirements: Vec<MissingRequirement>,
}

fn validate_section(plan: &str, section: &PlanSection) -> SectionReport {
    let heading_count = plan
        .lines()
        .filter(|line| line.trim() == section.heading)
        .count();
    let section_body = section_body(plan, section.heading);
    let missing_requirements = section
        .required_terms
        .iter()
        .filter_map(|term| missing_term(section.id, section_body.as_deref(), term))
        .collect();

    SectionReport {
        section_id: section.id,
        heading_count,
        missing_requirements,
    }
}

fn section_body(plan: &str, heading: &'static str) -> Option<String> {
    let mut inside_section = false;
    let mut body = String::new();

    for line in plan.lines() {
        let trimmed = line.trim();
        if trimmed == heading {
            inside_section = true;
            continue;
        }

        if inside_section && trimmed.starts_with("## ") {
            break;
        }

        if inside_section {
            body.push_str(line);
            body.push('\n');
        }
    }

    inside_section.then_some(body)
}

fn missing_term(
    section_id: &'static str,
    section_body: Option<&str>,
    term: &'static str,
) -> Option<MissingRequirement> {
    match section_body {
        Some(body) if body.contains(term) => None,
        _ => Some(MissingRequirement { section_id, term }),
    }
}
