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

pub struct OrcaV3TargetTransaction;

impl TargetTransaction for OrcaV3TargetTransaction {
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
                SWAP_V1 => self.decode_swap_instruction(reader, accounts, account_keys, graph),
                SWAP_V2 => self.decode_swap_instruction(reader, accounts, account_keys, graph),
                REMOVE_LIQUIDITY => {
                    self.decode_remove_liquidity_instruction(reader, accounts, account_keys, graph)
                }
                ADD_LIQUIDITY => {
                    self.decode_add_liquidity_instruction(reader, accounts, account_keys, graph)
                }
                _ => {
                    tracing::warn!("Unsupported OrcaV3 instruction: {:?}", transaction);
                    return Err(anyhow!("Unsupported swap instruction type on OrcaV3"));
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

    fn decode_swap_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        graph: &Arc<Graph>,
    ) -> Result<DecodedInstruction> {
        match accounts.len() {
            SWAP_V1_ACCOUNTS_LEN => {
                self.decode_swap_v1_instruction(data, accounts, account_keys, graph)
            }
            SWAP_V2_ACCOUNTS_LEN => {
                self.decode_swap_v2_instruction(data, accounts, account_keys, graph)
            }
            _ => Err(anyhow!("Unsupported swap instruction account length")),
        }
    }

    fn decode_remove_liquidity_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        graph: &Arc<Graph>,
    ) -> Result<DecodedInstruction> {
        tracing::warn!(
            "Unsupported Remove liquidity OrcaV3 instruction: {:?}",
            data
        );
        Err(anyhow!("Unsupported instructions"))
    }
    fn decode_add_liquidity_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        graph: &Arc<Graph>,
    ) -> Result<DecodedInstruction> {
        tracing::warn!("Unsupported Add liquidity OrcaV3 instruction: {:?}", data);
        Err(anyhow!("Unsupported instructions"))
    }
}

impl OrcaV3TargetTransaction {
    //example: https://solscan.io/tx/5FQXUNt7bTCngKhG59vik1sBoAoHorcBz1k8wByHrRJeKrF9EXKAmkZWpeF8KTBEHY9VYWrCj2sqPG6ds74V9Fjn
    fn decode_swap_v1_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        graph: &Arc<Graph>,
    ) -> Result<DecodedInstruction> {
        let pool_address = *account_keys
            .get(usize::from(accounts[2]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        // let token_a_vault = *account_keys
        //     .get(usize::from(accounts[4]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        // let token_b_vault = *account_keys
        //     .get(usize::from(accounts[6]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let specified_amount: u64 = u64::from_le_bytes(data[0..8].try_into()?);
        let amount_threshold: u64 = u64::from_le_bytes(data[8..16].try_into()?);

        let sqrt_price_limit: u128 = u128::from_le_bytes(data[16..32].try_into()?);

        let is_exact_input: bool = data[32] == 1;
        let is_direct: bool = data[33] == 1;

        let node_in_index: usize;
        let node_out_index: usize;
        if let Some(edge) = graph.get_edge(&pool_address) {
            let node_lowest = edge.node_lowest;
            let node_highest = edge.node_highest;
            let is_reversed: bool = edge.reversed;

            if is_reversed == is_direct {
                (node_in_index, node_out_index) = (node_highest, node_lowest);
            } else {
                (node_in_index, node_out_index) = (node_lowest, node_highest);
            }
        } else {
            return Err(anyhow!("Unsupported Pool"));
        }

        let token_in_address = graph.nodes[node_in_index].address;
        let token_out_address = graph.nodes[node_out_index].address;

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

    //example: https://solscan.io/tx/2wexJNbVuuLRcka94SbMuf43kVWoKH9UpGA6Fc5xDzRC3yRATcKxsPMrNSkoifTSwV1ELDCb2PDDoTixHQ1Bs1uF
    fn decode_swap_v2_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        _graph: &Arc<Graph>,
    ) -> Result<DecodedInstruction> {
        let pool_address = *account_keys
            .get(usize::from(accounts[4]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        // let token_a_vault = *account_keys
        //     .get(usize::from(accounts[8]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        // let token_b_vault = *account_keys
        //     .get(usize::from(accounts[10]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let token_a_address = *account_keys
            .get(usize::from(accounts[5]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let token_b_address = *account_keys
            .get(usize::from(accounts[6]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let specified_amount: u64 = u64::from_le_bytes(data[0..8].try_into()?);
        let amount_threshold: u64 = u64::from_le_bytes(data[8..16].try_into()?);

        let sqrt_price_limit: u128 = u128::from_le_bytes(data[16..32].try_into()?);

        let is_exact_input: bool = data[32] == 1;

        let is_direct: bool = data[33] == 1;

        let (token_in_address, token_out_address): (Pubkey, Pubkey) = if is_direct {
            (token_a_address, token_b_address)
        } else {
            (token_b_address, token_a_address)
        };

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
}

const SWAP_V1: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
const SWAP_V1_ACCOUNTS_LEN: usize = 16;

const SWAP_V2: [u8; 8] = [43, 4, 237, 11, 26, 201, 30, 98];
const SWAP_V2_ACCOUNTS_LEN: usize = 15;

const REMOVE_LIQUIDITY: [u8; 8] = [2, 13, 19, 20, 0, 3, 4, 17];
const REMOVE_LIQUIDITY_ACCOUNTS_LEN: usize = 15;

const ADD_LIQUIDITY: [u8; 8] = [5, 15, 0, 3, 4, 2, 6, 7];
const ADD_LIQUIDITY_ACCOUNTS_LEN: usize = 11;
