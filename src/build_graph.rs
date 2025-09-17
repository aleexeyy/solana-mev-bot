use std::{cell::RefCell, collections::{HashMap, HashSet}, fs::{read_to_string, File}, str::FromStr};

use solana_sdk::pubkey::Pubkey;

use crate::bootstrap::pool_schema::{DexType, PoolInfo, PoolType, StoredPools, TokenInfo};

#[derive(Debug)]
struct Node {
    pub address: Pubkey,
    pub decimals: u8,
    pub name: String,
    pub symbol: String,
}
#[derive(Debug)]
struct Edge {
    pub address: Pubkey,
    pub fee_rate: u32,
    pub pool_type: PoolType,
    pub dex: DexType,
    pub tick_spacing: u64,
    pub token_vault_lowest: Pubkey,  // lowest index
    pub token_vault_highest: Pubkey,  // highest index
    pub config: Pubkey,
}

#[derive(Debug)]
pub struct Graph {
    nodes: Vec<Node>,
    address_to_node: HashMap<Pubkey, usize>,
    // adjacency: HashMap<usize, HashSet<usize>>,


    edges: Vec<Edge>,
    // edge_to_nodes: HashMap<(usize, usize), HashSet<usize>>,
    // edge_to_address: HashMap<String, usize>,
}

impl Default for Graph {

    fn default() -> Self {
        Graph { nodes: vec![], edges: vec![], address_to_node: HashMap::new(), }
    }
}

impl Graph {

    // fn get_node_index(&self, token_address: &str) -> Option<&usize> {
    //     self.node_to_address.get(token_address)
    // }

    // fn get_edge_index(&self, pool_address: &str) -> Option<&usize> {
    //     self.edge_to_address.get(pool_address)
    // }


    //raw version
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

        let edge = Edge {
            address: Pubkey::from_str(&pool.address.unwrap()).unwrap(),
            fee_rate: pool.fee_rate.unwrap(),
            pool_type: pool.pool_type.unwrap(),
            dex: pool.dex.unwrap(),
            tick_spacing: pool.tick_spacing.unwrap(),
            token_vault_lowest: Pubkey::from_str(&token_vault_lowest).unwrap(),
            token_vault_highest: Pubkey::from_str(&token_vault_highest).unwrap(),
            config: Pubkey::from_str(&pool.config.unwrap()).unwrap(),
        };

        let index = self.edges.len();
        self.edges.push(edge);

        index
    }


    fn insert_pool(&mut self, mut pool: PoolInfo) {

        let node0_index = self.insert_node(pool.token_a.take().unwrap());
        let node1_index = self.insert_node(pool.token_b.take().unwrap());

        self.insert_edge(pool, node0_index, node1_index);
    }




}

pub fn build_graph() -> Result<Graph, Box<dyn std::error::Error>> {

    let pool_files_pathes: [&str; 2] = ["./cached-blockchain-data/orca_pools.json", "./cached-blockchain-data/raydium_pools.json"];
    let mut graph = Graph { ..Default::default() };
    for pool_path in pool_files_pathes {
        let raw_json = read_to_string(pool_path)?;
        
        let deserialized: StoredPools = serde_json::from_str(&raw_json)?;
        let pools: Vec<PoolInfo> = deserialized.all_pools;


        for pool in pools {
            graph.insert_pool(pool);

        }
    }

    println!("Graph: {:?}", &graph.nodes.len());

    Ok(graph)
}