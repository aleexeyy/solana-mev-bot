use anyhow::Result;
use once_cell::sync::Lazy;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

use crate::target_dexes::Program;

mod jupiter_v6;
mod meteora_dlmm;
mod meteora_v2;
mod meteora_v3;
mod orca_v3;
mod raydium_v2;
mod raydium_v3;

pub trait TargetTransaction: Sync + Send {
    fn decode(
        &self,
        transaction: &VersionedTransaction,
        program_index: usize,
    ) -> Result<DecodedTransaction>;

    fn decode_swap_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<DecodedInstruction>;

    fn decode_remove_liquidity_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<DecodedInstruction>;

    fn decode_add_liquidity_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<DecodedInstruction>;
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
    pool_address: Pubkey,
    token_in_address: Pubkey,  // also input token
    token_out_address: Pubkey, // also output token
    token_in_vault: Pubkey,
    token_out_vault: Pubkey,
    operation_type: OperationType, // TODO: Check Operation Type and Adjust the Sign of change liquidity based on Operation Type
}

#[derive(Debug)]
pub struct DecodedTransaction {
    instructions: Vec<DecodedInstruction>,
}

pub static RAYDIUM_V2_DECODER: raydium_v2::RaydiumV2TargetTransaction =
    raydium_v2::RaydiumV2TargetTransaction;
pub static RAYDIUM_V3_DECODER: raydium_v3::RaydiumV3TargetTransaction =
    raydium_v3::RaydiumV3TargetTransaction;
pub static ORCA_V3_DECODER: orca_v3::OrcaV3TargetTransaction = orca_v3::OrcaV3TargetTransaction;
pub static METEORA_V3_DECODER: meteora_v3::MeteoraV3TargetTransaction =
    meteora_v3::MeteoraV3TargetTransaction;
pub static METEORA_V2_DECODER: meteora_v2::MeteoraV2TargetTransaction =
    meteora_v2::MeteoraV2TargetTransaction;
pub static JUPITER_V6_DECODER: jupiter_v6::JupiterV6TargetTransaction =
    jupiter_v6::JupiterV6TargetTransaction;

static DECODERS: Lazy<[&'static dyn TargetTransaction; 6]> = Lazy::new(|| {
    [
        &RAYDIUM_V2_DECODER,
        &RAYDIUM_V3_DECODER,
        &ORCA_V3_DECODER,
        &METEORA_V3_DECODER,
        &METEORA_V2_DECODER,
        &JUPITER_V6_DECODER,
    ]
});

pub fn decode_transaction(
    program: Program,
    transaction: &VersionedTransaction,
    program_index: usize,
) -> Result<DecodedTransaction> {
    let idx = program.index();
    DECODERS[idx].decode(transaction, program_index)
}
