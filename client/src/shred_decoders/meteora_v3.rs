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

pub struct MeteoraV3TargetTransaction;

impl TargetTransaction for MeteoraV3TargetTransaction {
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
            SWAP => {
                self.decode_swap_instruction(reader, accounts, account_keys, market, lookup_tables)
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

        Ok(ShredEvent::new(
            signature,
            slot,
            pool_address,
            instruction_index,
            event,
        ))
    }
}

impl MeteoraV3TargetTransaction {
    //example: https://solscan.io/tx/2HnkYb6vS1Uuy9CY8K9Qi8jyVyXJ8cd4XhHuFwNFF5j2d6uC9bDXLytjjES3b4aCcvKq8Tz3LuMPtiQKANfYyqTt
    fn decode_swap_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        market: &MarketGraph,
        lookup_tables: &Arc<Vec<ReducedLookupTable>>,
    ) -> Result<(Pubkey, ShredEventType)> {
        if accounts.len() < SWAP_ACCOUNTS_LEN {
            return Err(anyhow!(
                "accounts len != SWAP_ACCOUNTS_LEN, received {} | expected {}",
                accounts.len(),
                SWAP_ACCOUNTS_LEN
            ));
        }
        let pool_address_index = usize::from(accounts[1]);
        let pool_address =
            DecodingUtils::get_account_address(pool_address_index, account_keys, lookup_tables)?;
        // let pool_address = *account_keys
        //     .get(usize::from(accounts[1]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let token_a_address_index = usize::from(accounts[6]);
        let token_a_address =
            DecodingUtils::get_account_address(token_a_address_index, account_keys, lookup_tables)?;

        let token_b_address_index = usize::from(accounts[7]);
        let token_b_address =
            DecodingUtils::get_account_address(token_b_address_index, account_keys, lookup_tables)?;
        // let token_a_address = *account_keys
        //     .get(usize::from(accounts[6]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        // let token_b_address = *account_keys
        //     .get(usize::from(accounts[7]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        let owner_index = usize::from(accounts[8]);
        let owner = DecodingUtils::get_account_address(owner_index, account_keys, lookup_tables)?;
        // let owner = *account_keys
        //     .get(usize::from(accounts[8]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let token_in_account_address_index = usize::from(accounts[2]);
        let token_in_account_address = DecodingUtils::get_account_address(
            token_in_account_address_index,
            account_keys,
            lookup_tables,
        )?;
        // let token_in_account_address = *account_keys
        //     .get(usize::from(accounts[2]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

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

        Ok((
            pool_address,
            ShredEventType::Swap {
                amount_specified: amount_in,
                limit: SwapConstraint::TokenAmountLimit(minimum_amount_out),
                a_to_b,
                is_base_input: true,
            },
        ))
    }
}

impl MeteoraV3TargetTransaction {}

//maybe it is to create position
const FILTER_2: [u8; 8] = [48, 215, 197, 153, 96, 203, 180, 133];

const CLAIM_FEES: [u8; 8] = [180, 38, 154, 17, 133, 33, 162, 211];

const CREATE_POOL: [u8; 8] = [95, 180, 10, 172, 84, 174, 232, 40];

const CREATE_POOL_2: [u8; 8] = [20, 161, 241, 24, 189, 221, 180, 2];

const SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
const SWAP_ACCOUNTS_LEN: usize = 9;
