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

pub struct RaydiumV2TargetTransaction;

impl TargetTransaction for RaydiumV2TargetTransaction {
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
            SWAP_EXACT_IN => self.decode_swap_instruction(
                reader,
                accounts,
                account_keys,
                true,
                market,
                lookup_tables,
            ),
            SWAP_EXACT_OUT => self.decode_swap_instruction(
                reader,
                accounts,
                account_keys,
                false,
                market,
                lookup_tables,
            ),
            _ => {
                return Err(anyhow!("Unsupported swap instruction type on RaydiumV2"));
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

impl RaydiumV2TargetTransaction {
    //example swap_in: https://solscan.io/tx/4xutPirKuttSaqhpzWNx87H1f4LsJ4ju5k3KWNgm2DYUMUnMdyFY29ixTa4B6pMGegQXGZQuxRn2PqBTbfRcmSTA
    //example swap_out: https://solscan.io/tx/5ZTy84uUMbg66b9QK141Ad16gb8HqQNSzmqTDyKLT3iW6AUqhuUKg5j4rEb5Q1q1V1XJV6vJELYSdmgRe71M1qJr
    fn decode_swap_instruction(
        &self,
        data: &[u8],
        accounts: &[u8],
        account_keys: &[Pubkey],
        is_base_input: bool,
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
        let pool_address_index = usize::from(accounts[3]);
        let pool_address =
            DecodingUtils::get_account_address(pool_address_index, account_keys, lookup_tables)?;
        // let pool_address = *account_keys
        //     .get(usize::from(accounts[3]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let token_in_address_index = usize::from(accounts[10]);
        let token_out_address_index = usize::from(accounts[11]);

        let token_in_address = DecodingUtils::get_account_address(
            token_in_address_index,
            account_keys,
            lookup_tables,
        )?;
        let token_out_address = DecodingUtils::get_account_address(
            token_out_address_index,
            account_keys,
            lookup_tables,
        )?;

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

        let amount_1: u64 = u64::from_le_bytes(data[0..8].try_into()?);
        let amount_2: u64 = u64::from_le_bytes(data[8..16].try_into()?);

        let (amount_specified, limit) = if is_base_input {
            (amount_1, SwapConstraint::TokenAmountLimit(amount_2))
        } else {
            (amount_2, SwapConstraint::TokenAmountLimit(amount_1))
        };

        Ok((
            pool_address,
            ShredEventType::Swap {
                amount_specified,
                limit,
                a_to_b,
                is_base_input,
            },
        ))
    }
}

const SWAP_EXACT_OUT: [u8; 8] = [55, 217, 98, 86, 163, 74, 180, 173];

const SWAP_EXACT_IN: [u8; 8] = [143, 190, 90, 218, 196, 30, 51, 222];
const SWAP_ACCOUNTS_LEN: usize = 12;
