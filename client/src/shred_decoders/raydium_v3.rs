use std::{io::Read, sync::Arc};

use anyhow::{Result, anyhow};
use solana_sdk::pubkey::Pubkey;

use crate::{
    graph::Graph,
    shred_decoders::{
        TargetTransaction,
        interfaces::{DecodedInstruction, OperationType},
    },
};

pub struct RaydiumV3TargetTransaction;

impl TargetTransaction for RaydiumV3TargetTransaction {
    fn decode(
        &self,
        account_keys: &Arc<[Pubkey]>,
        accounts: &[u8],
        data: &[u8],
        graph: &Arc<Graph>,
    ) -> Result<DecodedInstruction> {
        let mut reader = data;
        let mut instruction_type = [0u8; 8];
        reader.read_exact(&mut instruction_type)?;

        let decoded_instruction = match instruction_type {
            SWAP_V1 => self.decode_swap_v1_instruction(reader, accounts, account_keys, graph),
            SWAP_V2 => self.decode_swap_v2_instruction(reader, accounts, account_keys),
            _ => {
                return Err(anyhow!("Unsupported swap instruction type on RaydiumV3"));
            }
        }?;

        Ok(decoded_instruction)
    }
}

impl RaydiumV3TargetTransaction {
    //example: https://solscan.io/tx/5KVaSKctetQ1TdmJxeQUiacnqV15H6Thtwdt7EtLjBggdd4yRtwVx7Zzj3E9mcYcSuTWwENDQGxbhJUvPF5z96CE
    fn decode_swap_v1_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        graph: &Arc<Graph>,
    ) -> Result<DecodedInstruction> {
        if accounts.len() < SWAP_V1_ACCOUNTS_LEN {
            return Err(anyhow!(
                "accounts len != SWAP_V1_ACCOUNTS_LEN, received {} | expected {}",
                accounts.len(),
                SWAP_V1_ACCOUNTS_LEN
            ));
        }
        let pool_address = *account_keys
            .get(usize::from(accounts[2]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let input_vault = *account_keys
            .get(usize::from(accounts[5]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let specified_amount: u64 = u64::from_le_bytes(data[0..8].try_into()?);
        let amount_threshold: u64 = u64::from_le_bytes(data[8..16].try_into()?);

        let sqrt_price_limit: u128 = u128::from_le_bytes(data[16..32].try_into()?);

        let is_exact_input: bool = data[32] == 1;

        let node_in_index: usize;
        let node_out_index: usize;

        if let Some(edge) = graph.get_edge(&pool_address) {
            let node_lowest = edge.node_lowest;
            let node_highest = edge.node_highest;
            let vault_lowest = edge.token_vault_lowest;

            if input_vault == vault_lowest {
                (node_in_index, node_out_index) = (node_lowest, node_highest);
            } else {
                (node_in_index, node_out_index) = (node_highest, node_lowest);
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

    //example: https://solscan.io/tx/2j7ikfSmJ1AMt979tHmqKQGdv5KiBppveHoQTdhjxLkVkr3FjKf9C7kACkhAF6zUX3UZepumuQJzJMcWQESfAPyV
    fn decode_swap_v2_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<DecodedInstruction> {
        if accounts.len() < SWAP_V2_ACCOUNTS_LEN {
            return Err(anyhow!(
                "accounts len != SWAP_V2_ACCOUNTS_LEN, received {} | expected {}",
                accounts.len(),
                SWAP_V2_ACCOUNTS_LEN
            ));
        }
        let pool_address = *account_keys
            .get(usize::from(accounts[2]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

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

    //example: https://solscan.io/tx/3EeKhraYxNGbzotcDGzRhrgkdips7T5CTbyFpd4hJ5DM9XuRVANRfoqZKotJwL44bRnYDAXuVBAkgD9QGjzGaxsz
    fn decode_remove_liquidity_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        graph: &Arc<Graph>,
    ) -> Result<DecodedInstruction> {
        todo!()
    }

    //example: https://solscan.io/tx/5LQEmeRxwXoJ5qowbf44PS8d9e8hxtG4zWoTzDntoJiXcJPENnBmnV1PSyVCMn6ZHhjMoKWb4QutWiYp3ZA8dSXA
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

const SWAP_V1: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
const SWAP_V1_ACCOUNTS_LEN: usize = 6;

const SWAP_V2: [u8; 8] = [43, 4, 237, 11, 26, 201, 30, 98];
const SWAP_V2_ACCOUNTS_LEN: usize = 13;
