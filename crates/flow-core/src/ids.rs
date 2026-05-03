use smol_str::SmolStr;

pub type NodeId = SmolStr;
pub type EdgeId = SmolStr;
pub type PortId = SmolStr;
pub type GroupId = SmolStr;
pub type PluginId = SmolStr;

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node_id(s: &str) -> NodeId {
        SmolStr::from(s)
    }
    fn make_edge_id(s: &str) -> EdgeId {
        SmolStr::from(s)
    }
    fn make_port_id(s: &str) -> PortId {
        SmolStr::from(s)
    }
    fn make_group_id(s: &str) -> GroupId {
        SmolStr::from(s)
    }
    fn make_plugin_id(s: &str) -> PluginId {
        SmolStr::from(s)
    }

    #[test]
    fn node_id_construction() {
        let id = make_node_id("node-1");
        assert_eq!(id.as_str(), "node-1");
    }

    #[test]
    fn edge_id_construction() {
        let id = make_edge_id("edge-42");
        assert_eq!(id.as_str(), "edge-42");
    }

    #[test]
    fn port_id_construction() {
        let id = make_port_id("port-in-0");
        assert_eq!(id.as_str(), "port-in-0");
    }

    #[test]
    fn group_id_construction() {
        let id = make_group_id("group-alpha");
        assert_eq!(id.as_str(), "group-alpha");
    }

    #[test]
    fn plugin_id_construction() {
        let id = make_plugin_id("plugin-renderer");
        assert_eq!(id.as_str(), "plugin-renderer");
    }

    #[test]
    fn node_id_equality() {
        let a = make_node_id("x");
        let b = make_node_id("x");
        let c = make_node_id("y");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn edge_id_equality() {
        let a = make_edge_id("e1");
        let b = make_edge_id("e1");
        assert_eq!(a, b);
    }

    #[test]
    fn port_id_clone() {
        let id = make_port_id("p1");
        let cloned = id.clone();
        assert_eq!(id, cloned);
    }

    #[test]
    fn group_id_clone() {
        let id = make_group_id("g1");
        let cloned = id.clone();
        assert_eq!(id, cloned);
    }

    #[test]
    fn empty_id() {
        let id = make_node_id("");
        assert!(id.is_empty());
    }

    #[test]
    fn id_types_are_distinct_at_compile_time() {
        // This test just verifies the type aliases compile as expected.
        // NodeId and EdgeId are both SmolStr but used in distinct positions.
        let _node: NodeId = SmolStr::from("n");
        let _edge: EdgeId = SmolStr::from("e");
        let _port: PortId = SmolStr::from("p");
        let _group: GroupId = SmolStr::from("g");
        let _plugin: PluginId = SmolStr::from("pl");
    }

    #[test]
    fn id_from_string() {
        let id: NodeId = SmolStr::from(String::from("from-string"));
        assert_eq!(id.as_str(), "from-string");
    }

    #[test]
    fn id_from_static() {
        let id: NodeId = SmolStr::new_static("static-id");
        assert_eq!(id.as_str(), "static-id");
    }

    #[test]
    fn node_id_hashing() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(make_node_id("n1"));
        set.insert(make_node_id("n2"));
        set.insert(make_node_id("n1")); // duplicate
        assert_eq!(set.len(), 2);
    }
}
