use std::{io::Read, sync::Arc};

use anyhow::{Result, anyhow};
use solana_sdk::pubkey::Pubkey;

use crate::{
    graph::Graph,
    shred_decoders::{
        TargetTransaction,
        interfaces::{DecodedInstruction, OperationType},
        utils::DecodingUtils,
    },
};

pub struct MeteoraV3TargetTransaction;

impl TargetTransaction for MeteoraV3TargetTransaction {
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
            SWAP => self.decode_swap_instruction(reader, accounts, account_keys, graph),
            ADD_LIQUIDITY => {
                return Err(anyhow!("Unsupported swap instruction type"));
            }
            REMOVE_LIQUIDITY => {
                return Err(anyhow!("Unsupported swap instruction type"));
            }
            REMOVE_ALL_LIQUIDITY => {
                return Err(anyhow!("Unsupported swap instruction type"));
            }
            CLAIM_FEES => {
                return Err(anyhow!("Unsupported swap instruction type"));
            }
            CREATE_POOL => {
                return Err(anyhow!("Unsupported swap instruction type"));
            }
            CREATE_POOL_2 => {
                return Err(anyhow!("Unsupported swap instruction type"));
            }
            FILTER_2 => {
                return Err(anyhow!("Unsupported swap instruction type"));
            }
            _ => {
                tracing::warn!("Unsupported instruction type {:?}", instruction_type);
                tracing::warn!("Transaction on MeteoraV3 {:?}", &data);
                return Err(anyhow!("Unsupported swap instruction type"));
            }
        }?;
        Ok(decoded_instruction)
    }
}

impl MeteoraV3TargetTransaction {
    //example: https://solscan.io/tx/2HnkYb6vS1Uuy9CY8K9Qi8jyVyXJ8cd4XhHuFwNFF5j2d6uC9bDXLytjjES3b4aCcvKq8Tz3LuMPtiQKANfYyqTt
    fn decode_swap_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        _graph: &Arc<Graph>,
    ) -> Result<DecodedInstruction> {
        if accounts.len() < SWAP_ACCOUNTS_LEN {
            return Err(anyhow!(
                "accounts len != SWAP_ACCOUNTS_LEN, received {} | expected {}",
                accounts.len(),
                SWAP_ACCOUNTS_LEN
            ));
        }

        let pool_address = *account_keys
            .get(usize::from(accounts[1]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let token_a_address = *account_keys
            .get(usize::from(accounts[6]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let token_b_address = *account_keys
            .get(usize::from(accounts[7]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let owner = *account_keys
            .get(usize::from(accounts[8]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let token_in_account_address = *account_keys
            .get(usize::from(accounts[2]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let token_a_account_address =
            DecodingUtils::find_token_account_address(&owner, &token_a_address);

        let (token_in_address, token_out_address) =
            if token_a_account_address == token_in_account_address {
                (token_a_address, token_b_address)
            } else {
                (token_b_address, token_a_address)
            };

        let amount_in: u64 = u64::from_le_bytes(data[0..8].try_into()?);
        let minimum_amount_out: u64 = u64::from_le_bytes(data[8..16].try_into()?);

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
        graph: &Arc<Graph>,
    ) -> Result<DecodedInstruction> {
        if accounts.len() != REMOVE_LIQUIDITY_ACCOUNTS_LEN {
            return Err(anyhow!(
                "accounts len != REMOVE_LIQUIDITY_ACCOUNTS_LEN, received {} | expected {}",
                accounts.len(),
                REMOVE_LIQUIDITY_ACCOUNTS_LEN
            ));
        }

        let pool_address = *account_keys
            .get(usize::from(accounts[1]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let token_a_address = *account_keys
            .get(usize::from(accounts[7]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let token_b_address = *account_keys
            .get(usize::from(accounts[8]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let token_a_amount: u64 = u64::from_le_bytes(data[0..8].try_into()?);
        let token_b_amount: u64 = u64::from_le_bytes(data[8..16].try_into()?);

        Ok(DecodedInstruction {
            pool_address,
            token_in_address: token_a_address,
            token_out_address: token_b_address,
            // token_in_vault: token_a_vault,
            // token_out_vault: token_b_vault,
            operation_type: OperationType::RemoveLiquidity {
                remove_amount_a: token_a_amount,
                remove_amount_b: token_b_amount,
            },
        })
    }

    // example: https://solscan.io/tx/ejb7H6Ay3CTeEXmetbdcxwLD89G1z7pdVXmqwrtwJjBi1zodv3KiTV48rC6y84yDYS19Ldwdksy9P32qJQmkFc9
    fn decode_add_liquidity_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        graph: &Arc<Graph>,
    ) -> Result<DecodedInstruction> {
        if accounts.len() != ADD_LIQUIDITY_ACCOUNTS_LEN {
            return Err(anyhow!(
                "accounts len != ADD_LIQUIDITY_ACCOUNTS_LEN, received {} | expected {}",
                accounts.len(),
                ADD_LIQUIDITY_ACCOUNTS_LEN
            ));
        }

        let pool_address = *account_keys
            .get(usize::from(accounts[0]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let token_a_address = *account_keys
            .get(usize::from(accounts[6]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let token_b_address = *account_keys
            .get(usize::from(accounts[7]))
            .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let liquidity_delta: u128 = u128::from_le_bytes(data[0..16].try_into()?); //TODO: check if we need that
        let token_a_amount: u64 = u64::from_le_bytes(data[16..24].try_into()?);
        let token_b_amount: u64 = u64::from_le_bytes(data[24..32].try_into()?);

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

impl MeteoraV3TargetTransaction {}

const ADD_LIQUIDITY: [u8; 8] = [181, 157, 89, 67, 143, 182, 52, 72];
const ADD_LIQUIDITY_ACCOUNTS_LEN: usize = 14;

const REMOVE_ALL_LIQUIDITY: [u8; 8] = [10, 51, 61, 35, 112, 105, 24, 85];
// const REMOVE_ALL_LIQUIDITY_ACCOUNTS_LEN: usize = 15;

const REMOVE_LIQUIDITY: [u8; 8] = [80, 85, 209, 72, 24, 206, 177, 108];
const REMOVE_LIQUIDITY_ACCOUNTS_LEN: usize = 15;

//maybe it is to create position
const FILTER_2: [u8; 8] = [48, 215, 197, 153, 96, 203, 180, 133];

const CLAIM_FEES: [u8; 8] = [180, 38, 154, 17, 133, 33, 162, 211];

const CREATE_POOL: [u8; 8] = [95, 180, 10, 172, 84, 174, 232, 40];

const CREATE_POOL_2: [u8; 8] = [20, 161, 241, 24, 189, 221, 180, 2];

const SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
const SWAP_ACCOUNTS_LEN: usize = 9;
