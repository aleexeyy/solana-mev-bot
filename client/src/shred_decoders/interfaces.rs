use std::sync::Arc;

use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

use crate::graph::Graph;

pub trait TargetTransaction: Sync + Send {
    fn decode(
        &self,
        transaction: &VersionedTransaction,
        program_index: usize,
        graph: &Arc<Graph>,
    ) -> anyhow::Result<DecodedTransaction>;

    fn decode_swap_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        graph: &Arc<Graph>,
    ) -> anyhow::Result<DecodedInstruction>;

    fn decode_remove_liquidity_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        graph: &Arc<Graph>,
    ) -> anyhow::Result<DecodedInstruction>;

    fn decode_add_liquidity_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        graph: &Arc<Graph>,
    ) -> anyhow::Result<DecodedInstruction>;
}

// TODO: some DEXes write token_in and token_out, others just write if the swap is_direct, handle both cases, some do it without providing tokens
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

#[derive(Debug)]
pub struct DecodedInstruction {
    pub pool_address: Pubkey,
    pub token_in_address: Pubkey,  // also input token
    pub token_out_address: Pubkey, // also output token
    // pub token_in_vault: Pubkey,
    // pub token_out_vault: Pubkey,
    pub operation_type: OperationType, // TODO: Check Operation Type and Adjust the Sign of change liquidity based on Operation Type
}

#[derive(Debug)]
pub struct DecodedTransaction {
    pub instructions: Vec<DecodedInstruction>,
}
