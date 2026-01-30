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

pub struct RaydiumV3TargetTransaction;

impl TargetTransaction for RaydiumV3TargetTransaction {
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
                return Err(anyhow!("Unsupported swap instruction type on RaydiumV3"));
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

impl RaydiumV3TargetTransaction {
    //example: https://solscan.io/tx/5KVaSKctetQ1TdmJxeQUiacnqV15H6Thtwdt7EtLjBggdd4yRtwVx7Zzj3E9mcYcSuTWwENDQGxbhJUvPF5z96CE
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

        let input_vault_index = usize::from(accounts[5]);

        let input_vault =
            DecodingUtils::get_account_address(input_vault_index, account_keys, lookup_tables)?;

        // let input_vault = *account_keys
        //     .get(usize::from(accounts[5]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let specified_amount: u64 = u64::from_le_bytes(data[0..8].try_into()?);
        let _amount_threshold: u64 = u64::from_le_bytes(data[8..16].try_into()?);

        let sqrt_price_limit: u128 = u128::from_le_bytes(data[16..32].try_into()?);

        let is_exact_input: bool = data[32] == 1;

        let a_to_b = if let Some(edge) = market.get_edge(&pool_address) {
            input_vault == edge.token_vault_a
        } else {
            return Err(anyhow!("Unsupported Pool"));
        };

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

    //example: https://solscan.io/tx/2j7ikfSmJ1AMt979tHmqKQGdv5KiBppveHoQTdhjxLkVkr3FjKf9C7kACkhAF6zUX3UZepumuQJzJMcWQESfAPyV
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
                "accounts len != SWAP_V2_ACCOUNTS_LEN, received {} | expected {}",
                accounts.len(),
                SWAP_V2_ACCOUNTS_LEN
            ));
        }

        let pool_address_index = usize::from(accounts[2]);
        let pool_address =
            DecodingUtils::get_account_address(pool_address_index, account_keys, lookup_tables)?;
        // let pool_address = *account_keys
        //     .get(usize::from(accounts[2]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let token_in_address_index = usize::from(accounts[11]);
        let token_out_address_index = usize::from(accounts[12]);

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

        // let token_in_address = *account_keys
        //     .get(usize::from(accounts[11]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;
        // let token_out_address = *account_keys
        //     .get(usize::from(accounts[12]))
        //     .ok_or_else(|| anyhow::anyhow!("Index out of range in account_keys"))?;

        let specified_amount: u64 = u64::from_le_bytes(data[0..8].try_into()?);
        let _amount_threshold: u64 = u64::from_le_bytes(data[8..16].try_into()?);

        let sqrt_price_limit: u128 = u128::from_le_bytes(data[16..32].try_into()?);
        let is_exact_input: bool = data[32] == 1;

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
                amount_specified: specified_amount,
                limit: SwapConstraint::SqrtPriceLimit(sqrt_price_limit),
                a_to_b,
                is_base_input: is_exact_input,
            },
        ))
    }
}

const SWAP_V1: [u8; 8] = [248, 198, 158, 145, 225, 117, 135, 200];
const SWAP_V1_ACCOUNTS_LEN: usize = 6;

const SWAP_V2: [u8; 8] = [43, 4, 237, 11, 26, 201, 30, 98];
const SWAP_V2_ACCOUNTS_LEN: usize = 13;
