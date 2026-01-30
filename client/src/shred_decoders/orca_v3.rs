use std::{io::Read, sync::Arc};

use anyhow::{Result, anyhow};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

use crate::{
    graph::market_graph::MarketGraph,
    shred_decoders::{
        TargetTransaction,
        interfaces::{ReducedLookupTable, ShredEvent, ShredEventType, SwapConstraint},
        utils::DecodingUtils,
    },
};

pub struct OrcaV3TargetTransaction;

impl TargetTransaction for OrcaV3TargetTransaction {
    fn decode(
        &self,
        slot: u64,
        signature: Signature,
        instruction_index: u8,
        account_keys: &Arc<[Pubkey]>,
        accounts: &[u8],
        data: &[u8],
        market: &MarketGraph,
        lookup_tables: &Arc<Vec<ReducedLookupTable>>,
    ) -> Result<ShredEvent> {
        let mut reader = data;
        let mut instruction_type = [0u8; 8];
        reader.read_exact(&mut instruction_type)?;

        let (pool_address, event) = match instruction_type {
            SWAP_V1 => self.decode_swap_v1_instruction(
                reader,
                accounts,
                account_keys,
                market,
                lookup_tables,
            ),
            SWAP_V2 => self.decode_swap_v2_instruction(
                reader,
                accounts,
                account_keys,
                market,
                lookup_tables,
            ),
            _ => {
                return Err(anyhow!("Unsupported swap instruction type on OrcaV3"));
            }
        }?;

        Ok(ShredEvent::new(
            signature,
            slot,
            pool_address,
            instruction_index,
            event,
        ))
    }
}

impl OrcaV3TargetTransaction {
    //example: https://solscan.io/tx/5FQXUNt7bTCngKhG59vik1sBoAoHorcBz1k8wByHrRJeKrF9EXKAmkZWpeF8KTBEHY9VYWrCj2sqPG6ds74V9Fjn
    fn decode_swap_v1_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        market: &MarketGraph,
        lookup_tables: &Arc<Vec<ReducedLookupTable>>,
    ) -> Result<(Pubkey, ShredEventType)> {
        if accounts.len() < SWAP_V1_ACCOUNTS_LEN {
            return Err(anyhow!(
                "accounts len != SWAP_V1_ACCOUNTS_LEN, received {} | expected {}",
                accounts.len(),
                SWAP_V1_ACCOUNTS_LEN
            ));
        }
        let pool_address_index = usize::from(accounts[2]);

        let pool_address =
            DecodingUtils::get_account_address(pool_address_index, account_keys, lookup_tables)?;
        // let pool_address = *account_keys
        //     .get(usize::from(accounts[2]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let specified_amount: u64 = u64::from_le_bytes(data[0..8].try_into()?);
        let _amount_threshold: u64 = u64::from_le_bytes(data[8..16].try_into()?);

        let sqrt_price_limit: u128 = u128::from_le_bytes(data[16..32].try_into()?);

        let is_exact_input: bool = data[32] == 1;
        let is_direct: bool = data[33] == 1;

        let a_to_b = is_direct;
        let _ = market
            .get_edge(&pool_address)
            .ok_or_else(|| anyhow!("Unsupported Pool"))?;

        Ok((
            pool_address,
            ShredEventType::Swap {
                amount_specified: specified_amount,
                limit: SwapConstraint::SqrtPriceLimit(sqrt_price_limit),
                a_to_b,
                is_base_input: is_exact_input,
            },
        ))
    }

    //example: https://solscan.io/tx/2wexJNbVuuLRcka94SbMuf43kVWoKH9UpGA6Fc5xDzRC3yRATcKxsPMrNSkoifTSwV1ELDCb2PDDoTixHQ1Bs1uF
    fn decode_swap_v2_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        market: &MarketGraph,
        lookup_tables: &Arc<Vec<ReducedLookupTable>>,
    ) -> Result<(Pubkey, ShredEventType)> {
        if accounts.len() < SWAP_V2_ACCOUNTS_LEN {
            return Err(anyhow!(
                "accounts len != SWAP_V1_ACCOUNTS_LEN, received {} | expected {}",
                accounts.len(),
                SWAP_V2_ACCOUNTS_LEN
            ));
        }

        let pool_address_index = usize::from(accounts[4]);
        let pool_address =
            DecodingUtils::get_account_address(pool_address_index, account_keys, lookup_tables)?;
        // let pool_address = *account_keys
        //     .get(usize::from(accounts[4]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let token_a_address_index = usize::from(accounts[5]);
        let token_a_address =
            DecodingUtils::get_account_address(token_a_address_index, account_keys, lookup_tables)?;

        let token_b_address_index = usize::from(accounts[6]);
        let token_b_address =
            DecodingUtils::get_account_address(token_b_address_index, account_keys, lookup_tables)?;
        // let token_a_address = *account_keys
        //     .get(usize::from(accounts[5]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        // let token_b_address = *account_keys
        //     .get(usize::from(accounts[6]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let specified_amount: u64 = u64::from_le_bytes(data[0..8].try_into()?);
        let amount_threshold: u64 = u64::from_le_bytes(data[8..16].try_into()?);

        let sqrt_price_limit: u128 = u128::from_le_bytes(data[16..32].try_into()?);

        let is_exact_input: bool = data[32] == 1;

        let is_direct: bool = data[33] == 1;

        let token_in_address: Pubkey = if is_direct {
            token_a_address
        } else {
            token_b_address
        };
        let token_out_address: Pubkey = if is_direct {
            token_b_address
        } else {
            token_a_address
        };

        let edge = market
            .get_edge(&pool_address)
            .ok_or_else(|| anyhow!("Unsupported Pool"))?;
        let token_a = market
            .token_address(edge.node_a)
            .ok_or_else(|| anyhow!("Invalid token_a node index"))?;
        let token_b = market
            .token_address(edge.node_b)
            .ok_or_else(|| anyhow!("Invalid token_b node index"))?;

        let a_to_b = match token_in_address {
            _ if token_in_address == token_a && token_out_address == token_b => true,
            _ if token_in_address == token_b && token_out_address == token_a => false,
            _ => return Err(anyhow!("Swap token mints do not match pool")),
        };

        let _ = amount_threshold;

        Ok((
            pool_address,
            ShredEventType::Swap {
                amount_specified: specified_amount,
                limit: SwapConstraint::SqrtPriceLimit(sqrt_price_limit),
                a_to_b,
                is_base_input: is_exact_input,
            },
        ))
    }
}

const SWAP_V1: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
const SWAP_V1_ACCOUNTS_LEN: usize = 3;

const SWAP_V2: [u8; 8] = [43, 4, 237, 11, 26, 201, 30, 98];
const SWAP_V2_ACCOUNTS_LEN: usize = 7;
