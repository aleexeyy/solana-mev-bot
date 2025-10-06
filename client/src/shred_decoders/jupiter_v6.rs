use std::sync::Arc;

use anyhow::{Result, anyhow};
use solana_sdk::{pubkey::Pubkey, transaction::VersionedTransaction};

use crate::{
    graph::Graph,
    shred_decoders::{DecodedTransaction, TargetTransaction, interfaces::DecodedInstruction},
};

pub struct JupiterV6TargetTransaction;

impl TargetTransaction for JupiterV6TargetTransaction {
    fn decode(
        &self,
        transaction: &VersionedTransaction,
        program_index: usize,
        graph: &Arc<Graph>,
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
        graph: &Arc<Graph>,
    ) -> Result<DecodedInstruction> {
        todo!()
    }

    fn decode_remove_liquidity_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        graph: &Arc<Graph>,
    ) -> Result<DecodedInstruction> {
        todo!()
    }
    fn decode_add_liquidity_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        graph: &Arc<Graph>,
    ) -> Result<DecodedInstruction> {
        todo!()
    }
}
