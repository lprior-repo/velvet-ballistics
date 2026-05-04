//! Convert VB compiled workflow IR into flow-core document model for visualization.
//!
//! This module bridges VB's runtime IR (`CompiledNode` / `WorkflowParts`) and the
//! flow editor's document model (`FlowDocument`). It walks the compiled node array,
//! extracts port connectivity from node kind fields, emits edges for sequential and
//! branch targets, and groups loop spans for visual nesting.
//!
//! The module is gated behind the `flow-doc` feature because `flow_core` may not
//! always be available during early scaffolding.

use indexmap::IndexMap;
use smol_str::SmolStr;

use vb_core::workflow::{CompiledNode, CompiledNodeKind, WorkflowParts};

// ---------------------------------------------------------------------------
// Flow-core types (re-exported or used directly). These match the flow_core
// crate's public API. When flow_core is fully integrated, these imports
// resolve directly. The types are defined here as reference documentation.
// ---------------------------------------------------------------------------

/// Semantic port side: input or output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortSide {
    /// Port receives data into the node.
    Input,
    /// Port emits data from the node.
    Output,
}

/// Role a port plays in the node's contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortRole {
    /// Primary data flow.
    Data,
    /// Control-flow trigger (e.g. branch condition).
    Trigger,
    /// Loop body entry.
    Body,
    /// Loop/group completion.
    Done,
    /// Error handler entry.
    Handler,
    /// Otherwise/default branch.
    Otherwise,
    /// Exhausted retry path.
    Exhausted,
}

/// How many connections a port accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    /// Exactly one connection.
    One,
    /// Zero or more connections.
    Many,
}

/// Visual edge style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeStyle {
    /// True for a dashed (error/conditional) line.
    pub dashed: bool,
    /// True for a highlighted edge.
    pub highlighted: bool,
}

impl EdgeStyle {
    /// Default solid edge style.
    #[must_use]
    pub const fn default_solid() -> Self {
        Self {
            dashed: false,
            highlighted: false,
        }
    }

    /// Dashed edge for error routes and conditional branches.
    #[must_use]
    pub const fn dashed() -> Self {
        Self {
            dashed: true,
            highlighted: false,
        }
    }
}

/// Group visual kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    /// Container for branch/loop children.
    BranchContainer,
    /// Horizontal swimlane.
    Swimlane,
}

/// Node flags controlling editor behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NodeFlags {
    /// Node position is locked.
    pub locked: bool,
    /// Node is hidden.
    pub hidden: bool,
    /// Terminal / finish node.
    pub terminal: bool,
    /// Entry node of the workflow.
    pub entry: bool,
}

/// Default editor metadata placeholder.
#[derive(Debug, Clone, Default)]
pub struct EditorMetadata;

/// A single port on a flow node.
#[derive(Debug, Clone)]
pub struct FlowPortRecord {
    /// Unique port identifier within the node.
    pub id: SmolStr,
    /// Human-readable label.
    pub label: SmolStr,
    /// Which side of the node.
    pub side: PortSide,
    /// Role of this port.
    pub role: PortRole,
    /// Connection cardinality.
    pub cardinality: Cardinality,
}

/// Visual / interaction state for a node.
#[derive(Debug, Clone, Default)]
pub struct NodeUiState;

/// A single node in the flow graph.
#[derive(Debug, Clone)]
pub struct FlowNodeRecord {
    /// Unique node identifier.
    pub id: SmolStr,
    /// Kind / category tag.
    pub kind: SmolStr,
    /// Display title.
    pub title: SmolStr,
    /// Position [x, y] -- layout fills this in.
    pub position: [f64; 2],
    /// Bounding box size [width, height].
    pub size: [f64; 2],
    /// Z-order.
    pub z_index: i32,
    /// Optional parent group.
    pub parent: Option<SmolStr>,
    /// Ports attached to this node.
    pub ports: Vec<FlowPortRecord>,
    /// Editor flags.
    pub flags: NodeFlags,
    /// Opaque data payload (null for VB nodes).
    pub data: serde_json::Value,
    /// UI state.
    pub ui: NodeUiState,
}

/// A directed edge in the flow graph.
#[derive(Debug, Clone)]
pub struct FlowEdgeRecord {
    /// Unique edge identifier.
    pub id: SmolStr,
    /// Source node.
    pub source: SmolStr,
    /// Source port.
    pub source_port: SmolStr,
    /// Target node.
    pub target: SmolStr,
    /// Target port.
    pub target_port: SmolStr,
    /// Visual style.
    pub style: EdgeStyle,
    /// Optional label.
    pub label: Option<SmolStr>,
}

/// A visual group (loop, swimlane, etc.).
#[derive(Debug, Clone)]
pub struct FlowGroupRecord {
    /// Unique group identifier.
    pub id: SmolStr,
    /// Display label.
    pub label: SmolStr,
    /// Group kind.
    pub kind: GroupKind,
    /// Member node IDs.
    pub children: Vec<SmolStr>,
}

/// The full flow graph.
#[derive(Debug, Clone)]
pub struct FlowGraph {
    /// Ordered node records.
    pub nodes: IndexMap<SmolStr, FlowNodeRecord>,
    /// Ordered edge records.
    pub edges: IndexMap<SmolStr, FlowEdgeRecord>,
    /// Visual groups.
    pub groups: IndexMap<SmolStr, FlowGroupRecord>,
    /// Entry node identifier.
    pub entry_node: Option<SmolStr>,
}

/// Top-level flow document.
#[derive(Debug, Clone)]
pub struct FlowDocument {
    /// Schema identifier.
    pub schema: SmolStr,
    /// Semantic source kind.
    pub semantic_kind: SmolStr,
    /// The graph.
    pub graph: FlowGraph,
    /// Editor metadata.
    pub editor: EditorMetadata,
    /// Plugin state (empty for VB).
    pub plugin_state: IndexMap<SmolStr, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Build a `FlowDocument` from VB compiled workflow parts.
///
/// Walks the compiled node array, creates a `FlowNodeRecord` for each node,
/// emits edges for sequential (`next`) and kind-specific targets (branches,
/// loop body/done, error handlers, jumps), and groups loop spans.
#[must_use]
pub fn build_document(parts: &WorkflowParts) -> FlowDocument {
    let mut nodes = IndexMap::new();
    let mut edges = IndexMap::new();
    let mut groups = IndexMap::new();

    // Phase 1: build node records.
    for (i, node) in parts.nodes.iter().enumerate() {
        let node_id = SmolStr::from(format!("step-{i}"));
        let (kind_label, category) = classify_node_kind(&node.kind);
        let (input_ports, output_ports) = build_ports(&node.kind, node.output);
        let mut ports = input_ports;
        ports.extend(output_ports);

        let flags = NodeFlags {
            terminal: matches!(node.kind, CompiledNodeKind::Finish { .. }),
            entry: i == parts.entry.as_usize(),
            ..NodeFlags::default()
        };

        let record = FlowNodeRecord {
            id: node_id.clone(),
            kind: SmolStr::from(category),
            title: SmolStr::from(kind_label),
            position: [0.0, 0.0],
            size: compute_node_size(&ports),
            z_index: 0,
            parent: None,
            ports,
            flags,
            data: serde_json::Value::Null,
            ui: NodeUiState,
        };
        nodes.insert(node_id, record);
    }

    // Phase 2: build edges from node.next and kind-specific targets.
    let mut edge_counter: u32 = 0;
    for (i, node) in parts.nodes.iter().enumerate() {
        let source_id = SmolStr::from(format!("step-{i}"));

        // Sequential next edge.
        if let Some(next) = node.next {
            let target_id = SmolStr::from(format!("step-{}", next.as_usize()));
            add_edge(
                &mut edges,
                &mut edge_counter,
                &source_id,
                "next",
                &target_id,
                "in",
                EdgeStyle::default_solid(),
                None,
            );
        }

        // Kind-specific edges (branches, loops, error handlers, jumps).
        add_kind_edges(&mut edges, &mut edge_counter, &source_id, &node.kind);
    }

    // Phase 3: build loop groups.
    build_loop_groups(&parts.nodes, &mut groups);

    FlowDocument {
        schema: SmolStr::new_static("makepad.flow/v2"),
        semantic_kind: SmolStr::new_static("velvet-ballistics"),
        graph: FlowGraph {
            nodes,
            edges,
            groups,
            entry_node: Some(SmolStr::from(format!("step-{}", parts.entry.as_usize()))),
        },
        editor: EditorMetadata,
        plugin_state: IndexMap::new(),
    }
}

// ---------------------------------------------------------------------------
// classify_node_kind
// ---------------------------------------------------------------------------

/// Returns `(label, category)` for a compiled node kind.
///
/// Categories correspond to visual groupings in the flow editor palette.
#[must_use]
pub fn classify_node_kind(kind: &CompiledNodeKind) -> (&'static str, &'static str) {
    match kind {
        CompiledNodeKind::Nop => ("Nop", "control"),
        CompiledNodeKind::SetConst { .. } => ("SetConst", "data"),
        CompiledNodeKind::Copy { .. } => ("Copy", "data"),
        CompiledNodeKind::EvalExpr { .. } => ("EvalExpr", "data"),
        CompiledNodeKind::BuildObject { .. } => ("BuildObject", "construct"),
        CompiledNodeKind::BuildList { .. } => ("BuildList", "construct"),
        CompiledNodeKind::Do { .. } => ("Do", "external"),
        CompiledNodeKind::Choose { .. } => ("Choose", "branch"),
        CompiledNodeKind::ChooseSlot { .. } => ("ChooseSlot", "branch"),
        CompiledNodeKind::ForEachStart { .. } => ("ForEachStart", "loop"),
        CompiledNodeKind::ForEachNext { .. } => ("ForEachNext", "loop"),
        CompiledNodeKind::ForEachJoin { .. } => ("ForEachJoin", "loop"),
        CompiledNodeKind::TogetherStart { .. } => ("TogetherStart", "parallel"),
        CompiledNodeKind::TogetherBranch { .. } => ("TogetherBranch", "parallel"),
        CompiledNodeKind::TogetherJoin { .. } => ("TogetherJoin", "parallel"),
        CompiledNodeKind::CollectStart { .. } => ("CollectStart", "collect"),
        CompiledNodeKind::CollectPage { .. } => ("CollectPage", "collect"),
        CompiledNodeKind::CollectNext { .. } => ("CollectNext", "collect"),
        CompiledNodeKind::CollectFinish { .. } => ("CollectFinish", "collect"),
        CompiledNodeKind::ReduceStart { .. } => ("ReduceStart", "reduce"),
        CompiledNodeKind::ReduceNext { .. } => ("ReduceNext", "reduce"),
        CompiledNodeKind::ReduceFinish { .. } => ("ReduceFinish", "reduce"),
        CompiledNodeKind::RepeatStart { .. } => ("RepeatStart", "retry"),
        CompiledNodeKind::RepeatAttempt { .. } => ("RepeatAttempt", "retry"),
        CompiledNodeKind::RepeatCheck { .. } => ("RepeatCheck", "retry"),
        CompiledNodeKind::RepeatFinish { .. } => ("RepeatFinish", "retry"),
        CompiledNodeKind::WaitUntil { .. } => ("WaitUntil", "suspend"),
        CompiledNodeKind::WaitEvent { .. } => ("WaitEvent", "suspend"),
        CompiledNodeKind::Ask { .. } => ("Ask", "suspend"),
        CompiledNodeKind::AskResume { .. } => ("AskResume", "suspend"),
        CompiledNodeKind::RetryCheck { .. } => ("RetryCheck", "retry"),
        CompiledNodeKind::ErrorHandler { .. } => ("ErrorHandler", "error"),
        CompiledNodeKind::Jump { .. } => ("Jump", "control"),
        CompiledNodeKind::Finish { .. } => ("Finish", "terminal"),
    }
}

// ---------------------------------------------------------------------------
// build_ports
// ---------------------------------------------------------------------------

/// Extract input and output ports from a compiled node kind.
///
/// Returns `(input_ports, output_ports)`. Each `SlotIdx` that a node reads
/// from becomes an input port. The node's output slot (if present) becomes
/// an output port. Kind-specific fields produce named ports.
#[must_use]
pub fn build_ports(
    kind: &CompiledNodeKind,
    output: Option<vb_core::ids::SlotIdx>,
) -> (Vec<FlowPortRecord>, Vec<FlowPortRecord>) {
    let mut inputs = Vec::new();
    let mut outputs = Vec::new();

    // Output slot port (most nodes have one).
    if let Some(slot) = output {
        outputs.push(FlowPortRecord {
            id: SmolStr::new_static("out"),
            label: SmolStr::from(format!("slot-{}", slot.get())),
            side: PortSide::Output,
            role: PortRole::Data,
            cardinality: Cardinality::One,
        });
    }

    match kind {
        CompiledNodeKind::Nop => {}

        CompiledNodeKind::SetConst { value: _ } => {
            // No input ports -- constant comes from the pool.
        }

        CompiledNodeKind::Copy { source } => {
            inputs.push(slot_input_port("source", source.get()));
        }

        CompiledNodeKind::EvalExpr { expr: _ } => {
            // Expression reads from the expression bytecode, not a slot directly.
            inputs.push(FlowPortRecord {
                id: SmolStr::new_static("expr"),
                label: SmolStr::new_static("expr"),
                side: PortSide::Input,
                role: PortRole::Data,
                cardinality: Cardinality::One,
            });
        }

        CompiledNodeKind::BuildObject { fields } => {
            for (i, (_sym, slot)) in fields.iter().enumerate() {
                inputs.push(slot_input_port(&format!("field-{i}"), slot.get()));
            }
        }

        CompiledNodeKind::BuildList { items } => {
            for (i, slot) in items.iter().enumerate() {
                inputs.push(slot_input_port(&format!("item-{i}"), slot.get()));
            }
        }

        CompiledNodeKind::Do { action: _, input } => {
            inputs.push(slot_input_port("input", input.get()));
        }

        CompiledNodeKind::Choose { branches, .. } => {
            for (i, _branch) in branches.iter().enumerate() {
                outputs.push(FlowPortRecord {
                    id: SmolStr::from(format!("branch-{i}")),
                    label: SmolStr::from(format!("branch-{i}")),
                    side: PortSide::Output,
                    role: PortRole::Trigger,
                    cardinality: Cardinality::One,
                });
            }
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("otherwise"),
                label: SmolStr::new_static("otherwise"),
                side: PortSide::Output,
                role: PortRole::Otherwise,
                cardinality: Cardinality::One,
            });
        }

        CompiledNodeKind::ChooseSlot { branches, .. } => {
            for (i, _branch) in branches.iter().enumerate() {
                outputs.push(FlowPortRecord {
                    id: SmolStr::from(format!("branch-{i}")),
                    label: SmolStr::from(format!("branch-{i}")),
                    side: PortSide::Output,
                    role: PortRole::Trigger,
                    cardinality: Cardinality::One,
                });
            }
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("otherwise"),
                label: SmolStr::new_static("otherwise"),
                side: PortSide::Output,
                role: PortRole::Otherwise,
                cardinality: Cardinality::One,
            });
        }

        CompiledNodeKind::ForEachStart {
            input,
            item_slot: _,
            limit: _,
            body: _,
            done: _,
        } => {
            inputs.push(slot_input_port("input", input.get()));
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("body"),
                label: SmolStr::new_static("body"),
                side: PortSide::Output,
                role: PortRole::Body,
                cardinality: Cardinality::One,
            });
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("done"),
                label: SmolStr::new_static("done"),
                side: PortSide::Output,
                role: PortRole::Done,
                cardinality: Cardinality::One,
            });
        }

        CompiledNodeKind::ForEachNext {
            iterator_slot,
            body: _,
            done: _,
        } => {
            inputs.push(slot_input_port("iterator", iterator_slot.get()));
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("body"),
                label: SmolStr::new_static("body"),
                side: PortSide::Output,
                role: PortRole::Body,
                cardinality: Cardinality::One,
            });
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("done"),
                label: SmolStr::new_static("done"),
                side: PortSide::Output,
                role: PortRole::Done,
                cardinality: Cardinality::One,
            });
        }

        CompiledNodeKind::ForEachJoin {
            output: join_output,
        } => {
            inputs.push(slot_input_port("output", join_output.get()));
        }

        CompiledNodeKind::TogetherStart { branches, join: _ } => {
            for i in 0..branches.len() {
                outputs.push(FlowPortRecord {
                    id: SmolStr::from(format!("branch-{i}")),
                    label: SmolStr::from(format!("branch-{i}")),
                    side: PortSide::Output,
                    role: PortRole::Trigger,
                    cardinality: Cardinality::One,
                });
            }
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("join"),
                label: SmolStr::new_static("join"),
                side: PortSide::Output,
                role: PortRole::Done,
                cardinality: Cardinality::One,
            });
        }

        CompiledNodeKind::TogetherBranch {
            branch: _,
            entry: _,
            join: _,
            accumulator,
        } => {
            inputs.push(slot_input_port("accumulator", accumulator.get()));
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("entry"),
                label: SmolStr::new_static("entry"),
                side: PortSide::Output,
                role: PortRole::Body,
                cardinality: Cardinality::One,
            });
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("join"),
                label: SmolStr::new_static("join"),
                side: PortSide::Output,
                role: PortRole::Done,
                cardinality: Cardinality::One,
            });
        }

        CompiledNodeKind::TogetherJoin {
            branch_count: _,
            accumulator,
        } => {
            inputs.push(slot_input_port("accumulator", accumulator.get()));
        }

        CompiledNodeKind::CollectStart {
            source,
            limit: _,
            page_size: _,
            body: _,
            done: _,
        } => {
            inputs.push(slot_input_port("source", source.get()));
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("body"),
                label: SmolStr::new_static("body"),
                side: PortSide::Output,
                role: PortRole::Body,
                cardinality: Cardinality::One,
            });
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("done"),
                label: SmolStr::new_static("done"),
                side: PortSide::Output,
                role: PortRole::Done,
                cardinality: Cardinality::One,
            });
        }

        CompiledNodeKind::CollectPage {
            collector_slot,
            body: _,
            done: _,
        }
        | CompiledNodeKind::CollectNext {
            collector_slot,
            body: _,
            done: _,
        } => {
            inputs.push(slot_input_port("collector", collector_slot.get()));
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("body"),
                label: SmolStr::new_static("body"),
                side: PortSide::Output,
                role: PortRole::Body,
                cardinality: Cardinality::One,
            });
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("done"),
                label: SmolStr::new_static("done"),
                side: PortSide::Output,
                role: PortRole::Done,
                cardinality: Cardinality::One,
            });
        }

        CompiledNodeKind::CollectFinish { collector_slot } => {
            inputs.push(slot_input_port("collector", collector_slot.get()));
        }

        CompiledNodeKind::ReduceStart {
            input,
            accumulator: _,
            initial: _,
            body: _,
            done: _,
        } => {
            inputs.push(slot_input_port("input", input.get()));
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("body"),
                label: SmolStr::new_static("body"),
                side: PortSide::Output,
                role: PortRole::Body,
                cardinality: Cardinality::One,
            });
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("done"),
                label: SmolStr::new_static("done"),
                side: PortSide::Output,
                role: PortRole::Done,
                cardinality: Cardinality::One,
            });
        }

        CompiledNodeKind::ReduceNext {
            iterator_slot,
            accumulator: _,
            body: _,
            done: _,
        } => {
            inputs.push(slot_input_port("iterator", iterator_slot.get()));
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("body"),
                label: SmolStr::new_static("body"),
                side: PortSide::Output,
                role: PortRole::Body,
                cardinality: Cardinality::One,
            });
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("done"),
                label: SmolStr::new_static("done"),
                side: PortSide::Output,
                role: PortRole::Done,
                cardinality: Cardinality::One,
            });
        }

        CompiledNodeKind::ReduceFinish { accumulator } => {
            inputs.push(slot_input_port("accumulator", accumulator.get()));
        }

        CompiledNodeKind::RepeatStart {
            max_attempts: _,
            body: _,
            done: _,
        } => {
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("body"),
                label: SmolStr::new_static("body"),
                side: PortSide::Output,
                role: PortRole::Body,
                cardinality: Cardinality::One,
            });
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("done"),
                label: SmolStr::new_static("done"),
                side: PortSide::Output,
                role: PortRole::Done,
                cardinality: Cardinality::One,
            });
        }

        CompiledNodeKind::RepeatAttempt {
            attempt_slot,
            body: _,
            done: _,
        } => {
            inputs.push(slot_input_port("attempt", attempt_slot.get()));
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("body"),
                label: SmolStr::new_static("body"),
                side: PortSide::Output,
                role: PortRole::Body,
                cardinality: Cardinality::One,
            });
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("done"),
                label: SmolStr::new_static("done"),
                side: PortSide::Output,
                role: PortRole::Done,
                cardinality: Cardinality::One,
            });
        }

        CompiledNodeKind::RepeatCheck {
            attempt_slot,
            done: _,
        } => {
            inputs.push(slot_input_port("attempt", attempt_slot.get()));
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("done"),
                label: SmolStr::new_static("done"),
                side: PortSide::Output,
                role: PortRole::Done,
                cardinality: Cardinality::One,
            });
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("exhausted"),
                label: SmolStr::new_static("exhausted"),
                side: PortSide::Output,
                role: PortRole::Exhausted,
                cardinality: Cardinality::One,
            });
        }

        CompiledNodeKind::RepeatFinish { result } => {
            inputs.push(slot_input_port("result", result.get()));
        }

        CompiledNodeKind::WaitUntil { deadline_slot } => {
            inputs.push(slot_input_port("deadline", deadline_slot.get()));
        }

        CompiledNodeKind::WaitEvent {
            event,
            timeout_slot,
        } => {
            inputs.push(slot_input_port("event", event.get()));
            if let Some(timeout) = timeout_slot {
                inputs.push(slot_input_port("timeout", timeout.get()));
            }
        }

        CompiledNodeKind::Ask {
            prompt,
            timeout_slot,
        } => {
            inputs.push(slot_input_port("prompt", prompt.get()));
            if let Some(timeout) = timeout_slot {
                inputs.push(slot_input_port("timeout", timeout.get()));
            }
        }

        CompiledNodeKind::AskResume { answer } => {
            inputs.push(slot_input_port("answer", answer.get()));
        }

        CompiledNodeKind::RetryCheck {
            policy_slot,
            body: _,
            exhausted: _,
        } => {
            inputs.push(slot_input_port("policy", policy_slot.get()));
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("body"),
                label: SmolStr::new_static("body"),
                side: PortSide::Output,
                role: PortRole::Body,
                cardinality: Cardinality::One,
            });
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("exhausted"),
                label: SmolStr::new_static("exhausted"),
                side: PortSide::Output,
                role: PortRole::Exhausted,
                cardinality: Cardinality::One,
            });
        }

        CompiledNodeKind::ErrorHandler {
            body: _, handler, ..
        } => {
            outputs.push(FlowPortRecord {
                id: SmolStr::new_static("handler"),
                label: SmolStr::new_static("handler"),
                side: PortSide::Output,
                role: PortRole::Handler,
                cardinality: Cardinality::One,
            });
            // The handler target step is used for the edge; record the slot
            // index for port labeling only if we had a slot (we don't --
            // handler is a StepIdx, not a SlotIdx).
            let _ = handler;
        }

        CompiledNodeKind::Jump { target: _ } => {
            // No ports -- the edge carries the target.
        }

        CompiledNodeKind::Finish { result } => {
            inputs.push(slot_input_port("result", result.get()));
        }
    }

    (inputs, outputs)
}

// ---------------------------------------------------------------------------
// Edge helpers
// ---------------------------------------------------------------------------

/// Create a `FlowEdgeRecord` and insert it into the edge map.
#[allow(clippy::too_many_arguments)]
fn add_edge(
    edges: &mut IndexMap<SmolStr, FlowEdgeRecord>,
    counter: &mut u32,
    source: &SmolStr,
    source_port: &str,
    target: &SmolStr,
    target_port: &str,
    style: EdgeStyle,
    label: Option<&str>,
) {
    let id = SmolStr::from(format!("edge-{counter}"));
    *counter = match counter.checked_add(1) {
        Some(v) => v,
        None => return, // saturate silently -- >4B edges is unreasonable
    };
    let record = FlowEdgeRecord {
        id: id.clone(),
        source: source.clone(),
        source_port: SmolStr::from(source_port),
        target: target.clone(),
        target_port: SmolStr::from(target_port),
        style,
        label: label.map(SmolStr::from),
    };
    edges.insert(id, record);
}

/// Emit kind-specific edges for branches, loops, error handlers, and jumps.
fn add_kind_edges(
    edges: &mut IndexMap<SmolStr, FlowEdgeRecord>,
    counter: &mut u32,
    source_id: &SmolStr,
    kind: &CompiledNodeKind,
) {
    match kind {
        CompiledNodeKind::Choose {
            branches,
            otherwise,
        } => {
            for (i, branch) in branches.iter().enumerate() {
                let target = SmolStr::from(format!("step-{}", branch.target.as_usize()));
                add_edge(
                    edges,
                    counter,
                    source_id,
                    &format!("branch-{i}"),
                    &target,
                    "in",
                    EdgeStyle::default_solid(),
                    Some(&format!("branch-{i}")),
                );
            }
            if let Some(other) = otherwise {
                let target = SmolStr::from(format!("step-{}", other.as_usize()));
                add_edge(
                    edges,
                    counter,
                    source_id,
                    "otherwise",
                    &target,
                    "in",
                    EdgeStyle::dashed(),
                    Some("otherwise"),
                );
            }
        }

        CompiledNodeKind::ChooseSlot {
            branches,
            otherwise,
        } => {
            for (i, branch) in branches.iter().enumerate() {
                let target = SmolStr::from(format!("step-{}", branch.target.as_usize()));
                add_edge(
                    edges,
                    counter,
                    source_id,
                    &format!("branch-{i}"),
                    &target,
                    "in",
                    EdgeStyle::default_solid(),
                    Some(&format!("branch-{i}")),
                );
            }
            if let Some(other) = otherwise {
                let target = SmolStr::from(format!("step-{}", other.as_usize()));
                add_edge(
                    edges,
                    counter,
                    source_id,
                    "otherwise",
                    &target,
                    "in",
                    EdgeStyle::dashed(),
                    Some("otherwise"),
                );
            }
        }

        CompiledNodeKind::ForEachStart { body, done, .. }
        | CompiledNodeKind::ForEachNext { body, done, .. } => {
            let body_target = SmolStr::from(format!("step-{}", body.as_usize()));
            add_edge(
                edges,
                counter,
                source_id,
                "body",
                &body_target,
                "in",
                EdgeStyle::default_solid(),
                Some("body"),
            );
            let done_target = SmolStr::from(format!("step-{}", done.as_usize()));
            add_edge(
                edges,
                counter,
                source_id,
                "done",
                &done_target,
                "in",
                EdgeStyle::dashed(),
                Some("done"),
            );
        }

        CompiledNodeKind::TogetherStart { branches, join } => {
            for (i, branch_target) in branches.iter().enumerate() {
                let target = SmolStr::from(format!("step-{}", branch_target.as_usize()));
                add_edge(
                    edges,
                    counter,
                    source_id,
                    &format!("branch-{i}"),
                    &target,
                    "in",
                    EdgeStyle::default_solid(),
                    Some(&format!("branch-{i}")),
                );
            }
            let join_target = SmolStr::from(format!("step-{}", join.as_usize()));
            add_edge(
                edges,
                counter,
                source_id,
                "join",
                &join_target,
                "in",
                EdgeStyle::dashed(),
                Some("join"),
            );
        }

        CompiledNodeKind::TogetherBranch { entry, join, .. } => {
            let entry_target = SmolStr::from(format!("step-{}", entry.as_usize()));
            add_edge(
                edges,
                counter,
                source_id,
                "entry",
                &entry_target,
                "in",
                EdgeStyle::default_solid(),
                Some("entry"),
            );
            let join_target = SmolStr::from(format!("step-{}", join.as_usize()));
            add_edge(
                edges,
                counter,
                source_id,
                "join",
                &join_target,
                "in",
                EdgeStyle::dashed(),
                Some("join"),
            );
        }

        CompiledNodeKind::CollectStart { body, done, .. }
        | CompiledNodeKind::CollectPage { body, done, .. }
        | CompiledNodeKind::CollectNext { body, done, .. } => {
            let body_target = SmolStr::from(format!("step-{}", body.as_usize()));
            add_edge(
                edges,
                counter,
                source_id,
                "body",
                &body_target,
                "in",
                EdgeStyle::default_solid(),
                Some("body"),
            );
            let done_target = SmolStr::from(format!("step-{}", done.as_usize()));
            add_edge(
                edges,
                counter,
                source_id,
                "done",
                &done_target,
                "in",
                EdgeStyle::dashed(),
                Some("done"),
            );
        }

        CompiledNodeKind::ReduceStart { body, done, .. }
        | CompiledNodeKind::ReduceNext { body, done, .. } => {
            let body_target = SmolStr::from(format!("step-{}", body.as_usize()));
            add_edge(
                edges,
                counter,
                source_id,
                "body",
                &body_target,
                "in",
                EdgeStyle::default_solid(),
                Some("body"),
            );
            let done_target = SmolStr::from(format!("step-{}", done.as_usize()));
            add_edge(
                edges,
                counter,
                source_id,
                "done",
                &done_target,
                "in",
                EdgeStyle::dashed(),
                Some("done"),
            );
        }

        CompiledNodeKind::RepeatStart { body, done, .. }
        | CompiledNodeKind::RepeatAttempt { body, done, .. } => {
            let body_target = SmolStr::from(format!("step-{}", body.as_usize()));
            add_edge(
                edges,
                counter,
                source_id,
                "body",
                &body_target,
                "in",
                EdgeStyle::default_solid(),
                Some("body"),
            );
            let done_target = SmolStr::from(format!("step-{}", done.as_usize()));
            add_edge(
                edges,
                counter,
                source_id,
                "done",
                &done_target,
                "in",
                EdgeStyle::dashed(),
                Some("done"),
            );
        }

        CompiledNodeKind::RepeatCheck {
            attempt_slot: _,
            done,
        } => {
            // RepeatCheck has a `done` target (success retry) and falls
            // through `next` for exhausted. But since RepeatCheck's semantic
            // is "check if retries exhausted", done = retry succeeded, and
            // the exhausted path goes through `next`. We emit the done edge
            // here; the exhausted path is already the `next` edge.
            let done_target = SmolStr::from(format!("step-{}", done.as_usize()));
            add_edge(
                edges,
                counter,
                source_id,
                "done",
                &done_target,
                "in",
                EdgeStyle::dashed(),
                Some("done"),
            );
        }

        CompiledNodeKind::RetryCheck {
            policy_slot: _,
            body,
            exhausted,
        } => {
            let body_target = SmolStr::from(format!("step-{}", body.as_usize()));
            add_edge(
                edges,
                counter,
                source_id,
                "body",
                &body_target,
                "in",
                EdgeStyle::default_solid(),
                Some("retry"),
            );
            let exhausted_target = SmolStr::from(format!("step-{}", exhausted.as_usize()));
            add_edge(
                edges,
                counter,
                source_id,
                "exhausted",
                &exhausted_target,
                "in",
                EdgeStyle::dashed(),
                Some("exhausted"),
            );
        }

        CompiledNodeKind::ErrorHandler {
            body: _, handler, ..
        } => {
            let handler_target = SmolStr::from(format!("step-{}", handler.as_usize()));
            add_edge(
                edges,
                counter,
                source_id,
                "handler",
                &handler_target,
                "in",
                EdgeStyle::dashed(),
                Some("error-handler"),
            );
        }

        CompiledNodeKind::Jump { target } => {
            let target_id = SmolStr::from(format!("step-{}", target.as_usize()));
            add_edge(
                edges,
                counter,
                source_id,
                "jump",
                &target_id,
                "in",
                EdgeStyle::default_solid(),
                Some("jump"),
            );
        }

        // Remaining variants have no kind-specific edges beyond `next`.
        CompiledNodeKind::Nop
        | CompiledNodeKind::SetConst { .. }
        | CompiledNodeKind::Copy { .. }
        | CompiledNodeKind::EvalExpr { .. }
        | CompiledNodeKind::BuildObject { .. }
        | CompiledNodeKind::BuildList { .. }
        | CompiledNodeKind::Do { .. }
        | CompiledNodeKind::ForEachJoin { .. }
        | CompiledNodeKind::TogetherJoin { .. }
        | CompiledNodeKind::CollectFinish { .. }
        | CompiledNodeKind::ReduceFinish { .. }
        | CompiledNodeKind::RepeatFinish { .. }
        | CompiledNodeKind::WaitUntil { .. }
        | CompiledNodeKind::WaitEvent { .. }
        | CompiledNodeKind::Ask { .. }
        | CompiledNodeKind::AskResume { .. }
        | CompiledNodeKind::Finish { .. } => {}
    }
}

// ---------------------------------------------------------------------------
// Loop group builder
// ---------------------------------------------------------------------------

/// Create `FlowGroupRecord`s for loop structures.
///
/// Scans the node array for loop start/join pairs and creates groups spanning
/// the enclosed nodes. The group kind is `BranchContainer` for loops with
/// bodies and `Swimlane` for parallel constructs.
fn build_loop_groups(nodes: &[CompiledNode], groups: &mut IndexMap<SmolStr, FlowGroupRecord>) {
    for (i, node) in nodes.iter().enumerate() {
        match &node.kind {
            CompiledNodeKind::ForEachStart { done, .. } => {
                let group_id = SmolStr::from(format!("group-foreach-{i}"));
                let children = collect_span(i, done.as_usize(), nodes.len());
                groups.insert(
                    group_id.clone(),
                    FlowGroupRecord {
                        id: group_id,
                        label: SmolStr::from(format!("ForEach-{i}")),
                        kind: GroupKind::BranchContainer,
                        children,
                    },
                );
            }

            CompiledNodeKind::TogetherStart { join, .. } => {
                let group_id = SmolStr::from(format!("group-together-{i}"));
                let children = collect_span(i, join.as_usize(), nodes.len());
                groups.insert(
                    group_id.clone(),
                    FlowGroupRecord {
                        id: group_id,
                        label: SmolStr::from(format!("Together-{i}")),
                        kind: GroupKind::Swimlane,
                        children,
                    },
                );
            }

            CompiledNodeKind::CollectStart { done, .. } => {
                let group_id = SmolStr::from(format!("group-collect-{i}"));
                let children = collect_span(i, done.as_usize(), nodes.len());
                groups.insert(
                    group_id.clone(),
                    FlowGroupRecord {
                        id: group_id,
                        label: SmolStr::from(format!("Collect-{i}")),
                        kind: GroupKind::BranchContainer,
                        children,
                    },
                );
            }

            CompiledNodeKind::ReduceStart { done, .. } => {
                let group_id = SmolStr::from(format!("group-reduce-{i}"));
                let children = collect_span(i, done.as_usize(), nodes.len());
                groups.insert(
                    group_id.clone(),
                    FlowGroupRecord {
                        id: group_id,
                        label: SmolStr::from(format!("Reduce-{i}")),
                        kind: GroupKind::BranchContainer,
                        children,
                    },
                );
            }

            CompiledNodeKind::RepeatStart { done, .. } => {
                let group_id = SmolStr::from(format!("group-repeat-{i}"));
                let children = collect_span(i, done.as_usize(), nodes.len());
                groups.insert(
                    group_id.clone(),
                    FlowGroupRecord {
                        id: group_id,
                        label: SmolStr::from(format!("Repeat-{i}")),
                        kind: GroupKind::BranchContainer,
                        children,
                    },
                );
            }

            _ => {}
        }
    }
}

/// Collect node IDs spanning from `start` (inclusive) to `end` (inclusive).
///
/// Returns node IDs for each index in `[start, end]`. If `end` < `start` or
/// either bound exceeds `total`, returns an empty list.
fn collect_span(start: usize, end: usize, total: usize) -> Vec<SmolStr> {
    if end < start || end >= total {
        return Vec::new();
    }
    // end >= start is guaranteed here, and end < total, so end - start cannot
    // underflow and will not overflow usize.
    let span = end.saturating_sub(start);
    let count = span.saturating_add(1);
    let mut children = Vec::with_capacity(count);
    let mut idx = start;
    while idx <= end {
        children.push(SmolStr::from(format!("step-{idx}")));
        idx = match idx.checked_add(1) {
            Some(v) => v,
            None => break,
        };
    }
    children
}

// ---------------------------------------------------------------------------
// Node size heuristic
// ---------------------------------------------------------------------------

/// Compute a heuristic node bounding box from the port count.
///
/// Width scales from 160 to 320 based on port count. Height starts at 60
/// and grows by 20 per port. These are layout hints; the renderer may
/// override them.
#[must_use]
pub fn compute_node_size(ports: &[FlowPortRecord]) -> [f64; 2] {
    let port_count: u32 = u32::try_from(ports.len()).unwrap_or(u32::MAX);
    // Width: 160 base + 20 per port, capped at 320.
    let width = f64::from(port_count.saturating_mul(20).saturating_add(160).min(320));
    // Height: 60 base + 20 per port.
    let height = f64::from(port_count.saturating_mul(20).saturating_add(60));
    [width, height]
}

// ---------------------------------------------------------------------------
// Port constructor helper
// ---------------------------------------------------------------------------

/// Create a data input port for a slot reference.
fn slot_input_port(id: &str, slot: u16) -> FlowPortRecord {
    FlowPortRecord {
        id: SmolStr::from(id),
        label: SmolStr::from(format!("slot-{slot}")),
        side: PortSide::Input,
        role: PortRole::Data,
        cardinality: Cardinality::One,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::StepIdx;

    fn make_nop_node(id: u16, next: Option<u16>) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: next.map(StepIdx::new),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }
    }

    fn make_finish_node(id: u16, result_slot: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: vb_core::ids::SlotIdx::new(result_slot),
            },
        }
    }

    fn make_jump_node(id: u16, target: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(id),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Jump {
                target: StepIdx::new(target),
            },
        }
    }

    fn make_simple_parts(nodes: Vec<CompiledNode>, entry: u16) -> WorkflowParts {
        let node_count = nodes.len();
        let step_names: Vec<Box<str>> = (0..node_count)
            .map(|i| format!("step-{i}").into_boxed_str())
            .collect();
        WorkflowParts {
            name: String::from("test").into_boxed_str(),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Vec::new().into_boxed_slice(),
            accessors: Vec::new().into_boxed_slice(),
            constants: Vec::new().into_boxed_slice(),
            slot_count: 4,
            symbols_count: 0,
            entry: StepIdx::new(entry),
            resource_contract: vb_core::workflow::ResourceContract::DEFAULT,
            step_names: step_names.into_boxed_slice(),
        }
    }

    #[test]
    fn empty_workflow_produces_empty_document() {
        let node = make_finish_node(0, 0);
        let parts = make_simple_parts(vec![node], 0);
        let doc = build_document(&parts);
        assert_eq!(doc.graph.nodes.len(), 1);
        assert_eq!(doc.graph.edges.len(), 0);
    }

    #[test]
    fn two_node_chain_produces_one_edge() {
        let n0 = make_nop_node(0, Some(1));
        let n1 = make_finish_node(1, 0);
        let parts = make_simple_parts(vec![n0, n1], 0);
        let doc = build_document(&parts);
        assert_eq!(doc.graph.nodes.len(), 2);
        assert_eq!(doc.graph.edges.len(), 1);
        let edge = doc.graph.edges.get_index(0).map(|(_, e)| e.clone());
        assert!(edge.is_some());
        let e = edge.unwrap_or_else(|| panic!("edge missing"));
        assert_eq!(e.source.as_str(), "step-0");
        assert_eq!(e.target.as_str(), "step-1");
        assert_eq!(e.source_port.as_str(), "next");
        assert_eq!(e.target_port.as_str(), "in");
    }

    #[test]
    fn entry_node_flag_is_set() {
        let n0 = make_nop_node(0, Some(1));
        let n1 = make_finish_node(1, 0);
        let parts = make_simple_parts(vec![n0, n1], 0);
        let doc = build_document(&parts);
        let entry = doc.graph.nodes.get("step-0");
        assert!(entry.is_some());
        let n = entry.unwrap_or_else(|| panic!("node missing"));
        assert!(n.flags.entry);
        let non_entry = doc.graph.nodes.get("step-1");
        assert!(non_entry.is_some());
        let n2 = non_entry.unwrap_or_else(|| panic!("node missing"));
        assert!(!n2.flags.entry);
    }

    #[test]
    fn finish_node_is_terminal() {
        let n = make_finish_node(0, 0);
        let parts = make_simple_parts(vec![n], 0);
        let doc = build_document(&parts);
        let node = doc.graph.nodes.get("step-0");
        assert!(node.is_some());
        let record = node.unwrap_or_else(|| panic!("node missing"));
        assert!(record.flags.terminal);
    }

    #[test]
    fn jump_produces_jump_edge() {
        let n0 = make_jump_node(0, 1);
        let n1 = make_finish_node(1, 0);
        let parts = make_simple_parts(vec![n0, n1], 0);
        let doc = build_document(&parts);
        assert_eq!(doc.graph.edges.len(), 1);
        let e = doc
            .graph
            .edges
            .get_index(0)
            .map(|(_, e)| e.clone())
            .unwrap_or_else(|| panic!("edge missing"));
        assert_eq!(e.source_port.as_str(), "jump");
        assert_eq!(e.target.as_str(), "step-1");
    }

    #[test]
    fn classify_all_variants_have_labels() {
        // Spot-check a few categories.
        let (label, cat) = classify_node_kind(&CompiledNodeKind::Nop);
        assert_eq!(label, "Nop");
        assert_eq!(cat, "control");

        let (label, cat) = classify_node_kind(&CompiledNodeKind::Do {
            action: vb_core::ids::ActionId::new(0),
            input: vb_core::ids::SlotIdx::new(0),
        });
        assert_eq!(label, "Do");
        assert_eq!(cat, "external");

        let (label, cat) = classify_node_kind(&CompiledNodeKind::Finish {
            result: vb_core::ids::SlotIdx::new(0),
        });
        assert_eq!(label, "Finish");
        assert_eq!(cat, "terminal");
    }

    #[test]
    fn compute_node_size_scales_with_ports() {
        let small = compute_node_size(&[]);
        assert_eq!(small[0], 160.0);
        assert_eq!(small[1], 60.0);

        let ports = vec![
            FlowPortRecord {
                id: SmolStr::new_static("p0"),
                label: SmolStr::new_static("p0"),
                side: PortSide::Input,
                role: PortRole::Data,
                cardinality: Cardinality::One,
            };
            10
        ];
        let large = compute_node_size(&ports);
        assert!(large[0] > small[0]);
        assert!(large[1] > small[1]);
    }

    #[test]
    fn collect_span_returns_correct_range() {
        let span = collect_span(2, 5, 10);
        assert_eq!(span.len(), 4);
        assert_eq!(span[0].as_str(), "step-2");
        assert_eq!(span[3].as_str(), "step-5");
    }

    #[test]
    fn collect_span_empty_when_end_before_start() {
        let span = collect_span(5, 2, 10);
        assert!(span.is_empty());
    }

    #[test]
    fn collect_span_empty_when_end_out_of_bounds() {
        let span = collect_span(2, 10, 5);
        assert!(span.is_empty());
    }

    #[test]
    fn error_handler_produces_handler_edge() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(2),
                error_slot: None,
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = make_nop_node(2, None);
        let parts = make_simple_parts(vec![n0, n1, n2], 0);
        let doc = build_document(&parts);
        assert_eq!(doc.graph.edges.len(), 1);
        let e = doc
            .graph
            .edges
            .get_index(0)
            .map(|(_, e)| e.clone())
            .unwrap_or_else(|| panic!("edge missing"));
        assert_eq!(e.source_port.as_str(), "handler");
        assert_eq!(e.target.as_str(), "step-2");
        assert!(e.style.dashed);
    }

    #[test]
    fn foreach_start_produces_group_and_edges() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachStart {
                input: vb_core::ids::SlotIdx::new(0),
                item_slot: vb_core::ids::SlotIdx::new(1),
                limit: 10,
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        };
        let n1 = make_nop_node(1, Some(2));
        let n2 = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachNext {
                iterator_slot: vb_core::ids::SlotIdx::new(2),
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        };
        let n3 = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ForEachJoin {
                output: vb_core::ids::SlotIdx::new(3),
            },
        };
        let parts = make_simple_parts(vec![n0, n1, n2, n3], 0);
        let doc = build_document(&parts);

        // ForEachStart produces body + done edges, ForEachNext produces body + done edges,
        // n1.next produces an edge.
        assert!(doc.graph.edges.len() >= 3);

        // Should have a group.
        assert!(!doc.graph.groups.is_empty());
        let group = doc
            .graph
            .groups
            .get("group-foreach-0")
            .cloned()
            .unwrap_or_else(|| panic!("group missing"));
        assert_eq!(group.kind, GroupKind::BranchContainer);
        assert_eq!(group.children.len(), 4);
    }

    #[test]
    fn document_schema_and_semantic_kind() {
        let n = make_finish_node(0, 0);
        let parts = make_simple_parts(vec![n], 0);
        let doc = build_document(&parts);
        assert_eq!(doc.schema.as_str(), "makepad.flow/v2");
        assert_eq!(doc.semantic_kind.as_str(), "velvet-ballistics");
    }

    #[test]
    fn entry_node_matches_parts_entry() {
        let n0 = make_nop_node(0, Some(1));
        let n1 = make_nop_node(1, Some(2));
        let n2 = make_finish_node(2, 0);
        let parts = make_simple_parts(vec![n0, n1, n2], 1);
        let doc = build_document(&parts);
        assert_eq!(
            doc.graph.entry_node.as_ref().map(|s| s.as_str()),
            Some("step-1")
        );
        let entry = doc.graph.nodes.get("step-1");
        assert!(entry.is_some());
        assert!(entry.unwrap_or_else(|| panic!("node missing")).flags.entry);
    }

    // -----------------------------------------------------------------------
    // Additional tests for graph_builder
    // -----------------------------------------------------------------------

    #[test]
    fn choose_with_otherwise_produces_three_edges() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Choose {
                branches: Box::new([
                    vb_core::workflow::ExprBranch {
                        condition: vb_core::ids::ExprIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    vb_core::workflow::ExprBranch {
                        condition: vb_core::ids::ExprIdx::new(1),
                        target: StepIdx::new(2),
                    },
                ]),
                otherwise: Some(StepIdx::new(3)),
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = make_nop_node(2, None);
        let n3 = make_finish_node(3, 0);
        let parts = make_simple_parts(vec![n0, n1, n2, n3], 0);
        let doc = build_document(&parts);

        // 2 branch edges + 1 otherwise edge = 3 total.
        assert_eq!(doc.graph.edges.len(), 3);

        // Find the otherwise edge: it should be dashed.
        let mut found_otherwise = false;
        for (_id, e) in &doc.graph.edges {
            if e.source_port.as_str() == "otherwise" {
                found_otherwise = true;
                assert!(e.style.dashed, "otherwise edge should be dashed");
                assert_eq!(e.target.as_str(), "step-3");
            }
        }
        assert!(found_otherwise, "should find an otherwise edge");

        // Branch edges should be solid.
        let mut solid_branch_count = 0usize;
        for (_id, e) in &doc.graph.edges {
            if e.source_port.as_str().starts_with("branch-") {
                assert!(!e.style.dashed, "branch edge should be solid");
                solid_branch_count = solid_branch_count.saturating_add(1);
            }
        }
        assert_eq!(solid_branch_count, 2);
    }

    #[test]
    fn together_start_creates_swimlane_group() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(1), StepIdx::new(2)]),
                join: StepIdx::new(3),
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = make_nop_node(2, None);
        let n3 = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherJoin {
                branch_count: 2,
                accumulator: vb_core::ids::SlotIdx::new(0),
            },
        };
        let parts = make_simple_parts(vec![n0, n1, n2, n3], 0);
        let doc = build_document(&parts);

        // Should produce a swimlane group.
        let group = match doc.graph.groups.get("group-together-0") {
            Some(g) => g,
            None => return,
        };
        assert_eq!(group.kind, GroupKind::Swimlane);
        // Children should span steps 0 through 3 (inclusive).
        assert_eq!(group.children.len(), 4);
        assert_eq!(group.children[0].as_str(), "step-0");
        assert_eq!(group.children[3].as_str(), "step-3");
    }

    #[test]
    fn collect_start_creates_branch_container_group() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: vb_core::ids::SlotIdx::new(0),
                limit: 100,
                page_size: 10,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectFinish {
                collector_slot: vb_core::ids::SlotIdx::new(1),
            },
        };
        let parts = make_simple_parts(vec![n0, n1, n2], 0);
        let doc = build_document(&parts);

        let group = match doc.graph.groups.get("group-collect-0") {
            Some(g) => g,
            None => return,
        };
        assert_eq!(group.kind, GroupKind::BranchContainer);
        assert_eq!(group.children.len(), 3);
    }

    #[test]
    fn build_ports_for_build_object_with_fields() {
        use vb_core::ids::{SlotIdx, SymbolId};

        let kind = CompiledNodeKind::BuildObject {
            fields: Box::new([
                (SymbolId::new(0), SlotIdx::new(1)),
                (SymbolId::new(1), SlotIdx::new(2)),
                (SymbolId::new(2), SlotIdx::new(3)),
            ]),
        };
        let (inputs, outputs) = build_ports(&kind, Some(SlotIdx::new(0)));

        // 3 field input ports.
        assert_eq!(inputs.len(), 3);
        assert_eq!(inputs[0].id.as_str(), "field-0");
        assert_eq!(inputs[1].id.as_str(), "field-1");
        assert_eq!(inputs[2].id.as_str(), "field-2");
        // All input ports should be on the Input side with Data role.
        for port in &inputs {
            assert_eq!(port.side, PortSide::Input);
            assert_eq!(port.role, PortRole::Data);
        }
        // One output port for the output slot.
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].id.as_str(), "out");
        assert_eq!(outputs[0].side, PortSide::Output);
    }

    #[test]
    fn build_ports_for_build_list_with_items() {
        use vb_core::ids::SlotIdx;

        let kind = CompiledNodeKind::BuildList {
            items: Box::new([
                SlotIdx::new(10),
                SlotIdx::new(20),
                SlotIdx::new(30),
                SlotIdx::new(40),
            ]),
        };
        let (inputs, outputs) = build_ports(&kind, Some(SlotIdx::new(5)));

        // 4 item input ports.
        assert_eq!(inputs.len(), 4);
        assert_eq!(inputs[0].id.as_str(), "item-0");
        assert_eq!(inputs[3].id.as_str(), "item-3");
        // All input ports should be on the Input side with Data role.
        for port in &inputs {
            assert_eq!(port.side, PortSide::Input);
            assert_eq!(port.role, PortRole::Data);
        }
        // One output port for the output slot.
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].id.as_str(), "out");
    }

    #[test]
    fn edge_style_defaults_solid_vs_dashed() {
        // Verify the EdgeStyle constructors produce the expected values.
        let solid = EdgeStyle::default_solid();
        assert!(!solid.dashed);
        assert!(!solid.highlighted);

        let dashed = EdgeStyle::dashed();
        assert!(dashed.dashed);
        assert!(!dashed.highlighted);

        // Error handler edges should use dashed style.
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: Some(StepIdx::new(2)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ErrorHandler {
                body: StepIdx::new(1),
                handler: StepIdx::new(3),
                error_slot: None,
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = make_nop_node(2, None);
        let n3 = make_nop_node(3, None);
        let parts = make_simple_parts(vec![n0, n1, n2, n3], 0);
        let doc = build_document(&parts);

        // Should have a `next` edge (solid) and a `handler` edge (dashed).
        let mut found_solid_next = false;
        let mut found_dashed_handler = false;
        for (_id, e) in &doc.graph.edges {
            if e.source_port.as_str() == "next" {
                found_solid_next = true;
                assert!(!e.style.dashed, "next edge should be solid");
            }
            if e.source_port.as_str() == "handler" {
                found_dashed_handler = true;
                assert!(e.style.dashed, "handler edge should be dashed");
            }
        }
        assert!(found_solid_next, "should find a solid next edge");
        assert!(found_dashed_handler, "should find a dashed handler edge");
    }

    #[test]
    fn wait_event_with_timeout_produces_two_input_ports() {
        let kind = CompiledNodeKind::WaitEvent {
            event: vb_core::ids::SlotIdx::new(5),
            timeout_slot: Some(vb_core::ids::SlotIdx::new(8)),
        };
        let (inputs, outputs) = build_ports(&kind, None);

        // Event port + timeout port = 2 input ports.
        assert_eq!(inputs.len(), 2, "WaitEvent with timeout should have 2 inputs");
        assert_eq!(inputs[0].id.as_str(), "event");
        assert_eq!(inputs[1].id.as_str(), "timeout");

        for port in &inputs {
            assert_eq!(port.side, PortSide::Input);
            assert_eq!(port.role, PortRole::Data);
        }

        // No output slot provided, so no output ports.
        assert!(outputs.is_empty(), "WaitEvent has no output ports when output is None");
    }

    #[test]
    fn wait_event_without_timeout_produces_one_input_port() {
        let kind = CompiledNodeKind::WaitEvent {
            event: vb_core::ids::SlotIdx::new(3),
            timeout_slot: None,
        };
        let (inputs, outputs) = build_ports(&kind, None);

        // Only event port; no timeout port.
        assert_eq!(inputs.len(), 1, "WaitEvent without timeout should have 1 input");
        assert_eq!(inputs[0].id.as_str(), "event");
        assert_eq!(inputs[0].side, PortSide::Input);
        assert_eq!(inputs[0].role, PortRole::Data);

        assert!(outputs.is_empty());
    }

    #[test]
    fn wait_event_with_timeout_and_output_slot() {
        let kind = CompiledNodeKind::WaitEvent {
            event: vb_core::ids::SlotIdx::new(1),
            timeout_slot: Some(vb_core::ids::SlotIdx::new(2)),
        };
        let (inputs, outputs) = build_ports(&kind, Some(vb_core::ids::SlotIdx::new(10)));

        // Still 2 input ports.
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].id.as_str(), "event");
        assert_eq!(inputs[1].id.as_str(), "timeout");

        // Now has an output port for the output slot.
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].id.as_str(), "out");
        assert_eq!(outputs[0].side, PortSide::Output);
    }

    #[test]
    fn choose_ports_include_branch_triggers_and_otherwise() {
        use vb_core::ids::ExprIdx;

        let kind = CompiledNodeKind::Choose {
            branches: Box::new([
                vb_core::workflow::ExprBranch {
                    condition: ExprIdx::new(0),
                    target: StepIdx::new(1),
                },
                vb_core::workflow::ExprBranch {
                    condition: ExprIdx::new(1),
                    target: StepIdx::new(2),
                },
            ]),
            otherwise: Some(StepIdx::new(3)),
        };
        let (inputs, outputs) = build_ports(&kind, None);

        // No input ports (Choose branches from expressions, not slots).
        assert!(inputs.is_empty());

        // 2 branch trigger ports + 1 otherwise port = 3 output ports.
        assert_eq!(outputs.len(), 3);
        assert_eq!(outputs[0].id.as_str(), "branch-0");
        assert_eq!(outputs[0].role, PortRole::Trigger);
        assert_eq!(outputs[1].id.as_str(), "branch-1");
        assert_eq!(outputs[1].role, PortRole::Trigger);
        assert_eq!(outputs[2].id.as_str(), "otherwise");
        assert_eq!(outputs[2].role, PortRole::Otherwise);
    }

    #[test]
    fn together_start_ports_match_branch_count_plus_join() {
        let kind = CompiledNodeKind::TogetherStart {
            branches: Box::new([StepIdx::new(1), StepIdx::new(2), StepIdx::new(3)]),
            join: StepIdx::new(4),
        };
        let (inputs, outputs) = build_ports(&kind, None);

        assert!(inputs.is_empty());

        // 3 branch trigger ports + 1 join done port = 4.
        assert_eq!(outputs.len(), 4);
        assert_eq!(outputs[0].id.as_str(), "branch-0");
        assert_eq!(outputs[0].role, PortRole::Trigger);
        assert_eq!(outputs[1].id.as_str(), "branch-1");
        assert_eq!(outputs[2].id.as_str(), "branch-2");
        assert_eq!(outputs[3].id.as_str(), "join");
        assert_eq!(outputs[3].role, PortRole::Done);
    }

    #[test]
    fn collect_start_ports_have_input_body_and_done() {
        let kind = CompiledNodeKind::CollectStart {
            source: vb_core::ids::SlotIdx::new(0),
            limit: 50,
            page_size: 10,
            body: StepIdx::new(1),
            done: StepIdx::new(2),
        };
        let (inputs, outputs) = build_ports(&kind, None);

        // One input port for the source slot.
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].id.as_str(), "source");
        assert_eq!(inputs[0].side, PortSide::Input);
        assert_eq!(inputs[0].role, PortRole::Data);

        // Two output ports: body and done.
        assert_eq!(outputs.len(), 2);
        assert_eq!(outputs[0].id.as_str(), "body");
        assert_eq!(outputs[0].role, PortRole::Body);
        assert_eq!(outputs[1].id.as_str(), "done");
        assert_eq!(outputs[1].role, PortRole::Done);
    }

    // -----------------------------------------------------------------------
    // Additional tests: edge generation for CollectEnd/ReduceEnd,
    // multi-branch Choose, TogetherEnd merge, RepeatAttempt loop-back,
    // nested loop-inside-parallel, empty workflow single Finish node.
    // -----------------------------------------------------------------------

    /// CollectFinish (the "CollectEnd" node) produces no kind-specific edges
    /// beyond `next`. This test verifies that a CollectStart -> body ->
    /// CollectFinish chain produces exactly the expected edges.
    #[test]
    fn collect_finish_produces_no_extra_kind_edges() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectStart {
                source: vb_core::ids::SlotIdx::new(0),
                limit: 50,
                page_size: 10,
                body: StepIdx::new(1),
                done: StepIdx::new(2),
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::CollectFinish {
                collector_slot: vb_core::ids::SlotIdx::new(1),
            },
        };
        let parts = make_simple_parts(vec![n0, n1, n2], 0);
        let doc = build_document(&parts);

        // CollectStart emits body + done edges (2). CollectFinish emits no
        // kind-specific edges. Nop (n1) has no next so no next edge.
        // Total = 2.
        assert_eq!(doc.graph.edges.len(), 2, "expected 2 edges from CollectStart only");

        // Verify the body edge targets step-1 and done edge targets step-2.
        let mut found_body = false;
        let mut found_done = false;
        for (_id, e) in &doc.graph.edges {
            if e.source_port.as_str() == "body" {
                found_body = true;
                assert_eq!(e.target.as_str(), "step-1");
                assert!(!e.style.dashed, "body edge should be solid");
            }
            if e.source_port.as_str() == "done" {
                found_done = true;
                assert_eq!(e.target.as_str(), "step-2");
                assert!(e.style.dashed, "done edge should be dashed");
            }
        }
        assert!(found_body, "should find body edge");
        assert!(found_done, "should find done edge");
    }

    /// ReduceStart -> body -> ReduceNext -> body -> ReduceFinish chain.
    /// ReduceFinish produces no kind-specific edges. Verify edge count and
    /// that ReduceStart and ReduceNext each produce body + done edges.
    #[test]
    fn reduce_start_and_next_produce_body_done_edges() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceStart {
                input: vb_core::ids::SlotIdx::new(0),
                accumulator: vb_core::ids::SlotIdx::new(1),
                initial: vb_core::ids::ConstIdx::new(0),
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceNext {
                iterator_slot: vb_core::ids::SlotIdx::new(2),
                accumulator: vb_core::ids::SlotIdx::new(1),
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        };
        let n3 = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ReduceFinish {
                accumulator: vb_core::ids::SlotIdx::new(1),
            },
        };
        let parts = make_simple_parts(vec![n0, n1, n2, n3], 0);
        let doc = build_document(&parts);

        // ReduceStart: body + done (2 edges)
        // ReduceNext: body + done (2 edges)
        // Total kind-specific edges = 4.
        assert_eq!(doc.graph.edges.len(), 4, "expected 4 edges from ReduceStart + ReduceNext");

        // Verify the done edges both target step-3.
        let mut done_count = 0usize;
        for (_id, e) in &doc.graph.edges {
            if e.source_port.as_str() == "done" {
                done_count = done_count.saturating_add(1);
                assert_eq!(e.target.as_str(), "step-3");
                assert!(e.style.dashed, "done edges should be dashed");
            }
        }
        assert_eq!(done_count, 2, "should find 2 done edges");
    }

    /// Choose with three branches and an otherwise target produces 4 edges total.
    #[test]
    fn choose_with_three_branches_produces_four_edges() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Choose {
                branches: Box::new([
                    vb_core::workflow::ExprBranch {
                        condition: vb_core::ids::ExprIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    vb_core::workflow::ExprBranch {
                        condition: vb_core::ids::ExprIdx::new(1),
                        target: StepIdx::new(2),
                    },
                    vb_core::workflow::ExprBranch {
                        condition: vb_core::ids::ExprIdx::new(2),
                        target: StepIdx::new(3),
                    },
                ]),
                otherwise: Some(StepIdx::new(4)),
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = make_nop_node(2, None);
        let n3 = make_nop_node(3, None);
        let n4 = make_finish_node(4, 0);
        let parts = make_simple_parts(vec![n0, n1, n2, n3, n4], 0);
        let doc = build_document(&parts);

        // 3 branch edges + 1 otherwise edge = 4 total.
        assert_eq!(doc.graph.edges.len(), 4, "expected 3 branch + 1 otherwise edges");

        let mut branch_count = 0usize;
        let mut otherwise_count = 0usize;
        for (_id, e) in &doc.graph.edges {
            if e.source_port.as_str().starts_with("branch-") {
                branch_count = branch_count.saturating_add(1);
                assert!(!e.style.dashed, "branch edges should be solid");
            }
            if e.source_port.as_str() == "otherwise" {
                otherwise_count = otherwise_count.saturating_add(1);
                assert!(e.style.dashed, "otherwise edge should be dashed");
                assert_eq!(e.target.as_str(), "step-4");
            }
        }
        assert_eq!(branch_count, 3, "expected 3 branch edges");
        assert_eq!(otherwise_count, 1, "expected 1 otherwise edge");
    }

    /// TogetherStart -> TogetherBranch -> TogetherJoin produces branch edges from
    /// TogetherStart and entry/join edges from TogetherBranch, and the
    /// TogetherJoin node has no kind-specific edges, acting as the single merge
    /// output.
    #[test]
    fn together_end_merges_back_to_single_output() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(1), StepIdx::new(2)]),
                join: StepIdx::new(3),
            },
        };
        let n1 = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherBranch {
                branch: 0,
                entry: StepIdx::new(4),
                join: StepIdx::new(3),
                accumulator: vb_core::ids::SlotIdx::new(0),
            },
        };
        let n2 = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherBranch {
                branch: 1,
                entry: StepIdx::new(5),
                join: StepIdx::new(3),
                accumulator: vb_core::ids::SlotIdx::new(0),
            },
        };
        let n3 = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherJoin {
                branch_count: 2,
                accumulator: vb_core::ids::SlotIdx::new(0),
            },
        };
        let n4 = make_nop_node(4, None);
        let n5 = make_nop_node(5, None);
        let parts = make_simple_parts(vec![n0, n1, n2, n3, n4, n5], 0);
        let doc = build_document(&parts);

        // TogetherStart: 2 branch edges + 1 join edge = 3
        // TogetherBranch (n1): entry + join = 2
        // TogetherBranch (n2): entry + join = 2
        // TogetherJoin (n3): 0 kind-specific edges
        // Total = 7.
        assert_eq!(doc.graph.edges.len(), 7, "expected 7 edges total");

        // All join edges should target step-3 (TogetherJoin).
        let mut join_edge_count = 0usize;
        for (_id, e) in &doc.graph.edges {
            if e.source_port.as_str() == "join" {
                join_edge_count = join_edge_count.saturating_add(1);
                assert_eq!(
                    e.target.as_str(), "step-3",
                    "all join edges should target TogetherJoin at step-3"
                );
                assert!(e.style.dashed, "join edges should be dashed");
            }
        }
        assert_eq!(join_edge_count, 3, "expected 3 join edges (1 from start, 2 from branches)");
    }

    /// RepeatAttempt creates a body edge that loops back to an earlier step,
    /// plus a done edge that exits the loop. This test verifies the loop-back
    /// edge targets an earlier step index.
    #[test]
    fn repeat_attempt_creates_loop_back_edge() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 3,
                body: StepIdx::new(1),
                done: StepIdx::new(3),
            },
        };
        // RepeatAttempt loops body back to itself (step 1) for retry.
        let n1 = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatAttempt {
                attempt_slot: vb_core::ids::SlotIdx::new(0),
                body: StepIdx::new(1), // loop back to self
                done: StepIdx::new(2),
            },
        };
        let n2 = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatCheck {
                attempt_slot: vb_core::ids::SlotIdx::new(0),
                done: StepIdx::new(3),
            },
        };
        let n3 = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatFinish {
                result: vb_core::ids::SlotIdx::new(1),
            },
        };
        let parts = make_simple_parts(vec![n0, n1, n2, n3], 0);
        let doc = build_document(&parts);

        // RepeatStart: body + done = 2
        // RepeatAttempt: body + done = 2
        // RepeatCheck: done = 1
        // RepeatFinish: 0
        // Total = 5 edges.
        assert_eq!(doc.graph.edges.len(), 5, "expected 5 edges total");

        // Find the loop-back body edge from step-1 targeting step-1.
        let mut found_loop_back = false;
        for (_id, e) in &doc.graph.edges {
            if e.source.as_str() == "step-1" && e.source_port.as_str() == "body" {
                found_loop_back = true;
                assert_eq!(
                    e.target.as_str(), "step-1",
                    "RepeatAttempt body should loop back to itself"
                );
                assert!(!e.style.dashed, "body loop-back edge should be solid");
            }
        }
        assert!(found_loop_back, "should find a loop-back body edge from RepeatAttempt");

        // Verify RepeatAttempt's done edge exits to step-2.
        let mut found_done_exit = false;
        for (_id, e) in &doc.graph.edges {
            if e.source.as_str() == "step-1" && e.source_port.as_str() == "done" {
                found_done_exit = true;
                assert_eq!(e.target.as_str(), "step-2");
                assert!(e.style.dashed, "done edge should be dashed");
            }
        }
        assert!(found_done_exit, "should find a done exit edge from RepeatAttempt");

        // Verify group was created for the repeat loop.
        let group = match doc.graph.groups.get("group-repeat-0") {
            Some(g) => g,
            None => return,
        };
        assert_eq!(group.kind, GroupKind::BranchContainer);
        assert_eq!(group.children.len(), 4, "repeat group should span steps 0-3");
    }

    /// Nested structure: a RepeatStart loop containing a TogetherStart/TogetherJoin
    /// parallel block inside it. Verifies that both groups are created and that
    /// edges from inner parallel construct are present alongside loop edges.
    #[test]
    fn nested_repeat_containing_together_produces_both_groups() {
        // Layout:
        // 0: RepeatStart(body=1, done=5)
        // 1: TogetherStart(branches=[2,3], join=4)
        // 2: TogetherBranch(entry=..., join=4)
        // 3: TogetherBranch(entry=..., join=4)
        // 4: TogetherJoin
        // 5: RepeatFinish
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatStart {
                max_attempts: 2,
                body: StepIdx::new(1),
                done: StepIdx::new(5),
            },
        };
        let n1 = CompiledNode {
            id: StepIdx::new(1),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherStart {
                branches: Box::new([StepIdx::new(2), StepIdx::new(3)]),
                join: StepIdx::new(4),
            },
        };
        let n2 = CompiledNode {
            id: StepIdx::new(2),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherBranch {
                branch: 0,
                entry: StepIdx::new(4),
                join: StepIdx::new(4),
                accumulator: vb_core::ids::SlotIdx::new(0),
            },
        };
        let n3 = CompiledNode {
            id: StepIdx::new(3),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherBranch {
                branch: 1,
                entry: StepIdx::new(4),
                join: StepIdx::new(4),
                accumulator: vb_core::ids::SlotIdx::new(0),
            },
        };
        let n4 = CompiledNode {
            id: StepIdx::new(4),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::TogetherJoin {
                branch_count: 2,
                accumulator: vb_core::ids::SlotIdx::new(0),
            },
        };
        let n5 = CompiledNode {
            id: StepIdx::new(5),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::RepeatFinish {
                result: vb_core::ids::SlotIdx::new(1),
            },
        };
        let parts = make_simple_parts(vec![n0, n1, n2, n3, n4, n5], 0);
        let doc = build_document(&parts);

        // Verify both groups are present.
        let repeat_group = match doc.graph.groups.get("group-repeat-0") {
            Some(g) => g,
            None => return,
        };
        assert_eq!(repeat_group.kind, GroupKind::BranchContainer);
        // Repeat spans steps 0-5 inclusive.
        assert_eq!(repeat_group.children.len(), 6);

        let together_group = match doc.graph.groups.get("group-together-1") {
            Some(g) => g,
            None => return,
        };
        assert_eq!(together_group.kind, GroupKind::Swimlane);
        // Together spans steps 1-4 inclusive.
        assert_eq!(together_group.children.len(), 4);

        // RepeatStart: body + done = 2
        // TogetherStart: 2 branch + 1 join = 3
        // TogetherBranch (n2): entry + join = 2
        // TogetherBranch (n3): entry + join = 2
        // TogetherJoin (n4): 0
        // RepeatFinish (n5): 0
        // Total = 9
        assert_eq!(doc.graph.edges.len(), 9, "expected 9 edges from nested structure");

        // Verify RepeatStart body edge targets step-1 (TogetherStart).
        let mut found_repeat_body = false;
        for (_id, e) in &doc.graph.edges {
            if e.source.as_str() == "step-0" && e.source_port.as_str() == "body" {
                found_repeat_body = true;
                assert_eq!(e.target.as_str(), "step-1");
            }
        }
        assert!(found_repeat_body, "should find RepeatStart body edge targeting TogetherStart");
    }

    /// A workflow consisting only of a single Finish node produces exactly one
    /// node, zero edges, and zero groups. The node must have the terminal flag
    /// set and be the entry node.
    #[test]
    fn single_finish_node_only_workflow() {
        let n = make_finish_node(0, 0);
        let parts = make_simple_parts(vec![n], 0);
        let doc = build_document(&parts);

        // Exactly one node.
        assert_eq!(doc.graph.nodes.len(), 1, "single Finish should produce exactly 1 node");

        // Zero edges (Finish has no next or kind-specific edges).
        assert_eq!(doc.graph.edges.len(), 0, "single Finish should produce 0 edges");

        // Zero groups (no loops or parallel constructs).
        assert!(doc.graph.groups.is_empty(), "single Finish should produce 0 groups");

        // The single node should be both terminal and entry.
        let node_rec = match doc.graph.nodes.get("step-0") {
            Some(n) => n,
            None => return,
        };
        assert!(node_rec.flags.terminal, "Finish node should be terminal");
        assert!(node_rec.flags.entry, "Finish node should be the entry node");

        // Entry node in the graph metadata should be step-0.
        let entry = match &doc.graph.entry_node {
            Some(e) => e,
            None => return,
        };
        assert_eq!(entry.as_str(), "step-0");
    }

    /// ChooseSlot with two branches and an otherwise target produces 3 edges
    /// total, using SlotBranch instead of ExprBranch.
    #[test]
    fn choose_slot_with_branches_produces_correct_edges() {
        let n0 = CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::ChooseSlot {
                branches: Box::new([
                    vb_core::workflow::SlotBranch {
                        condition: vb_core::ids::SlotIdx::new(0),
                        target: StepIdx::new(1),
                    },
                    vb_core::workflow::SlotBranch {
                        condition: vb_core::ids::SlotIdx::new(1),
                        target: StepIdx::new(2),
                    },
                ]),
                otherwise: Some(StepIdx::new(3)),
            },
        };
        let n1 = make_nop_node(1, None);
        let n2 = make_nop_node(2, None);
        let n3 = make_finish_node(3, 0);
        let parts = make_simple_parts(vec![n0, n1, n2, n3], 0);
        let doc = build_document(&parts);

        // 2 branch edges + 1 otherwise edge = 3 total.
        assert_eq!(doc.graph.edges.len(), 3, "expected 3 edges from ChooseSlot");

        // Verify branch edges are solid and otherwise is dashed.
        let mut solid_branches = 0usize;
        let mut dashed_otherwise = 0usize;
        for (_id, e) in &doc.graph.edges {
            if e.source_port.as_str().starts_with("branch-") {
                solid_branches = solid_branches.saturating_add(1);
                assert!(!e.style.dashed, "branch edges should be solid");
            }
            if e.source_port.as_str() == "otherwise" {
                dashed_otherwise = dashed_otherwise.saturating_add(1);
                assert!(e.style.dashed, "otherwise should be dashed");
                assert_eq!(e.target.as_str(), "step-3");
            }
        }
        assert_eq!(solid_branches, 2, "expected 2 solid branch edges");
        assert_eq!(dashed_otherwise, 1, "expected 1 dashed otherwise edge");
    }
}
