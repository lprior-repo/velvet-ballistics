#![forbid(unsafe_code)]
//! Screen 3 — Execution Details Graph View (Phase 4B).
//!
//! A read-only run inspection screen showing:
//! 1. **Run summary panel** — run_id, workflow_name, status, started_timestamp, shard_id, durability_profile
//! 2. **Runtime graph** — workflow graph with node coloring (green=success, blue=running/selected, gray=pending, red=failed, purple=taint)
//! 3. **Event table** — paginated (10k max), columns: seq, time, step, event, shard, evidence_id
//! 4. **Step details panel** — step_name, action_id, action_type, attempt, started_time, elapsed,
//!    idempotency_key_hash, Input/Output/Details tabs
//!
//! All screen state is derived read-only from `RunSummary`, `Vec<IpcTraceEvent>`, and `WorkflowGraph`.
//! No mutations are published back to the runtime.

use vb_core::ids::StepIdx;

use crate::workflow::canvas::{WorkflowCanvas, WorkflowGraph};

// ---------------------------------------------------------------------------
// Extended RunSummary — augments vb_ipc::RunSummary with fields needed by
// the UI but not yet in the IPC type (see open question in contract).
// ---------------------------------------------------------------------------

/// Augmented run summary for the Execution Details screen.
///
/// Adds `started_timestamp`, `shard_id`, and `durability_profile` which are
/// noted as open questions in the contract (source TBD: workflow metadata or
/// IPC enrichment).
#[derive(Debug, Clone)]
pub struct ExecutionRunSummary {
    pub run_id: u64,
    pub workflow_name: String,
    pub status: RunDisplayStatus,
    pub started_timestamp: Option<String>,
    pub shard_id: u32,
    pub durability_profile: DurabilityDisplayProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RunDisplayStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DurabilityDisplayProfile {
    Nominal,
    Strict,
    BestEffort,
    Unknown,
}

impl ExecutionRunSummary {
    #[must_use]
    pub fn from_ipc_summary(
        ipc_summary: &vb_ipc::RunSummary,
        workflow_name: String,
        started_timestamp: Option<String>,
        shard_id: u32,
        durability_profile: DurabilityDisplayProfile,
    ) -> Self {
        let status = match ipc_summary.state {
            vb_ipc::RunListState::Active => RunDisplayStatus::Running,
            vb_ipc::RunListState::Finished => RunDisplayStatus::Succeeded,
            vb_ipc::RunListState::Failed => RunDisplayStatus::Failed,
            vb_ipc::RunListState::Cancelled => RunDisplayStatus::Cancelled,
        };
        Self {
            run_id: ipc_summary.run_id.get(),
            workflow_name,
            status,
            started_timestamp,
            shard_id,
            durability_profile,
        }
    }

    #[must_use]
    pub fn placeholder() -> Self {
        Self {
            run_id: 0,
            workflow_name: String::from("unknown"),
            status: RunDisplayStatus::Unknown,
            started_timestamp: None,
            shard_id: 0,
            durability_profile: DurabilityDisplayProfile::Unknown,
        }
    }
}

// ---------------------------------------------------------------------------
// RuntimeNodeState — node color semantics for the runtime graph overlay
// ---------------------------------------------------------------------------

/// Runtime execution state of a single node, used for color mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RuntimeNodeState {
    Pending,
    Running,
    Succeeded,
    Failed,
    Selected,
    Tainted,
}

// ---------------------------------------------------------------------------
// DetailTab — tab selection for the step details panel
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DetailTab {
    Input,
    Output,
    Details,
}

// ---------------------------------------------------------------------------
// EventTableRow — a single row in the event table
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EventTableRow {
    pub seq: u64,
    pub time: String,
    pub step: String,
    pub event: String,
    pub shard: u32,
    pub evidence_id: String,
}

// ---------------------------------------------------------------------------
// StepDetails — full step details panel data
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct StepDetails {
    pub step_name: String,
    pub action_id: String,
    pub action_type: String,
    pub attempt: u32,
    pub started_time: String,
    pub elapsed: String,
    pub idempotency_key_hash: String,
    pub input: String,
    pub output: String,
    pub details: String,
}

// ---------------------------------------------------------------------------
// ExecutionDetailsState — top-level screen state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ExecutionDetailsState {
    pub run_summary: ExecutionRunSummary,
    pub canvas: WorkflowCanvas,
    pub event_rows: Vec<EventTableRow>,
    pub selected_step: Option<StepIdx>,
    pub step_details: Option<StepDetails>,
    pub event_page: u32,
    pub event_page_count: u32,
}

// ---------------------------------------------------------------------------
// Color constants — velvet_ui_tokens.toml semantic colors as RGBA [f32; 4]
// ---------------------------------------------------------------------------

const COLOR_SUCCESS: [f32; 4] = [0.0863, 0.651, 0.4157, 1.0]; // #16a66a
const COLOR_RUNNING: [f32; 4] = [0.1451, 0.3882, 0.9216, 1.0]; // #2563EB
const COLOR_PENDING: [f32; 4] = [0.5804, 0.6353, 0.7020, 1.0]; // #94A3B3
const COLOR_FAILED: [f32; 4] = [0.8980, 0.2824, 0.3020, 1.0]; // #e5484d
const COLOR_TAINTED: [f32; 4] = [0.4863, 0.2275, 0.9294, 1.0]; // #7c3aed

impl RuntimeNodeState {
    #[must_use]
    pub fn color(&self) -> [f32; 4] {
        match self {
            Self::Succeeded => COLOR_SUCCESS,
            Self::Running | Self::Selected => COLOR_RUNNING,
            Self::Pending => COLOR_PENDING,
            Self::Failed => COLOR_FAILED,
            Self::Tainted => COLOR_TAINTED,
        }
    }

    #[must_use]
    pub fn border_color(&self) -> [f32; 4] {
        match self {
            Self::Failed => COLOR_FAILED,
            _ => [0.0, 0.0, 0.0, 0.0],
        }
    }

    #[must_use]
    pub fn overlay_color(&self) -> Option<[f32; 4]> {
        match self {
            Self::Tainted => Some(COLOR_TAINTED),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Contract functions
// ---------------------------------------------------------------------------

const PAGE_SIZE: usize = 50;
const MAX_EVENTS: usize = 10_000;

fn compute_page_count(total: usize, page_size: usize) -> u32 {
    let total = total.min(MAX_EVENTS);
    let count = if total == 0 || page_size == 0 {
        1
    } else {
        total.div_ceil(page_size)
    };
    u32::try_from(count).unwrap_or(u32::MAX)
}

pub fn execution_details_screen_new(
    run_summary: ExecutionRunSummary,
    events: &[vb_ipc::IpcTraceEvent],
    _graph: &WorkflowGraph,
    document: crate::graph_builder::FlowDocument,
) -> ExecutionDetailsState {
    let canvas = WorkflowCanvas::new(document);
    let event_rows = build_event_table_rows_internal(events, run_summary.shard_id);
    let event_page_count = compute_page_count(event_rows.len(), PAGE_SIZE);

    ExecutionDetailsState {
        run_summary,
        canvas,
        event_rows,
        selected_step: None,
        step_details: None,
        event_page: 0,
        event_page_count,
    }
}

pub fn node_runtime_state(
    step_idx: StepIdx,
    events: &[vb_ipc::IpcTraceEvent],
    selected: Option<StepIdx>,
) -> RuntimeNodeState {
    let is_selected = selected.is_some_and(|s| s == step_idx);

    let has_started = events.iter().any(|e| {
        matches!(
            &e.kind,
            vb_ipc::IpcTraceEventKind::StepStarted { step, .. } if *step == step_idx
        )
    });
    let has_failed = events.iter().any(|e| {
        matches!(
            &e.kind,
            vb_ipc::IpcTraceEventKind::ActionFailed { step, .. } if *step == step_idx
        )
    });
    let has_completed = events.iter().any(|e| {
        matches!(
            &e.kind,
            vb_ipc::IpcTraceEventKind::StepEnded { step, .. } if *step == step_idx
        )
    });

    if is_selected {
        if has_failed {
            RuntimeNodeState::Failed
        } else {
            RuntimeNodeState::Selected
        }
    } else if has_completed {
        RuntimeNodeState::Succeeded
    } else if has_failed {
        RuntimeNodeState::Failed
    } else if has_started {
        RuntimeNodeState::Running
    } else {
        RuntimeNodeState::Pending
    }
}

fn build_event_table_rows_internal(
    events: &[vb_ipc::IpcTraceEvent],
    shard_id: u32,
) -> Vec<EventTableRow> {
    let mut rows = Vec::with_capacity(events.len().min(MAX_EVENTS));
    for event in events.iter().take(MAX_EVENTS) {
        let step_str = step_label_for_event(&event.kind);
        let event_str = event_kind_name(&event.kind);
        let time_str = format_time_from_seq(event.sequence);
        rows.push(EventTableRow {
            seq: event.sequence,
            time: time_str,
            step: step_str,
            event: event_str,
            shard: shard_id,
            evidence_id: String::from("--"),
        });
    }
    rows
}

pub fn build_event_table_rows(
    events: &[vb_ipc::IpcTraceEvent],
    shard_id: u32,
) -> Vec<EventTableRow> {
    build_event_table_rows_internal(events, shard_id)
}

fn step_label_for_event(kind: &vb_ipc::IpcTraceEventKind) -> String {
    match kind {
        vb_ipc::IpcTraceEventKind::StepStarted { step, .. } => format!("step-{}", step.as_usize()),
        vb_ipc::IpcTraceEventKind::StepEnded { step, .. } => format!("step-{}", step.as_usize()),
        vb_ipc::IpcTraceEventKind::SlotWritten { slot, .. } => format!("slot-{}", slot.as_usize()),
        vb_ipc::IpcTraceEventKind::ActionScheduled { step, .. } => {
            format!("step-{}", step.as_usize())
        }
        vb_ipc::IpcTraceEventKind::ActionCompleted { step, .. } => {
            format!("step-{}", step.as_usize())
        }
        vb_ipc::IpcTraceEventKind::ActionFailed { step, .. } => format!("step-{}", step.as_usize()),
        vb_ipc::IpcTraceEventKind::AskAnswered { step, .. } => format!("step-{}", step.as_usize()),
        vb_ipc::IpcTraceEventKind::RunSubmitted { .. } => String::from("--"),
        vb_ipc::IpcTraceEventKind::RunFinished { .. } => String::from("--"),
        vb_ipc::IpcTraceEventKind::RunFailed { .. } => String::from("--"),
        vb_ipc::IpcTraceEventKind::RunCancelled { .. } => String::from("--"),
    }
}

fn event_kind_name(kind: &vb_ipc::IpcTraceEventKind) -> String {
    match kind {
        vb_ipc::IpcTraceEventKind::StepStarted { .. } => String::from("StepStarted"),
        vb_ipc::IpcTraceEventKind::StepEnded { .. } => String::from("StepEnded"),
        vb_ipc::IpcTraceEventKind::SlotWritten { .. } => String::from("SlotWritten"),
        vb_ipc::IpcTraceEventKind::ActionScheduled { .. } => String::from("ActionScheduled"),
        vb_ipc::IpcTraceEventKind::ActionCompleted { .. } => String::from("ActionCompleted"),
        vb_ipc::IpcTraceEventKind::ActionFailed { .. } => String::from("ActionFailed"),
        vb_ipc::IpcTraceEventKind::AskAnswered { .. } => String::from("AskAnswered"),
        vb_ipc::IpcTraceEventKind::RunSubmitted { .. } => String::from("RunSubmitted"),
        vb_ipc::IpcTraceEventKind::RunFinished { .. } => String::from("RunFinished"),
        vb_ipc::IpcTraceEventKind::RunFailed { .. } => String::from("RunFailed"),
        vb_ipc::IpcTraceEventKind::RunCancelled { .. } => String::from("RunCancelled"),
    }
}

fn format_time_from_seq(seq: u64) -> String {
    let secs = (seq / 1_000_000) % 86_400;
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    let micros = seq % 1_000_000;
    format!("{:02}:{:02}:{:02}.{:06}", hours, minutes, seconds, micros)
}

pub fn build_step_details(
    step_idx: StepIdx,
    events: &[vb_ipc::IpcTraceEvent],
    graph: &WorkflowGraph,
) -> Result<StepDetails, ExecutionDetailsError> {
    let node = graph
        .nodes
        .get(step_idx.as_usize())
        .ok_or(ExecutionDetailsError::StepNotInGraph)?;

    let started_time_opt = find_step_started_time(step_idx, events);
    let started_time_str = started_time_opt
        .clone()
        .unwrap_or_else(|| String::from("--"));
    let elapsed = compute_elapsed_from_events(step_idx, events);
    let attempt = count_attempts(step_idx, events);
    let idempotency_key_hash =
        compute_idempotency_key_hash(step_idx, events).unwrap_or_else(|| String::from("--"));

    let (action_id_str, action_type_str) = extract_action_id_and_type(graph, step_idx);
    let (input, output) = extract_input_output(step_idx, events);
    let details = build_details_text(step_idx, events);

    Ok(StepDetails {
        step_name: node.kind_name.clone(),
        action_id: action_id_str,
        action_type: action_type_str,
        attempt,
        started_time: started_time_str,
        elapsed,
        idempotency_key_hash,
        input,
        output,
        details,
    })
}

fn find_step_started_time(step_idx: StepIdx, events: &[vb_ipc::IpcTraceEvent]) -> Option<String> {
    events
        .iter()
        .find(|e| {
            matches!(
                &e.kind,
                vb_ipc::IpcTraceEventKind::StepStarted { step, .. } if *step == step_idx
            )
        })
        .map(|e| format_time_from_seq(e.sequence))
}

fn compute_elapsed_from_events(step_idx: StepIdx, events: &[vb_ipc::IpcTraceEvent]) -> String {
    let start_event = match events.iter().find(|e| {
        matches!(
            &e.kind,
            vb_ipc::IpcTraceEventKind::StepStarted { step, .. } if *step == step_idx
        )
    }) {
        Some(e) => e,
        None => return String::from("--"),
    };

    let end_seq = events
        .iter()
        .find(|e| {
            matches!(
                &e.kind,
                vb_ipc::IpcTraceEventKind::StepEnded { step, .. }
                    | vb_ipc::IpcTraceEventKind::ActionFailed { step, .. }
                    if *step == step_idx
            )
        })
        .map(|e| e.sequence)
        .unwrap_or(start_event.sequence);

    let elapsed_us = end_seq.saturating_sub(start_event.sequence);
    format_elapsed_from_micros(elapsed_us)
}

pub fn format_elapsed(
    started: std::time::SystemTime,
    ended: Option<std::time::SystemTime>,
) -> String {
    let ended = ended.unwrap_or_else(std::time::SystemTime::now);
    let dur = match ended.duration_since(started) {
        Ok(d) => d,
        Err(_) => return String::from("--"),
    };
    let total_secs = dur.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else if total_secs > 0 {
        format!("{}s", seconds)
    } else {
        let ms = dur.subsec_millis();
        if ms > 0 {
            format!("{}ms", ms)
        } else {
            let us = dur.subsec_micros();
            format!("{}us", us)
        }
    }
}

fn format_elapsed_from_micros(us: u64) -> String {
    let total_secs = us / 1_000_000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else if total_secs > 0 {
        format!("{}s", total_secs)
    } else {
        let ms = us / 1_000;
        if ms > 0 {
            format!("{}ms", ms)
        } else {
            format!("{}us", us)
        }
    }
}

fn count_attempts(step_idx: StepIdx, events: &[vb_ipc::IpcTraceEvent]) -> u32 {
    let mut attempts = 0u32;
    for event in events {
        if matches!(
            &event.kind,
            vb_ipc::IpcTraceEventKind::ActionScheduled { step, .. } if *step == step_idx
        ) {
            attempts = attempts.saturating_add(1);
        }
    }
    if attempts == 0 { 1 } else { attempts }
}

fn compute_idempotency_key_hash(
    _step_idx: StepIdx,
    _events: &[vb_ipc::IpcTraceEvent],
) -> Option<String> {
    None
}

fn extract_action_id_and_type(graph: &WorkflowGraph, step_idx: StepIdx) -> (String, String) {
    let node = match graph.nodes.get(step_idx.as_usize()) {
        Some(n) => n,
        None => return (String::from("--"), String::from("--")),
    };
    let kind_name = &node.kind_name;
    let action_type = kind_name.clone();
    let action_id = String::from("--");
    (action_id, action_type)
}

fn extract_input_output(step_idx: StepIdx, events: &[vb_ipc::IpcTraceEvent]) -> (String, String) {
    let mut input = String::from("--");
    let mut output = String::from("--");
    for event in events {
        match &event.kind {
            vb_ipc::IpcTraceEventKind::ActionScheduled { step, .. } if *step == step_idx => {
                input = String::from("{\"type\":\"json\",\"data\":{}}");
            }
            vb_ipc::IpcTraceEventKind::ActionCompleted { step, .. } if *step == step_idx => {
                output = String::from("{\"type\":\"json\",\"data\":{}}");
            }
            vb_ipc::IpcTraceEventKind::ActionFailed { step, .. } if *step == step_idx => {
                output = String::from("{\"type\":\"error\",\"message\":\"action failed\"}");
            }
            _ => {}
        }
    }
    (input, output)
}

fn build_details_text(step_idx: StepIdx, events: &[vb_ipc::IpcTraceEvent]) -> String {
    let mut parts = Vec::new();

    let slot_writes: Vec<String> = events
        .iter()
        .filter_map(|e| {
            if let vb_ipc::IpcTraceEventKind::SlotWritten { slot, .. } = &e.kind {
                Some(format!("Slot {:?} written", slot))
            } else {
                None
            }
        })
        .collect();
    if !slot_writes.is_empty() {
        parts.push(format!("Slot writes: {}", slot_writes.join(", ")));
    }

    let has_ask_answer = events.iter().any(|e| {
        matches!(
            &e.kind,
            vb_ipc::IpcTraceEventKind::AskAnswered { step, .. } if *step == step_idx
        )
    });
    if has_ask_answer {
        parts.push(String::from("Ask/Answer: answered"));
    }

    if parts.is_empty() {
        String::from("No additional details available.")
    } else {
        parts.join("\n")
    }
}

pub fn step_tab_content(
    tab: DetailTab,
    step_idx: StepIdx,
    events: &[vb_ipc::IpcTraceEvent],
) -> Result<String, ExecutionDetailsError> {
    let fake_graph = WorkflowGraph {
        nodes: Vec::new(),
        edges: Vec::new(),
        entry_step: StepIdx::new(0),
        slot_count: 0,
        workflow_name: String::new(),
    };
    let details = build_step_details(step_idx, events, &fake_graph)?;
    let content = match tab {
        DetailTab::Input => details.input,
        DetailTab::Output => details.output,
        DetailTab::Details => details.details,
    };
    if content == "--" {
        Ok(String::from("--"))
    } else {
        Ok(content)
    }
}

// ---------------------------------------------------------------------------
// Error taxonomy — mirrors contract error enum
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ExecutionDetailsError {
    RunNotFound,
    WorkflowGraphUnavailable,
    EventsUnavailable,
    PayloadDecodeFailed,
    StepNotInGraph,
    InvalidStateTransition,
}

impl core::fmt::Display for ExecutionDetailsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::RunNotFound => write!(f, "RunNotFound"),
            Self::WorkflowGraphUnavailable => write!(f, "WorkflowGraphUnavailable"),
            Self::EventsUnavailable => write!(f, "EventsUnavailable"),
            Self::PayloadDecodeFailed => write!(f, "PayloadDecodeFailed"),
            Self::StepNotInGraph => write!(f, "StepNotInGraph"),
            Self::InvalidStateTransition => write!(f, "InvalidStateTransition"),
        }
    }
}

impl std::error::Error for ExecutionDetailsError {}

// ---------------------------------------------------------------------------
// Paginated event access
// ---------------------------------------------------------------------------

impl ExecutionDetailsState {
    #[must_use]
    pub fn paginated_event_rows(&self) -> &[EventTableRow] {
        let start = usize::try_from(self.event_page)
            .unwrap_or(usize::MAX)
            .saturating_mul(PAGE_SIZE);
        let end = start.saturating_add(PAGE_SIZE).min(self.event_rows.len());
        self.event_rows.get(start..end).unwrap_or(&[])
    }

    pub fn set_page(&mut self, page: u32) {
        if page < self.event_page_count {
            self.event_page = page;
        }
    }

    #[must_use]
    pub fn current_page(&self) -> u32 {
        self.event_page
    }

    #[must_use]
    pub fn total_pages(&self) -> u32 {
        self.event_page_count
    }
}

// ---------------------------------------------------------------------------
// Selection
// ---------------------------------------------------------------------------

impl ExecutionDetailsState {
    pub fn select_step(&mut self, step_idx: Option<StepIdx>, events: &[vb_ipc::IpcTraceEvent]) {
        self.selected_step = step_idx;
        if let Some(idx) = step_idx {
            self.step_details = match build_step_details(
                idx,
                events,
                &WorkflowGraph {
                    nodes: Vec::new(),
                    edges: Vec::new(),
                    entry_step: StepIdx::new(0),
                    slot_count: 0,
                    workflow_name: String::new(),
                },
            ) {
                Ok(details) => Some(details),
                Err(_) => None,
            };
        } else {
            self.step_details = None;
        }
    }
}

// ---------------------------------------------------------------------------
// Run summary display helpers
// ---------------------------------------------------------------------------

impl ExecutionRunSummary {
    #[must_use]
    pub fn status_label(&self) -> &'static str {
        match self.status {
            RunDisplayStatus::Running => "Running",
            RunDisplayStatus::Succeeded => "Succeeded",
            RunDisplayStatus::Failed => "Failed",
            RunDisplayStatus::Cancelled => "Cancelled",
            RunDisplayStatus::Unknown => "Unknown",
        }
    }

    #[must_use]
    pub fn status_color(&self) -> [f32; 4] {
        match self.status {
            RunDisplayStatus::Running => COLOR_RUNNING,
            RunDisplayStatus::Succeeded => COLOR_SUCCESS,
            RunDisplayStatus::Failed => COLOR_FAILED,
            RunDisplayStatus::Cancelled | RunDisplayStatus::Unknown => COLOR_PENDING,
        }
    }

    #[must_use]
    pub fn durability_label(&self) -> &'static str {
        match self.durability_profile {
            DurabilityDisplayProfile::Nominal => "Nominal",
            DurabilityDisplayProfile::Strict => "Strict",
            DurabilityDisplayProfile::BestEffort => "BestEffort",
            DurabilityDisplayProfile::Unknown => "Unknown",
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Use real WorkflowCanvas via a minimal FlowDocument.
    fn make_minimal_state() -> ExecutionDetailsState {
        use crate::workflow::canvas::build_graph;
        use std::borrow::ToOwned;
        use vb_core::ids::StepIdx;
        use vb_core::ids::WorkflowDigest;
        use vb_core::workflow::CompiledWorkflow;
        use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowParts};

        let node = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: vb_core::ids::SlotIdx::new(0),
            },
        };
        let parts = WorkflowParts {
            name: "test-workflow".to_owned().into_boxed_str(),
            digest: WorkflowDigest::from_bytes([0u8; 32]),
            nodes: Box::new([node]),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count: 1,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new(["step-0".to_owned().into_boxed_str()]),
        };
        let document = crate::graph_builder::build_document(&parts);
        if let Ok(workflow) = CompiledWorkflow::try_from_parts(parts) {
            let graph = build_graph(&workflow);
            assert_eq!(graph.nodes.len(), 1);
        }
        let summary = ExecutionRunSummary::placeholder();
        ExecutionDetailsState {
            run_summary: summary,
            canvas: WorkflowCanvas::new(document),
            event_rows: Vec::new(),
            selected_step: None,
            step_details: None,
            event_page: 0,
            event_page_count: 1,
        }
    }

    #[test]
    fn runtime_node_state_colors_are_valid() {
        for state in [
            RuntimeNodeState::Pending,
            RuntimeNodeState::Running,
            RuntimeNodeState::Succeeded,
            RuntimeNodeState::Failed,
            RuntimeNodeState::Selected,
            RuntimeNodeState::Tainted,
        ] {
            let c = state.color();
            assert!(c[3] > 0.0, "alpha should be positive for {state:?}");
            for (ch, channel) in c.iter().enumerate().take(3) {
                assert!(
                    *channel >= 0.0 && *channel <= 1.0,
                    "channel {ch} out of range for {state:?}"
                );
            }
        }
    }

    #[test]
    fn runtime_node_state_failed_border_is_red() {
        assert_eq!(RuntimeNodeState::Failed.border_color(), COLOR_FAILED);
        for state in [
            RuntimeNodeState::Pending,
            RuntimeNodeState::Running,
            RuntimeNodeState::Succeeded,
            RuntimeNodeState::Selected,
        ] {
            assert_eq!(
                state.border_color(),
                [0.0, 0.0, 0.0, 0.0],
                "border should be zero for {state:?}"
            );
        }
    }

    #[test]
    fn runtime_node_state_tainted_overlay_is_some() {
        assert!(RuntimeNodeState::Tainted.overlay_color().is_some());
        for state in [
            RuntimeNodeState::Pending,
            RuntimeNodeState::Running,
            RuntimeNodeState::Succeeded,
            RuntimeNodeState::Failed,
            RuntimeNodeState::Selected,
        ] {
            assert!(
                state.overlay_color().is_none(),
                "overlay should be None for {state:?}"
            );
        }
    }

    #[test]
    fn run_display_status_labels() {
        fn make_base() -> ExecutionRunSummary {
            ExecutionRunSummary {
                run_id: 1,
                workflow_name: String::new(),
                status: RunDisplayStatus::Running,
                started_timestamp: None,
                shard_id: 0,
                durability_profile: DurabilityDisplayProfile::Nominal,
            }
        }
        assert_eq!(make_base().status_label(), "Running");

        let succeeded = ExecutionRunSummary {
            status: RunDisplayStatus::Succeeded,
            ..make_base()
        };
        assert_eq!(succeeded.status_label(), "Succeeded");

        let failed = ExecutionRunSummary {
            status: RunDisplayStatus::Failed,
            ..make_base()
        };
        assert_eq!(failed.status_label(), "Failed");

        let cancelled = ExecutionRunSummary {
            status: RunDisplayStatus::Cancelled,
            ..make_base()
        };
        assert_eq!(cancelled.status_label(), "Cancelled");

        let unknown = ExecutionRunSummary {
            status: RunDisplayStatus::Unknown,
            ..make_base()
        };
        assert_eq!(unknown.status_label(), "Unknown");
    }

    #[test]
    fn run_display_status_colors() {
        fn make_base() -> ExecutionRunSummary {
            ExecutionRunSummary {
                run_id: 1,
                workflow_name: String::new(),
                status: RunDisplayStatus::Running,
                started_timestamp: None,
                shard_id: 0,
                durability_profile: DurabilityDisplayProfile::Nominal,
            }
        }
        assert_eq!(make_base().status_color(), COLOR_RUNNING);

        let succeeded = ExecutionRunSummary {
            status: RunDisplayStatus::Succeeded,
            ..make_base()
        };
        assert_eq!(succeeded.status_color(), COLOR_SUCCESS);

        let failed = ExecutionRunSummary {
            status: RunDisplayStatus::Failed,
            ..make_base()
        };
        assert_eq!(failed.status_color(), COLOR_FAILED);

        let unknown = ExecutionRunSummary {
            status: RunDisplayStatus::Unknown,
            ..make_base()
        };
        assert_eq!(unknown.status_color(), COLOR_PENDING);
    }

    #[test]
    fn execution_run_summary_placeholder() {
        let p = ExecutionRunSummary::placeholder();
        assert_eq!(p.run_id, 0);
        assert_eq!(p.workflow_name, "unknown");
        assert_eq!(p.status, RunDisplayStatus::Unknown);
        assert!(p.started_timestamp.is_none());
    }

    #[test]
    fn execution_details_state_pagination_empty() {
        let state = make_minimal_state();
        assert!(state.paginated_event_rows().is_empty());
        assert_eq!(state.current_page(), 0);
        assert_eq!(state.total_pages(), 1);
    }

    #[test]
    fn execution_details_state_set_page_valid() {
        let mut state = make_minimal_state();
        state.event_rows = vec![
            EventTableRow {
                seq: 0,
                time: String::new(),
                step: String::new(),
                event: String::new(),
                shard: 0,
                evidence_id: String::new(),
            };
            100
        ];
        state.event_page_count = 2;
        state.set_page(1);
        assert_eq!(state.current_page(), 1);
    }

    #[test]
    fn execution_details_state_set_page_out_of_bounds() {
        let mut state = make_minimal_state();
        state.set_page(99);
        assert_eq!(state.current_page(), 0);
    }

    #[test]
    fn compute_page_count_empty() {
        assert_eq!(compute_page_count(0, PAGE_SIZE), 1);
    }

    #[test]
    fn compute_page_count_exact() {
        assert_eq!(compute_page_count(100, 50), 2);
    }

    #[test]
    fn compute_page_count_partial() {
        assert_eq!(compute_page_count(101, 50), 3);
    }

    #[test]
    fn compute_page_count_capped_at_10k() {
        assert_eq!(compute_page_count(20_000, 50), 200);
    }

    #[test]
    fn event_table_row_debug() {
        let row = EventTableRow {
            seq: 42,
            time: String::from("01:02:03.000042"),
            step: String::from("step-3"),
            event: String::from("StepStarted"),
            shard: 1,
            evidence_id: String::from("--"),
        };
        let debug_str = format!("{row:?}");
        assert!(debug_str.contains("42"));
        assert!(debug_str.contains("step-3"));
        assert!(debug_str.contains("StepStarted"));
    }

    #[test]
    fn step_details_debug() {
        let details = StepDetails {
            step_name: String::from("Do"),
            action_id: String::from("A17"),
            action_type: String::from("Do"),
            attempt: 1,
            started_time: String::from("01:02:03.000000"),
            elapsed: String::from("45ms"),
            idempotency_key_hash: String::from("abc123"),
            input: String::from("{}"),
            output: String::from("{}"),
            details: String::from("No additional details available."),
        };
        let debug_str = format!("{details:?}");
        assert!(debug_str.contains("Do"));
        assert!(debug_str.contains("A17"));
        assert!(debug_str.contains("45ms"));
    }

    #[test]
    fn format_elapsed_from_micros_all_ranges() {
        assert_eq!(format_elapsed_from_micros(0), "0us");
        assert_eq!(format_elapsed_from_micros(999), "999us");
        assert_eq!(format_elapsed_from_micros(1_000), "1ms");
        assert_eq!(format_elapsed_from_micros(1_500), "1ms");
        assert_eq!(format_elapsed_from_micros(999_999), "999ms");
        assert_eq!(format_elapsed_from_micros(1_000_000), "1s");
        assert_eq!(format_elapsed_from_micros(60_000_000), "1m 0s");
        assert_eq!(format_elapsed_from_micros(3_600_000_000), "1h 0m 0s");
    }

    #[test]
    fn detail_tab_variants() {
        let tabs = [DetailTab::Input, DetailTab::Output, DetailTab::Details];
        assert_eq!(tabs.len(), 3);
    }

    #[test]
    fn durability_profile_labels() {
        let base = ExecutionRunSummary::placeholder();
        assert_eq!(base.durability_label(), "Unknown");

        let nominal = ExecutionRunSummary {
            durability_profile: DurabilityDisplayProfile::Nominal,
            ..base.clone()
        };
        assert_eq!(nominal.durability_label(), "Nominal");

        let strict = ExecutionRunSummary {
            durability_profile: DurabilityDisplayProfile::Strict,
            ..base
        };
        assert_eq!(strict.durability_label(), "Strict");

        let best_effort = ExecutionRunSummary {
            durability_profile: DurabilityDisplayProfile::BestEffort,
            ..ExecutionRunSummary::placeholder()
        };
        assert_eq!(best_effort.durability_label(), "BestEffort");
    }

    #[test]
    fn error_display() {
        assert_eq!(
            format!("{}", ExecutionDetailsError::RunNotFound),
            "RunNotFound"
        );
        assert_eq!(
            format!("{}", ExecutionDetailsError::WorkflowGraphUnavailable),
            "WorkflowGraphUnavailable"
        );
        assert_eq!(
            format!("{}", ExecutionDetailsError::EventsUnavailable),
            "EventsUnavailable"
        );
        assert_eq!(
            format!("{}", ExecutionDetailsError::PayloadDecodeFailed),
            "PayloadDecodeFailed"
        );
        assert_eq!(
            format!("{}", ExecutionDetailsError::StepNotInGraph),
            "StepNotInGraph"
        );
        assert_eq!(
            format!("{}", ExecutionDetailsError::InvalidStateTransition),
            "InvalidStateTransition"
        );
    }

    #[test]
    fn event_kind_name_all_variants() {
        use vb_core::ids::RunId;
        use vb_ipc::IpcTraceEventKind;
        let variants: &[(&str, IpcTraceEventKind)] = &[
            (
                "StepStarted",
                IpcTraceEventKind::StepStarted {
                    run: RunId::new(0),
                    step: StepIdx::new(0),
                },
            ),
            (
                "StepEnded",
                IpcTraceEventKind::StepEnded {
                    run: RunId::new(0),
                    step: StepIdx::new(0),
                },
            ),
            (
                "SlotWritten",
                IpcTraceEventKind::SlotWritten {
                    run: RunId::new(0),
                    slot: vb_core::ids::SlotIdx::new(0),
                    value: Vec::new(),
                },
            ),
            (
                "ActionScheduled",
                IpcTraceEventKind::ActionScheduled {
                    run: RunId::new(0),
                    step: StepIdx::new(0),
                },
            ),
            (
                "StepEnded",
                IpcTraceEventKind::StepEnded {
                    run: vb_core::ids::RunId::new(0),
                    step: StepIdx::new(0),
                },
            ),
            (
                "SlotWritten",
                IpcTraceEventKind::SlotWritten {
                    run: vb_core::ids::RunId::new(0),
                    slot: vb_core::ids::SlotIdx::new(0),
                    value: Vec::new(),
                },
            ),
            (
                "ActionScheduled",
                IpcTraceEventKind::ActionScheduled {
                    run: vb_core::ids::RunId::new(0),
                    step: StepIdx::new(0),
                },
            ),
            (
                "ActionCompleted",
                IpcTraceEventKind::ActionCompleted {
                    run: vb_core::ids::RunId::new(0),
                    step: StepIdx::new(0),
                },
            ),
            (
                "ActionFailed",
                IpcTraceEventKind::ActionFailed {
                    run: vb_core::ids::RunId::new(0),
                    step: StepIdx::new(0),
                    code: vb_core::action::ActionFailureCode::Unknown,
                },
            ),
            (
                "AskAnswered",
                IpcTraceEventKind::AskAnswered {
                    run: vb_core::ids::RunId::new(0),
                    step: StepIdx::new(0),
                    slot: vb_core::ids::SlotIdx::new(0),
                },
            ),
            (
                "RunSubmitted",
                IpcTraceEventKind::RunSubmitted {
                    run: vb_core::ids::RunId::new(0),
                },
            ),
            (
                "RunFinished",
                IpcTraceEventKind::RunFinished {
                    run: vb_core::ids::RunId::new(0),
                },
            ),
            (
                "RunFailed",
                IpcTraceEventKind::RunFailed {
                    run: vb_core::ids::RunId::new(0),
                },
            ),
            (
                "RunCancelled",
                IpcTraceEventKind::RunCancelled {
                    run: vb_core::ids::RunId::new(0),
                },
            ),
        ];
        for (expected_name, kind) in variants {
            assert_eq!(
                event_kind_name(kind),
                *expected_name,
                "mismatch for {expected_name}"
            );
        }
    }

    #[test]
    fn step_label_for_event_all_variants() {
        use vb_ipc::IpcTraceEventKind;
        let variants: &[(&str, IpcTraceEventKind)] = &[
            (
                "step-5",
                IpcTraceEventKind::StepStarted {
                    run: vb_core::ids::RunId::new(0),
                    step: StepIdx::new(5),
                },
            ),
            (
                "step-3",
                IpcTraceEventKind::StepEnded {
                    run: vb_core::ids::RunId::new(0),
                    step: StepIdx::new(3),
                },
            ),
            (
                "slot-2",
                IpcTraceEventKind::SlotWritten {
                    run: vb_core::ids::RunId::new(0),
                    slot: vb_core::ids::SlotIdx::new(2),
                    value: Vec::new(),
                },
            ),
            (
                "step-1",
                IpcTraceEventKind::ActionScheduled {
                    run: vb_core::ids::RunId::new(0),
                    step: StepIdx::new(1),
                },
            ),
            (
                "--",
                IpcTraceEventKind::RunSubmitted {
                    run: vb_core::ids::RunId::new(0),
                },
            ),
        ];
        for (expected_label, kind) in variants {
            assert_eq!(step_label_for_event(kind), *expected_label);
        }
    }

    #[test]
    fn node_runtime_state_pending_when_no_events() {
        let state = node_runtime_state(StepIdx::new(0), &[], None);
        assert_eq!(state, RuntimeNodeState::Pending);
    }

    #[test]
    fn node_runtime_state_running_when_step_started() {
        let event = vb_ipc::IpcTraceEvent {
            sequence: 1_000_000,
            kind: vb_ipc::IpcTraceEventKind::StepStarted {
                run: vb_core::ids::RunId::new(1),
                step: StepIdx::new(0),
            },
        };
        let state = node_runtime_state(StepIdx::new(0), &[event], None);
        assert_eq!(state, RuntimeNodeState::Running);
    }

    #[test]
    fn node_runtime_state_succeeded_when_step_ended() {
        let events = &[
            vb_ipc::IpcTraceEvent {
                sequence: 1_000_000,
                kind: vb_ipc::IpcTraceEventKind::StepStarted {
                    run: vb_core::ids::RunId::new(1),
                    step: StepIdx::new(0),
                },
            },
            vb_ipc::IpcTraceEvent {
                sequence: 2_000_000,
                kind: vb_ipc::IpcTraceEventKind::StepEnded {
                    run: vb_core::ids::RunId::new(1),
                    step: StepIdx::new(0),
                },
            },
        ];
        let state = node_runtime_state(StepIdx::new(0), events, None);
        assert_eq!(state, RuntimeNodeState::Succeeded);
    }

    #[test]
    fn node_runtime_state_failed_when_action_failed() {
        let events = &[
            vb_ipc::IpcTraceEvent {
                sequence: 1_000_000,
                kind: vb_ipc::IpcTraceEventKind::StepStarted {
                    run: vb_core::ids::RunId::new(1),
                    step: StepIdx::new(0),
                },
            },
            vb_ipc::IpcTraceEvent {
                sequence: 2_000_000,
                kind: vb_ipc::IpcTraceEventKind::ActionFailed {
                    run: vb_core::ids::RunId::new(0),
                    step: StepIdx::new(0),
                    code: vb_core::action::ActionFailureCode::Unknown,
                },
            },
        ];
        let state = node_runtime_state(StepIdx::new(0), events, None);
        assert_eq!(state, RuntimeNodeState::Failed);
    }

    #[test]
    fn node_runtime_state_selected_takes_precedence() {
        let events = &[
            vb_ipc::IpcTraceEvent {
                sequence: 1_000_000,
                kind: vb_ipc::IpcTraceEventKind::StepStarted {
                    run: vb_core::ids::RunId::new(1),
                    step: StepIdx::new(0),
                },
            },
            vb_ipc::IpcTraceEvent {
                sequence: 2_000_000,
                kind: vb_ipc::IpcTraceEventKind::StepEnded {
                    run: vb_core::ids::RunId::new(1),
                    step: StepIdx::new(0),
                },
            },
        ];
        let state = node_runtime_state(StepIdx::new(0), events, Some(StepIdx::new(0)));
        assert_eq!(state, RuntimeNodeState::Selected);
    }

    #[test]
    fn build_event_table_rows_respects_max() {
        let events: Vec<vb_ipc::IpcTraceEvent> = (0..15_000)
            .map(|i| vb_ipc::IpcTraceEvent {
                sequence: u64::try_from(i).unwrap_or(0) * 1_000_000,
                kind: vb_ipc::IpcTraceEventKind::RunSubmitted {
                    run: vb_core::ids::RunId::new(1),
                },
            })
            .collect();
        let rows = build_event_table_rows(&events, 0);
        assert_eq!(rows.len(), MAX_EVENTS);
        assert_eq!(rows.first().map(|r| r.seq), Some(0));
        assert_eq!(
            rows.last().map(|r| r.seq),
            u64::try_from(MAX_EVENTS)
                .ok()
                .and_then(|max| max.checked_sub(1))
                .and_then(|last| last.checked_mul(1_000_000))
        );
    }

    #[test]
    fn build_event_table_rows_empty() {
        let rows = build_event_table_rows(&[], 0);
        assert!(rows.is_empty());
    }

    #[test]
    fn execution_details_error_is_error_trait() {
        let err = ExecutionDetailsError::RunNotFound;
        assert!(err.to_string().contains("RunNotFound"));
    }

    #[test]
    fn runtime_node_state_all_variants_clone() {
        let states = [
            RuntimeNodeState::Pending,
            RuntimeNodeState::Running,
            RuntimeNodeState::Succeeded,
            RuntimeNodeState::Failed,
            RuntimeNodeState::Selected,
            RuntimeNodeState::Tainted,
        ];
        for state in &states {
            let cloned = *state;
            assert_eq!(cloned, *state);
        }
    }

    #[test]
    fn detail_tab_all_variants_clone() {
        let tabs = [DetailTab::Input, DetailTab::Output, DetailTab::Details];
        for tab in &tabs {
            let cloned = *tab;
            assert_eq!(cloned, *tab);
        }
    }

    #[test]
    fn count_attempts_zero_events() {
        assert_eq!(count_attempts(StepIdx::new(0), &[]), 1);
    }

    #[test]
    fn count_attempts_multiple_schedules() {
        let events = &[
            vb_ipc::IpcTraceEvent {
                sequence: 1_000_000,
                kind: vb_ipc::IpcTraceEventKind::ActionScheduled {
                    run: vb_core::ids::RunId::new(1),
                    step: StepIdx::new(0),
                },
            },
            vb_ipc::IpcTraceEvent {
                sequence: 2_000_000,
                kind: vb_ipc::IpcTraceEventKind::ActionScheduled {
                    run: vb_core::ids::RunId::new(1),
                    step: StepIdx::new(0),
                },
            },
            vb_ipc::IpcTraceEvent {
                sequence: 3_000_000,
                kind: vb_ipc::IpcTraceEventKind::ActionScheduled {
                    run: vb_core::ids::RunId::new(1),
                    step: StepIdx::new(0),
                },
            },
        ];
        assert_eq!(count_attempts(StepIdx::new(0), events), 3);
    }

    #[test]
    fn build_details_text_empty() {
        let text = build_details_text(StepIdx::new(0), &[]);
        assert_eq!(text, "No additional details available.");
    }

    #[test]
    fn build_details_text_with_slot_write() {
        let events = &[vb_ipc::IpcTraceEvent {
            sequence: 1_000_000,
            kind: vb_ipc::IpcTraceEventKind::SlotWritten {
                run: vb_core::ids::RunId::new(1),
                slot: vb_core::ids::SlotIdx::new(3),
                value: Vec::new(),
            },
        }];
        let text = build_details_text(StepIdx::new(0), events);
        assert!(text.contains("Slot"));
        assert!(text.contains("written"));
    }
}
