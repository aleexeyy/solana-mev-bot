use std::{
    collections::{HashMap, HashSet},
    fs::read_to_string,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicI32, AtomicU128, Ordering},
    },
};

use anyhow::{Result, anyhow};
use ethnum::U256;
use solana_sdk::pubkey::Pubkey;
use tracing::{info, warn};

use crate::{
    bootstrap::pool_schema::{DexType, PoolInfo, PoolType, PoolUpdate, StoredPools, TokenInfo},
    get_all_pool_files,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TokenId(pub usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PoolId(pub usize);

#[allow(dead_code)]
#[derive(Debug)]
pub struct Node {
    pub address: Pubkey,
    pub(crate) decimals: u8,
    pub(crate) name: String,
    pub symbol: String,
}

// TODO: reimplement edge to save tokens as tokenA and tokenB
#[allow(dead_code)]
#[derive(Debug)]
pub struct Edge {
    // static fields
    pub address: Pubkey,
    pub(crate) fee_rate: u32,
    pub(crate) pool_type: PoolType,
    pub(crate) dex: DexType,
    pub(crate) tick_spacing: u64,
    pub token_vault_a: Pubkey,
    pub token_vault_b: Pubkey,
    pub(crate) config: Pubkey,
    pub node_a: TokenId,
    pub node_b: TokenId,
    pub(crate) decimals_a: u8,
    pub(crate) decimals_b: u8,

    // dynamic fields
    pub sqrt_price: AtomicU128,
    pub(crate) liquidity: AtomicU128,
    pub(crate) current_tick_index: AtomicI32,
}

impl Edge {
    pub fn get_log_exchange_rate(&self, a_to_b: bool) -> f64 {
        self.get_exchange_rate(a_to_b).log10()
    }

    pub fn get_exchange_rate(&self, a_to_b: bool) -> f64 {
        let decimals_diff: i32 = self.decimals_a as i32 - self.decimals_b as i32;
        let denominator = 10f64.powi(decimals_diff);

        let scaled_price: U256 = U256::from(self.sqrt_price.load(Ordering::Relaxed));
        let squared: U256 = scaled_price * scaled_price;

        let high: U256 = squared >> 128;
        let low: U256 = squared & U256::from(u128::MAX);
        let price_f64 = high.as_u128() as f64 * 2f64.powi(64) + low.as_u128() as f64;

        let price_f64 = price_f64 / 2f64.powi(128);

        let exchange_rate_a_to_b = price_f64 * denominator;

        if a_to_b {
            exchange_rate_a_to_b
        } else {
            1.0 / exchange_rate_a_to_b
        }
    }

    pub(crate) fn get_other_node(&self, this_token: TokenId) -> Option<TokenId> {
        if this_token == self.node_a {
            Some(self.node_b)
        } else if this_token == self.node_b {
            Some(self.node_a)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub(crate) fn get_swap_direction(&self, token_in: TokenId) -> Option<bool> {
        if self.node_a == token_in {
            return Some(true);
        } else if self.node_b == token_in {
            return Some(false);
        }
        None
    }
}

#[derive(Debug)]
pub struct MarketGraph {
    pub(crate) wsol_address: Pubkey,
    pub(crate) wsol_node: TokenId,

    pub nodes: Vec<Node>,
    pub edges: Vec<Arc<Edge>>,

    pub(crate) address_to_node: HashMap<Pubkey, TokenId>,
    pub(crate) address_to_edge: HashMap<Pubkey, PoolId>,
    pub(crate) adjacency: HashMap<TokenId, HashSet<PoolId>>, // adjacent pools to the token
}

impl MarketGraph {
    pub fn new() -> Self {
        MarketGraph {
            wsol_address: Pubkey::from_str("So11111111111111111111111111111111111111112").unwrap(),
            wsol_node: TokenId(usize::MAX),
            nodes: vec![],
            edges: vec![],
            address_to_node: HashMap::new(),
            address_to_edge: HashMap::new(),
            adjacency: HashMap::new(),
        }
    }

    pub fn get_node(&self, address: &Pubkey) -> Option<&Node> {
        if let Some(node_index) = self.address_to_node.get(address) {
            Some(&self.nodes[node_index.0])
        } else {
            None
        }
    }

    pub fn get_edge(&self, address: &Pubkey) -> Option<&Arc<Edge>> {
        if let Some(edge_index) = self.address_to_edge.get(address) {
            Some(&self.edges[edge_index.0])
        } else {
            None
        }
    }

    pub fn insert_node(&mut self, token: TokenInfo) -> Result<TokenId> {
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
            self.wsol_node = TokenId(index);
        }

        self.nodes.push(node);
        self.address_to_node.insert(token_address, TokenId(index));
        self.adjacency.insert(TokenId(index), HashSet::new());

        Ok(TokenId(index))
    }

    pub fn insert_edge(
        &mut self,
        pool: PoolInfo,
        node0_index: TokenId,
        node1_index: TokenId,
    ) -> Result<PoolId> {
        let node0_address = self
            .nodes
            .get(node0_index.0)
            .ok_or_else(|| anyhow!("Invalid node0_index"))?
            .address;
        let node1_address = self
            .nodes
            .get(node1_index.0)
            .ok_or_else(|| anyhow!("Invalid node1_index"))?
            .address;

        let (idx_a, idx_b, token_vault_a_str, token_vault_b_str) =
            if node0_address.to_bytes() <= node1_address.to_bytes() {
                (
                    node0_index,
                    node1_index,
                    pool.token_vault_a
                        .clone()
                        .ok_or_else(|| anyhow!("Missing Token Vault A"))?,
                    pool.token_vault_b
                        .clone()
                        .ok_or_else(|| anyhow!("Missing Token Vault B"))?,
                )
            } else {
                (
                    node1_index,
                    node0_index,
                    pool.token_vault_b
                        .clone()
                        .ok_or_else(|| anyhow!("Missing Token Vault B"))?,
                    pool.token_vault_a
                        .clone()
                        .ok_or_else(|| anyhow!("Missing Token Vault A"))?,
                )
            };
        let address = Pubkey::from_str(&pool.address.unwrap())?;
        let edge = Edge {
            address,
            fee_rate: pool.fee_rate.unwrap(),
            pool_type: pool.pool_type.unwrap(),
            dex: pool.dex.unwrap(),
            tick_spacing: pool.tick_spacing.unwrap(),
            token_vault_a: Pubkey::from_str(&token_vault_a_str)?,
            token_vault_b: Pubkey::from_str(&token_vault_b_str)?,
            config: Pubkey::from_str(&pool.config.unwrap())?,
            node_a: idx_a,
            node_b: idx_b,
            decimals_a: self.nodes[idx_a.0].decimals,
            decimals_b: self.nodes[idx_b.0].decimals,
            sqrt_price: AtomicU128::new(0),
            liquidity: AtomicU128::new(0),
            current_tick_index: AtomicI32::new(0),
        };

        let index = self.edges.len();
        self.edges.push(Arc::new(edge));
        self.address_to_edge.insert(address, PoolId(index));

        self.adjacency
            .get_mut(&idx_a)
            .unwrap()
            .insert(PoolId(index));
        self.adjacency
            .get_mut(&idx_b)
            .unwrap()
            .insert(PoolId(index));

        Ok(PoolId(index))
    }

    pub fn insert_pool(&mut self, mut pool: PoolInfo) -> Result<()> {
        let node0_index = self.insert_node(pool.token_a.take().unwrap())?;
        let node1_index = self.insert_node(pool.token_b.take().unwrap())?;

        self.insert_edge(pool, node0_index, node1_index)?;

        Ok(())
    }

    pub fn update_edge(&self, address: &Pubkey, data: PoolUpdate) -> Result<()> {
        if let Some(&PoolId(edge_index)) = self.address_to_edge.get(address)
            && let Some(edge) = self.edges.get(edge_index)
        {
            edge.liquidity.store(data.new_liquidity, Ordering::Relaxed);
            edge.sqrt_price
                .store(data.new_sqrt_price, Ordering::Relaxed);
            edge.current_tick_index
                .store(data.new_current_tick_index, Ordering::Relaxed);
            return Ok(());
        }
        Err(anyhow!("Edge with address {} doesn't exist", address))
    }

    pub fn build_graph(data_folder_path: &str) -> Result<Self> {
        let pool_files = get_all_pool_files(data_folder_path)?;

        let mut graph = MarketGraph::new();
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

    pub fn token_address(&self, id: TokenId) -> Option<Pubkey> {
        self.nodes.get(id.0).map(|n| n.address)
    }
}

impl Default for MarketGraph {
    fn default() -> Self {
        Self::new()
    }
}
