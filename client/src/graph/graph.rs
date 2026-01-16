use std::sync::Arc;

use anyhow::Result;
use bootstrap::pool_schema::{PoolInfo, PoolUpdate, TokenInfo};
pub use graph::market_graph::{Edge as GraphEdge, Node as GraphNode};
use graph::{
    arb_cycles::{ArbitrageCycles, CycleKey},
    market_graph::{Edge, MarketGraph, Node, PoolId, TokenId},
};
use solana_sdk::pubkey::Pubkey;

use crate::{bootstrap, graph};

#[derive(Debug, Default)]
pub struct Graph {
    market: MarketGraph,
    cycles: ArbitrageCycles,
}

impl Graph {
    pub fn new() -> Self {
        Self {
            market: MarketGraph::new(),
            cycles: ArbitrageCycles::new(),
        }
    }

    pub fn build_graph(data_folder_path: &str) -> Result<Self> {
        let market = MarketGraph::build_graph(data_folder_path)?;
        Ok(Self {
            market,
            cycles: ArbitrageCycles::new(),
        })
    }

    pub fn build_cycles(&mut self, max_depth: usize) -> Result<()> {
        self.cycles.build(&self.market, max_depth)
    }

    pub fn get_node(&self, address: &Pubkey) -> Option<&Node> {
        self.market.get_node(address)
    }
    pub fn get_edge(&self, address: &Pubkey) -> Option<&Arc<Edge>> {
        self.market.get_edge(address)
    }

    pub fn update_edge(&self, address: &Pubkey, data: PoolUpdate) -> Result<()> {
        self.market.update_edge(address, data)
    }

    pub fn insert_node(&mut self, token: TokenInfo) -> Result<TokenId> {
        self.market.insert_node(token)
    }
    pub fn insert_edge(
        &mut self,
        pool: PoolInfo,
        node0_index: TokenId,
        node1_index: TokenId,
    ) -> Result<PoolId> {
        self.market.insert_edge(pool, node0_index, node1_index)
    }
    pub fn insert_pool(&mut self, pool: PoolInfo) -> Result<()> {
        self.market.insert_pool(pool)
    }

    pub fn nodes_len(&self) -> usize {
        self.market.nodes.len()
    }
    pub fn edges_len(&self) -> usize {
        self.market.edges.len()
    }
    pub fn all_cycles_len(&self) -> usize {
        self.cycles.all_cycles.len()
    }
    pub fn all_cycles(&self) -> &std::collections::HashMap<CycleKey, Vec<Vec<PoolId>>> {
        &self.cycles.all_cycles
    }
    pub fn node_address(&self, index: TokenId) -> Option<Pubkey> {
        self.market.nodes.get(index.0).map(|n| n.address)
    }

    pub fn market_graph(&self) -> &MarketGraph {
        &self.market
    }
    pub fn cycles_index(&self) -> &ArbitrageCycles {
        &self.cycles
    }

    pub fn canonicalize(cycle: &[PoolId]) -> Vec<PoolId> {
        ArbitrageCycles::canonicalize(cycle)
    }
    pub fn check_cycle(&self, cycle: &mut [PoolId]) -> bool {
        ArbitrageCycles::check_cycle_with_graph(&self.market, cycle)
    }
}

#[cfg(test)]
mod tests {
    use std::{str::FromStr, sync::atomic::Ordering};

    use super::*;
    use crate::bootstrap::pool_schema::{DexType, PoolType};

    #[test]
    fn test_canonicalize_empty_cycle() {
        let cycle: Vec<PoolId> = vec![];
        let result = Graph::canonicalize(&cycle);
        assert!(result.is_empty());
    }

    #[test]
    fn test_canonicalize_single_step() {
        let cycle = vec![PoolId(42)];
        let result = Graph::canonicalize(&cycle);
        assert_eq!(result, vec![PoolId(42)]);
    }

    #[test]
    fn test_canonicalize_two_steps_forward() {
        let cycle = vec![PoolId(10), PoolId(20)];
        let result = Graph::canonicalize(&cycle);
        assert_eq!(result, cycle);
    }

    #[test]
    fn test_canonicalize_two_steps_reverse_orientation() {
        let cycle = vec![PoolId(20), PoolId(10)];
        let result = Graph::canonicalize(&cycle);
        assert_eq!(result, vec![PoolId(10), PoolId(20)]);
    }

    #[test]
    fn test_canonicalize_rotated_cycle() {
        let cycle = vec![PoolId(123), PoolId(321), PoolId(0), PoolId(222)];
        let rotated = vec![PoolId(321), PoolId(0), PoolId(222), PoolId(123)];

        let result = Graph::canonicalize(&cycle);
        let rotated_result = Graph::canonicalize(&rotated);

        assert_eq!(result, rotated_result);
    }

    #[test]
    fn test_canonicalize_reversed_cycle() {
        let cycle = vec![PoolId(123), PoolId(321), PoolId(0), PoolId(222)];
        let reversed = vec![PoolId(222), PoolId(0), PoolId(321), PoolId(123)];

        let result = Graph::canonicalize(&cycle);
        let reversed_result = Graph::canonicalize(&reversed);

        assert_eq!(result, reversed_result);
    }

    #[test]
    fn test_insert_node_with_invalid_address_returns_error() {
        let mut graph = Graph::new();
        let result = graph.insert_node(TokenInfo {
            address: Some("invalid address".to_string()),
            decimals: Some(18),
            name: Some("Test Name".to_string()),
            symbol: Some("Test Symbol".to_string()),
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_insert_node_add_two_same_nodes_returns_same_index() {
        let mut graph = Graph::new();
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

        assert_eq!(graph.nodes_len(), 1);
        assert_eq!(result_1.unwrap().0, 0);
        assert_eq!(result_2.unwrap().0, 0);
    }

    #[test]
    fn test_insert_node_add_two_nodes_returns_indexes() {
        let mut graph = Graph::new();
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

        // WSOL wasn't inserted in this test; ensure it's not present
        let node0 = graph.get_node(&wsol_address);
        assert!(node0.is_none());
        assert_eq!(graph.nodes_len(), 2);
        assert_eq!(result_1.unwrap().0, 0);
        assert_eq!(result_2.unwrap().0, 1);
    }

    #[test]
    fn test_insert_edge_add_one_edge_returns_index() {
        let mut graph = Graph::new();

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
            token_a: None,
            token_b: None,
            token_vault_a: Some("EUuUbDcafPrmVTD5M6qoJAoyyNbihBhugADAxRMn5he9".to_string()),
            token_vault_b: Some("2WLWEuKDgkDUccTpbwYp1GToYktiSB1cXvreHUwiSUVP".to_string()),
            config: Some("2LecshUwdy9xi7meFgHtFJQNSKk4KdTrcpvaB56dP2NQ".to_string()),
        };

        let result = graph.insert_edge(test_pool, idx1, idx2);

        assert!(result.is_ok());
        assert_eq!(graph.nodes_len(), 2);
        assert_eq!(graph.edges_len(), 1);
    }

    #[test]
    fn test_insert_pool_add_one_edge_and_two_nodes_returns_ok() {
        let mut graph = Graph::new();

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
        assert_eq!(graph.nodes_len(), 2);
        assert_eq!(graph.edges_len(), 1);
    }

    #[test]
    fn test_update_edge_create_edge_and_update_returns_ok() {
        let mut graph = Graph::new();

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
        let edge = graph.get_edge(&test_addres).unwrap();
        assert_eq!(edge.address, test_addres);
        assert_eq!(edge.liquidity.load(Ordering::Relaxed), 123456);
        assert_eq!(edge.sqrt_price.load(Ordering::Relaxed), 1234567);
        assert_eq!(edge.current_tick_index.load(Ordering::Relaxed), -1234);
    }
}
