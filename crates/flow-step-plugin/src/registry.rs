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
