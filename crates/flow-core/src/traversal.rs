use crate::doc::*;
use crate::ids::*;
use std::collections::{HashMap, HashSet, VecDeque};

impl FlowGraph {
    /// Returns all edges whose target is the given node.
    pub fn incomers(&self, node_id: &NodeId) -> Vec<&FlowEdgeRecord> {
        self.edges
            .values()
            .filter(|edge| edge.target_node == *node_id)
            .collect()
    }

    /// Returns all edges whose source is the given node.
    pub fn outgoers(&self, node_id: &NodeId) -> Vec<&FlowEdgeRecord> {
        self.edges
            .values()
            .filter(|edge| edge.source_node == *node_id)
            .collect()
    }

    /// Returns all edges connected to the given node (incoming or outgoing).
    pub fn connected_edges(&self, node_id: &NodeId) -> Vec<&FlowEdgeRecord> {
        self.edges
            .values()
            .filter(|edge| edge.source_node == *node_id || edge.target_node == *node_id)
            .collect()
    }

    /// Returns a topological ordering of nodes, or `None` if the graph contains a cycle.
    /// Uses Kahn's algorithm (BFS-based).
    pub fn topological_sort(&self) -> Option<Vec<NodeId>> {
        let mut in_degree: HashMap<&NodeId, usize> = HashMap::new();
        let mut adjacency: HashMap<&NodeId, Vec<&NodeId>> = HashMap::new();

        for node_id in self.nodes.keys() {
            in_degree.entry(node_id).or_insert(0);
            adjacency.entry(node_id).or_default();
        }

        for edge in self.edges.values() {
            if self.nodes.contains_key(&edge.source_node)
                && self.nodes.contains_key(&edge.target_node)
            {
                if let Some(count) = in_degree.get_mut(&edge.target_node) {
                    *count += 1;
                }
                if let Some(neighbors) = adjacency.get_mut(&edge.source_node) {
                    neighbors.push(&edge.target_node);
                }
            }
        }

        let mut queue: VecDeque<&NodeId> = VecDeque::new();
        for (node_id, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(node_id);
            }
        }

        let mut sorted: Vec<NodeId> = Vec::with_capacity(self.nodes.len());

        while let Some(node_id) = queue.pop_front() {
            sorted.push((*node_id).clone());

            if let Some(neighbors) = adjacency.get(node_id) {
                for &neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        if sorted.len() == self.nodes.len() {
            Some(sorted)
        } else {
            None
        }
    }

    /// Finds all simple cycles in the graph using Johnson's algorithm approach.
    /// Returns a list of cycles, where each cycle is a list of node IDs.
    pub fn find_cycles(&self) -> Vec<Vec<NodeId>> {
        let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for node_id in self.nodes.keys() {
            adj.entry(node_id.clone()).or_default();
        }
        for edge in self.edges.values() {
            if self.nodes.contains_key(&edge.source_node)
                && self.nodes.contains_key(&edge.target_node)
            {
                adj.entry(edge.source_node.clone())
                    .or_default()
                    .push(edge.target_node.clone());
            }
        }

        let mut cycles: Vec<Vec<NodeId>> = Vec::new();
        let mut visited: HashSet<NodeId> = HashSet::new();

        let node_ids: Vec<NodeId> = self.nodes.keys().cloned().collect();

        for start in &node_ids {
            if visited.contains(start) {
                continue;
            }

            let mut path: Vec<NodeId> = Vec::new();
            let mut path_set: HashSet<NodeId> = HashSet::new();
            let mut stack: Vec<(NodeId, usize)> = Vec::new();

            stack.push((start.clone(), 0));
            path.push(start.clone());
            path_set.insert(start.clone());

            while let Some((current, mut neighbor_idx)) = stack.pop() {
                let neighbors = adj.get(&current).cloned().unwrap_or_default();

                let mut found_next = false;
                while neighbor_idx < neighbors.len() {
                    let neighbor = neighbors[neighbor_idx].clone();
                    neighbor_idx += 1;

                    if neighbor == *start && path.len() > 1 {
                        cycles.push(path.clone());
                    } else if !path_set.contains(&neighbor) && !visited.contains(&neighbor) {
                        stack.push((current.clone(), neighbor_idx));
                        path.push(neighbor.clone());
                        path_set.insert(neighbor.clone());
                        stack.push((neighbor, 0));
                        found_next = true;
                        break;
                    }
                }

                if !found_next {
                    if let Some(popped) = path.pop() {
                        path_set.remove(&popped);
                    }
                }
            }

            visited.insert(start.clone());
        }

        cycles
    }

    /// Returns all node IDs reachable from `start` via outgoing edges (BFS).
    pub fn reachable_from(&self, start: &NodeId) -> Vec<NodeId> {
        if !self.nodes.contains_key(start) {
            return Vec::new();
        }

        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut queue: VecDeque<NodeId> = VecDeque::new();
        let mut result: Vec<NodeId> = Vec::new();

        queue.push_back(start.clone());
        visited.insert(start.clone());

        while let Some(current) = queue.pop_front() {
            result.push(current.clone());

            for edge in self.edges.values() {
                if edge.source_node == current
                    && !visited.contains(&edge.target_node)
                    && self.nodes.contains_key(&edge.target_node)
                {
                    visited.insert(edge.target_node.clone());
                    queue.push_back(edge.target_node.clone());
                }
            }
        }

        result
    }
}
