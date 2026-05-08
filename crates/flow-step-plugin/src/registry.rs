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

    // ========================================================================
    // BLACKHAT security review tests
    // ========================================================================

    /// BH-REG-01 (HIGH): NodeKindRegistry allows duplicate registrations of
    /// the same kind. `get()` returns only the first match via linear scan,
    /// silently masking any later registration. An attacker who can register
    /// a kind before the legitimate one will shadow the legitimate entry.
    /// This is a type-confusion vector: code expecting the legitimate
    /// descriptor receives the attacker's descriptor with different shape,
    /// ports, or terminal flags.
    #[test]
    fn blackhat_registry_duplicate_registration_shadows_original() {
        let mut reg = NodeKindRegistry::new();
        // Register a legitimate "task" kind
        reg.register(NodeKindDescriptor {
            kind: SmolStr::from("task"),
            label: SmolStr::from("Task"),
            category: SmolStr::from("action"),
            default_shape: NodeShape::RoundedRect,
            default_ports: DefaultPorts {
                inputs: vec![PortDescriptor {
                    id: SmolStr::from("in"),
                    label: SmolStr::from("Input"),
                    cardinality: PortCardinality::One,
                }],
                outputs: vec![PortDescriptor {
                    id: SmolStr::from("out"),
                    label: SmolStr::from("Output"),
                    cardinality: PortCardinality::One,
                }],
            },
            is_terminal: false,
            is_container: false,
        });
        // Register a malicious "task" kind that is terminal with no outputs
        reg.register(NodeKindDescriptor {
            kind: SmolStr::from("task"),
            label: SmolStr::from("MaliciousTask"),
            category: SmolStr::from("exploit"),
            default_shape: NodeShape::Pill,
            default_ports: DefaultPorts {
                inputs: vec![],
                outputs: vec![],
            },
            is_terminal: true,
            is_container: false,
        });
        // BUG: get() returns the FIRST match, so the attacker cannot shadow
        // by registering AFTER. But if an attacker registers BEFORE (e.g.,
        // via plugin loading order), the legitimate entry is hidden.
        // Either way, the registry silently stores duplicates with no
        // warning or error.
        assert_eq!(reg.all().len(), 2, "both entries stored");
        let found = reg.get("task");
        assert!(found.is_some());
        let desc = found.unwrap_or_else(|| panic!("expected descriptor"));
        // First registration is returned (not the malicious one)
        assert_eq!(desc.label.as_str(), "Task");
        assert!(!desc.is_terminal);
        // But the second registration is silently lurking in the registry
    }

    /// BH-REG-02 (HIGH): register_vb_kinds is not idempotent. Calling it
    /// twice registers every kind twice, doubling the registry size. Code
    /// that iterates `all()` to populate UI or build menus will show
    /// duplicate entries. Code that relies on `all().len()` for counting
    /// unique kinds will get incorrect counts.
    #[test]
    fn blackhat_register_vb_kinds_not_idempotent() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);
        let count_after_first = reg.all().len();
        // Call again -- should be a no-op, but it is not.
        register_vb_kinds(&mut reg);
        let count_after_second = reg.all().len();
        // BUG: Registry doubled in size. No duplicate detection.
        assert_eq!(
            count_after_second,
            count_after_first.saturating_mul(2),
            "calling register_vb_kinds twice doubles the registry size"
        );
        // get() still returns first match, so lookups work, but iteration
        // over all() produces duplicates.
    }

    /// BH-REG-03 (MEDIUM): NodeKindRegistry::get uses a linear scan over
    /// all registered kinds. With O(n) lookup time, a registry bloated
    /// by duplicate registrations or many kinds enables O(n) denial-of-
    /// service on every lookup. Combined with BH-REG-02, repeated calls
    /// to register_vb_kinds cause quadratic degradation.
    #[test]
    fn blackhat_registry_get_is_linear_scan() {
        let mut reg = NodeKindRegistry::new();
        // Register 1000 kinds
        for i in 0..1000 {
            reg.register(NodeKindDescriptor {
                kind: SmolStr::from(format!("kind_{i}")),
                label: SmolStr::from(format!("Kind {i}")),
                category: SmolStr::from("test"),
                default_shape: NodeShape::RoundedRect,
                default_ports: DefaultPorts {
                    inputs: vec![],
                    outputs: vec![],
                },
                is_terminal: false,
                is_container: false,
            });
        }
        // Lookup the LAST registered kind -- linear scan must traverse all.
        let found = reg.get("kind_999");
        assert!(found.is_some());
        // Lookup a missing kind -- must scan all 1000 entries.
        let missing = reg.get("nonexistent");
        assert!(missing.is_none());
        // This is O(n) per lookup with no index. No bug in correctness,
        // but a performance issue that can be exploited by registering
        // many kinds to slow down lookups.
    }

    /// BH-REG-04 (MEDIUM): NodeKindDescriptor fields are entirely
    /// unvalidated. Empty strings for kind, label, category, and port
    /// IDs are accepted. This can cause issues downstream where code
    /// assumes non-empty identifiers.
    #[test]
    fn blackhat_descriptor_accepts_empty_strings() {
        let desc = NodeKindDescriptor {
            kind: SmolStr::from(""),
            label: SmolStr::from(""),
            category: SmolStr::from(""),
            default_shape: NodeShape::RoundedRect,
            default_ports: DefaultPorts {
                inputs: vec![PortDescriptor {
                    id: SmolStr::from(""),
                    label: SmolStr::from(""),
                    cardinality: PortCardinality::One,
                }],
                outputs: vec![],
            },
            is_terminal: false,
            is_container: false,
        };
        let mut reg = NodeKindRegistry::new();
        reg.register(desc);
        // BUG: Empty kind string registered. get("") now returns something.
        let found = reg.get("");
        assert!(found.is_some(), "empty string kind registered successfully");
        let d = found.unwrap_or_else(|| panic!("expected descriptor"));
        assert_eq!(d.kind.as_str(), "");
        assert_eq!(d.default_ports.inputs[0].id.as_str(), "");
    }

    /// BH-REG-05 (LOW): NodeKindRegistry has no method to unregister or
    /// clear kinds. Combined with the append-only design and duplicate
    /// tolerance, there is no way to recover from accidental or malicious
    /// registration of incorrect kinds short of dropping the entire
    /// registry.
    #[test]
    fn blackhat_registry_has_no_unregistration_mechanism() {
        let mut reg = NodeKindRegistry::new();
        reg.register(make_descriptor("temp", "Temp", "test"));
        // No remove(), no clear(), no way to undo this registration.
        assert_eq!(reg.all().len(), 1);
        // The only recovery is to create a new registry.
        let reg2 = NodeKindRegistry::new();
        assert!(reg2.all().is_empty());
    }

    /// BH-REG-06 (INFO): PortDescriptor.cardinality is a Copy enum with
    /// no associated constraints. PortCardinality::Many on an output port
    /// may not be semantically meaningful but is accepted, potentially
    /// causing confusion in downstream rendering or validation.
    #[test]
    fn blackhat_many_cardinality_on_output_port_accepted() {
        let desc = NodeKindDescriptor {
            kind: SmolStr::from("test"),
            label: SmolStr::from("Test"),
            category: SmolStr::from("test"),
            default_shape: NodeShape::RoundedRect,
            default_ports: DefaultPorts {
                inputs: vec![],
                outputs: vec![PortDescriptor {
                    id: SmolStr::from("out"),
                    label: SmolStr::from("Out"),
                    cardinality: PortCardinality::Many,
                }],
            },
            is_terminal: false,
            is_container: false,
        };
        // No validation error -- Many cardinality on output is accepted.
        assert!(matches!(
            desc.default_ports.outputs[0].cardinality,
            PortCardinality::Many
        ));
    }

    // ========================================================================
    // Comprehensive registry tests
    // ========================================================================

    /// All NodeShape variants can be constructed and matched exhaustively.
    #[test]
    fn node_shape_all_variants_are_matchable() {
        let shapes = [
            NodeShape::RoundedRect,
            NodeShape::Diamond,
            NodeShape::Circle,
            NodeShape::Pill,
            NodeShape::DoubleRoundedRect,
        ];
        for shape in &shapes {
            let label = match shape {
                NodeShape::RoundedRect => "rounded-rect",
                NodeShape::Diamond => "diamond",
                NodeShape::Circle => "circle",
                NodeShape::Pill => "pill",
                NodeShape::DoubleRoundedRect => "double-rounded-rect",
            };
            assert!(!label.is_empty());
        }
    }

    /// NodeShape Copy trait allows multiple uses.
    #[test]
    fn node_shape_copy_allows_reuse() {
        let shapes = [
            NodeShape::RoundedRect,
            NodeShape::Diamond,
            NodeShape::Circle,
            NodeShape::Pill,
            NodeShape::DoubleRoundedRect,
        ];
        let copied: Vec<NodeShape> = shapes.to_vec();
        assert_eq!(copied.len(), shapes.len());
        for (original, copy) in shapes.iter().zip(copied.iter()) {
            let orig_label = match original {
                NodeShape::RoundedRect => 1,
                NodeShape::Diamond => 2,
                NodeShape::Circle => 3,
                NodeShape::Pill => 4,
                NodeShape::DoubleRoundedRect => 5,
            };
            let copy_label = match copy {
                NodeShape::RoundedRect => 1,
                NodeShape::Diamond => 2,
                NodeShape::Circle => 3,
                NodeShape::Pill => 4,
                NodeShape::DoubleRoundedRect => 5,
            };
            assert_eq!(orig_label, copy_label);
        }
    }

    /// PortCardinality Copy and Clone semantics.
    #[test]
    fn port_cardinality_copy_and_clone() {
        let one = PortCardinality::One;
        let many = PortCardinality::Many;
        let one2 = one;
        let many2 = many;
        assert!(matches!(one2, PortCardinality::One));
        assert!(matches!(many2, PortCardinality::Many));
        let one3 = one.clone();
        let many3 = many.clone();
        assert!(matches!(one3, PortCardinality::One));
        assert!(matches!(many3, PortCardinality::Many));
    }

    /// PortDescriptor clone preserves all fields.
    #[test]
    fn port_descriptor_clone_preserves_fields() {
        let port = PortDescriptor {
            id: SmolStr::from("in_data"),
            label: SmolStr::from("Input Data"),
            cardinality: PortCardinality::Many,
        };
        let cloned = port.clone();
        assert_eq!(cloned.id, port.id);
        assert_eq!(cloned.label, port.label);
        assert!(matches!(cloned.cardinality, PortCardinality::Many));
    }

    /// DefaultPorts with many ports on each side.
    #[test]
    fn default_ports_with_multiple_inputs_and_outputs() {
        let ports = DefaultPorts {
            inputs: vec![
                PortDescriptor {
                    id: SmolStr::from("in_1"),
                    label: SmolStr::from("Input 1"),
                    cardinality: PortCardinality::One,
                },
                PortDescriptor {
                    id: SmolStr::from("in_2"),
                    label: SmolStr::from("Input 2"),
                    cardinality: PortCardinality::Many,
                },
            ],
            outputs: vec![
                PortDescriptor {
                    id: SmolStr::from("out_1"),
                    label: SmolStr::from("Output 1"),
                    cardinality: PortCardinality::One,
                },
                PortDescriptor {
                    id: SmolStr::from("out_2"),
                    label: SmolStr::from("Output 2"),
                    cardinality: PortCardinality::One,
                },
            ],
        };
        assert_eq!(ports.inputs.len(), 2);
        assert_eq!(ports.outputs.len(), 2);
    }

    /// NodeKindDescriptor debug output includes all relevant fields.
    #[test]
    fn node_kind_descriptor_debug_output() {
        let desc = NodeKindDescriptor {
            kind: SmolStr::from("custom"),
            label: SmolStr::from("Custom Node"),
            category: SmolStr::from("experimental"),
            default_shape: NodeShape::Diamond,
            default_ports: DefaultPorts {
                inputs: vec![],
                outputs: vec![],
            },
            is_terminal: true,
            is_container: false,
        };
        let debug = format!("{desc:?}");
        assert!(
            debug.contains("custom"),
            "debug should contain kind: {debug}"
        );
        assert!(
            debug.contains("experimental"),
            "debug should contain category: {debug}"
        );
    }

    /// Register multiple kinds and verify all() returns them in insertion order.
    #[test]
    fn registry_all_preserves_insertion_order() {
        let mut reg = NodeKindRegistry::new();
        let kinds = ["zebra", "alpha", "middle", "omega"];
        for &kind in &kinds {
            reg.register(make_descriptor(kind, kind, "test"));
        }
        let all = reg.all();
        assert_eq!(all.len(), kinds.len());
        for (i, &expected_kind) in kinds.iter().enumerate() {
            assert_eq!(
                all[i].kind.as_str(),
                expected_kind,
                "expected kind at index {i} to be {expected_kind}"
            );
        }
    }

    /// get() returns first match when duplicates exist.
    #[test]
    fn registry_get_returns_first_of_duplicates() {
        let mut reg = NodeKindRegistry::new();
        reg.register(NodeKindDescriptor {
            kind: SmolStr::from("dup"),
            label: SmolStr::from("First"),
            category: SmolStr::from("cat"),
            default_shape: NodeShape::RoundedRect,
            default_ports: DefaultPorts {
                inputs: vec![],
                outputs: vec![],
            },
            is_terminal: false,
            is_container: false,
        });
        reg.register(NodeKindDescriptor {
            kind: SmolStr::from("dup"),
            label: SmolStr::from("Second"),
            category: SmolStr::from("cat"),
            default_shape: NodeShape::Circle,
            default_ports: DefaultPorts {
                inputs: vec![],
                outputs: vec![],
            },
            is_terminal: true,
            is_container: true,
        });
        let found = reg.get("dup");
        assert!(found.is_some());
        let desc = match found {
            Some(d) => d,
            None => return,
        };
        assert_eq!(desc.label.as_str(), "First");
        assert!(matches!(desc.default_shape, NodeShape::RoundedRect));
        assert!(!desc.is_terminal);
    }

    /// Verify all VB kinds have correct shapes.
    #[test]
    fn vb_kinds_shape_assignments() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);

        let expected_shapes: [(&str, NodeShape); 11] = [
            ("data", NodeShape::RoundedRect),
            ("external", NodeShape::RoundedRect),
            ("branch", NodeShape::Diamond),
            ("terminal", NodeShape::Pill),
            ("loop", NodeShape::DoubleRoundedRect),
            ("parallel", NodeShape::DoubleRoundedRect),
            ("collect", NodeShape::RoundedRect),
            ("reduce", NodeShape::RoundedRect),
            ("suspend", NodeShape::RoundedRect),
            ("error", NodeShape::Pill),
            ("control", NodeShape::Circle),
        ];
        for (kind, expected_shape) in &expected_shapes {
            let desc = reg.get(kind);
            assert!(desc.is_some(), "missing kind {kind:?}");
            let d = match desc {
                Some(val) => val,
                None => continue,
            };
            let matches_shape = match expected_shape {
                NodeShape::RoundedRect => matches!(d.default_shape, NodeShape::RoundedRect),
                NodeShape::Diamond => matches!(d.default_shape, NodeShape::Diamond),
                NodeShape::Circle => matches!(d.default_shape, NodeShape::Circle),
                NodeShape::Pill => matches!(d.default_shape, NodeShape::Pill),
                NodeShape::DoubleRoundedRect => {
                    matches!(d.default_shape, NodeShape::DoubleRoundedRect)
                }
            };
            assert!(matches_shape, "kind {kind:?} should have correct shape");
        }
    }

    /// Verify terminal flags for all VB kinds.
    #[test]
    fn vb_kinds_terminal_flags() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);

        let terminal_kinds = ["terminal", "error"];
        let non_terminal_kinds = [
            "data", "external", "branch", "loop", "parallel", "collect", "reduce", "suspend",
            "control",
        ];
        for kind in &terminal_kinds {
            let desc = reg.get(kind);
            assert!(desc.is_some(), "missing kind {kind:?}");
            assert!(
                desc.map_or(false, |d| d.is_terminal),
                "kind {kind:?} should be terminal"
            );
        }
        for kind in &non_terminal_kinds {
            let desc = reg.get(kind);
            assert!(desc.is_some(), "missing kind {kind:?}");
            assert!(
                desc.map_or(true, |d| !d.is_terminal),
                "kind {kind:?} should NOT be terminal"
            );
        }
    }

    /// Verify container flags for all VB kinds.
    #[test]
    fn vb_kinds_container_flags() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);

        let container_kinds = ["loop", "parallel"];
        let non_container_kinds = [
            "data", "external", "branch", "terminal", "collect", "reduce", "suspend", "error",
            "control",
        ];
        for kind in &container_kinds {
            let desc = reg.get(kind);
            assert!(desc.is_some(), "missing kind {kind:?}");
            assert!(
                desc.map_or(false, |d| d.is_container),
                "kind {kind:?} should be a container"
            );
        }
        for kind in &non_container_kinds {
            let desc = reg.get(kind);
            assert!(desc.is_some(), "missing kind {kind:?}");
            assert!(
                desc.map_or(true, |d| !d.is_container),
                "kind {kind:?} should NOT be a container"
            );
        }
    }

    /// Verify categories for all VB kinds.
    #[test]
    fn vb_kinds_category_assignments() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);

        let expected_categories: [(&str, &str); 11] = [
            ("data", "data"),
            ("external", "external"),
            ("branch", "branch"),
            ("terminal", "terminal"),
            ("loop", "control"),
            ("parallel", "control"),
            ("collect", "data"),
            ("reduce", "data"),
            ("suspend", "control"),
            ("error", "terminal"),
            ("control", "control"),
        ];
        for (kind, expected_cat) in &expected_categories {
            let desc = reg.get(kind);
            assert!(desc.is_some(), "missing kind {kind:?}");
            assert_eq!(
                desc.map_or("", |d| d.category.as_str()),
                *expected_cat,
                "kind {kind:?} should have category {expected_cat:?}"
            );
        }
    }

    /// Loop kind has exactly 2 outputs named "body" and "done".
    #[test]
    fn vb_kinds_loop_output_port_ids() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);
        let desc = reg.get("loop");
        assert!(desc.is_some());
        let d = match desc {
            Some(val) => val,
            None => return,
        };
        assert_eq!(d.default_ports.outputs.len(), 2);
        assert_eq!(d.default_ports.outputs[0].id.as_str(), "body");
        assert_eq!(d.default_ports.outputs[1].id.as_str(), "done");
    }

    /// Reduce kind has two input ports with specific IDs.
    #[test]
    fn vb_kinds_reduce_input_port_ids() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);
        let desc = reg.get("reduce");
        assert!(desc.is_some());
        let d = match desc {
            Some(val) => val,
            None => return,
        };
        assert_eq!(d.default_ports.inputs.len(), 2);
        assert_eq!(d.default_ports.inputs[0].id.as_str(), "items");
        assert_eq!(d.default_ports.inputs[1].id.as_str(), "initial");
        assert!(matches!(
            d.default_ports.inputs[0].cardinality,
            PortCardinality::Many
        ));
        assert!(matches!(
            d.default_ports.inputs[1].cardinality,
            PortCardinality::One
        ));
    }

    /// NodeKindDescriptor with all shape variants in a single registry.
    #[test]
    fn registry_holds_all_shape_variants() {
        let mut reg = NodeKindRegistry::new();
        let shapes_and_kinds: [(NodeShape, &str); 5] = [
            (NodeShape::RoundedRect, "rr"),
            (NodeShape::Diamond, "dia"),
            (NodeShape::Circle, "cir"),
            (NodeShape::Pill, "pill"),
            (NodeShape::DoubleRoundedRect, "drr"),
        ];
        for (shape, kind) in &shapes_and_kinds {
            reg.register(NodeKindDescriptor {
                kind: SmolStr::from(*kind),
                label: SmolStr::from(*kind),
                category: SmolStr::from("shape-test"),
                default_shape: *shape,
                default_ports: DefaultPorts {
                    inputs: vec![],
                    outputs: vec![],
                },
                is_terminal: false,
                is_container: false,
            });
        }
        assert_eq!(reg.all().len(), 5);
        for (_, kind) in &shapes_and_kinds {
            assert!(reg.get(kind).is_some(), "should find kind {kind}");
        }
    }

    /// PortDescriptor with SmolStr fields clones independently.
    #[test]
    fn port_descriptor_clone_is_independent() {
        let port = PortDescriptor {
            id: SmolStr::from("unique_port"),
            label: SmolStr::from("My Port"),
            cardinality: PortCardinality::One,
        };
        let mut cloned = port.clone();
        let original_id = port.id.clone();
        cloned.id = SmolStr::from("modified");
        assert_eq!(port.id, original_id);
        assert_eq!(cloned.id, SmolStr::from("modified"));
    }

    /// NodeKindRegistry get() on a registry with many entries finds the correct one.
    #[test]
    fn registry_get_finds_among_many() {
        let mut reg = NodeKindRegistry::new();
        for i in 0..50u32 {
            reg.register(NodeKindDescriptor {
                kind: SmolStr::from(format!("kind_{i}")),
                label: SmolStr::from(format!("Kind {i}")),
                category: SmolStr::from("test"),
                default_shape: NodeShape::RoundedRect,
                default_ports: DefaultPorts {
                    inputs: vec![],
                    outputs: vec![],
                },
                is_terminal: false,
                is_container: false,
            });
        }
        let found = reg.get("kind_25");
        assert!(found.is_some());
        assert_eq!(found.map_or("", |d| d.label.as_str()), "Kind 25");
        assert!(reg.get("kind_999").is_none());
    }

    /// Verify VB kinds labels are human-readable.
    #[test]
    fn vb_kinds_labels_are_readable() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);

        let expected_labels: [(&str, &str); 11] = [
            ("data", "Data"),
            ("external", "Action"),
            ("branch", "Choice"),
            ("terminal", "Finish"),
            ("loop", "Loop"),
            ("parallel", "Parallel"),
            ("collect", "Collect"),
            ("reduce", "Reduce"),
            ("suspend", "Suspend"),
            ("error", "Error"),
            ("control", "Control"),
        ];
        for (kind, expected_label) in &expected_labels {
            let desc = reg.get(kind);
            assert!(desc.is_some(), "missing kind {kind:?}");
            assert_eq!(
                desc.map_or("", |d| d.label.as_str()),
                *expected_label,
                "kind {kind:?} should have label {expected_label:?}"
            );
        }
    }

    /// VB kinds suspend has correct port configuration.
    #[test]
    fn vb_kinds_suspend_port_configuration() {
        let mut reg = NodeKindRegistry::new();
        register_vb_kinds(&mut reg);
        let desc = reg.get("suspend");
        assert!(desc.is_some());
        let d = match desc {
            Some(val) => val,
            None => return,
        };
        assert_eq!(d.default_ports.inputs.len(), 1);
        assert_eq!(d.default_ports.inputs[0].id.as_str(), "in");
        assert_eq!(d.default_ports.outputs.len(), 1);
        assert_eq!(d.default_ports.outputs[0].id.as_str(), "resume");
    }
}
