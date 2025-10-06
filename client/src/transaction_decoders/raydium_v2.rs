use anyhow::{Result, anyhow};
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

use crate::transaction_decoders::{DecodedInstruction, DecodedTransaction, TargetTransaction};

pub struct RaydiumV2TargetTransaction;

impl TargetTransaction for RaydiumV2TargetTransaction {
    fn decode(
        &self,
        transaction: &VersionedTransaction,
        program_index: usize,
    ) -> Result<DecodedTransaction> {
        Err(anyhow!(
            "RaydiumV2TargetTransaction does not support decode"
        ))
    }

    fn decode_swap_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<DecodedInstruction> {
        todo!()
    }

    fn decode_remove_liquidity_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<DecodedInstruction> {
        todo!()
    }

    fn decode_add_liquidity_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<DecodedInstruction> {
        todo!()
    }
}
