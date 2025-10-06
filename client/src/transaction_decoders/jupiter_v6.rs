use anyhow::{Result, anyhow};
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

use crate::transaction_decoders::{DecodedInstruction, DecodedTransaction, TargetTransaction};
pub struct JupiterV6TargetTransaction;

impl TargetTransaction for JupiterV6TargetTransaction {
    fn decode(
        &self,
        transaction: &VersionedTransaction,
        program_index: usize,
    ) -> Result<DecodedTransaction> {
        Err(anyhow!(
            "JupiterV6TargetTransaction does not support decode"
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
