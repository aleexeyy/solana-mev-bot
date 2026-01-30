use std::{io::Read, sync::Arc};

use anyhow::{Result, anyhow};
use solana_sdk::{pubkey::Pubkey, signature::Signature};

use crate::{
    graph::market_graph::{MarketGraph, TokenId},
    shred_decoders::{
        TargetTransaction,
        interfaces::{ReducedLookupTable, ShredEvent, ShredEventType, SwapConstraint},
        utils::DecodingUtils,
    },
};

pub struct MeteoraV2TargetTransaction;

impl TargetTransaction for MeteoraV2TargetTransaction {
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
                self.decode_swap_instruction(data, accounts, account_keys, market, lookup_tables)
            }
            _ => {
                tracing::warn!("Unhandled instruction type {:?}", instruction_type);
                tracing::warn!("Transaction on MeteoraV2 {:?}", &data);
                Err(anyhow!("Unsupported instruction type"))
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

impl MeteoraV2TargetTransaction {
    //example: https://solscan.io/tx/2BboLatABm5hV8V9uBpSUtY6LXRxa6XBoCAephQkFnfwiEiLEja4fC8MFnQ5L7UV5NPvvHFVjy2K6kHoUAbW5miQ
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
        let pool_address_index = usize::from(accounts[0]);
        let pool_address =
            DecodingUtils::get_account_address(pool_address_index, account_keys, lookup_tables)?;
        // let pool_address = *account_keys
        //     .get(usize::from(accounts[0]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let token_in_account_address_index = usize::from(accounts[1]);
        let token_in_account_address = DecodingUtils::get_account_address(
            token_in_account_address_index,
            account_keys,
            lookup_tables,
        )?;
        // let token_in_account_address = *account_keys
        //     .get(usize::from(accounts[1]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let owner_index = usize::from(accounts[12]);
        let owner = DecodingUtils::get_account_address(owner_index, account_keys, lookup_tables)?;
        // let owner = *account_keys
        //     .get(usize::from(accounts[12]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let amount_in: u64 = u64::from_le_bytes(data[0..8].try_into()?);
        let minimum_amount_out: u64 = u64::from_le_bytes(data[8..16].try_into()?);

        let edge = market
            .get_edge(&pool_address)
            .ok_or_else(|| anyhow!("Unsupported Pool"))?;
        let node_a_index: TokenId = edge.node_a;
        let node_b_index: TokenId = edge.node_b;

        let token_a_address = market
            .token_address(node_a_index)
            .ok_or_else(|| anyhow!("Invalid token_a node index"))?;
        let token_b_address = market
            .token_address(node_b_index)
            .ok_or_else(|| anyhow!("Invalid token_b node index"))?;

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

const SWAP: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
const SWAP_ACCOUNTS_LEN: usize = 13;
