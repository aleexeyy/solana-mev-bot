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
    fn decode(&self, transaction: &VersionedTransaction, program_index: usize) -> Result<()>;

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

pub enum OperationType {
    Swap,
    AddLiquidity,
    RemoveLiquidity,
}

pub struct DecodedInstruction {
    pool_address: Pubkey,
    token_a_address: Pubkey,
    token_b_address: Pubkey,
    token_a_vault: Pubkey,
    token_b_vault: Pubkey,
    operation_type: OperationType, // TODO: Check Operation Type and Adjust the Sign of change liquidity based on Operation Type

    change_liquidity_a: u64, // test field
    change_liquidity_b: u64, // test field
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
) -> Result<()> {
    let idx = program.index();
    DECODERS[idx].decode(transaction, program_index)
}
