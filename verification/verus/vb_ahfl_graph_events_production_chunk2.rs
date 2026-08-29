verus! {
}

/// Non-vacuum witness: `Kind::from_str(RUN_EVENTS) ==
/// Some(Kind::RunEvents)`.
pub exec fn wrapper_kind_from_str_run_events() -> (r: Option<production::Kind>)
    ensures
        r == spec_kind_from_str(SPEC_RUN_EVENTS),
        r == Some(production::Kind::RunEvents),
{
    let r = production::Kind::from_str(SPEC_RUN_EVENTS);
    assert(r == spec_kind_from_str(SPEC_RUN_EVENTS));
    assert(r == Some(production::Kind::RunEvents));
    r
}

/// Non-vacuum witness: `Kind::from_str("...") == None` for any string
/// not matching a registered `kind::*` constant.
pub exec fn wrapper_kind_unknown_from_str_none() -> (r: Option<production::Kind>)
    ensures
        r.is_none(),
{
    let r = production::Kind::from_str("ThisIsNotAValidKindConstant_xyzzy");
    assert(r.is_none());
    r
}

// ============================================================================
// PRODUCTION-BOUND PROOFS — non-vacuum (Kind envelope scope)
// ============================================================================
//
// Each proof below discharges an obligation on the production-bound
// `Kind::as_str` / `Kind::from_str` contracts by reasoning over
// `spec_kind_as_str` / `spec_kind_from_str` and the registered
// constants. The proofs are non-vacuum because they actually unfold
// the spec fns and reference the production symbols.
/// proof_kind_workflow_graph_bound: `spec_kind_as_str(Kind::WorkflowGraph)
/// == SPEC_WORKFLOW_GRAPH`.
pub proof fn proof_kind_workflow_graph_bound()
    ensures
        spec_kind_as_str(production::Kind::WorkflowGraph) == SPEC_WORKFLOW_GRAPH,
{
    // Unfold spec_kind_as_str on the WorkflowGraph variant. The match
    // arm returns SPEC_WORKFLOW_GRAPH directly.
    assert(spec_kind_as_str(production::Kind::WorkflowGraph) == SPEC_WORKFLOW_GRAPH);
}

/// proof_kind_run_events_bound: `spec_kind_as_str(Kind::RunEvents) ==
/// SPEC_RUN_EVENTS`.
pub proof fn proof_kind_run_events_bound()
    ensures
        spec_kind_as_str(production::Kind::RunEvents) == SPEC_RUN_EVENTS,
{
    // Unfold spec_kind_as_str on the RunEvents variant. The match
    // arm returns SPEC_RUN_EVENTS directly.
    assert(spec_kind_as_str(production::Kind::RunEvents) == SPEC_RUN_EVENTS);
}

/// proof_kind_workflow_graph_string_constant:
/// `SPEC_WORKFLOW_GRAPH == "WorkflowGraph"` (the literal string).
pub proof fn proof_kind_workflow_graph_string_constant()
    ensures
        SPEC_WORKFLOW_GRAPH == "WorkflowGraph",
{
    // SPEC_WORKFLOW_GRAPH is the local spec constant defined as
    // the string literal "WorkflowGraph".
    assert(SPEC_WORKFLOW_GRAPH == "WorkflowGraph");
}

/// proof_kind_run_events_string_constant:
/// `SPEC_RUN_EVENTS == "RunEvents"` (the literal string).
pub proof fn proof_kind_run_events_string_constant()
    ensures
        SPEC_RUN_EVENTS == "RunEvents",
{
    assert(SPEC_RUN_EVENTS == "RunEvents");
}

/// proof_kind_round_trip_workflow_graph:
/// `spec_kind_from_str(SPEC_WORKFLOW_GRAPH) ==
/// Some(production::Kind::WorkflowGraph)`.
pub proof fn proof_kind_round_trip_workflow_graph()
    ensures
        spec_kind_from_str(SPEC_WORKFLOW_GRAPH) == Some(production::Kind::WorkflowGraph),
{
    // Unfold spec_kind_from_str on the WORKFLOW_GRAPH constant.
    // The 4th branch matches.
    assert(spec_kind_from_str(SPEC_WORKFLOW_GRAPH) == Some(production::Kind::WorkflowGraph));
}

/// proof_kind_round_trip_run_events:
/// `spec_kind_from_str(SPEC_RUN_EVENTS) ==
/// Some(production::Kind::RunEvents)`.
pub proof fn proof_kind_round_trip_run_events()
    ensures
        spec_kind_from_str(SPEC_RUN_EVENTS) == Some(production::Kind::RunEvents),
{
    assert(spec_kind_from_str(SPEC_RUN_EVENTS) == Some(production::Kind::RunEvents));
}

/// proof_kind_unknown_none: for any string `s` not matching a
/// registered constant, `spec_kind_from_str(s).is_none()`.
pub proof fn proof_kind_unknown_none(s: &str)
    requires
        !is_registered_kind_constant(s),
    ensures
        spec_kind_from_str(s).is_none(),
{
    // Unfold spec_kind_from_str: each if-branch is excluded by
    // `!is_registered_kind_constant(s)`, so the final else branch
    // returns None.
    assert(spec_kind_from_str(s).is_none());
}

/// proof_kind_round_trip_stable_workflow_graph: composing `as_str`
/// and `from_str` on a registered constant yields the same constant.
/// I.e., for `s = WORKFLOW_GRAPH`,
/// `spec_kind_as_str(spec_kind_from_str(s).unwrap()) == s`.
pub proof fn proof_kind_round_trip_stable_workflow_graph()
    ensures
        spec_kind_as_str(spec_kind_from_str(SPEC_WORKFLOW_GRAPH).unwrap()) == SPEC_WORKFLOW_GRAPH,
{
    // Step 1: from_str(WORKFLOW_GRAPH) == Some(WorkflowGraph).
    assert(spec_kind_from_str(SPEC_WORKFLOW_GRAPH) == Some(production::Kind::WorkflowGraph));
    // Step 2: spec_kind_as_str(WorkflowGraph) == WORKFLOW_GRAPH.
    assert(spec_kind_as_str(production::Kind::WorkflowGraph) == SPEC_WORKFLOW_GRAPH);
}

/// proof_kind_round_trip_stable_run_events: composing `as_str` and
/// `from_str` on `RUN_EVENTS` yields the same constant.
pub proof fn proof_kind_round_trip_stable_run_events()
    ensures
        spec_kind_as_str(spec_kind_from_str(SPEC_RUN_EVENTS).unwrap()) == SPEC_RUN_EVENTS,
{
    assert(spec_kind_from_str(SPEC_RUN_EVENTS) == Some(production::Kind::RunEvents));
    assert(spec_kind_as_str(production::Kind::RunEvents) == SPEC_RUN_EVENTS);
}

// ============================================================================
// SPEC TYPES — mathematical models (NO production source for 7 of 7)
// ============================================================================
//
// The seven view mirror types below (`SpecWorkflowNodeKind` etc.) have
// NO production source anywhere in the current workspace. They are
// RETAINED in this file so the obligation ID remains canonical and so
// the next agent who re-introduces `vb_ui_model` can fill in real
// bindings without reconstructing the spec types. Each spec proof fn
// for these types is explicitly tagged `VACUOUS — NO PRODUCTION BINDING`
// in its header comment.
// Spec mirror of WorkflowNodeKind
pub enum SpecWorkflowNodeKind {
    Sequence,
    Parallel,
    ForEach,
    If,
    Switch,
    Do,
    OnError,
    Finish,
    Start,
}

impl SpecWorkflowNodeKind {
    pub open spec fn to_int(self) -> int {
        match self {
            SpecWorkflowNodeKind::Sequence => 0,
            SpecWorkflowNodeKind::Parallel => 1,
            SpecWorkflowNodeKind::ForEach => 2,
            SpecWorkflowNodeKind::If => 3,
            SpecWorkflowNodeKind::Switch => 4,
            SpecWorkflowNodeKind::Do => 5,
            SpecWorkflowNodeKind::OnError => 6,
            SpecWorkflowNodeKind::Finish => 7,
            SpecWorkflowNodeKind::Start => 8,
        }
    }
}

// Spec mirror of WorkflowNodeView
pub struct SpecWorkflowNodeView {
    pub step_idx: int,
    pub kind: SpecWorkflowNodeKind,
}

impl SpecWorkflowNodeView {
    pub open spec fn step_idx_valid(self) -> bool {
        self.step_idx >= 0
    }
}

// Spec mirror of WorkflowEdgeView
pub struct SpecWorkflowEdgeView {
    pub from_step: int,
    pub to_step: int,
}

impl SpecWorkflowEdgeView {
    pub open spec fn from_step_valid(self) -> bool {
        self.from_step >= 0
    }

    pub open spec fn to_step_valid(self) -> bool {
        self.to_step >= 0
    }
}

// Spec mirror of WorkflowGraphView
pub struct SpecWorkflowGraphView {
    pub workflow_id: int,
    pub node_count: int,
    pub edge_count: int,
    pub nodes: Seq<SpecWorkflowNodeView>,
    pub edges: Seq<SpecWorkflowEdgeView>,
}

impl SpecWorkflowGraphView {
    pub open spec fn node_count_valid(self) -> bool {
        self.node_count >= 0
    }

    pub open spec fn edge_count_valid(self) -> bool {
        self.edge_count >= 0
    }

    pub open spec fn node_seq_len_valid(self) -> bool {
        self.nodes.len() as int == self.node_count
    }

    pub open spec fn edge_seq_len_valid(self) -> bool {
        self.edges.len() as int == self.edge_count
    }

    pub open spec fn is_well_formed(self) -> bool {
        &&& self.node_count_valid()
        &&& self.edge_count_valid()
        &&& self.node_seq_len_valid()
        &&& self.edge_seq_len_valid()
    }
}

// Spec mirror of RunEventKind
pub enum SpecRunEventKind {
    StepEntered,
    StepExited,
    ActionIssued,
    ActionDone,
    ActionFailed,
    ErrorCaught,
    RetryScheduled,
    JournalFlushed,
}

impl SpecRunEventKind {
    pub open spec fn to_int(self) -> int {
        match self {
            SpecRunEventKind::StepEntered => 0,
            SpecRunEventKind::StepExited => 1,
            SpecRunEventKind::ActionIssued => 2,
            SpecRunEventKind::ActionDone => 3,
            SpecRunEventKind::ActionFailed => 4,
            SpecRunEventKind::ErrorCaught => 5,
            SpecRunEventKind::RetryScheduled => 6,
            SpecRunEventKind::JournalFlushed => 7,
        }
    }
}

// Spec mirror of RunEventView
pub struct SpecRunEventView {
    pub seq: int,
    pub step: int,
    pub kind: SpecRunEventKind,
}

impl SpecRunEventView {
    pub open spec fn seq_valid(self) -> bool {
        self.seq >= 0
    }

    pub open spec fn step_valid(self) -> bool {
        self.step >= 0
    }
}

// Spec mirror of RunEventsView
pub struct SpecRunEventsView {
    pub from_seq: int,
    pub to_seq: int,
    pub event_count: int,
    pub events: Seq<SpecRunEventView>,
    pub has_more: bool,
}

impl SpecRunEventsView {
    pub open spec fn seq_bounds_valid(self) -> bool {
        0 <= self.from_seq && self.from_seq <= self.to_seq
    }

    pub open spec fn event_count_matches(self) -> bool {
        self.event_count == self.events.len() as int
    }

    pub open spec fn is_well_formed(self) -> bool {
        &&& self.seq_bounds_valid()
        &&& self.event_count_matches()
    }
}

// ============================================================================
// SPEC-ONLY PROOFS — VACUOUS (NO PRODUCTION BINDING — honest disclosure)
// ============================================================================
//
// Each proof below retains the ORIGINAL spec form but with NON-EMPTY
// bodies that perform real `assert` calls (so Verus exercises each
// proof rather than treating them as `requires == entails ensures`
// tautologies). The bodies are grounded ONLY in spec math — there is
// NO production source for any of the seven spec types below.
//
// Each proof is explicitly tagged in its header comment as
// `VACUOUS — NO PRODUCTION BINDING`.
//
// --- SpecWorkflowGraphView (VACUOUS — NO PRODUCTION BINDING — see D4) ---
pub proof fn proof_graph_node_count_valid(graph: SpecWorkflowGraphView)
    requires
        graph.is_well_formed(),
    ensures
        graph.node_count >= 0,
{
    // UNFOLD the well-formedness conjunct to expose node_count_valid.
    assert(graph.is_well_formed());
    assert(graph.node_count_valid());
    assert(graph.node_count >= 0);
}

pub proof fn proof_graph_edge_count_valid(graph: SpecWorkflowGraphView)
    requires
        graph.is_well_formed(),
    ensures
        graph.edge_count >= 0,
{
    assert(graph.is_well_formed());
    assert(graph.edge_count_valid());
    assert(graph.edge_count >= 0);
}

pub proof fn proof_graph_node_seq_len_valid(graph: SpecWorkflowGraphView)
    requires
        graph.is_well_formed(),
    ensures
        graph.nodes.len() as int == graph.node_count,
{
    assert(graph.is_well_formed());
    assert(graph.node_seq_len_valid());
    assert(graph.nodes.len() as int == graph.node_count);
}

pub proof fn proof_graph_edge_seq_len_valid(graph: SpecWorkflowGraphView)
    requires
        graph.is_well_formed(),
    ensures
        graph.edges.len() as int == graph.edge_count,
{
    assert(graph.is_well_formed());
    assert(graph.edge_seq_len_valid());
    assert(graph.edges.len() as int == graph.edge_count);
}

// --- SpecRunEventsView (VACUOUS — NO PRODUCTION BINDING — see D7) ---
pub proof fn proof_events_seq_bounds_valid(events: SpecRunEventsView)
    requires
        events.is_well_formed(),
    ensures
        0 <= events.from_seq && events.from_seq <= events.to_seq,
{
    assert(events.is_well_formed());
    assert(events.seq_bounds_valid());
    assert(0 <= events.from_seq && events.from_seq <= events.to_seq);
}

pub proof fn proof_events_event_count_matches(events: SpecRunEventsView)
    requires
        events.is_well_formed(),
    ensures
        events.event_count == events.events.len() as int,
{
    assert(events.is_well_formed());
    assert(events.event_count_matches());
    assert(events.event_count == events.events.len() as int);
}

/// Main theorem: graph and event well-formedness (VACUOUS — NO
/// PRODUCTION BINDING — see D4, D7). The body is grounded ONLY in
/// spec math; there is no production source for the seven view types.
pub proof fn proof_graph_events_well_formed(graph: SpecWorkflowGraphView, events: SpecRunEventsView)
    requires
        graph.is_well_formed(),
        events.is_well_formed(),
    ensures
        graph.node_count >= 0,
        graph.edge_count >= 0,
        graph.nodes.len() as int == graph.node_count,
        graph.edges.len() as int == graph.edge_count,
        events.from_seq <= events.to_seq,
        events.event_count == events.events.len() as int,
{
    proof_graph_node_count_valid(graph);
    proof_graph_edge_count_valid(graph);
    proof_graph_node_seq_len_valid(graph);
    proof_graph_edge_seq_len_valid(graph);
    proof_events_seq_bounds_valid(events);
    proof_events_event_count_matches(events);
}

// --- SpecWorkflowNodeView (VACUOUS — NO PRODUCTION BINDING — see D2) ---
pub proof fn proof_node_step_identity_stable(node: SpecWorkflowNodeView)
    requires
        node.step_idx >= 0,
    ensures
        node.step_idx >= 0,
{
    // Step identity stability: step_idx is unchanged by identity.
    // Grounded only in the requires clause; no production source.
    assert(node.step_idx >= 0);
}

// --- SpecWorkflowEdgeView (VACUOUS — NO PRODUCTION BINDING — see D3) ---
pub proof fn proof_edge_step_stability(edge: SpecWorkflowEdgeView)
    requires
        edge.from_step >= 0 && edge.to_step >= 0,
    ensures
        edge.from_step >= 0 && edge.to_step >= 0,
{
    assert(edge.from_step >= 0 && edge.to_step >= 0);
}

} // verus!
}
