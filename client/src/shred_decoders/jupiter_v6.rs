use std::sync::Arc;

use anyhow::{Result, anyhow};
use solana_sdk::pubkey::Pubkey;

use crate::{
    graph::Graph,
    shred_decoders::{TargetTransaction, interfaces::DecodedInstruction},
};

pub struct JupiterV6TargetTransaction;

impl TargetTransaction for JupiterV6TargetTransaction {
    fn decode(
        &self,
        account_keys: &Arc<[Pubkey]>,
        accounts: &[u8],
        data: &[u8],
        graph: &Arc<Graph>,
    ) -> Result<DecodedInstruction> {
        Err(anyhow!(
            "JupiterV6TargetTransaction does not support decode"
        ))
    }
}

impl JupiterV6TargetTransaction {
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
