use petgraph::graph::Graph;
use std::collections::HashMap;

pub mod layers;
// Link prediction using KAN (Knowledge-aware Attention Network).

/// Converts an adjacency list into a `petgraph::Graph<usize, ()>`.
pub fn adjacency_list_to_graph(adj_list: &HashMap<usize, Vec<usize>>) -> Graph<usize, ()> {
	let mut graph = Graph::<usize, ()>::new();
	let mut node_map = HashMap::new();

	// Add nodes to the graph
	for &node in adj_list.keys() {
		let index = graph.add_node(node);
		node_map.insert(node, index);
	}

	// Add edges to the graph
	for (&src, neighbors) in adj_list.iter() {
		if let Some(&src_index) = node_map.get(&src) {
			for &neighbor in neighbors {
				if let Some(&tgt_index) = node_map.get(&neighbor) {
					graph.add_edge(src_index, tgt_index, ());
				}
			}
		}
	}

	graph
}
