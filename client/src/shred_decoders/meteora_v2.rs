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
        utils::DecodingUtils,
    },
};

pub struct MeteoraV2TargetTransaction;

impl TargetTransaction for MeteoraV2TargetTransaction {
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

        if target_instructions.len() == 0 {
            return Err(anyhow!("Unsupported instructions"));
        }

        let account_keys = transaction.message.static_account_keys();

        let mut decoded_instructions = Vec::with_capacity(target_instructions.len());

        for instruction in target_instructions {
            let data = &instruction.data;
            let accounts = &instruction.accounts;

            let mut reader = data.as_slice();
            let mut instruction_type = [0u8; 8];
            reader.read_exact(&mut instruction_type)?;

            let decoded_instruction = match instruction_type {
                SWAP => self.decode_swap_instruction(data, accounts, account_keys, graph),
                REMOVE_LIQUIDITY_SINGLE_SIDE => Err(anyhow!("Unsupported instruction type")),
                ADD_IMBALANCE_LIQUIDITY => Err(anyhow!("Unsupported instruction type")),
                REMOVE_BALANCE_LIQUIDITY => Err(anyhow!("Unsupported instruction type")),
                ADD_BALANCE_LIQUIDITY => Err(anyhow!("Unsupported instruction type")),
                CLAIM_FEES => Err(anyhow!("Unsupported instruction type")),
                _ => {
                    tracing::warn!("Unhandled instruction type {:?}", instruction_type);
                    tracing::warn!("Transaction on MeteoraV2 {:?}", &transaction);
                    Err(anyhow!("Unsupported instruction type"))
                }
            }?;

            decoded_instructions.push(decoded_instruction);
        }

        if decoded_instructions.len() == 0 {
            return Err(anyhow!("Unsupported instructions"));
        }

        let decoded_transaction = DecodedTransaction {
            instructions: decoded_instructions,
        };

        Ok(decoded_transaction)
    }
}

impl MeteoraV2TargetTransaction {
    //example: https://solscan.io/tx/2BboLatABm5hV8V9uBpSUtY6LXRxa6XBoCAephQkFnfwiEiLEja4fC8MFnQ5L7UV5NPvvHFVjy2K6kHoUAbW5miQ
    fn decode_swap_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        graph: &Arc<Graph>,
    ) -> Result<DecodedInstruction> {
        if accounts.len() != SWAP_ACCOUNTS_LEN {
            return Err(anyhow!(
                "accounts len != SWAP_ACCOUNTS_LEN, received {} | expected {}",
                accounts.len(),
                SWAP_ACCOUNTS_LEN
            ));
        }

        let pool_address = *account_keys
            .get(usize::from(accounts[0]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let token_in_account_address = *account_keys
            .get(usize::from(accounts[1]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let owner = *account_keys
            .get(usize::from(accounts[12]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let amount_in: u64 = u64::from_le_bytes(data[0..8].try_into()?);
        let minimum_amount_out: u64 = u64::from_le_bytes(data[8..16].try_into()?);

        let node_a_index: usize;
        let node_b_index: usize;
        if let Some(edge) = graph.get_edge(&pool_address) {
            let node_lowest = edge.node_lowest;
            let node_highest = edge.node_highest;
            let is_reversed: bool = edge.reversed;

            if is_reversed {
                (node_a_index, node_b_index) = (node_highest, node_lowest);
            } else {
                (node_a_index, node_b_index) = (node_lowest, node_highest);
            }
        } else {
            return Err(anyhow!("Unsupported Pool"));
        }

        let token_a_address = graph.nodes[node_a_index].address;
        let token_b_address = graph.nodes[node_b_index].address;

        let token_a_account_address =
            DecodingUtils::find_token_account_address(&owner, &token_a_address);

        tracing::trace!(
            "token_a_account_address: {:?} | token_in_account_address: {:?}",
            token_a_account_address,
            token_in_account_address
        );

        let (token_in_address, token_out_address) =
            if token_a_account_address == token_in_account_address {
                (token_a_address, token_b_address)
            } else {
                (token_b_address, token_a_address)
            };

        Ok(DecodedInstruction {
            pool_address,
            token_in_address,
            token_out_address,
            operation_type: OperationType::SwapExactInput {
                amount_in,
                minimum_amount_out,
                sqrt_price_limit: 0,
            },
        })
    }

    fn decode_remove_liquidity_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<DecodedInstruction> {
        if accounts.len() != REMOVE_LIQUIDITY_SINGLE_SIDE_ACCOUNTS_LEN {
            return Err(anyhow!(
                "accounts len != REMOVE_LIQUIDITY_SINGLE_SIDE_ACCOUNTS_LEN, received {} | expected {}",
                accounts.len(),
                REMOVE_LIQUIDITY_SINGLE_SIDE_ACCOUNTS_LEN
            ));
        }

        let pool_address = *account_keys
            .get(usize::from(accounts[0]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let token_a_address = *account_keys
            .get(usize::from(accounts[11]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let token_b_address = Pubkey::new_from_array([0u8; 32]);

        let pool_token_amount = u64::from_le_bytes(data[0..8].try_into()?);
        let minimum_amount_out = u64::from_le_bytes(data[8..16].try_into()?);

        Ok(DecodedInstruction {
            pool_address,
            token_in_address: token_a_address,
            token_out_address: token_b_address,
            operation_type: OperationType::RemoveLiquidity {
                remove_amount_a: pool_token_amount,
                remove_amount_b: minimum_amount_out,
            },
        })
    }
    fn decode_add_liquidity_instruction(
        //TODO: It is not correctly implemented, find transaction and fix
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<DecodedInstruction> {
        if accounts.len() != ADD_IMBALANCE_LIQUIDITY_ACCOUNTS_LEN {
            return Err(anyhow!(
                "accounts len != ADD_IMBALANCE_LIQUIDITY_ACCOUNTS_LEN, received {} | expected {}",
                accounts.len(),
                ADD_IMBALANCE_LIQUIDITY_ACCOUNTS_LEN
            ));
        }

        let pool_address = *account_keys
            .get(usize::from(accounts[0]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let token_a_address = *account_keys
            .get(usize::from(accounts[11]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let token_b_address = Pubkey::new_from_array([0u8; 32]);

        let pool_token_amount = u64::from_le_bytes(data[0..8].try_into()?);
        let minimum_amount_out = u64::from_le_bytes(data[8..16].try_into()?);

        Ok(DecodedInstruction {
            pool_address,
            token_in_address: token_a_address,
            token_out_address: token_b_address,
            operation_type: OperationType::AddLiquidity {
                add_amount_a: pool_token_amount,
                add_amount_b: minimum_amount_out,
            },
        })
    }
}

const SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
const SWAP_ACCOUNTS_LEN: usize = 15;

const REMOVE_LIQUIDITY_SINGLE_SIDE: [u8; 8] = [84, 84, 177, 66, 254, 185, 10, 251];
const REMOVE_LIQUIDITY_SINGLE_SIDE_ACCOUNTS_LEN: usize = 15;

const ADD_IMBALANCE_LIQUIDITY: [u8; 8] = [79, 35, 122, 84, 173, 15, 93, 191];
const ADD_IMBALANCE_LIQUIDITY_ACCOUNTS_LEN: usize = 16;

const REMOVE_BALANCE_LIQUIDITY: [u8; 8] = [133, 109, 44, 179, 56, 238, 114, 33];
const REMOVE_BALANCE_LIQUIDITY_ACCOUNTS_LEN: usize = 16;

const ADD_BALANCE_LIQUIDITY: [u8; 8] = [168, 227, 50, 62, 189, 171, 84, 176];
const ADD_BALANCE_LIQUIDITY_ACCOUNTS_LEN: usize = 16;

//REFERENCE: https://github.com/Shyft-to/solana-defi/blob/main/Meteora/Rust/stream_and_parse_meteora_pools_accounts/parsers/meteora_pools_interface/src/instructions.rs#L2334
