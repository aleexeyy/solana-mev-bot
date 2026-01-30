use std::{cmp::Ordering, collections::HashMap, time::Instant};

use anyhow::Result;
use solana_sdk::pubkey::Pubkey;

use crate::graph::market_graph::{MarketGraph, PoolId, TokenId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CycleKey {
    pub left: Pubkey,
    pub right: Pubkey,
    pub len: u8,
}

#[derive(Debug, Default, Clone)]
pub struct ArbitrageCycles {
    pub all_cycles: HashMap<CycleKey, Vec<Vec<PoolId>>>,
}

impl ArbitrageCycles {
    pub fn new() -> Self {
        Self {
            all_cycles: HashMap::new(),
        }
    }

    pub fn build(&mut self, graph: &MarketGraph, max_depth: usize) -> Result<()> {
        let start = Instant::now();

        let start_node: TokenId = graph.wsol_node;
        let mut visited_edges: Vec<bool> = vec![false; graph.edges.len()]; // bitmap
        let mut path: Vec<PoolId> = Vec::with_capacity(max_depth);
        let mut cycles: HashMap<CycleKey, Vec<Vec<PoolId>>> = HashMap::new();

        Self::dfs_recursive(
            graph,
            start_node,
            start_node,
            &mut visited_edges,
            &mut path,
            max_depth,
            &mut cycles,
        );

        self.all_cycles = cycles;

        let duration = start.elapsed();
        tracing::info!("Cycles Building Took: {:?}", duration);
        tracing::info!("Number of Keys: {:?}", &self.all_cycles.len());
        Ok(())
    }

    fn dfs_recursive(
        graph: &MarketGraph,
        start_node: TokenId,
        current_node: TokenId,
        visited_edges: &mut Vec<bool>,
        path: &mut Vec<PoolId>,
        max_depth: usize,
        cycles: &mut HashMap<CycleKey, Vec<Vec<PoolId>>>,
    ) {
        if path.len() >= max_depth {
            return;
        }

        if let Some(adjacent) = graph.adjacency.get(&current_node) {
            for &edge_index in adjacent {
                if visited_edges[edge_index.0] {
                    continue;
                }

                let edge = &graph.edges[edge_index.0];
                let other_node = edge.get_other_node(current_node).unwrap();

                visited_edges[edge_index.0] = true;
                path.push(edge_index);

                if other_node == start_node && path.len() >= 2 {
                    let mut canonical = Self::canonicalize(path.as_ref());
                    let path_length: usize = canonical.len();

                    if let Some(pos) = canonical.iter().position(|pool_index| {
                        let edge = &graph.edges[pool_index.0];
                        let node_a = &graph.nodes[edge.node_a.0];
                        let node_b = &graph.nodes[edge.node_b.0];
                        node_a.address == graph.wsol_address || node_b.address == graph.wsol_address
                    }) {
                        canonical.rotate_left(pos);
                    }

                    Self::check_cycle_with_graph(graph, &mut canonical);

                    for pool_index in &canonical {
                        let edge = &graph.edges[pool_index.0];
                        let node_a = &graph.nodes[edge.node_a.0];
                        let node_b = &graph.nodes[edge.node_b.0];

                        let a_bytes = node_a.address.to_bytes();
                        let b_bytes = node_b.address.to_bytes();
                        let (left_pub, right_pub) = match a_bytes.cmp(&b_bytes) {
                            Ordering::Less | Ordering::Equal => (node_a.address, node_b.address),
                            Ordering::Greater => (node_b.address, node_a.address),
                        };

                        let key = CycleKey {
                            left: left_pub,
                            right: right_pub,
                            len: path_length as u8,
                        };
                        match cycles.get_mut(&key) {
                            Some(cycle_vec) => cycle_vec.push(canonical.clone()),
                            None => {
                                cycles.insert(key, vec![canonical.clone()]);
                            }
                        }
                    }
                }

                Self::dfs_recursive(
                    graph,
                    start_node,
                    other_node,
                    visited_edges,
                    path,
                    max_depth,
                    cycles,
                );
                path.pop();
                visited_edges[edge_index.0] = false;
            }
        }
    }

    #[inline]
    pub fn canonicalize(cycle: &[PoolId]) -> Vec<PoolId> {
        let n = cycle.len();
        if n == 0 {
            return Vec::new();
        }

        let (min_idx, _) = cycle
            .iter()
            .enumerate()
            .min_by_key(|&(_, edge_idx)| edge_idx.0)
            .unwrap();

        let forward: Vec<PoolId> = (0..n).map(|i| cycle[(min_idx + i) % n]).collect();

        let mut reversed: Vec<PoolId> = cycle.iter().rev().copied().collect();

        let (rev_min_idx, _) = reversed
            .iter()
            .enumerate()
            .min_by_key(|&(_, edge_idx)| edge_idx.0)
            .unwrap();
        reversed.rotate_left(rev_min_idx);

        if forward <= reversed {
            forward
        } else {
            reversed
        }
    }

    #[inline]
    pub fn check_cycle_with_graph(graph: &MarketGraph, cycle: &mut [PoolId]) -> bool {
        let cycle_len = cycle.len();
        let mut need_change = false;
        let mut last_node: TokenId = graph.wsol_node; // WSOL
        let mut problematic_edge_index: usize = cycle_len; // set to unreal index

        for (index, pool) in cycle.iter().enumerate() {
            let edge = &graph.edges[pool.0];
            match edge.get_other_node(last_node) {
                Some(other_node) => last_node = other_node,
                None => {
                    need_change = true;
                    problematic_edge_index = index;
                    break;
                }
            }
        }
        if !need_change && last_node != graph.wsol_node {
            problematic_edge_index = cycle_len - 1;
            need_change = true;
        }

        if need_change {
            if problematic_edge_index < cycle_len && problematic_edge_index > 0 {
                cycle.rotate_left(1);
            } else if problematic_edge_index == 0 {
                cycle.rotate_left(cycle_len - 1);
            }
        }
        need_change
    }
}
