#![forbid(unsafe_code)]
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
#[non_exhaustive]
pub enum PortSide {
    /// Port receives data into the node.
    Input,
    /// Port emits data from the node.
    Output,
}

/// Role a port plays in the node's contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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
#[non_exhaustive]
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
#[non_exhaustive]
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
        semantic_kind: SmolStr::new_static("velvet-ballastics"),
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
        CompiledNodeKind::ReduceStart { .. } => ("ReduceStart", "aggregate"),
        CompiledNodeKind::ReduceNext { .. } => ("ReduceNext", "aggregate"),
        CompiledNodeKind::ReduceFinish { .. } => ("ReduceFinish", "aggregate"),
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
pub(crate) fn collect_span(start: usize, end: usize, total: usize) -> Vec<SmolStr> {
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

#[path = "graph_builder_tests.rs"]
mod tests;
