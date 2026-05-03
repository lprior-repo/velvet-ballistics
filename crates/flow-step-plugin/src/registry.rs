use smol_str::SmolStr;

/// Visual properties for a registered node kind.
#[derive(Debug, Clone)]
pub struct NodeKindDescriptor {
    pub kind: SmolStr,
    pub label: SmolStr,
    pub category: SmolStr,
    pub default_shape: NodeShape,
    pub default_ports: DefaultPorts,
    pub is_terminal: bool,
    pub is_container: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum NodeShape {
    RoundedRect,
    Diamond,
    Circle,
    Pill,
    DoubleRoundedRect,
}

#[derive(Debug, Clone)]
pub struct DefaultPorts {
    pub inputs: Vec<PortDescriptor>,
    pub outputs: Vec<PortDescriptor>,
}

#[derive(Debug, Clone)]
pub struct PortDescriptor {
    pub id: SmolStr,
    pub label: SmolStr,
    pub cardinality: PortCardinality,
}

#[derive(Debug, Clone, Copy)]
pub enum PortCardinality {
    One,
    Many,
}

/// Registry of all known node kinds.
pub struct NodeKindRegistry {
    kinds: Vec<NodeKindDescriptor>,
}

impl Default for NodeKindRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl NodeKindRegistry {
    pub fn new() -> Self {
        Self { kinds: Vec::new() }
    }

    pub fn register(&mut self, descriptor: NodeKindDescriptor) {
        self.kinds.push(descriptor);
    }

    pub fn get(&self, kind: &str) -> Option<&NodeKindDescriptor> {
        self.kinds.iter().find(|k| k.kind == kind)
    }

    pub fn all(&self) -> &[NodeKindDescriptor] {
        &self.kinds
    }
}

/// Populate registry with Velvet Ballistics node kinds.
pub fn register_vb_kinds(registry: &mut NodeKindRegistry) {
    // Data operations
    registry.register(NodeKindDescriptor {
        kind: SmolStr::from("data"),
        label: SmolStr::from("Data"),
        category: SmolStr::from("data"),
        default_shape: NodeShape::RoundedRect,
        default_ports: DefaultPorts {
            inputs: vec![],
            outputs: vec![],
        },
        is_terminal: false,
        is_container: false,
    });

    // External actions
    registry.register(NodeKindDescriptor {
        kind: SmolStr::from("external"),
        label: SmolStr::from("Action"),
        category: SmolStr::from("external"),
        default_shape: NodeShape::RoundedRect,
        default_ports: DefaultPorts {
            inputs: vec![PortDescriptor {
                id: SmolStr::from("input"),
                label: SmolStr::from("input"),
                cardinality: PortCardinality::One,
            }],
            outputs: vec![PortDescriptor {
                id: SmolStr::from("output"),
                label: SmolStr::from("output"),
                cardinality: PortCardinality::One,
            }],
        },
        is_terminal: false,
        is_container: false,
    });

    // Branching
    registry.register(NodeKindDescriptor {
        kind: SmolStr::from("branch"),
        label: SmolStr::from("Choice"),
        category: SmolStr::from("branch"),
        default_shape: NodeShape::Diamond,
        default_ports: DefaultPorts {
            inputs: vec![PortDescriptor {
                id: SmolStr::from("in"),
                label: SmolStr::from("in"),
                cardinality: PortCardinality::One,
            }],
            outputs: vec![PortDescriptor {
                id: SmolStr::from("default"),
                label: SmolStr::from("default"),
                cardinality: PortCardinality::One,
            }],
        },
        is_terminal: false,
        is_container: false,
    });

    // Terminal
    registry.register(NodeKindDescriptor {
        kind: SmolStr::from("terminal"),
        label: SmolStr::from("Finish"),
        category: SmolStr::from("terminal"),
        default_shape: NodeShape::Pill,
        default_ports: DefaultPorts {
            inputs: vec![PortDescriptor {
                id: SmolStr::from("result"),
                label: SmolStr::from("result"),
                cardinality: PortCardinality::One,
            }],
            outputs: vec![],
        },
        is_terminal: true,
        is_container: false,
    });

    // Loop
    registry.register(NodeKindDescriptor {
        kind: SmolStr::from("loop"),
        label: SmolStr::from("Loop"),
        category: SmolStr::from("control"),
        default_shape: NodeShape::DoubleRoundedRect,
        default_ports: DefaultPorts {
            inputs: vec![PortDescriptor {
                id: SmolStr::from("in"),
                label: SmolStr::from("in"),
                cardinality: PortCardinality::One,
            }],
            outputs: vec![
                PortDescriptor {
                    id: SmolStr::from("body"),
                    label: SmolStr::from("body"),
                    cardinality: PortCardinality::One,
                },
                PortDescriptor {
                    id: SmolStr::from("done"),
                    label: SmolStr::from("done"),
                    cardinality: PortCardinality::One,
                },
            ],
        },
        is_terminal: false,
        is_container: true,
    });

    // Parallel
    registry.register(NodeKindDescriptor {
        kind: SmolStr::from("parallel"),
        label: SmolStr::from("Parallel"),
        category: SmolStr::from("control"),
        default_shape: NodeShape::DoubleRoundedRect,
        default_ports: DefaultPorts {
            inputs: vec![PortDescriptor {
                id: SmolStr::from("in"),
                label: SmolStr::from("in"),
                cardinality: PortCardinality::One,
            }],
            outputs: vec![PortDescriptor {
                id: SmolStr::from("out"),
                label: SmolStr::from("out"),
                cardinality: PortCardinality::One,
            }],
        },
        is_terminal: false,
        is_container: true,
    });

    // Collect
    registry.register(NodeKindDescriptor {
        kind: SmolStr::from("collect"),
        label: SmolStr::from("Collect"),
        category: SmolStr::from("data"),
        default_shape: NodeShape::RoundedRect,
        default_ports: DefaultPorts {
            inputs: vec![PortDescriptor {
                id: SmolStr::from("items"),
                label: SmolStr::from("items"),
                cardinality: PortCardinality::Many,
            }],
            outputs: vec![PortDescriptor {
                id: SmolStr::from("output"),
                label: SmolStr::from("output"),
                cardinality: PortCardinality::One,
            }],
        },
        is_terminal: false,
        is_container: false,
    });

    // Reduce
    registry.register(NodeKindDescriptor {
        kind: SmolStr::from("reduce"),
        label: SmolStr::from("Reduce"),
        category: SmolStr::from("data"),
        default_shape: NodeShape::RoundedRect,
        default_ports: DefaultPorts {
            inputs: vec![
                PortDescriptor {
                    id: SmolStr::from("items"),
                    label: SmolStr::from("items"),
                    cardinality: PortCardinality::Many,
                },
                PortDescriptor {
                    id: SmolStr::from("initial"),
                    label: SmolStr::from("initial"),
                    cardinality: PortCardinality::One,
                },
            ],
            outputs: vec![PortDescriptor {
                id: SmolStr::from("result"),
                label: SmolStr::from("result"),
                cardinality: PortCardinality::One,
            }],
        },
        is_terminal: false,
        is_container: false,
    });

    // Suspend
    registry.register(NodeKindDescriptor {
        kind: SmolStr::from("suspend"),
        label: SmolStr::from("Suspend"),
        category: SmolStr::from("control"),
        default_shape: NodeShape::RoundedRect,
        default_ports: DefaultPorts {
            inputs: vec![PortDescriptor {
                id: SmolStr::from("in"),
                label: SmolStr::from("in"),
                cardinality: PortCardinality::One,
            }],
            outputs: vec![PortDescriptor {
                id: SmolStr::from("resume"),
                label: SmolStr::from("resume"),
                cardinality: PortCardinality::One,
            }],
        },
        is_terminal: false,
        is_container: false,
    });

    // Error
    registry.register(NodeKindDescriptor {
        kind: SmolStr::from("error"),
        label: SmolStr::from("Error"),
        category: SmolStr::from("terminal"),
        default_shape: NodeShape::Pill,
        default_ports: DefaultPorts {
            inputs: vec![PortDescriptor {
                id: SmolStr::from("in"),
                label: SmolStr::from("in"),
                cardinality: PortCardinality::One,
            }],
            outputs: vec![],
        },
        is_terminal: true,
        is_container: false,
    });

    // Control
    registry.register(NodeKindDescriptor {
        kind: SmolStr::from("control"),
        label: SmolStr::from("Control"),
        category: SmolStr::from("control"),
        default_shape: NodeShape::Circle,
        default_ports: DefaultPorts {
            inputs: vec![PortDescriptor {
                id: SmolStr::from("in"),
                label: SmolStr::from("in"),
                cardinality: PortCardinality::One,
            }],
            outputs: vec![PortDescriptor {
                id: SmolStr::from("out"),
                label: SmolStr::from("out"),
                cardinality: PortCardinality::One,
            }],
        },
        is_terminal: false,
        is_container: false,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_descriptor(kind: &str, label: &str, category: &str) -> NodeKindDescriptor {
        NodeKindDescriptor {
            kind: SmolStr::from(kind),
            label: SmolStr::from(label),
            category: SmolStr::from(category),
            default_shape: NodeShape::RoundedRect,
            default_ports: DefaultPorts {
                inputs: vec![],
                outputs: vec![],
            },
            is_terminal: false,
            is_container: false,
        }
    }

    // -- Registry basics --

    #[test]
    fn new_registry_is_empty() {
        let reg = NodeKindRegistry::new();
        assert!(reg.all().is_empty(), "new registry should have zero kinds");
    }

    #[test]
    fn default_registry_is_empty() {
        let reg = NodeKindRegistry::default();
        assert!(reg.all().is_empty());
    }

    #[test]
    fn register_and_lookup() {
        let mut reg = NodeKindRegistry::new();
        reg.register(make_descriptor("alpha", "Alpha", "test"));
        assert_eq!(reg.all().len(), 1);
        let found = reg.get("alpha");
        assert!(found.is_some(), "should find registered kind");
        let desc = found.unwrap_or_else(|| panic!("expected descriptor"));
        assert_eq!(desc.kind.as_str(), "alpha");
        assert_eq!(desc.label.as_str(), "Alpha");
        assert_eq!(desc.category.as_str(), "test");
    }

    #[test]
    fn lookup_missing_returns_none() {
        let reg = NodeKindRegistry::new();
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn register_duplicate_allows_both() {
        // The registry is append-only, so duplicates are stored.
        let mut reg = NodeKindRegistry::new();
        reg.register(make_descriptor("dup", "First", "cat"));
        reg.register(make_descriptor("dup", "Second", "cat"));
        assert_eq!(reg.all().len(), 2, "duplicate kinds should both be stored");
        // get() returns the first match
        let first = reg.get("dup");
        assert!(first.is_some());
        assert_eq!(
            first.map(|d| d.label.as_str()),
            Some("First"),
            "get should return first registered match"
        );
    }

    #[test]
    fn register_multiple_distinct_kinds() {
        let mut reg = NodeKindRegistry::new();
        reg.register(make_descriptor("a", "A", "cat1"));
        reg.register(make_descriptor("b", "B", "cat2"));
        reg.register(make_descriptor("c", "C", "cat1"));
        assert_eq!(reg.all().len(), 3);
        assert!(reg.get("a").is_some());
        assert!(reg.get("b").is_some());
        assert!(reg.get("c").is_some());
        assert!(reg.get("d").is_none());
    }

    // -- Descriptor structure tests --

    #[test]
    fn descriptor_with_ports() {
        let desc = NodeKindDescriptor {
            kind: SmolStr::from("portnode"),
            label: SmolStr::from("PortNode"),
            category: SmolStr::from("test"),
            default_shape: NodeShape::Diamond,
            default_ports: DefaultPorts {
                inputs: vec![PortDescriptor {
                    id: SmolStr::from("in1"),
                    label: SmolStr::from("Input"),
                    cardinality: PortCardinality::Many,
                }],
                outputs: vec![PortDescriptor {
                    id: SmolStr::from("out1"),
                    label: SmolStr::from("Output"),
                    cardinality: PortCardinality::One,
                }],
            },
            is_terminal: false,
            is_container: true,
        };
        assert_eq!(desc.default_ports.inputs.len(), 1);
        assert_eq!(desc.default_ports.outputs.len(), 1);
        assert!(desc.is_container);
        assert!(!desc.is_terminal);
    }

    #[test]
    fn node_shape_copy_semantics() {
        let shape = NodeShape::Pill;
        let shape2 = shape;
        // Both usable (Copy trait)
        let _ = shape;
        let _ = shape2;
    }

    #[test]
    fn port_cardinality_copy_semantics() {
        let c = PortCardinality::Many;
        let c2 = c;
        assert!(matches!(c, PortCardinality::Many));
        assert!(matches!(c2, PortCardinality::Many));
    }

    // -- register_vb_kinds tests --

    #[test]
    fn vb_kinds_registers_all_expected_kinds() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);
        let expected_kinds = [
            "data", "external", "branch", "terminal", "loop", "parallel", "collect", "reduce",
            "suspend", "error", "control",
        ];
        assert_eq!(
            reg.all().len(),
            expected_kinds.len(),
            "should register exactly {} kinds",
            expected_kinds.len()
        );
        for kind in &expected_kinds {
            assert!(
                reg.get(kind).is_some(),
                "missing kind {kind:?} in VB registry"
            );
        }
    }

    #[test]
    fn vb_kinds_data_has_no_ports() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);
        let desc = reg.get("data").unwrap_or_else(|| panic!("missing data"));
        assert!(desc.default_ports.inputs.is_empty());
        assert!(desc.default_ports.outputs.is_empty());
        assert!(!desc.is_terminal);
        assert!(!desc.is_container);
    }

    #[test]
    fn vb_kinds_external_has_input_output_ports() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);
        let desc = reg
            .get("external")
            .unwrap_or_else(|| panic!("missing external"));
        assert_eq!(desc.default_ports.inputs.len(), 1);
        assert_eq!(desc.default_ports.outputs.len(), 1);
        assert_eq!(desc.default_ports.inputs[0].id.as_str(), "input");
        assert_eq!(desc.default_ports.outputs[0].id.as_str(), "output");
    }

    #[test]
    fn vb_kinds_branch_is_diamond() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);
        let desc = reg
            .get("branch")
            .unwrap_or_else(|| panic!("missing branch"));
        assert!(matches!(desc.default_shape, NodeShape::Diamond));
        assert!(!desc.is_terminal);
        assert!(!desc.is_container);
        // One input, one default output
        assert_eq!(desc.default_ports.inputs.len(), 1);
        assert_eq!(desc.default_ports.outputs.len(), 1);
    }

    #[test]
    fn vb_kinds_terminal_is_pill_and_terminal() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);
        let desc = reg
            .get("terminal")
            .unwrap_or_else(|| panic!("missing terminal"));
        assert!(matches!(desc.default_shape, NodeShape::Pill));
        assert!(desc.is_terminal);
        assert!(!desc.is_container);
        assert_eq!(desc.default_ports.inputs.len(), 1);
        assert!(desc.default_ports.outputs.is_empty());
    }

    #[test]
    fn vb_kinds_loop_is_container_double_rounded() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);
        let desc = reg.get("loop").unwrap_or_else(|| panic!("missing loop"));
        assert!(matches!(desc.default_shape, NodeShape::DoubleRoundedRect));
        assert!(desc.is_container);
        assert!(!desc.is_terminal);
        // 1 input, 2 outputs (body + done)
        assert_eq!(desc.default_ports.inputs.len(), 1);
        assert_eq!(desc.default_ports.outputs.len(), 2);
    }

    #[test]
    fn vb_kinds_parallel_is_container_double_rounded() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);
        let desc = reg
            .get("parallel")
            .unwrap_or_else(|| panic!("missing parallel"));
        assert!(matches!(desc.default_shape, NodeShape::DoubleRoundedRect));
        assert!(desc.is_container);
        assert!(!desc.is_terminal);
    }

    #[test]
    fn vb_kinds_collect_has_many_input() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);
        let desc = reg
            .get("collect")
            .unwrap_or_else(|| panic!("missing collect"));
        assert_eq!(desc.category.as_str(), "data");
        assert_eq!(desc.default_ports.inputs.len(), 1);
        assert!(matches!(
            desc.default_ports.inputs[0].cardinality,
            PortCardinality::Many
        ));
    }

    #[test]
    fn vb_kinds_reduce_has_two_inputs() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);
        let desc = reg
            .get("reduce")
            .unwrap_or_else(|| panic!("missing reduce"));
        assert_eq!(desc.default_ports.inputs.len(), 2);
        assert_eq!(desc.default_ports.outputs.len(), 1);
    }

    #[test]
    fn vb_kinds_error_is_terminal_pill() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);
        let desc = reg.get("error").unwrap_or_else(|| panic!("missing error"));
        assert!(matches!(desc.default_shape, NodeShape::Pill));
        assert!(desc.is_terminal);
        assert!(desc.default_ports.outputs.is_empty());
    }

    #[test]
    fn vb_kinds_control_is_circle() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);
        let desc = reg
            .get("control")
            .unwrap_or_else(|| panic!("missing control"));
        assert!(matches!(desc.default_shape, NodeShape::Circle));
        assert_eq!(desc.default_ports.inputs.len(), 1);
        assert_eq!(desc.default_ports.outputs.len(), 1);
    }

    #[test]
    fn vb_kinds_suspend_category_is_control() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);
        let desc = reg
            .get("suspend")
            .unwrap_or_else(|| panic!("missing suspend"));
        assert_eq!(desc.category.as_str(), "control");
        assert!(!desc.is_terminal);
        assert!(!desc.is_container);
    }

    // -- Clone round-trip tests --

    #[test]
    fn descriptor_clone_roundtrip() {
        let desc = make_descriptor("clone_test", "CloneTest", "cat");
        let cloned = desc.clone();
        assert_eq!(cloned.kind, desc.kind);
        assert_eq!(cloned.label, desc.label);
        assert_eq!(cloned.category, desc.category);
    }

    #[test]
    fn default_ports_clone_roundtrip() {
        let ports = DefaultPorts {
            inputs: vec![PortDescriptor {
                id: SmolStr::from("in"),
                label: SmolStr::from("In"),
                cardinality: PortCardinality::One,
            }],
            outputs: vec![],
        };
        let cloned = ports.clone();
        assert_eq!(cloned.inputs.len(), 1);
        assert!(cloned.outputs.is_empty());
    }
}
