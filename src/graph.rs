use std::{
    collections::HashMap,
    fs::{read_dir, read_to_string},
    str::FromStr,
};

use solana_sdk::pubkey::Pubkey;
use tracing::{info, warn};

use crate::bootstrap::pool_schema::{
    DexType, PoolInfo, PoolType, PoolUpdate, StoredPools, TokenInfo,
};
use anyhow::{Result, anyhow};

#[allow(dead_code)]
#[derive(Debug)]
struct Node {
    address: Pubkey,
    decimals: u8,
    name: String,
    symbol: String,
}

#[allow(dead_code)]
#[derive(Debug)]
struct Edge {
    //static fields
    address: Pubkey,
    fee_rate: u32,
    pool_type: PoolType,
    dex: DexType,
    tick_spacing: u64,
    token_vault_lowest: Pubkey,  // lowest index
    token_vault_highest: Pubkey, // highest index
    config: Pubkey,

    //dynamic fields
    sqrt_price: Option<u128>,
    liquidity: Option<u128>,
    current_tick_index: Option<i32>,
}

#[derive(Debug, Default)]
pub struct Graph {
    nodes: Vec<Node>,
    address_to_node: HashMap<Pubkey, usize>,
    // adjacency: HashMap<usize, HashSet<usize>>,
    edges: Vec<Edge>,
    address_to_edge: HashMap<Pubkey, usize>,
    // edge_to_nodes: HashMap<(usize, usize), HashSet<usize>>,
}

impl Graph {
    fn default() -> Self {
        Graph {
            nodes: vec![],
            edges: vec![],
            address_to_node: HashMap::new(),
            address_to_edge: HashMap::new(),
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
        self.nodes.push(node);
        self.address_to_node.insert(token_address, index);

        Ok(index)
    }

    fn insert_edge(
        &mut self,
        pool: PoolInfo,
        node0_index: usize,
        node1_index: usize,
    ) -> Result<usize> {
        let (token_vault_lowest, token_vault_highest) = if node0_index < node1_index {
            (pool.token_vault_a.unwrap(), pool.token_vault_b.unwrap())
        } else {
            (pool.token_vault_b.unwrap(), pool.token_vault_a.unwrap())
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
            sqrt_price: None,
            liquidity: None,
            current_tick_index: None,
        };

        let index = self.edges.len();
        self.edges.push(edge);
        self.address_to_edge.insert(address, index);

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

    pub fn build_graph() -> Result<Self> {
        let pool_files = Vec::from_iter(
            read_dir("./cached-blockchain-data")?
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
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let _ = graph.insert_pool(test_pool).unwrap();

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
