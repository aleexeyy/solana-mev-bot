use std::{
    collections::{HashMap, HashSet},
    fs::{read_dir, read_to_string},
    str::FromStr,
    time::Instant,
};

use solana_sdk::pubkey::Pubkey;
use tracing::{info, warn};

use crate::bootstrap::pool_schema::{
    DexType, PoolInfo, PoolType, PoolUpdate, StoredPools, TokenInfo,
};
use anyhow::{Result, anyhow};
use ethnum::U256;

#[allow(dead_code)]
#[derive(Debug)]
pub struct Node {
    address: Pubkey,
    decimals: u8,
    name: String,
    pub symbol: String,
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Edge {
    //static fields
    pub address: Pubkey,
    fee_rate: u32,
    pool_type: PoolType,
    dex: DexType,
    tick_spacing: u64,
    token_vault_lowest: Pubkey,  // lowest index
    token_vault_highest: Pubkey, // highest index
    config: Pubkey,
    node_lowest: usize,
    node_highest: usize,
    decimals_lowest: u8,
    decimals_highest: u8,
    pub reversed: bool,

    //dynamic fields
    pub sqrt_price: Option<u128>,
    liquidity: Option<u128>,
    current_tick_index: Option<i32>,
}

impl Edge {
    pub fn get_log_exchange_rate(&self, direct: bool) -> f64 {
        self.get_exchange_rate(direct).log10()
    }

    pub fn get_exchange_rate(&self, direct: bool) -> f64 {
        let decimals_diff: i32 = if self.reversed {
            self.decimals_highest as i32 - self.decimals_lowest as i32
        } else {
            self.decimals_lowest as i32 - self.decimals_highest as i32
        };
        let denominator = 10f64.powi(decimals_diff);

        let scaled_price: U256 = U256::from(self.sqrt_price.unwrap());
        let squared: U256 = scaled_price * scaled_price;

        let high: U256 = squared >> 128;
        let low: U256 = squared & U256::from(u128::MAX);
        let price_f64 = high.as_u128() as f64 * 2f64.powi(64) + low.as_u128() as f64;

        let price_f64 = price_f64 / 2f64.powi(128);

        let exchange_rate = price_f64 * denominator;

        if self.reversed == direct {
            1.0 / exchange_rate
        } else {
            exchange_rate
        }
    }

    fn get_other_node(&self, this_token: usize) -> Option<usize> {
        if this_token == self.node_lowest {
            Some(self.node_highest)
        } else if this_token == self.node_highest {
            Some(self.node_lowest)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    fn get_swap_direction(&self, token_in: usize) -> Option<bool> {
        if self.node_lowest == token_in {
            return Some(!self.reversed);
        } else if self.node_highest == token_in {
            return Some(self.reversed);
        }

        None
    }
}

#[derive(Debug, Default)]
pub struct Graph {
    wsol_address: Pubkey,
    wsol_node: usize,

    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,

    address_to_node: HashMap<Pubkey, usize>,
    address_to_edge: HashMap<Pubkey, usize>,
    adjacency: HashMap<usize, HashSet<usize>>, // adjacent pools to the token

    pub all_cycles: HashSet<Vec<usize>>,
    // nodes_to_edges: HashMap<(usize, usize), HashSet<usize>>,
}

impl Graph {
    fn default() -> Self {
        Graph {
            wsol_address: Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
            wsol_node: usize::MAX,

            nodes: vec![],
            edges: vec![],

            address_to_node: HashMap::new(),
            address_to_edge: HashMap::new(),
            adjacency: HashMap::new(),

            all_cycles: HashSet::new(),
            // nodes_to_edges: HashMap::new(),
        }
    }
}

impl Graph {
    fn insert_node(&mut self, token: TokenInfo) -> Result<usize> {
        let token_address = Pubkey::from_str(&token.address.unwrap())?;

        if let Some(&existing_index) = self.address_to_node.get(&token_address) {
            return Ok(existing_index);
        }

        let node = Node {
            address: token_address,
            decimals: token.decimals.unwrap(),
            name: token.name.unwrap_or("Empty Name".to_string()),
            symbol: token.symbol.unwrap_or("Empty Symbol".to_string()),
        };
        let index = self.nodes.len();

        if token_address == self.wsol_address {
            self.wsol_node = index;
        }

        self.nodes.push(node);
        self.address_to_node.insert(token_address, index);
        self.adjacency.insert(index, HashSet::new());

        Ok(index)
    }

    fn insert_edge(
        &mut self,
        pool: PoolInfo,
        node0_index: usize,
        node1_index: usize,
    ) -> Result<usize> {
        let (token_vault_lowest, token_vault_highest, idx_lowest, idx_highest, reversed) =
            if node0_index < node1_index {
                (
                    pool.token_vault_a.unwrap(),
                    pool.token_vault_b.unwrap(),
                    node0_index,
                    node1_index,
                    false,
                )
            } else {
                (
                    pool.token_vault_b.unwrap(),
                    pool.token_vault_a.unwrap(),
                    node1_index,
                    node0_index,
                    true,
                )
            };
        let address = Pubkey::from_str(&pool.address.unwrap())?;
        let edge = Edge {
            address,
            fee_rate: pool.fee_rate.unwrap(),
            pool_type: pool.pool_type.unwrap(),
            dex: pool.dex.unwrap(),
            tick_spacing: pool.tick_spacing.unwrap(),
            token_vault_lowest: Pubkey::from_str(&token_vault_lowest)?,
            token_vault_highest: Pubkey::from_str(&token_vault_highest)?,
            config: Pubkey::from_str(&pool.config.unwrap())?,
            node_lowest: idx_lowest,
            node_highest: idx_highest,
            decimals_lowest: self.nodes[idx_lowest].decimals,
            decimals_highest: self.nodes[idx_highest].decimals,
            reversed,
            sqrt_price: None,
            liquidity: None,
            current_tick_index: None,
        };

        let index = self.edges.len();
        self.edges.push(edge);
        self.address_to_edge.insert(address, index);

        self.adjacency.get_mut(&idx_lowest).unwrap().insert(index);
        self.adjacency.get_mut(&idx_highest).unwrap().insert(index);

        Ok(index)
    }

    fn insert_pool(&mut self, mut pool: PoolInfo) -> Result<()> {
        let node0_index = self.insert_node(pool.token_a.take().unwrap())?;
        let node1_index = self.insert_node(pool.token_b.take().unwrap())?;

        self.insert_edge(pool, node0_index, node1_index)?;

        Ok(())
    }

    pub fn update_edge(&mut self, address: &Pubkey, data: PoolUpdate) -> Result<()> {
        if let Some(edge_index) = self.address_to_edge.get(address)
            && let Some(edge) = self.edges.get_mut(*edge_index)
        {
            edge.liquidity = Some(data.new_liquidity);
            edge.sqrt_price = Some(data.new_sqrt_price);
            edge.current_tick_index = Some(data.new_current_tick_index);
            return Ok(());
        }
        Err(anyhow!("Edge with address {} doesn't exist", address))
    }

    pub fn build_graph(data_folder_path: &str) -> Result<Self> {
        let pool_files = Vec::from_iter(
            read_dir(data_folder_path)?
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().and_then(|ext| ext.to_str()) == Some("json")),
        );
        let mut graph = Graph::default();
        for pool_path in pool_files {
            let raw_json = read_to_string(pool_path)?;

            let deserialized: StoredPools = serde_json::from_str(&raw_json)?;
            let pools: Vec<PoolInfo> = deserialized.all_pools;

            for pool in pools {
                if let Err(e) = graph.insert_pool(pool) {
                    warn!("Failed to insert the pool: {:?}", e);
                }
            }
        }

        info!("Amount of Edges in the Graph: {:?}", graph.edges.len());
        info!("Amount of Nodes in the Graph: {:?}", graph.nodes.len());
        Ok(graph)
    }

    // pub fn find_arbitrage_cycles(&self) -> Result<()> {
    //     for cycle in &self.all_cycles {
    //         // Forward direction
    //         let forward_log_sum: f64 = cycle
    //             .iter()
    //             .map(|&edge_index| self.edges[edge_index].get_log_exchange_rate(true))
    //             .sum();

    //         // Reverse direction
    //         let backward_log_sum: f64 = cycle
    //             .iter()
    //             .rev()
    //             .map(|&edge_index| self.edges[edge_index].get_log_exchange_rate(false))
    //             .sum();

    //         // Check for arbitrage
    //         if forward_log_sum > 0.0 {
    //             println!("Arbitrage opportunity (forward): {:?} | with sum: {:?}", cycle, forward_log_sum);
    //         }
    //         if backward_log_sum > 0.0 {
    //             println!("Arbitrage opportunity (backward): {:?} | with sum: {:?}", cycle, backward_log_sum);
    //         }
    //     }

    //     Ok(())
    // }

    pub fn build_cycles(&mut self, max_depth: usize) -> Result<()> {
        let start = Instant::now();

        let start_node = self.wsol_node;
        let mut visited_edges: Vec<bool> = vec![false; self.edges.len()]; // bitmap
        let mut path: Vec<usize> = Vec::with_capacity(max_depth);
        let mut cycles: HashSet<Vec<usize>> = HashSet::new();

        self.dfs_recursive(
            start_node,
            start_node,
            &mut visited_edges,
            &mut path,
            max_depth,
            &mut cycles,
        );

        let mut all_cycles: HashSet<Vec<usize>> = HashSet::new();
        let mut wrong_cycle_counter: usize = 0;

        for mut cycle in cycles {
            let need_change = self.check_cycle(cycle.as_mut());

            all_cycles.insert(cycle);
            if need_change {
                wrong_cycle_counter += 1;
            }
        }

        info!("Number of Cycles: {:?}", &all_cycles.len());
        info!("Number of Wrong Cycles: {:?}", wrong_cycle_counter);

        // wrong_cycle_counter = 0;
        // for (index, mut cycle) in all_cycles.into_iter().enumerate() {
        //     let need_change = self.check_cycle(cycle.as_mut());
        //     // all_cycles.insert(cycle);
        //     if need_change {
        //         wrong_cycle_counter += 1;
        //         println!("Cycle {:?} is Wrong", index);
        //         for pool in cycle {
        //             println!("Pool: {:?}", self.edges[pool].address);
        //         }
        //     }
        // }

        self.all_cycles = all_cycles;

        // info!("Number of Wrong Cycles After Fix: {:?}", wrong_cycle_counter);
        let duration = start.elapsed();
        info!("Cycles Building Took: {:?}", duration);

        Ok(())
    }

    pub fn check_cycle(&self, cycle: &mut [usize]) -> bool {
        let cycle_len = cycle.len();
        let mut need_change = false;
        let mut last_node: usize = self.wsol_node; // WSOL
        let mut problematic_edge_index: usize = cycle_len; // set to unreal index

        for (index, pool) in cycle.iter().enumerate() {
            let edge = &self.edges[*pool];
            match edge.get_other_node(last_node) {
                Some(other_node) => last_node = other_node,
                None => {
                    need_change = true;
                    problematic_edge_index = index;
                    break;
                }
            }
        }
        if !need_change && last_node != 0 {
            problematic_edge_index = cycle_len - 1;
            need_change = true;
            println!("Last Edge Was Wrong");
        }

        if need_change {
            // info!(%problematic_edge_index, "Wrong Edge Index");
            // println!("Cycle before rotation: {:?}", &cycle);
            if problematic_edge_index < cycle_len && problematic_edge_index > 0 {
                cycle.rotate_left(1);
            } else if problematic_edge_index == 0 {
                cycle.rotate_left(cycle_len - 1);
            }
            // println!("Cycle after rotation: {:?}", &cycle);
        }

        need_change
    }

    fn dfs_recursive(
        &self,
        start_node: usize,
        current_node: usize,
        visited_edges: &mut Vec<bool>,
        path: &mut Vec<usize>,
        max_depth: usize,
        cycles: &mut HashSet<Vec<usize>>,
    ) {
        if path.len() >= max_depth {
            return;
        }

        for &edge_index in &self.adjacency[&current_node] {
            if visited_edges[edge_index] {
                continue;
            }

            let edge = &self.edges[edge_index];
            let other_node = edge.get_other_node(current_node).unwrap();

            visited_edges[edge_index] = true;

            path.push(edge_index);

            if other_node == start_node && path.len() >= 2 {
                let mut canonical = Self::canonicalize(path.as_ref());

                if let Some(pos) = canonical.iter().position(|pool_index| {
                    let edge = &self.edges[*pool_index];
                    let node_a = &self.nodes[edge.node_lowest];
                    let node_b = &self.nodes[edge.node_highest];
                    node_a.address == self.wsol_address || node_b.address == self.wsol_address
                }) {
                    canonical.rotate_left(pos);
                }
                cycles.insert(canonical);
            }

            self.dfs_recursive(
                start_node,
                other_node,
                visited_edges,
                path,
                max_depth,
                cycles,
            );

            path.pop();
            visited_edges[edge_index] = false;
        }
    }

    #[inline]
    fn canonicalize(cycle: &[usize]) -> Vec<usize> {
        let n = cycle.len();
        if n == 0 {
            return Vec::new();
        }

        let (min_idx, _) = cycle
            .iter()
            .enumerate()
            .min_by_key(|&(_, edge_idx)| edge_idx)
            .unwrap();

        let forward: Vec<usize> = (0..n).map(|i| cycle[(min_idx + i) % n]).collect();

        let mut reversed: Vec<usize> = cycle.iter().rev().copied().collect();

        let (rev_min_idx, _) = reversed
            .iter()
            .enumerate()
            .min_by_key(|&(_, edge_idx)| edge_idx)
            .unwrap();
        reversed.rotate_left(rev_min_idx);

        if forward <= reversed {
            forward
        } else {
            reversed
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;

    #[test]
    fn test_canonicalize_empty_cycle() {
        let cycle: Vec<usize> = vec![];
        let result = Graph::canonicalize(&cycle);
        assert!(result.is_empty());
    }

    #[test]
    fn test_canonicalize_single_step() {
        let cycle = vec![42];
        let result = Graph::canonicalize(&cycle);
        assert_eq!(result, vec![42]);
    }

    #[test]
    fn test_canonicalize_two_steps_forward() {
        let cycle = vec![10, 20];
        let result = Graph::canonicalize(&cycle);
        assert_eq!(result, cycle);
    }

    #[test]
    fn test_canonicalize_two_steps_reverse_orientation() {
        let cycle = vec![20, 10];
        let result = Graph::canonicalize(&cycle);
        assert_eq!(result, vec![10, 20]);
    }

    #[test]
    fn test_canonicalize_rotated_cycle() {
        let cycle = vec![123, 321, 0, 222];
        let rotated = vec![321, 0, 222, 123];

        let result = Graph::canonicalize(&cycle);
        let rotated_result = Graph::canonicalize(&rotated);

        assert_eq!(result, rotated_result);
    }

    #[test]
    fn test_canonicalize_reversed_cycle() {
        let cycle = vec![123, 321, 0, 222];
        let reversed = vec![222, 0, 321, 123];

        let result = Graph::canonicalize(&cycle);
        let reversed_result = Graph::canonicalize(&reversed);

        assert_eq!(result, reversed_result);
    }

    #[test]
    fn test_insert_node_with_invalid_address_returns_error() {
        let mut graph = Graph::default();
        let result = graph.insert_node(TokenInfo {
            address: Some("invalid address".to_string()),
            decimals: Some(18),
            name: Some("Test Name".to_string()),
            symbol: Some("Test Symbol".to_string()),
        });

        assert!(
            result.is_err(),
            "Expected insert_node to return an error for invalid address"
        );
    }

    #[test]
    fn test_insert_node_add_two_same_nodes_returns_same_index() {
        let mut graph = Graph::default();
        let result_1 = graph.insert_node(TokenInfo {
            address: Some("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE".to_string()),
            decimals: Some(18),
            name: Some("Test Name".to_string()),
            symbol: Some("Test Symbol".to_string()),
        });

        let result_2 = graph.insert_node(TokenInfo {
            address: Some("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE".to_string()),
            decimals: Some(18),
            name: Some("Test Name".to_string()),
            symbol: Some("Test Symbol".to_string()),
        });

        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(result_1.unwrap(), 0);
        assert_eq!(result_2.unwrap(), 0);
    }

    #[test]
    fn test_insert_node_add_two_nodes_returns_indexes() {
        let mut graph = Graph::default();
        let wsol_address = Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap();
        let result_1 = graph.insert_node(TokenInfo {
            address: Some("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE".to_string()),
            decimals: Some(18),
            name: Some("Test Name".to_string()),
            symbol: Some("Test Symbol".to_string()),
        });

        let result_2 = graph.insert_node(TokenInfo {
            address: Some("7eMnzvi48Nbz2yRaQrCWqfQ7awPNPfV3AboaejktyGMD".to_string()),
            decimals: Some(18),
            name: Some("Test Name".to_string()),
            symbol: Some("Test Symbol".to_string()),
        });

        assert_eq!(graph.wsol_address, wsol_address);
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(result_1.unwrap(), 0);
        assert_eq!(result_2.unwrap(), 1);
    }

    #[test]
    fn test_insert_edge_add_one_edge_returns_index() {
        let mut graph = Graph::default();

        let idx1 = graph
            .insert_node(TokenInfo {
                address: Some("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE".to_string()),
                decimals: Some(18),
                name: Some("Test Name".to_string()),
                symbol: Some("Test Symbol".to_string()),
            })
            .unwrap();

        let idx2 = graph
            .insert_node(TokenInfo {
                address: Some("7eMnzvi48Nbz2yRaQrCWqfQ7awPNPfV3AboaejktyGMD".to_string()),
                decimals: Some(18),
                name: Some("Test Name".to_string()),
                symbol: Some("Test Symbol".to_string()),
            })
            .unwrap();

        let test_pool = PoolInfo {
            address: Some("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE".to_string()),
            fee_rate: Some(400),
            pool_type: Some(PoolType::Concentrated),
            dex: Some(DexType::Orca),
            tick_spacing: Some(64),
            token_a: None, // moved value
            token_b: None, // moved value
            token_vault_a: Some("EUuUbDcafPrmVTD5M6qoJAoyyNbihBhugADAxRMn5he9".to_string()),
            token_vault_b: Some("2WLWEuKDgkDUccTpbwYp1GToYktiSB1cXvreHUwiSUVP".to_string()),
            config: Some("2LecshUwdy9xi7meFgHtFJQNSKk4KdTrcpvaB56dP2NQ".to_string()),
        };

        let result = graph.insert_edge(test_pool, idx1, idx2);

        assert!(result.is_ok());
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.address_to_edge.len(), 1);
        assert_eq!(graph.address_to_node.len(), 2);
    }

    #[test]
    fn test_insert_pool_add_one_edge_and_two_nodes_returns_ok() {
        let mut graph = Graph::default();

        let test_pool = PoolInfo {
            address: Some("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE".to_string()),
            fee_rate: Some(400),
            pool_type: Some(PoolType::Concentrated),
            dex: Some(DexType::Orca),
            tick_spacing: Some(64),
            token_a: Some(TokenInfo {
                address: Some("So11111111111111111111111111111111111111112".to_string()),
                decimals: Some(18),
                name: Some("Test Name 1".to_string()),
                symbol: Some("Test Symbol 1".to_string()),
            }),
            token_b: Some(TokenInfo {
                address: Some("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()),
                decimals: Some(18),
                name: Some("Test Name 2".to_string()),
                symbol: Some("Test Symbol 2".to_string()),
            }),
            token_vault_a: Some("EUuUbDcafPrmVTD5M6qoJAoyyNbihBhugADAxRMn5he9".to_string()),
            token_vault_b: Some("2WLWEuKDgkDUccTpbwYp1GToYktiSB1cXvreHUwiSUVP".to_string()),
            config: Some("2LecshUwdy9xi7meFgHtFJQNSKk4KdTrcpvaB56dP2NQ".to_string()),
        };

        let result = graph.insert_pool(test_pool);

        assert!(result.is_ok());
        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);
        assert_eq!(graph.address_to_edge.len(), 1);
        assert_eq!(graph.address_to_node.len(), 2);
        assert_eq!(graph.wsol_node, 0);
    }

    #[test]
    fn test_update_edge_create_edge_and_update_returns_ok() {
        let mut graph = Graph::default();

        let test_pool = PoolInfo {
            address: Some("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE".to_string()),
            fee_rate: Some(400),
            pool_type: Some(PoolType::Concentrated),
            dex: Some(DexType::Orca),
            tick_spacing: Some(64),
            token_a: Some(TokenInfo {
                address: Some("So11111111111111111111111111111111111111112".to_string()),
                decimals: Some(18),
                name: Some("Test Name 1".to_string()),
                symbol: Some("Test Symbol 1".to_string()),
            }),
            token_b: Some(TokenInfo {
                address: Some("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()),
                decimals: Some(18),
                name: Some("Test Name 2".to_string()),
                symbol: Some("Test Symbol 2".to_string()),
            }),
            token_vault_a: Some("EUuUbDcafPrmVTD5M6qoJAoyyNbihBhugADAxRMn5he9".to_string()),
            token_vault_b: Some("2WLWEuKDgkDUccTpbwYp1GToYktiSB1cXvreHUwiSUVP".to_string()),
            config: Some("2LecshUwdy9xi7meFgHtFJQNSKk4KdTrcpvaB56dP2NQ".to_string()),
        };

        graph.insert_pool(test_pool).unwrap();

        let test_edge_update_data = PoolUpdate {
            new_liquidity: 123456,
            new_sqrt_price: 1234567,
            new_current_tick_index: -1234,
        };
        let test_addres = Pubkey::from_str("Czfq3xZZDmsdGdUyrNLtRhGc47cXcZtLG4crryfu44zE").unwrap();
        let result = graph.update_edge(&test_addres, test_edge_update_data);

        assert!(result.is_ok());
        assert_eq!(graph.edges[0].address, test_addres);
        assert_eq!(graph.edges[0].liquidity.unwrap(), 123456);
        assert_eq!(graph.edges[0].sqrt_price.unwrap(), 1234567);
        assert_eq!(graph.edges[0].current_tick_index.unwrap(), -1234);
    }
}
