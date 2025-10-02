use anyhow::Result;
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

use crate::transaction_decoders::{DecodedInstruction, TargetTransaction}; // path relative to mod.rs

// unit struct — cheap to store as a 'static instance
pub struct OrcaV3TargetTransaction;

impl TargetTransaction for OrcaV3TargetTransaction {
    fn decode(&self, transaction: &VersionedTransaction, program_index: usize) -> Result<()> {
        // keep heavy logic in private functions if needed:
        // decode_impl(transaction, program_index)?;
        println!("OrcaV3 decode called for program index {}", program_index);
        Ok(())
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
