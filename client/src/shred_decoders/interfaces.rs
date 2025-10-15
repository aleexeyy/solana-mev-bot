use std::{collections::HashMap, sync::Arc};

use solana_sdk::{pubkey::Pubkey, signature::Signature};

use crate::{graph::Graph, target_dexes::Program};

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
    pub transaction_address: Signature,
    pub account_keys: Arc<[Pubkey]>,
    pub lookup_tables: Arc<Vec<ReducedLookupTable>>,
    pub instructions: Vec<InstructionData>,
}

pub trait TargetTransaction: Sync + Send {
    fn decode(
        &self,
        account_keys: &Arc<[Pubkey]>,
        accounts: &[u8],
        data: &[u8],
        graph: &Arc<Graph>,
        lookup_tables: &Arc<Vec<ReducedLookupTable>>,
    ) -> anyhow::Result<DecodedInstruction>;

    // fn decode_swap_instruction(
    //     &self,
    //     data: &[u8],
    //     accounts: &[u8],
    //     account_keys: &[Pubkey],
    //     graph: &Arc<Graph>,
    // ) -> anyhow::Result<DecodedInstruction>;
    //
    // fn decode_remove_liquidity_instruction(
    //     &self,
    //     data: &[u8],
    //     accounts: &[u8],
    //     account_keys: &[Pubkey],
    //     graph: &Arc<Graph>,
    // ) -> anyhow::Result<DecodedInstruction>;
    //
    // fn decode_add_liquidity_instruction(
    //     &self,
    //     data: &[u8],
    //     accounts: &[u8],
    //     account_keys: &[Pubkey],
    //     graph: &Arc<Graph>,
    // ) -> anyhow::Result<DecodedInstruction>;
}

#[derive(Debug)]
pub enum OperationType {
    SwapExactInput {
        amount_in: u64,
        minimum_amount_out: u64,
        sqrt_price_limit: u128,
    },
    SwapExactOutput {
        amount_out: u64,
        maximum_amount_in: u64,
        sqrt_price_limit: u128,
    },
    AddLiquidity {
        add_amount_a: u64,
        add_amount_b: u64,
    },
    RemoveLiquidity {
        remove_amount_a: u64,
        remove_amount_b: u64,
    },
}

//TODO: probably need also to add Pool Type, V2, V3, DLMM
#[derive(Debug)]
pub struct DecodedInstruction {
    pub pool_address: Pubkey,
    pub token_in_address: Pubkey,
    pub token_out_address: Pubkey,
    pub operation_type: OperationType, // TODO: Check Operation Type and Adjust the Sign of change liquidity based on Operation Type
}

#[derive(Debug)]
pub struct DecodedTransaction {
    pub instructions: Vec<DecodedInstruction>,
}
