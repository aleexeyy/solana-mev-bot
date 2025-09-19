use std::{collections::{HashMap, HashSet}, fs::{read_dir, read_to_string}, str::FromStr};

use solana_sdk::pubkey::Pubkey;
use tracing::{info, debug};

use crate::bootstrap::pool_schema::{DexType, PoolInfo, PoolType, PoolUpdate, StoredPools, TokenInfo};
use anyhow::{anyhow, Result};

#[derive(Debug)]
struct Node {
    pub address: Pubkey,
    pub decimals: u8,
    pub name: String,
    pub symbol: String,
}
#[derive(Debug)]
struct Edge {
    //static fields
    pub address: Pubkey,
    pub fee_rate: u32,
    pub pool_type: PoolType,
    pub dex: DexType,
    pub tick_spacing: u64,
    pub token_vault_lowest: Pubkey,  // lowest index
    pub token_vault_highest: Pubkey,  // highest index
    pub config: Pubkey,

    //dynamic fields

    pub sqrt_price: Option<u128>,
    pub liquidity: Option<u128>,
    pub current_tick_index: Option<i32>,
}

#[derive(Debug)]
pub struct Graph {
    nodes: Vec<Node>,
    address_to_node: HashMap<Pubkey, usize>,
    // adjacency: HashMap<usize, HashSet<usize>>,

    edges: Vec<Edge>,
    address_to_edge: HashMap<Pubkey, usize>,
    // edge_to_nodes: HashMap<(usize, usize), HashSet<usize>>,
}

impl Default for Graph {

    fn default() -> Self {
        Graph { nodes: vec![], edges: vec![], address_to_node: HashMap::new(), address_to_edge: HashMap::new(), }
    }
}

impl Graph {

    
    fn insert_node(&mut self, token: TokenInfo) -> usize {

        let token_address = Pubkey::from_str(&token.address.unwrap()).unwrap();

        if let Some(&existing_index) = self.address_to_node.get(&token_address) {
            return existing_index;
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

        index
    }

    fn insert_edge(&mut self, pool: PoolInfo, node0_index: usize, node1_index: usize) -> usize {

        let (token_vault_lowest, token_vault_highest) = if node0_index < node1_index {
            (pool.token_vault_a.unwrap(), pool.token_vault_b.unwrap())
        } else {
            (pool.token_vault_b.unwrap(), pool.token_vault_a.unwrap())
        };
        let address = Pubkey::from_str(&pool.address.unwrap()).unwrap();
        let edge = Edge {
            address: address.clone(),
            fee_rate: pool.fee_rate.unwrap(),
            pool_type: pool.pool_type.unwrap(),
            dex: pool.dex.unwrap(),
            tick_spacing: pool.tick_spacing.unwrap(),
            token_vault_lowest: Pubkey::from_str(&token_vault_lowest).unwrap(),
            token_vault_highest: Pubkey::from_str(&token_vault_highest).unwrap(),
            config: Pubkey::from_str(&pool.config.unwrap()).unwrap(),
            sqrt_price: None,
            liquidity: None,
            current_tick_index: None,
        };

        let index = self.edges.len();
        self.edges.push(edge);
        self.address_to_edge.insert(address, index);

        index
    }


    fn insert_pool(&mut self, mut pool: PoolInfo) {

        let node0_index = self.insert_node(pool.token_a.take().unwrap());
        let node1_index = self.insert_node(pool.token_b.take().unwrap());

        self.insert_edge(pool, node0_index, node1_index);
    }


    pub fn update_edge(&mut self, address: &Pubkey, data: PoolUpdate) -> Result<()> {
        if let Some(edge_index) = self.address_to_edge.get(address) {
            if let Some(edge) = self.edges.get_mut(*edge_index) {
                edge.liquidity = Some(data.new_liquidity);
                edge.sqrt_price = Some(data.new_sqrt_price);
                edge.current_tick_index = Some(data.new_current_tick_index);
                return Ok(());
            }
        }
        Err(anyhow!("Edge with address {} doesn't exist", address))
    }


}

pub fn build_graph() -> Result<Graph> {

    let pool_files = Vec::from_iter(
        read_dir("./cached-blockchain-data")?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|ext| ext.to_str()) == Some("json")),
    );
    let mut graph = Graph { ..Default::default() };
    for pool_path in pool_files{
        let raw_json = read_to_string(pool_path)?;
        
        let deserialized: StoredPools = serde_json::from_str(&raw_json)?;
        let pools: Vec<PoolInfo> = deserialized.all_pools;


        for pool in pools {
            graph.insert_pool(pool);
        }
    }

    info!("Amount of Edges in the Graph: {:?}", graph.edges.len());
    info!("Amount of Nodes in the Graph: {:?}", graph.nodes.len());
    Ok(graph)
}