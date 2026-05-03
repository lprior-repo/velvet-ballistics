//! FlowEditor widget -- the main canvas for graph visualization and editing.

use flow_core::doc::FlowDocument;
use makepad_widgets::*;

/// Action types emitted by the flow editor.
#[derive(Clone, Debug)]
pub enum FlowEditorAction {
    DocumentChanged,
    SelectionChanged,
    ViewportChanged { pan_x: f64, pan_y: f64, zoom: f64 },
    NodeClicked { node_id: flow_core::ids::NodeId },
    EdgeClicked { edge_id: flow_core::ids::EdgeId },
    CanvasClicked { world_x: f64, world_y: f64 },
}

script_mod! {
    use mod.prelude.widgets_internal.*

    mod.widgets.FlowEditorBase = #(FlowEditor::register_widget(vm))
    mod.widgets.FlowEditor = set_type_default() do mod.widgets.FlowEditorBase{
        width: Fill
        height: Fill
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct FlowEditor {
    #[uid]
    uid: WidgetUid,
    #[source]
    source: ScriptObjectRef,
    #[walk]
    walk: Walk,
    #[layout]
    layout: Layout,
    #[redraw]
    #[rust]
    area: Area,
    #[rust]
    document: Option<FlowDocument>,
    #[rust]
    pan_x: f64,
    #[rust]
    pan_y: f64,
    #[rust]
    zoom: f64,
}

impl Widget for FlowEditor {
    #[allow(elided_lifetimes_in_paths)]
    fn handle_event(&mut self, _cx: &mut Cx, _event: &Event, _scope: &mut Scope) {
        // Interaction handling (pan, zoom, click, drag) to be implemented.
    }

    #[allow(elided_lifetimes_in_paths)]
    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        let _rect = cx.walk_turtle(walk);
        // Node, edge, and grid drawing to be implemented.
        DrawStep::done()
    }
}

impl FlowEditor {
    pub fn set_document(&mut self, _cx: &mut Cx, doc: FlowDocument) {
        self.document = Some(doc);
    }

    pub fn document(&self) -> Option<&FlowDocument> {
        self.document.as_ref()
    }

    pub fn set_viewport(&mut self, _cx: &mut Cx, pan_x: f64, pan_y: f64, zoom: f64) {
        self.pan_x = pan_x;
        self.pan_y = pan_y;
        self.zoom = zoom.clamp(0.1, 8.0);
    }
}
