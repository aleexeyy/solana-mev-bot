use std::io::Read;

use anyhow::{Result, anyhow};
use solana_sdk::{
    message::compiled_instruction::CompiledInstruction, pubkey::Pubkey,
    transaction::VersionedTransaction,
};

use crate::transaction_decoders::{
    DecodedInstruction, DecodedTransaction, OperationType, TargetTransaction,
};
pub struct OrcaV3TargetTransaction;

impl TargetTransaction for OrcaV3TargetTransaction {
    fn decode(
        &self,
        transaction: &VersionedTransaction,
        program_index: usize,
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
        let mut decoded_instructions: Vec<DecodedInstruction> =
            Vec::with_capacity(target_instructions.len());

        for instruction in target_instructions {
            let data = &instruction.data;
            let accounts = &instruction.accounts;

            let mut reader = data.as_slice();
            let mut instruction_type = [0u8; 8];
            reader.read_exact(&mut instruction_type)?;

            let decoded_instruction = match instruction_type {
                SWAP_V1 => self.decode_swap_instruction(reader, accounts, account_keys),
                SWAP_V2 => self.decode_swap_instruction(reader, accounts, account_keys),
                REMOVE_LIQUIDITY => {
                    self.decode_remove_liquidity_instruction(reader, accounts, account_keys)
                }
                ADD_LIQUIDITY => {
                    self.decode_add_liquidity_instruction(reader, accounts, account_keys)
                }
                _ => return Err(anyhow!("Unsupported swap instruction type on OrcaV3")),
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

    fn decode_swap_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<DecodedInstruction> {
        match accounts.len() {
            SWAP_V1_ACCOUNTS_LEN => self.decode_swap_v1_instruction(data, accounts, account_keys),
            SWAP_V2_ACCOUNTS_LEN => self.decode_swap_v2_instruction(data, accounts, account_keys),
            _ => Err(anyhow!("Unsupported swap instruction account length")),
        }
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

impl OrcaV3TargetTransaction {
    //example: https://solscan.io/tx/5FQXUNt7bTCngKhG59vik1sBoAoHorcBz1k8wByHrRJeKrF9EXKAmkZWpeF8KTBEHY9VYWrCj2sqPG6ds74V9Fjn
    fn decode_swap_v1_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<DecodedInstruction> {
        let pool_address = *account_keys
            .get(usize::from(accounts[2]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let token_a_vault = *account_keys
            .get(usize::from(accounts[4]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let token_b_vault = *account_keys
            .get(usize::from(accounts[6]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let token_a_address = Pubkey::new_from_array([0; 32]);
        let token_b_address = Pubkey::new_from_array([0; 32]);

        let specified_amount: u64 = u64::from_le_bytes(data[0..8].try_into()?);
        let amount_threshold: u64 = u64::from_le_bytes(data[8..16].try_into()?);

        let sqrt_price_limit: u128 = u128::from_le_bytes(data[16..32].try_into()?);

        let is_exact_input: bool = data[32] == 1;

        let is_direct: bool = data[33] == 1;

        let (token_in_address, token_out_address, token_in_vault, token_out_vault): (
            Pubkey,
            Pubkey,
            Pubkey,
            Pubkey,
        ) = if is_direct {
            (
                token_a_address,
                token_b_address,
                token_a_vault,
                token_b_vault,
            )
        } else {
            (
                token_b_address,
                token_a_address,
                token_b_vault,
                token_a_vault,
            )
        };

        if is_exact_input {
            Ok(DecodedInstruction {
                pool_address,
                token_in_address,
                token_out_address,
                token_in_vault,
                token_out_vault,
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
                token_in_vault,
                token_out_vault,
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
    ) -> Result<DecodedInstruction> {
        let pool_address = *account_keys
            .get(usize::from(accounts[4]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let token_a_vault = *account_keys
            .get(usize::from(accounts[8]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let token_b_vault = *account_keys
            .get(usize::from(accounts[10]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
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

        let (token_in_address, token_out_address, token_in_vault, token_out_vault): (
            Pubkey,
            Pubkey,
            Pubkey,
            Pubkey,
        ) = if is_direct {
            (
                token_a_address,
                token_b_address,
                token_a_vault,
                token_b_vault,
            )
        } else {
            (
                token_b_address,
                token_a_address,
                token_b_vault,
                token_a_vault,
            )
        };

        if is_exact_input {
            Ok(DecodedInstruction {
                pool_address,
                token_in_address,
                token_out_address,
                token_in_vault,
                token_out_vault,
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
                token_in_vault,
                token_out_vault,
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
