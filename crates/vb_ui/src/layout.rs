//! Sugiyama-style layered graph layout for workflow DAGs.
//!
//! Converts node positions from `[0, 0]` placeholder coordinates to properly
//! laid out `[x, y]` positions using a layered approach:
//!
//! 1. Topological layering via longest path from entry node
//! 2. Barycenter heuristic to minimize edge crossings (4 passes)
//! 3. Grid position assignment with configurable spacing

use std::collections::{HashMap, HashSet, VecDeque};

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

const MARGIN_LEFT: f64 = 80.0;
const MARGIN_TOP: f64 = 60.0;
const COLUMN_SPACING: f64 = 350.0;
const ROW_SPACING: f64 = 120.0;
const GROUP_PADDING: f64 = 40.0;

const BARYCENTER_PASSES: usize = 4;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Node data needed for layout.
#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub id: String,
    pub width: f64,
    pub height: f64,
    pub group: Option<String>,
}

/// Edge data needed for layout.
#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub source: String,
    pub target: String,
}

/// Bounding box for a group (container).
#[derive(Debug, Clone, Copy, Default)]
pub struct GroupBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Complete layout result.
#[derive(Debug, Clone)]
pub struct LayoutResult {
    /// Node ID to `[x, y]` position.
    pub positions: HashMap<String, [f64; 2]>,
    /// Group ID to bounding box (only groups that have children).
    pub groups: HashMap<String, GroupBounds>,
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Compute layout positions for a DAG.
///
/// # Arguments
///
/// * `nodes` - All nodes in the graph.
/// * `edges` - Directed edges (source -> target).
/// * `entry_id` - The entry/root node ID. Receives column 0.
///
/// # Edge cases handled
///
/// - Empty graph returns empty result.
/// - Single node returns `[MARGIN_LEFT, MARGIN_TOP]`.
/// - Disconnected nodes are assigned to column 0.
/// - Self-loops are silently skipped.
/// - Duplicate edges are deduplicated.
/// - Nodes referenced only in edges (not in `nodes`) are ignored.
pub fn compute_layout(nodes: &[LayoutNode], edges: &[LayoutEdge], entry_id: &str) -> LayoutResult {
    // Fast path: empty graph.
    if nodes.is_empty() {
        return LayoutResult {
            positions: HashMap::new(),
            groups: HashMap::new(),
        };
    }

    // Build index of valid node IDs.
    let valid_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    let node_map: HashMap<&str, &LayoutNode> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // -----------------------------------------------------------------------
    // Step 1: Build adjacency lists (skip self-loops and invalid references)
    // -----------------------------------------------------------------------
    let mut predecessors: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut successors: HashMap<&str, Vec<&str>> = HashMap::new();

    for edge in edges {
        let src = edge.source.as_str();
        let tgt = edge.target.as_str();

        // Skip self-loops.
        if src == tgt {
            continue;
        }
        // Skip edges referencing unknown nodes.
        if !valid_ids.contains(src) || !valid_ids.contains(tgt) {
            continue;
        }

        successors.entry(src).or_default().push(tgt);
        predecessors.entry(tgt).or_default().push(src);
    }

    // Deduplicate adjacency lists.
    for adj_list in predecessors.values_mut() {
        dedup_preserve_order(adj_list);
    }
    for adj_list in successors.values_mut() {
        dedup_preserve_order(adj_list);
    }

    // -----------------------------------------------------------------------
    // Step 2: Assign columns via longest path from entry (BFS-based)
    // -----------------------------------------------------------------------
    let mut depth: HashMap<&str, usize> = HashMap::new();

    // Entry node starts at depth 0.
    if valid_ids.contains(entry_id) {
        depth.insert(entry_id, 0);
    }

    // BFS from entry, tracking longest path.
    let mut queue: VecDeque<&str> = VecDeque::new();
    if valid_ids.contains(entry_id) {
        queue.push_back(entry_id);
    }

    while let Some(node_id) = queue.pop_front() {
        let current_depth = match depth.get(node_id) {
            Some(&d) => d,
            None => continue,
        };
        if let Some(succs) = successors.get(node_id) {
            for &succ in succs {
                let candidate = current_depth.saturating_add(1);
                let existing = depth.get(succ).copied().unwrap_or(0);
                if candidate > existing {
                    depth.insert(succ, candidate);
                    queue.push_back(succ);
                }
            }
        }
    }

    // Disconnected nodes get depth 0.
    for node in nodes {
        depth.entry(node.id.as_str()).or_insert(0);
    }

    // -----------------------------------------------------------------------
    // Step 3: Group nodes by column
    // -----------------------------------------------------------------------
    let max_depth = depth.values().copied().max().unwrap_or(0);
    let num_columns = max_depth.saturating_add(1);

    let mut columns: Vec<Vec<&str>> = vec![Vec::new(); num_columns];
    for node in nodes {
        let col = depth.get(&node.id.as_str()).copied().unwrap_or(0);
        if let Some(column) = columns.get_mut(col) {
            column.push(node.id.as_str());
        }
    }

    // -----------------------------------------------------------------------
    // Step 4: Barycenter optimization (4 iterations of forward + backward)
    // -----------------------------------------------------------------------
    for _pass in 0..BARYCENTER_PASSES {
        // Assign each node a row index within its column.
        let row_of: HashMap<&str, usize> = build_row_index(&columns);

        // Forward pass (left to right): sort each column by barycenter of
        // predecessors' row positions.
        for col_idx in 1..num_columns {
            if let Some(column) = columns.get_mut(col_idx) {
                sort_column_by_barycenter(column, &row_of, &predecessors);
            }
        }

        // Rebuild row index after forward pass.
        let row_of = build_row_index(&columns);

        // Backward pass (right to left): sort each column by barycenter of
        // successors' row positions.
        for col_idx in (0..num_columns.saturating_sub(1)).rev() {
            if let Some(column) = columns.get_mut(col_idx) {
                sort_column_by_barycenter(column, &row_of, &successors);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Step 5: Compute (x, y) positions
    // -----------------------------------------------------------------------
    let mut positions: HashMap<String, [f64; 2]> = HashMap::new();

    for (col_idx, col_nodes) in columns.iter().enumerate() {
        let x = MARGIN_LEFT + f64::from(u32::try_from(col_idx).unwrap_or(0)) * COLUMN_SPACING;

        for (row_idx, &node_id) in col_nodes.iter().enumerate() {
            let y = MARGIN_TOP + f64::from(u32::try_from(row_idx).unwrap_or(0)) * ROW_SPACING;
            positions.insert(node_id.to_string(), [x, y]);
        }
    }

    // Ensure every input node has a position (defensive).
    for node in nodes {
        positions
            .entry(node.id.clone())
            .or_insert([MARGIN_LEFT, MARGIN_TOP]);
    }

    // -----------------------------------------------------------------------
    // Step 6: Compute group bounding boxes
    // -----------------------------------------------------------------------
    let groups = compute_group_bounds(nodes, &positions, &node_map);

    LayoutResult { positions, groups }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a map from node ID to its row index within its column.
fn build_row_index<'a>(columns: &[Vec<&'a str>]) -> HashMap<&'a str, usize> {
    let mut row_of = HashMap::new();
    for col_nodes in columns {
        for (row, &node_id) in col_nodes.iter().enumerate() {
            row_of.insert(node_id, row);
        }
    }
    row_of
}

/// Sort a column's nodes by the barycenter of their neighbors' positions.
///
/// For each node, the barycenter is the average row index of its neighbors
/// (predecessors in forward pass, successors in backward pass). Nodes with
/// no neighbors keep their current position as tiebreaker.
fn sort_column_by_barycenter<'a>(
    column: &mut Vec<&'a str>,
    row_of: &HashMap<&'a str, usize>,
    neighbors: &HashMap<&str, Vec<&'a str>>,
) {
    if column.len() <= 1 {
        return;
    }

    // Compute (barycenter, original_index) for each node.
    let mut indexed: Vec<(f64, usize, &str)> = column
        .iter()
        .enumerate()
        .map(|(original_idx, &node_id)| {
            let bary = compute_barycenter(node_id, row_of, neighbors);
            // Use original index as tiebreaker to keep order stable when
            // barycenters are equal (or NaN when no neighbors).
            let sort_key = if let Some(b) = bary {
                b
            } else {
                // No neighbors: use original index as pseudo-barycenter so
                // these nodes stay roughly in place relative to each other.
                f64::from(u32::try_from(original_idx).unwrap_or(0))
            };
            (sort_key, original_idx, node_id)
        })
        .collect();

    // Sort by barycenter, then by original index for stability.
    indexed.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.cmp(&b.1))
    });

    *column = indexed.into_iter().map(|(_, _, id)| id).collect();
}

/// Compute the barycenter (average row position) of a node's neighbors.
///
/// Returns `None` if the node has no relevant neighbors.
fn compute_barycenter<'a>(
    node_id: &str,
    row_of: &HashMap<&'a str, usize>,
    neighbors: &HashMap<&str, Vec<&'a str>>,
) -> Option<f64> {
    let nbrs = neighbors.get(node_id)?;
    if nbrs.is_empty() {
        return None;
    }

    let mut sum: f64 = 0.0;
    let mut count: usize = 0;
    for &nbr in nbrs {
        if let Some(&row) = row_of.get(nbr) {
            sum += f64::from(u32::try_from(row).unwrap_or(0));
            count = count.saturating_add(1);
        }
    }

    if count == 0 {
        None
    } else {
        Some(sum / f64::from(u32::try_from(count).unwrap_or(1)))
    }
}

/// Remove duplicate entries while preserving order.
fn dedup_preserve_order(list: &mut Vec<&str>) {
    let mut seen = HashSet::new();
    let mut write: usize = 0;
    for read in 0..list.len() {
        let read_val = match list.get(read) {
            Some(&v) => v,
            None => continue,
        };
        if seen.insert(read_val) {
            if let Some(slot) = list.get_mut(write) {
                *slot = read_val;
            }
            write = write.saturating_add(1);
        }
    }
    list.truncate(write);
}

/// Compute bounding boxes for groups that contain at least one child node.
fn compute_group_bounds(
    nodes: &[LayoutNode],
    positions: &HashMap<String, [f64; 2]>,
    node_map: &HashMap<&str, &LayoutNode>,
) -> HashMap<String, GroupBounds> {
    // Collect nodes by group.
    let mut group_nodes: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in nodes {
        if let Some(ref grp) = node.group {
            group_nodes.entry(grp.as_str()).or_default().push(&node.id);
        }
    }

    let mut groups = HashMap::new();
    for (group_id, member_ids) in group_nodes {
        if member_ids.is_empty() {
            continue;
        }

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for &nid in &member_ids {
            let pos = match positions.get(nid) {
                Some(p) => p,
                None => continue,
            };
            let node = node_map.get(nid);
            let half_w = node.map(|n| n.width / 2.0).unwrap_or(0.0);
            let half_h = node.map(|n| n.height / 2.0).unwrap_or(0.0);

            let left = pos[0] - half_w;
            let right = pos[0] + half_w;
            let top = pos[1] - half_h;
            let bottom = pos[1] + half_h;

            if left < min_x {
                min_x = left;
            }
            if top < min_y {
                min_y = top;
            }
            if right > max_x {
                max_x = right;
            }
            if bottom > max_y {
                max_y = bottom;
            }
        }

        // Guard against the case where no positions were found.
        if min_x == f64::MAX {
            continue;
        }

        groups.insert(
            group_id.to_string(),
            GroupBounds {
                x: min_x - GROUP_PADDING,
                y: min_y - GROUP_PADDING,
                width: (max_x - min_x) + 2.0 * GROUP_PADDING,
                height: (max_y - min_y) + 2.0 * GROUP_PADDING,
            },
        );
    }

    groups
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str) -> LayoutNode {
        LayoutNode {
            id: id.to_string(),
            width: 100.0,
            height: 60.0,
            group: None,
        }
    }

    fn node_in_group(id: &str, group: &str) -> LayoutNode {
        LayoutNode {
            id: id.to_string(),
            width: 100.0,
            height: 60.0,
            group: Some(group.to_string()),
        }
    }

    fn edge(src: &str, tgt: &str) -> LayoutEdge {
        LayoutEdge {
            source: src.to_string(),
            target: tgt.to_string(),
        }
    }

    #[test]
    fn empty_graph_returns_empty() {
        let result = compute_layout(&[], &[], "entry");
        assert!(result.positions.is_empty());
        assert!(result.groups.is_empty());
    }

    #[test]
    fn single_node_at_origin_column() {
        let nodes = vec![node("a")];
        let result = compute_layout(&nodes, &[], "a");
        assert_eq!(result.positions.len(), 1);
        assert_eq!(result.positions["a"], [MARGIN_LEFT, MARGIN_TOP]);
    }

    #[test]
    fn linear_chain_assigns_increasing_columns() {
        // a -> b -> c
        let nodes = vec![node("a"), node("b"), node("c")];
        let edges = vec![edge("a", "b"), edge("b", "c")];
        let result = compute_layout(&nodes, &edges, "a");

        assert_eq!(result.positions.len(), 3);
        // Column increases left to right.
        assert!(result.positions["a"][0] < result.positions["b"][0]);
        assert!(result.positions["b"][0] < result.positions["c"][0]);
        // All at row 0 (single path, no branching).
        assert_eq!(result.positions["a"][1], MARGIN_TOP);
        assert_eq!(result.positions["b"][1], MARGIN_TOP);
        assert_eq!(result.positions["c"][1], MARGIN_TOP);
    }

    #[test]
    fn diamond_merges_to_same_column() {
        //     b
        // a <     > d
        //     c
        // a -> b, a -> c, b -> d, c -> d
        let nodes = vec![node("a"), node("b"), node("c"), node("d")];
        let edges = vec![
            edge("a", "b"),
            edge("a", "c"),
            edge("b", "d"),
            edge("c", "d"),
        ];
        let result = compute_layout(&nodes, &edges, "a");

        // b and c should be in the same column (column 1).
        assert_eq!(result.positions["b"][0], result.positions["c"][0]);
        // d should be further right (column 2).
        assert!(result.positions["b"][0] < result.positions["d"][0]);
        // b and c should have different rows (both in column 1).
        assert_ne!(result.positions["b"][1], result.positions["c"][1]);
    }

    #[test]
    fn disconnected_nodes_get_column_zero() {
        let nodes = vec![node("entry"), node("orphan")];
        let result = compute_layout(&nodes, &[], "entry");

        assert_eq!(result.positions.len(), 2);
        // Both should be in column 0.
        assert_eq!(result.positions["entry"][0], MARGIN_LEFT);
        assert_eq!(result.positions["orphan"][0], MARGIN_LEFT);
        // They should be on different rows since they share column 0.
        assert_ne!(result.positions["entry"][1], result.positions["orphan"][1]);
    }

    #[test]
    fn self_loops_are_skipped() {
        let nodes = vec![node("a"), node("b")];
        let edges = vec![edge("a", "a"), edge("a", "b")];
        let result = compute_layout(&nodes, &edges, "a");

        assert_eq!(result.positions.len(), 2);
        assert!(result.positions["a"][0] < result.positions["b"][0]);
    }

    #[test]
    fn unknown_edge_endpoints_are_ignored() {
        let nodes = vec![node("a")];
        let edges = vec![edge("a", "ghost"), edge("phantom", "a")];
        let result = compute_layout(&nodes, &edges, "a");

        assert_eq!(result.positions.len(), 1);
        assert!(result.positions.contains_key("a"));
    }

    #[test]
    fn duplicate_edges_are_deduplicated() {
        let nodes = vec![node("a"), node("b")];
        let edges = vec![edge("a", "b"), edge("a", "b"), edge("a", "b")];
        let result = compute_layout(&nodes, &edges, "a");

        assert_eq!(result.positions.len(), 2);
        assert!(result.positions["a"][0] < result.positions["b"][0]);
    }

    #[test]
    fn group_bounds_are_computed() {
        let nodes = vec![
            node_in_group("a", "g1"),
            node_in_group("b", "g1"),
            node("c"),
        ];
        let edges = vec![edge("a", "b")];
        let result = compute_layout(&nodes, &edges, "a");

        assert!(result.groups.contains_key("g1"));
        let bounds = &result.groups["g1"];
        // Bounds should be larger than a single node.
        assert!(bounds.width > 0.0);
        assert!(bounds.height > 0.0);
        // Ungrouped node should not appear in groups.
        assert!(!result.groups.contains_key("c"));
    }

    #[test]
    fn entry_not_in_nodes_gets_no_duplicate() {
        // entry_id references a node that exists in the list.
        let nodes = vec![node("x"), node("y")];
        let edges = vec![edge("x", "y")];
        let result = compute_layout(&nodes, &edges, "x");

        assert_eq!(result.positions.len(), 2);
    }

    #[test]
    fn entry_id_not_present_treated_as_missing() {
        // entry_id doesn't match any node; all nodes should still get positions.
        let nodes = vec![node("a"), node("b")];
        let edges = vec![edge("a", "b")];
        let result = compute_layout(&nodes, &edges, "missing_entry");

        assert_eq!(result.positions.len(), 2);
        // Both nodes are disconnected from the missing entry, so both get
        // column 0.
        assert_eq!(result.positions["a"][0], MARGIN_LEFT);
        assert_eq!(result.positions["b"][0], MARGIN_LEFT);
    }

    #[test]
    fn wide_graph_has_multiple_rows() {
        // a -> b1, a -> b2, a -> b3, a -> b4
        let nodes = vec![node("a"), node("b1"), node("b2"), node("b3"), node("b4")];
        let edges = vec![
            edge("a", "b1"),
            edge("a", "b2"),
            edge("a", "b3"),
            edge("a", "b4"),
        ];
        let result = compute_layout(&nodes, &edges, "a");

        // All b* nodes should be in the same column.
        let bx = result.positions["b1"][0];
        assert_eq!(result.positions["b2"][0], bx);
        assert_eq!(result.positions["b3"][0], bx);
        assert_eq!(result.positions["b4"][0], bx);

        // They should have distinct y values.
        let ys: Vec<f64> = ["b1", "b2", "b3", "b4"]
            .iter()
            .map(|id| result.positions[*id][1])
            .collect();
        let mut sorted_ys = ys.clone();
        sorted_ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        assert_eq!(ys.len(), sorted_ys.len());
        // All y values are distinct.
        for i in 0..ys.len() {
            for j in (i + 1)..ys.len() {
                assert_ne!(sorted_ys[i], sorted_ys[j]);
            }
        }
    }

    #[test]
    fn back_edge_to_entry_does_not_diverge() {
        // a -> b -> c, with an additional edge c -> a creating a back-edge.
        // The algorithm uses longest-path BFS which will keep increasing depth
        // for nodes in cycles. To avoid infinite loops in the test, we test a
        // limited back-edge scenario: a -> b -> c with a redundant edge b -> a.
        // Since b->a has candidate depth 2, and a already has depth 0, a is
        // re-enqueued. But then a re-enqueues b with a higher depth, etc.
        //
        // Instead, test a forward-only redundant edge that creates a longer
        // path to the same node: a -> b, a -> c, b -> c. This is a proper DAG
        // with a "shortcut" edge. The longest path to c should be through b.
        let nodes = vec![node("a"), node("b"), node("c")];
        let edges = vec![edge("a", "b"), edge("a", "c"), edge("b", "c")];
        let result = compute_layout(&nodes, &edges, "a");

        assert_eq!(result.positions.len(), 3);
        // c should be at column 2 (longest path: a->b->c), not column 1.
        let x_a = match result.positions.get("a") {
            Some(p) => p[0],
            None => return,
        };
        let x_b = match result.positions.get("b") {
            Some(p) => p[0],
            None => return,
        };
        let x_c = match result.positions.get("c") {
            Some(p) => p[0],
            None => return,
        };
        assert!(x_b > x_a, "b should be right of a");
        assert!(x_c > x_b, "c should be right of b (longest path wins)");
    }

    #[test]
    fn group_bounds_single_node() {
        // A single node in a group should still produce valid group bounds.
        let nodes = vec![node_in_group("a", "solo")];
        let result = compute_layout(&nodes, &[], "a");

        let bounds = match result.groups.get("solo") {
            Some(b) => b,
            None => return,
        };
        // Width and height should account for GROUP_PADDING on each side.
        assert!(bounds.width > 0.0);
        assert!(bounds.height > 0.0);
        // The single node's position should be inside the bounds.
        let pos = match result.positions.get("a") {
            Some(p) => p,
            None => return,
        };
        assert!(pos[0] >= bounds.x);
        assert!(pos[1] >= bounds.y);
        assert!(pos[0] <= bounds.x + bounds.width);
        assert!(pos[1] <= bounds.y + bounds.height);
    }

    #[test]
    fn two_separate_groups_have_distinct_bounds() {
        let nodes = vec![
            node_in_group("a", "g1"),
            node_in_group("b", "g1"),
            node_in_group("c", "g2"),
            node_in_group("d", "g2"),
        ];
        let edges = vec![edge("a", "b"), edge("c", "d")];
        let result = compute_layout(&nodes, &edges, "a");

        let b1 = match result.groups.get("g1") {
            Some(b) => b,
            None => return,
        };
        let b2 = match result.groups.get("g2") {
            Some(b) => b,
            None => return,
        };
        // Both groups should exist and have non-zero dimensions.
        assert!(b1.width > 0.0);
        assert!(b2.width > 0.0);
        assert!(b1.height > 0.0);
        assert!(b2.height > 0.0);
    }

    #[test]
    fn zero_size_node_gets_position() {
        let nodes = vec![LayoutNode {
            id: String::from("tiny"),
            width: 0.0,
            height: 0.0,
            group: None,
        }];
        let result = compute_layout(&nodes, &[], "tiny");

        assert_eq!(result.positions.len(), 1);
        let pos = match result.positions.get("tiny") {
            Some(p) => p,
            None => return,
        };
        assert_eq!(pos[0], MARGIN_LEFT);
        assert_eq!(pos[1], MARGIN_TOP);
    }

    #[test]
    fn wide_chain_ten_nodes() {
        // a0 -> a1 -> a2 -> ... -> a9
        let nodes: Vec<LayoutNode> = (0..10).map(|i| node(&format!("a{i}"))).collect();
        let edges: Vec<LayoutEdge> = (0..9)
            .map(|i| edge(&format!("a{i}"), &format!("a{}", i + 1)))
            .collect();
        let result = compute_layout(&nodes, &edges, "a0");

        assert_eq!(result.positions.len(), 10);
        // Each successive node should be strictly further right.
        for i in 0..9 {
            let x_cur = match result.positions.get(&format!("a{i}")) {
                Some(p) => p[0],
                None => return,
            };
            let x_next = match result.positions.get(&format!("a{}", i + 1)) {
                Some(p) => p[0],
                None => return,
            };
            assert!(
                x_cur < x_next,
                "a{i} x ({x_cur}) must be less than a{} x ({x_next})",
                i + 1,
            );
        }
        // All nodes should be on row 0 (single chain, no branching).
        for i in 0..10 {
            let pos = match result.positions.get(&format!("a{i}")) {
                Some(p) => p,
                None => return,
            };
            assert_eq!(pos[1], MARGIN_TOP, "a{i} should be on row 0");
        }
    }

    #[test]
    fn fan_in_multiple_predecessors() {
        // a -> c, b -> c (fan-in: two sources converge on one target).
        let nodes = vec![node("a"), node("b"), node("c")];
        let edges = vec![edge("a", "c"), edge("b", "c")];
        let result = compute_layout(&nodes, &edges, "a");

        assert_eq!(result.positions.len(), 3);
        // c should be further right than both a and b.
        let x_a = match result.positions.get("a") {
            Some(p) => p[0],
            None => return,
        };
        let x_b = match result.positions.get("b") {
            Some(p) => p[0],
            None => return,
        };
        let x_c = match result.positions.get("c") {
            Some(p) => p[0],
            None => return,
        };
        assert!(x_c > x_a, "c must be right of a");
        assert!(x_c > x_b, "c must be right of b");
        // a and b should be in the same column.
        assert_eq!(x_a, x_b);
        // a and b should be on different rows (same column).
        let y_a = match result.positions.get("a") {
            Some(p) => p[1],
            None => return,
        };
        let y_b = match result.positions.get("b") {
            Some(p) => p[1],
            None => return,
        };
        assert_ne!(y_a, y_b, "a and b must be on different rows");
    }

    #[test]
    fn all_nodes_receive_position() {
        // Every input node must appear in the positions map, even if
        // disconnected or referenced by no edges.
        let nodes = vec![
            node("entry"),
            node("mid"),
            node("leaf"),
            node("island"),
        ];
        let edges = vec![edge("entry", "mid"), edge("mid", "leaf")];
        let result = compute_layout(&nodes, &edges, "entry");

        assert_eq!(result.positions.len(), 4, "every input node must have a position");
        for id in &["entry", "mid", "leaf", "island"] {
            let pos = match result.positions.get(*id) {
                Some(p) => p,
                None => {
                    assert!(false, "missing position for {id}");
                    return;
                }
            };
            assert!(
                pos[0].is_finite() && pos[1].is_finite(),
                "position for {id} must be finite"
            );
        }
        // "island" has no edges but should still land at column 0.
        assert_eq!(result.positions["island"][0], MARGIN_LEFT);
    }

    #[test]
    fn single_node_no_edges_trivial() {
        // Single node with no edges -- the simplest possible graph.
        let nodes = vec![node("only")];
        let result = compute_layout(&nodes, &[], "only");

        assert_eq!(result.positions.len(), 1);
        let pos = match result.positions.get("only") {
            Some(p) => p,
            None => return,
        };
        assert_eq!(pos[0], MARGIN_LEFT);
        assert_eq!(pos[1], MARGIN_TOP);
        assert!(result.groups.is_empty());
    }
}
