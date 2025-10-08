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

pub struct RaydiumV2TargetTransaction;

impl TargetTransaction for RaydiumV2TargetTransaction {
    fn decode(
        &self,
        transaction: &VersionedTransaction,
        program_index: usize,
        _graph: &Arc<Graph>,
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
                SWAP_EXACT_IN => self.decode_swap_instruction(reader, accounts, account_keys, true),
                SWAP_EXACT_OUT => {
                    self.decode_swap_instruction(reader, accounts, account_keys, false)
                }
                _ => {
                    return Err(anyhow!("Unsupported swap instruction type on RaydiumV2"));
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
}

impl RaydiumV2TargetTransaction {
    //example swap_in: https://solscan.io/tx/4xutPirKuttSaqhpzWNx87H1f4LsJ4ju5k3KWNgm2DYUMUnMdyFY29ixTa4B6pMGegQXGZQuxRn2PqBTbfRcmSTA
    //example swap_out: https://solscan.io/tx/5ZTy84uUMbg66b9QK141Ad16gb8HqQNSzmqTDyKLT3iW6AUqhuUKg5j4rEb5Q1q1V1XJV6vJELYSdmgRe71M1qJr
    fn decode_swap_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        is_exact_in: bool,
    ) -> Result<DecodedInstruction> {
        if accounts.len() != SWAP_ACCOUNTS_LEN {
            return Err(anyhow!(
                "accounts len != SWAP_ACCOUNTS_LEN, received {} | expected {}",
                accounts.len(),
                SWAP_ACCOUNTS_LEN
            ));
        }

        let pool_address = *account_keys
            .get(usize::from(accounts[3]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let token_in_address = *account_keys
            .get(usize::from(accounts[10]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let token_out_address = *account_keys
            .get(usize::from(accounts[11]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let amount_1: u64 = u64::from_le_bytes(data[0..8].try_into()?);
        let amount_2: u64 = u64::from_le_bytes(data[8..16].try_into()?);

        if is_exact_in {
            Ok(DecodedInstruction {
                pool_address,
                token_in_address,
                token_out_address,
                operation_type: OperationType::SwapExactInput {
                    amount_in: amount_1,
                    minimum_amount_out: amount_2,
                    sqrt_price_limit: 0,
                },
            })
        } else {
            Ok(DecodedInstruction {
                pool_address,
                token_in_address,
                token_out_address,
                operation_type: OperationType::SwapExactOutput {
                    amount_out: amount_2,
                    maximum_amount_in: amount_1,
                    sqrt_price_limit: 0,
                },
            })
        }
    }

    //example: https://solscan.io/tx/3VwzRQTv8NKYUgttqnBw766yde2LiaLxGQLJoF2DzQXCxvHRy7rVqE84eeSiJA1EuHvuXcUHhGS4MKgd3crnEet7
    fn decode_add_liquidity_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<DecodedInstruction> {
        if accounts.len() != ADD_LIQUIDITY_ACCOUNTS_LEN {
            return Err(anyhow!(
                "accounts len != ADD_LIQUIDITY_ACCOUNTS_LEN, received {} | expected {}",
                accounts.len(),
                ADD_LIQUIDITY_ACCOUNTS_LEN
            ));
        }

        let pool_address = *account_keys
            .get(usize::from(accounts[4]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let token_a_address = *account_keys
            .get(usize::from(accounts[18]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let token_b_address = *account_keys
            .get(usize::from(accounts[19]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        //TODO: it is not complete, we need also to fetch tick range

        let token_a_amount = u64::from_le_bytes(data[32..40].try_into()?);
        let token_b_amount = u64::from_le_bytes(data[40..48].try_into()?);

        Ok(DecodedInstruction {
            pool_address,
            token_in_address: token_a_address,
            token_out_address: token_b_address,
            operation_type: OperationType::AddLiquidity {
                add_amount_a: token_a_amount,
                add_amount_b: token_b_amount,
            },
        })
    }
}

const SWAP_EXACT_OUT: [u8; 8] = [55, 217, 98, 86, 163, 74, 180, 173];

const SWAP_EXACT_IN: [u8; 8] = [143, 190, 90, 218, 196, 30, 51, 222];
const SWAP_ACCOUNTS_LEN: usize = 13;

const ADD_LIQUIDITY: [u8; 8] = [77, 255, 174, 82, 125, 29, 201, 46];
const ADD_LIQUIDITY_ACCOUNTS_LEN: usize = 20;
