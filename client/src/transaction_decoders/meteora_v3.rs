use std::io::Read;

use anyhow::{Result, anyhow};
use solana_sdk::{
    message::compiled_instruction::CompiledInstruction, pubkey::Pubkey,
    transaction::VersionedTransaction,
};

use crate::transaction_decoders::{DecodedInstruction, OperationType, TargetTransaction};

pub struct MeteoraV3TargetTransaction;

// DecodedTransaction -> Vec[DecodedInstruction] with common Interface for every DEX

impl TargetTransaction for MeteoraV3TargetTransaction {
    fn decode(&self, transaction: &VersionedTransaction, program_index: usize) -> Result<()> {
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

        for instruction in target_instructions {
            let data = &instruction.data;
            let accounts = &instruction.accounts;

            let mut reader = data.as_slice();
            let mut instruction_type = [0u8; 8];
            reader.read_exact(&mut instruction_type)?;

            let result = match instruction_type {
                SWAP => self.decode_swap_instruction(reader, accounts, account_keys),
                ADD_LIQUIDITY => {
                    self.decode_add_liquidity_instruction(reader, accounts, account_keys)
                }
                REMOVE_LIQUIDITY => {
                    self.decode_remove_liquidity_instruction(reader, accounts, account_keys)
                }
                REMOVE_ALL_LIQUIDITY => {
                    self.decode_remove_liquidity_instruction(reader, accounts, account_keys)
                }
                _ => return Err(anyhow!("Unsupported swap instruction type")),
            }?;
        }

        Ok(())
    }

    fn decode_swap_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<DecodedInstruction> {
        if accounts.len() != SWAP_ACCOUNTS_LEN {
            return Err(anyhow!(
                "accounts len != SWAP_ACCOUNTS_LEN, received {} | expected {}",
                accounts.len(),
                SWAP_ACCOUNTS_LEN
            ));
        }

        let pool_address = account_keys[usize::from(accounts[1])];
        let token_a_vault = account_keys[usize::from(accounts[4])];
        let token_b_vault = account_keys[usize::from(accounts[5])];
        let token_a_address = account_keys[usize::from(accounts[6])];
        let token_b_address = account_keys[usize::from(accounts[7])];

        let amount_in: u64 = u64::from_le_bytes(data[8..16].try_into()?);
        let minimum_amount_out: u64 = u64::from_le_bytes(data[16..24].try_into()?);

        Ok(DecodedInstruction {
            pool_address,
            token_a_address,
            token_b_address,
            token_a_vault,
            token_b_vault,
            operation_type: OperationType::Swap,
            change_liquidity_a: amount_in,
            change_liquidity_b: minimum_amount_out,
        })
    }

    fn decode_remove_liquidity_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
    ) -> Result<DecodedInstruction> {
        if accounts.len() != REMOVE_LIQUIDITY_ACCOUNTS_LEN {
            return Err(anyhow!(
                "accounts len != REMOVE_LIQUIDITY_ACCOUNTS_LEN, received {} | expected {}",
                accounts.len(),
                REMOVE_LIQUIDITY_ACCOUNTS_LEN
            ));
        }

        let pool_address = account_keys[usize::from(accounts[1])];
        let token_a_vault = account_keys[usize::from(accounts[5])];
        let token_b_vault = account_keys[usize::from(accounts[6])];
        let token_a_address = account_keys[usize::from(accounts[7])];
        let token_b_address = account_keys[usize::from(accounts[8])];

        let token_a_amount: u64 = u64::from_le_bytes(data[8..16].try_into()?);
        let token_b_amount: u64 = u64::from_le_bytes(data[16..24].try_into()?);

        Ok(DecodedInstruction {
            pool_address,
            token_a_address,
            token_b_address,
            token_a_vault,
            token_b_vault,
            operation_type: OperationType::RemoveLiquidity,
            change_liquidity_a: token_a_amount,
            change_liquidity_b: token_b_amount,
        })
    }

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

        let pool_address = account_keys[usize::from(accounts[0])];
        let token_a_vault = account_keys[usize::from(accounts[4])];
        let token_b_vault = account_keys[usize::from(accounts[5])];
        let token_a_address = account_keys[usize::from(accounts[6])];
        let token_b_address = account_keys[usize::from(accounts[7])];

        let liquidity_delta: u128 = u128::from_le_bytes(data[8..24].try_into()?);
        let token_a_amount: u64 = u64::from_le_bytes(data[24..32].try_into()?);
        let token_b_amount: u64 = u64::from_le_bytes(data[32..40].try_into()?);

        Ok(DecodedInstruction {
            pool_address,
            token_a_address,
            token_b_address,
            token_a_vault,
            token_b_vault,
            operation_type: OperationType::AddLiquidity,
            change_liquidity_a: token_a_amount,
            change_liquidity_b: token_b_amount,
        })
    }
}

const ADD_LIQUIDITY: [u8; 8] = [181, 157, 89, 67, 143, 182, 52, 72];
const ADD_LIQUIDITY_ACCOUNTS_LEN: usize = 14;

const REMOVE_ALL_LIQUIDITY: [u8; 8] = [10, 51, 61, 35, 112, 105, 24, 85];
// const REMOVE_ALL_LIQUIDITY_ACCOUNTS_LEN: usize = 15;

const REMOVE_LIQUIDITY: [u8; 8] = [80, 85, 209, 72, 24, 206, 177, 108];
const REMOVE_LIQUIDITY_ACCOUNTS_LEN: usize = 15;

const SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
const SWAP_ACCOUNTS_LEN: usize = 14;
