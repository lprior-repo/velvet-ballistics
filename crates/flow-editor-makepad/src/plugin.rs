#![forbid(unsafe_code)]
//! FlowPlugin trait system for the flow editor.
//!
//! Defines the plugin interface, registry, function-pointer types, and data
//! types that allow third-party code to extend the flow editor with custom
//! node renderers, edge renderers, inspectors, and validators.

use flow_core::doc::{FlowDocument, FlowEdgeRecord, FlowNodeRecord};
use flow_core::ids::NodeId;

// ---------------------------------------------------------------------------
// Shape enum
// ---------------------------------------------------------------------------

/// Shape variants a node renderer can request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeShape {
    RoundedRect,
    Diamond,
    Round,
    Pill,
}

// ---------------------------------------------------------------------------
// Validation level
// ---------------------------------------------------------------------------

/// Severity level for a validation finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationLevel {
    Error,
    Warning,
}

// ---------------------------------------------------------------------------
// Data types produced by function-pointer callbacks
// ---------------------------------------------------------------------------

/// Render data produced by a [`NodeRendererFn`].
#[derive(Clone, Debug)]
pub struct NodeRenderData {
    pub label: String,
    pub shape: NodeShape,
    pub color: [f32; 4],
    pub badges: Vec<String>,
}

/// Render data produced by an [`EdgeRendererFn`].
#[derive(Clone, Debug)]
pub struct EdgeRenderData {
    pub color: [f32; 4],
    pub width: f32,
    pub dash: bool,
}

/// Inspection data produced by an [`InspectorFn`].
#[derive(Clone, Debug)]
pub struct InspectorData {
    pub title: String,
    pub fields: Vec<(String, String)>,
}

/// A single validation finding from a [`ValidatorFn`].
#[derive(Clone, Debug)]
pub struct ValidationFinding {
    pub level: ValidationLevel,
    pub message: String,
    pub node_id: Option<NodeId>,
}

// ---------------------------------------------------------------------------
// Function pointer types
// ---------------------------------------------------------------------------

/// Function pointer that renders a flow node into visual data.
pub type NodeRendererFn = fn(&FlowNodeRecord) -> NodeRenderData;

/// Function pointer that renders a flow edge into visual data.
pub type EdgeRendererFn = fn(&FlowEdgeRecord) -> EdgeRenderData;

/// Function pointer that inspects a flow node and returns property-panel data.
pub type InspectorFn = fn(&FlowNodeRecord) -> InspectorData;

/// Function pointer that validates a flow document and returns findings.
pub type ValidatorFn = fn(&FlowDocument) -> Vec<ValidationFinding>;

// ---------------------------------------------------------------------------
// FlowPlugin trait (object-safe)
// ---------------------------------------------------------------------------

/// The main plugin interface.
///
/// A plugin provides collections of renderers, inspectors, and validators
/// that extend the flow editor's capabilities. Each method returns a vec of
/// `(key, function_pointer)` pairs, keyed by the node kind / edge type they
/// handle.
pub trait FlowPlugin {
    /// Human-readable name of this plugin.
    fn name(&self) -> &str;

    /// Node renderers, keyed by node kind string.
    fn node_renderers(&self) -> Vec<(&str, NodeRendererFn)> {
        Vec::new()
    }

    /// Edge renderers, keyed by edge type string.
    fn edge_renderers(&self) -> Vec<(&str, EdgeRendererFn)> {
        Vec::new()
    }

    /// Inspectors, keyed by node kind string.
    fn inspectors(&self) -> Vec<(&str, InspectorFn)> {
        Vec::new()
    }

    /// Validators, keyed by name string.
    fn validators(&self) -> Vec<(&str, ValidatorFn)> {
        Vec::new()
    }
}

// ---------------------------------------------------------------------------
// PluginRegistry
// ---------------------------------------------------------------------------

/// Holds all registered plugins and provides lookup by key.
pub struct PluginRegistry {
    plugins: Vec<Box<dyn FlowPlugin>>,
    node_renderers: Vec<(String, NodeRendererFn)>,
    edge_renderers: Vec<(String, EdgeRendererFn)>,
    inspectors: Vec<(String, InspectorFn)>,
    validators: Vec<(String, ValidatorFn)>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            node_renderers: Vec::new(),
            edge_renderers: Vec::new(),
            inspectors: Vec::new(),
            validators: Vec::new(),
        }
    }

    /// Register a plugin, indexing all its entries for lookup.
    ///
    /// If a later plugin registers the same key as an earlier one, the later
    /// entry overwrites it.
    pub fn register(&mut self, plugin: Box<dyn FlowPlugin>) {
        for (kind, func) in plugin.node_renderers() {
            let idx = self.node_renderers.iter().position(|(k, _)| k == kind);
            match idx.and_then(|i| self.node_renderers.get_mut(i)) {
                Some(entry) => *entry = (String::from(kind), func),
                None => self.node_renderers.push((String::from(kind), func)),
            }
        }
        for (edge_type, func) in plugin.edge_renderers() {
            let idx = self.edge_renderers.iter().position(|(k, _)| k == edge_type);
            match idx.and_then(|i| self.edge_renderers.get_mut(i)) {
                Some(entry) => *entry = (String::from(edge_type), func),
                None => self.edge_renderers.push((String::from(edge_type), func)),
            }
        }
        for (node_kind, func) in plugin.inspectors() {
            let idx = self.inspectors.iter().position(|(k, _)| k == node_kind);
            match idx.and_then(|i| self.inspectors.get_mut(i)) {
                Some(entry) => *entry = (String::from(node_kind), func),
                None => self.inspectors.push((String::from(node_kind), func)),
            }
        }
        for (name, func) in plugin.validators() {
            let idx = self.validators.iter().position(|(k, _)| k == name);
            match idx.and_then(|i| self.validators.get_mut(i)) {
                Some(entry) => *entry = (String::from(name), func),
                None => self.validators.push((String::from(name), func)),
            }
        }
        self.plugins.push(plugin);
    }

    /// Look up a node renderer by node kind.
    pub fn node_renderer_for(&self, kind: &str) -> Option<NodeRendererFn> {
        self.node_renderers
            .iter()
            .find(|(k, _)| k == kind)
            .map(|(_, func)| *func)
    }

    /// Look up an edge renderer by edge type.
    pub fn edge_renderer_for(&self, edge_type: &str) -> Option<EdgeRendererFn> {
        self.edge_renderers
            .iter()
            .find(|(k, _)| k == edge_type)
            .map(|(_, func)| *func)
    }

    /// Look up an inspector by node kind.
    pub fn inspector_for(&self, node_kind: &str) -> Option<InspectorFn> {
        self.inspectors
            .iter()
            .find(|(k, _)| k == node_kind)
            .map(|(_, func)| *func)
    }

    /// Run all registered validators and collect their findings.
    pub fn run_all_validators(&self, doc: &FlowDocument) -> Vec<ValidationFinding> {
        let mut findings = Vec::new();
        for (_, func) in &self.validators {
            findings.extend(func(doc));
        }
        findings
    }

    /// Number of registered plugins.
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Number of registered node renderers.
    pub fn node_renderer_count(&self) -> usize {
        self.node_renderers.len()
    }

    /// Number of registered edge renderers.
    pub fn edge_renderer_count(&self) -> usize {
        self.edge_renderers.len()
    }

    /// Number of registered inspectors.
    pub fn inspector_count(&self) -> usize {
        self.inspectors.len()
    }

    /// Number of registered validators.
    pub fn validator_count(&self) -> usize {
        self.validators.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use flow_core::doc::{EdgeStyle, EdgeUiState, FlowEdgeRecord, NodeFlags, NodeUiState};
    use smol_str::SmolStr;

    // ---- Helpers ----

    fn make_node(id: &str, kind: &str) -> FlowNodeRecord {
        FlowNodeRecord {
            id: SmolStr::from(id),
            kind: SmolStr::from(kind),
            title: SmolStr::from(id),
            position: [0.0, 0.0],
            size: [100.0, 50.0],
            z_index: 0,
            parent: None,
            ports: Vec::new(),
            flags: NodeFlags::default(),
            data: serde_json::Value::Null,
            ui: NodeUiState::default(),
        }
    }

    fn make_edge(id: &str) -> FlowEdgeRecord {
        FlowEdgeRecord {
            id: SmolStr::from(id),
            source_node: SmolStr::from("n1"),
            source_port: SmolStr::from("out"),
            target_node: SmolStr::from("n2"),
            target_port: SmolStr::from("in"),
            label: None,
            style: EdgeStyle::default(),
            data: serde_json::Value::Null,
            ui: EdgeUiState::default(),
        }
    }

    // ---- Stub renderers / validators as free functions ----

    fn stub_node_renderer(node: &FlowNodeRecord) -> NodeRenderData {
        NodeRenderData {
            label: node.title.to_string(),
            shape: NodeShape::RoundedRect,
            color: [1.0, 0.0, 0.0, 1.0],
            badges: vec![String::from("stub")],
        }
    }

    fn stub_edge_renderer(_edge: &FlowEdgeRecord) -> EdgeRenderData {
        EdgeRenderData {
            color: [0.0, 1.0, 0.0, 1.0],
            width: 2.0,
            dash: false,
        }
    }

    fn stub_inspector(node: &FlowNodeRecord) -> InspectorData {
        InspectorData {
            title: node.title.to_string(),
            fields: vec![(String::from("kind"), node.kind.to_string())],
        }
    }

    fn stub_validator(_doc: &FlowDocument) -> Vec<ValidationFinding> {
        vec![ValidationFinding {
            level: ValidationLevel::Warning,
            message: String::from("stub warning"),
            node_id: None,
        }]
    }

    fn empty_validator(_doc: &FlowDocument) -> Vec<ValidationFinding> {
        Vec::new()
    }

    fn error_validator(_doc: &FlowDocument) -> Vec<ValidationFinding> {
        vec![ValidationFinding {
            level: ValidationLevel::Error,
            message: String::from("stub error"),
            node_id: Some(SmolStr::from("n1")),
        }]
    }

    // ---- Stub plugin ----

    struct StubPlugin {
        plugin_name: String,
    }

    impl StubPlugin {
        fn new(name: &str) -> Self {
            Self {
                plugin_name: String::from(name),
            }
        }
    }

    impl FlowPlugin for StubPlugin {
        fn name(&self) -> &str {
            &self.plugin_name
        }

        fn node_renderers(&self) -> Vec<(&str, NodeRendererFn)> {
            vec![("test-node", stub_node_renderer)]
        }

        fn edge_renderers(&self) -> Vec<(&str, EdgeRendererFn)> {
            vec![("data", stub_edge_renderer)]
        }

        fn inspectors(&self) -> Vec<(&str, InspectorFn)> {
            vec![("test-node", stub_inspector)]
        }

        fn validators(&self) -> Vec<(&str, ValidatorFn)> {
            vec![("stub-validator", stub_validator)]
        }
    }

    // ======================================================================
    // 1. Registry construction tests
    // ======================================================================

    #[test]
    fn new_registry_is_empty() {
        let reg = PluginRegistry::new();
        assert_eq!(reg.plugin_count(), 0);
        assert_eq!(reg.node_renderer_count(), 0);
        assert_eq!(reg.edge_renderer_count(), 0);
        assert_eq!(reg.inspector_count(), 0);
        assert_eq!(reg.validator_count(), 0);
    }

    #[test]
    fn default_registry_is_empty() {
        let reg = PluginRegistry::default();
        assert_eq!(reg.plugin_count(), 0);
    }

    // ======================================================================
    // 2. Registration tests
    // ======================================================================

    #[test]
    fn register_populates_all_entries() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(StubPlugin::new("stub")));
        assert_eq!(reg.plugin_count(), 1);
        assert_eq!(reg.node_renderer_count(), 1);
        assert_eq!(reg.edge_renderer_count(), 1);
        assert_eq!(reg.inspector_count(), 1);
        assert_eq!(reg.validator_count(), 1);
    }

    #[test]
    fn register_multiple_plugins_adds_all_entries() {
        struct PluginA;
        impl FlowPlugin for PluginA {
            fn name(&self) -> &str {
                "plugin-a"
            }
            fn node_renderers(&self) -> Vec<(&str, NodeRendererFn)> {
                vec![("kind-a", stub_node_renderer)]
            }
            fn edge_renderers(&self) -> Vec<(&str, EdgeRendererFn)> {
                vec![]
            }
            fn inspectors(&self) -> Vec<(&str, InspectorFn)> {
                vec![]
            }
            fn validators(&self) -> Vec<(&str, ValidatorFn)> {
                vec![("validator-a", empty_validator)]
            }
        }

        struct PluginB;
        impl FlowPlugin for PluginB {
            fn name(&self) -> &str {
                "plugin-b"
            }
            fn node_renderers(&self) -> Vec<(&str, NodeRendererFn)> {
                vec![("kind-b", stub_node_renderer)]
            }
            fn edge_renderers(&self) -> Vec<(&str, EdgeRendererFn)> {
                vec![]
            }
            fn inspectors(&self) -> Vec<(&str, InspectorFn)> {
                vec![]
            }
            fn validators(&self) -> Vec<(&str, ValidatorFn)> {
                vec![("validator-b", empty_validator)]
            }
        }

        let mut reg = PluginRegistry::new();
        reg.register(Box::new(PluginA));
        reg.register(Box::new(PluginB));
        assert_eq!(reg.plugin_count(), 2);
        assert_eq!(reg.node_renderer_count(), 2);
        assert_eq!(reg.validator_count(), 2);
        assert!(reg.node_renderer_for("kind-a").is_some());
        assert!(reg.node_renderer_for("kind-b").is_some());
    }

    #[test]
    fn later_plugin_overwrites_same_key() {
        struct PluginFirst;
        impl FlowPlugin for PluginFirst {
            fn name(&self) -> &str {
                "first"
            }
            fn node_renderers(&self) -> Vec<(&str, NodeRendererFn)> {
                vec![("shared", stub_node_renderer)]
            }
            fn edge_renderers(&self) -> Vec<(&str, EdgeRendererFn)> {
                vec![]
            }
            fn inspectors(&self) -> Vec<(&str, InspectorFn)> {
                vec![]
            }
            fn validators(&self) -> Vec<(&str, ValidatorFn)> {
                vec![]
            }
        }

        struct PluginSecond;
        impl FlowPlugin for PluginSecond {
            fn name(&self) -> &str {
                "second"
            }
            fn node_renderers(&self) -> Vec<(&str, NodeRendererFn)> {
                vec![("shared", stub_node_renderer)]
            }
            fn edge_renderers(&self) -> Vec<(&str, EdgeRendererFn)> {
                vec![]
            }
            fn inspectors(&self) -> Vec<(&str, InspectorFn)> {
                vec![]
            }
            fn validators(&self) -> Vec<(&str, ValidatorFn)> {
                vec![]
            }
        }

        let mut reg = PluginRegistry::new();
        reg.register(Box::new(PluginFirst));
        reg.register(Box::new(PluginSecond));
        // Same key: count should remain 1
        assert_eq!(reg.node_renderer_count(), 1);
        assert!(reg.node_renderer_for("shared").is_some());
    }

    // ======================================================================
    // 3. Node renderer lookup tests
    // ======================================================================

    #[test]
    fn node_renderer_lookup_succeeds() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(StubPlugin::new("stub")));
        let renderer = reg.node_renderer_for("test-node");
        assert!(renderer.is_some());
        let node = make_node("n1", "test-node");
        let data = renderer.map(|r| r(&node));
        assert!(data.is_some());
        let d = data.unwrap_or_else(|| NodeRenderData {
            label: String::new(),
            shape: NodeShape::Round,
            color: [0.0; 4],
            badges: vec![],
        });
        assert_eq!(d.shape, NodeShape::RoundedRect);
        assert_eq!(d.label, "n1");
        assert_eq!(d.badges.len(), 1);
    }

    #[test]
    fn node_renderer_missing_returns_none() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(StubPlugin::new("stub")));
        assert!(reg.node_renderer_for("nonexistent").is_none());
    }

    // ======================================================================
    // 4. Edge renderer lookup tests
    // ======================================================================

    #[test]
    fn edge_renderer_lookup_succeeds() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(StubPlugin::new("stub")));
        let renderer = reg.edge_renderer_for("data");
        assert!(renderer.is_some());
        let edge = make_edge("e1");
        let data = renderer.map(|r| r(&edge));
        assert!(data.is_some());
        let d = data.unwrap_or_else(|| EdgeRenderData {
            color: [0.0; 4],
            width: 0.0,
            dash: true,
        });
        assert!(!d.dash);
        assert!((d.width - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn edge_renderer_missing_returns_none() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(StubPlugin::new("stub")));
        assert!(reg.edge_renderer_for("nonexistent").is_none());
    }

    // ======================================================================
    // 5. Inspector lookup tests
    // ======================================================================

    #[test]
    fn inspector_lookup_succeeds() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(StubPlugin::new("stub")));
        let insp = reg.inspector_for("test-node");
        assert!(insp.is_some());
        let node = make_node("n1", "test-node");
        let data = insp.map(|i| i(&node));
        assert!(data.is_some());
        let d = data.unwrap_or_else(|| InspectorData {
            title: String::new(),
            fields: vec![],
        });
        assert_eq!(d.title, "n1");
        assert_eq!(d.fields.len(), 1);
        assert_eq!(d.fields[0].0, "kind");
        assert_eq!(d.fields[0].1, "test-node");
    }

    #[test]
    fn inspector_missing_returns_none() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(StubPlugin::new("stub")));
        assert!(reg.inspector_for("nonexistent").is_none());
    }

    // ======================================================================
    // 6. Validator tests
    // ======================================================================

    #[test]
    fn run_all_validators_collects_findings() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(StubPlugin::new("stub")));
        let doc = FlowDocument::default();
        let findings = reg.run_all_validators(&doc);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].level, ValidationLevel::Warning);
        assert_eq!(findings[0].message, "stub warning");
    }

    #[test]
    fn run_all_validators_empty_when_no_validators() {
        let reg = PluginRegistry::new();
        let doc = FlowDocument::default();
        let findings = reg.run_all_validators(&doc);
        assert!(findings.is_empty());
    }

    #[test]
    fn run_all_validators_from_multiple_plugins() {
        struct PluginA;
        impl FlowPlugin for PluginA {
            fn name(&self) -> &str {
                "a"
            }
            fn node_renderers(&self) -> Vec<(&str, NodeRendererFn)> {
                vec![]
            }
            fn edge_renderers(&self) -> Vec<(&str, EdgeRendererFn)> {
                vec![]
            }
            fn inspectors(&self) -> Vec<(&str, InspectorFn)> {
                vec![]
            }
            fn validators(&self) -> Vec<(&str, ValidatorFn)> {
                vec![("validator-a", error_validator)]
            }
        }

        struct PluginB;
        impl FlowPlugin for PluginB {
            fn name(&self) -> &str {
                "b"
            }
            fn node_renderers(&self) -> Vec<(&str, NodeRendererFn)> {
                vec![]
            }
            fn edge_renderers(&self) -> Vec<(&str, EdgeRendererFn)> {
                vec![]
            }
            fn inspectors(&self) -> Vec<(&str, InspectorFn)> {
                vec![]
            }
            fn validators(&self) -> Vec<(&str, ValidatorFn)> {
                vec![("validator-b", stub_validator)]
            }
        }

        let mut reg = PluginRegistry::new();
        reg.register(Box::new(PluginA));
        reg.register(Box::new(PluginB));
        let doc = FlowDocument::default();
        let findings = reg.run_all_validators(&doc);
        // error_validator produces 1 finding, stub_validator produces 1 finding
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].level, ValidationLevel::Error);
        assert_eq!(findings[1].level, ValidationLevel::Warning);
    }

    #[test]
    fn run_validators_finding_has_node_id() {
        let mut reg = PluginRegistry::new();
        reg.register(Box::new(StubPlugin::new("stub")));

        // Register a second plugin with error_validator
        struct ErrorPlugin;
        impl FlowPlugin for ErrorPlugin {
            fn name(&self) -> &str {
                "error-plugin"
            }
            fn node_renderers(&self) -> Vec<(&str, NodeRendererFn)> {
                vec![]
            }
            fn edge_renderers(&self) -> Vec<(&str, EdgeRendererFn)> {
                vec![]
            }
            fn inspectors(&self) -> Vec<(&str, InspectorFn)> {
                vec![]
            }
            fn validators(&self) -> Vec<(&str, ValidatorFn)> {
                vec![("error-validator", error_validator)]
            }
        }
        reg.register(Box::new(ErrorPlugin));

        let doc = FlowDocument::default();
        let findings = reg.run_all_validators(&doc);
        // Find the error-level finding from error_validator
        let error_finding = findings.iter().find(|f| f.level == ValidationLevel::Error);
        assert!(error_finding.is_some());
        if let Some(ef) = error_finding {
            assert!(ef.node_id.is_some());
            assert_eq!(ef.node_id.as_ref().map(|s| s.as_str()), Some("n1"));
        }
    }

    // ======================================================================
    // 7. Empty registry lookup tests
    // ======================================================================

    #[test]
    fn empty_registry_lookup_returns_none() {
        let reg = PluginRegistry::new();
        assert!(reg.node_renderer_for("anything").is_none());
        assert!(reg.edge_renderer_for("anything").is_none());
        assert!(reg.inspector_for("anything").is_none());
    }

    // ======================================================================
    // 8. Data type tests
    // ======================================================================

    #[test]
    fn node_shape_variants_are_distinct() {
        assert_ne!(NodeShape::RoundedRect, NodeShape::Diamond);
        assert_ne!(NodeShape::Round, NodeShape::Pill);
        assert_ne!(NodeShape::Diamond, NodeShape::Round);
    }

    #[test]
    fn node_shape_copy_semantics() {
        let shape = NodeShape::Diamond;
        let shape2 = shape;
        assert_eq!(shape, shape2);
    }

    #[test]
    fn validation_level_variants_are_distinct() {
        assert_ne!(ValidationLevel::Error, ValidationLevel::Warning);
    }

    #[test]
    fn node_render_data_debug_format() {
        let data = NodeRenderData {
            label: String::from("test"),
            shape: NodeShape::RoundedRect,
            color: [1.0, 0.0, 0.0, 1.0],
            badges: vec![String::from("alpha")],
        };
        let s = format!("{data:?}");
        assert!(s.contains("RoundedRect"));
        assert!(s.contains("test"));
    }

    #[test]
    fn edge_render_data_debug_format() {
        let data = EdgeRenderData {
            color: [0.0, 1.0, 0.0, 1.0],
            width: 3.0,
            dash: true,
        };
        let s = format!("{data:?}");
        assert!(s.contains("dash"));
    }

    #[test]
    fn inspector_data_with_fields() {
        let data = InspectorData {
            title: String::from("Properties"),
            fields: vec![
                (String::from("a"), String::from("1")),
                (String::from("b"), String::from("2")),
            ],
        };
        assert_eq!(data.title, "Properties");
        assert_eq!(data.fields.len(), 2);
        assert_eq!(data.fields[0].0, "a");
        assert_eq!(data.fields[1].1, "2");
    }

    #[test]
    fn validation_finding_fields() {
        let finding = ValidationFinding {
            level: ValidationLevel::Error,
            message: String::from("missing output"),
            node_id: Some(SmolStr::from("n1")),
        };
        assert_eq!(finding.level, ValidationLevel::Error);
        assert!(finding.node_id.is_some());
        assert_eq!(finding.message, "missing output");
    }

    #[test]
    fn node_render_data_clone() {
        let data = NodeRenderData {
            label: String::from("clone-test"),
            shape: NodeShape::Pill,
            color: [0.1, 0.2, 0.3, 0.4],
            badges: vec![String::from("badge1")],
        };
        let cloned = data.clone();
        assert_eq!(cloned.label, data.label);
        assert_eq!(cloned.shape, data.shape);
        assert_eq!(cloned.color, data.color);
        assert_eq!(cloned.badges, data.badges);
    }

    #[test]
    fn edge_render_data_clone() {
        let data = EdgeRenderData {
            color: [1.0, 0.0, 0.0, 1.0],
            width: 4.0,
            dash: true,
        };
        let cloned = data.clone();
        assert_eq!(cloned.color, data.color);
        assert!((cloned.width - data.width).abs() < f32::EPSILON);
        assert_eq!(cloned.dash, data.dash);
    }

    #[test]
    fn validation_finding_clone() {
        let finding = ValidationFinding {
            level: ValidationLevel::Warning,
            message: String::from("check connection"),
            node_id: Some(SmolStr::from("n5")),
        };
        let cloned = finding.clone();
        assert_eq!(cloned.level, finding.level);
        assert_eq!(cloned.node_id, finding.node_id);
        assert_eq!(cloned.message, finding.message);
    }

    #[test]
    fn plugin_with_multiple_node_renderers() {
        struct MultiPlugin;
        impl FlowPlugin for MultiPlugin {
            fn name(&self) -> &str {
                "multi"
            }
            fn node_renderers(&self) -> Vec<(&str, NodeRendererFn)> {
                vec![
                    ("type-a", stub_node_renderer),
                    ("type-b", stub_node_renderer),
                    ("type-c", stub_node_renderer),
                ]
            }
            fn edge_renderers(&self) -> Vec<(&str, EdgeRendererFn)> {
                vec![]
            }
            fn inspectors(&self) -> Vec<(&str, InspectorFn)> {
                vec![]
            }
            fn validators(&self) -> Vec<(&str, ValidatorFn)> {
                vec![]
            }
        }

        let mut reg = PluginRegistry::new();
        reg.register(Box::new(MultiPlugin));
        assert_eq!(reg.node_renderer_count(), 3);
        assert!(reg.node_renderer_for("type-a").is_some());
        assert!(reg.node_renderer_for("type-b").is_some());
        assert!(reg.node_renderer_for("type-c").is_some());
    }

    #[test]
    fn plugin_with_no_entries_returns_empty() {
        struct EmptyPlugin;
        impl FlowPlugin for EmptyPlugin {
            fn name(&self) -> &str {
                "empty"
            }
        }

        let mut reg = PluginRegistry::new();
        reg.register(Box::new(EmptyPlugin));
        assert_eq!(reg.plugin_count(), 1);
        assert_eq!(reg.node_renderer_count(), 0);
        assert_eq!(reg.edge_renderer_count(), 0);
        assert_eq!(reg.inspector_count(), 0);
        assert_eq!(reg.validator_count(), 0);
    }
}
