use std::{collections::HashMap, sync::Arc};

use solana_sdk::{pubkey::Pubkey, signature::Signature};

use crate::{graph::market_graph::MarketGraph, target_dexes::Program};

pub type RawLookupTables = HashMap<String, Vec<String>>;

pub struct ReducedLookupTable {
    pub account_key: Pubkey,
    pub indexes: Arc<[u8]>,
}

impl Default for ReducedLookupTable {
    fn default() -> Self {
        Self::new()
    }
}
impl ReducedLookupTable {
    pub fn new() -> Self {
        ReducedLookupTable {
            account_key: Pubkey::default(),
            indexes: Arc::default(),
        }
    }
}
pub struct InstructionData {
    pub program: Program,
    pub accounts: Arc<[u8]>,
    pub data: Arc<[u8]>,
}

pub struct DecodeJob {
    pub slot: u64,
    pub transaction_address: Signature,
    pub account_keys: Arc<[Pubkey]>,
    pub lookup_tables: Arc<Vec<ReducedLookupTable>>,
    pub instructions: Vec<InstructionData>,
}

pub trait TargetTransaction: Sync + Send {
    fn decode(
        &self,
        slot: u64,
        signature: Signature,
        instruction_index: u8,
        account_keys: &Arc<[Pubkey]>,
        accounts: &[u8],
        data: &[u8],
        market: &MarketGraph,
        lookup_tables: &Arc<Vec<ReducedLookupTable>>,
    ) -> anyhow::Result<ShredEvent>;
}

#[derive(Debug, Clone)]
pub enum SwapConstraint {
    SqrtPriceLimit(u128),
    TokenAmountLimit(u64),
}

#[derive(Debug, Clone)]
pub enum ShredEventType {
    Swap {
        amount_specified: u64,
        limit: SwapConstraint,
        a_to_b: bool,
        is_base_input: bool,
    },
    AddLiquidity,
    RemoveLiquidity,
}

#[derive(Debug, Clone)]
pub struct ShredEvent {
    pub signature: Signature,
    pub slot: u64,
    pub pool_address: Pubkey,
    pub instruction_index: u8,
    pub event: ShredEventType,
}

impl ShredEvent {
    pub fn new(
        signature: Signature,
        slot: u64,
        pool_address: Pubkey,
        instruction_index: u8,
        event: ShredEventType,
    ) -> Self {
        Self {
            signature,
            slot,
            pool_address,
            instruction_index,
            event,
        }
    }
}
