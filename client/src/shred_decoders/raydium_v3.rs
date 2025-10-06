use std::{io::Read, sync::Arc};

use anyhow::{Result, anyhow};
use solana_sdk::{
    message::compiled_instruction::CompiledInstruction, pubkey::Pubkey,
    transaction::VersionedTransaction,
};

use crate::{
    graph::Graph,
    shred_decoders::{
        DecodedTransaction, TargetTransaction,
        interfaces::{DecodedInstruction, OperationType},
    },
};

pub struct RaydiumV3TargetTransaction;

impl TargetTransaction for RaydiumV3TargetTransaction {
    fn decode(
        &self,
        transaction: &VersionedTransaction,
        program_index: usize,
        graph: &Arc<Graph>,
    ) -> Result<DecodedTransaction> {
        let target_instructions: Vec<&CompiledInstruction> = transaction
            .message
            .instructions()
            .iter()
            .filter(|instruction| usize::from(instruction.program_id_index) == program_index)
            .collect();

        if target_instructions.is_empty() {
            return Err(anyhow!("Unsupported instructions"));
        }

        let account_keys = transaction.message.static_account_keys();
        let mut decoded_instructions: Vec<DecodedInstruction> =
            Vec::with_capacity(target_instructions.len());

        for instruction in target_instructions {
            let data = &instruction.data;
            let accounts = &instruction.accounts;

            let mut reader = data.as_slice();
            let mut instruction_type = [0u8; 8];
            reader.read_exact(&mut instruction_type)?;

            let decoded_instruction = match instruction_type {
                SWAP => self.decode_swap_instruction(reader, accounts, account_keys, graph),
                _ => {
                    tracing::warn!(
                        "Got Unsupported RaydiumV3 instruction type: {:?}",
                        transaction
                    );
                    return Err(anyhow!("Unsupported swap instruction type on RaydiumV3"));
                }
            }?;
            decoded_instructions.push(decoded_instruction);
        }

        if decoded_instructions.is_empty() {
            return Err(anyhow!("Unsupported instructions"));
        }

        let decoded_transaction = DecodedTransaction {
            instructions: decoded_instructions,
        };
        Ok(decoded_transaction)
    }

    //example: https://solscan.io/tx/2j7ikfSmJ1AMt979tHmqKQGdv5KiBppveHoQTdhjxLkVkr3FjKf9C7kACkhAF6zUX3UZepumuQJzJMcWQESfAPyV
    fn decode_swap_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        _graph: &Arc<Graph>,
    ) -> Result<DecodedInstruction> {
        if accounts.len() != SWAP_ACCOUNTS_LEN {
            return Err(anyhow!(
                "accounts len != SWAP_ACCOUNTS_LEN, received {} | expected {}",
                accounts.len(),
                SWAP_ACCOUNTS_LEN
            ));
        }

        let pool_address = *account_keys
            .get(usize::from(accounts[2]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        // let token_in_vault = *account_keys
        //     .get(usize::from(accounts[5]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        // let token_out_vault = *account_keys
        //     .get(usize::from(accounts[6]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let token_in_address = *account_keys
            .get(usize::from(accounts[11]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let token_out_address = *account_keys
            .get(usize::from(accounts[12]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let specified_amount: u64 = u64::from_le_bytes(data[0..8].try_into()?);
        let amount_threshold: u64 = u64::from_le_bytes(data[8..16].try_into()?);

        let sqrt_price_limit: u128 = u128::from_le_bytes(data[16..32].try_into()?);
        let is_exact_input: bool = data[32] == 1;

        if is_exact_input {
            Ok(DecodedInstruction {
                pool_address,
                token_in_address,
                token_out_address,
                operation_type: OperationType::SwapExactInput {
                    amount_in: specified_amount,
                    minimum_amount_out: amount_threshold,
                    sqrt_price_limit,
                },
            })
        } else {
            Ok(DecodedInstruction {
                pool_address,
                token_in_address,
                token_out_address,
                operation_type: OperationType::SwapExactOutput {
                    amount_out: specified_amount,
                    maximum_amount_in: amount_threshold,
                    sqrt_price_limit,
                },
            })
        }
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

const SWAP: [u8; 8] = [43, 4, 237, 11, 26, 201, 30, 98];
const SWAP_ACCOUNTS_LEN: usize = 16;
