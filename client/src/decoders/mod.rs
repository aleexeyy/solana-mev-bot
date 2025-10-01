use crate::bootstrap::pool_schema::PoolUpdate;
use anyhow::anyhow;
use solana_sdk::{account::Account, pubkey::Pubkey};
use std::{collections::HashMap, str::FromStr};
use tracing::info;
mod orca_decoder;
mod raydium_decoder;

const RAYDIUM_OWNER: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";
const ORCA_OWNER: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
type DecoderFn = fn(&Account) -> anyhow::Result<PoolUpdate>;

lazy_static::lazy_static! {
    static ref RAYDIUM_PUBKEY: Pubkey = Pubkey::from_str(RAYDIUM_OWNER).unwrap();
    static ref ORCA_PUBKEY: Pubkey = Pubkey::from_str(ORCA_OWNER).unwrap();

    static ref DECODERS: HashMap<Pubkey, DecoderFn> = {
        let mut m = HashMap::new();
        m.insert(*RAYDIUM_PUBKEY, raydium_decoder::decode_raydium_account as DecoderFn);
        m.insert(*ORCA_PUBKEY, orca_decoder::decode_orca_account as DecoderFn);
        m
    };
}

pub fn decode_account(account: &Account) -> anyhow::Result<PoolUpdate> {
    if let Some(decoder) = DECODERS.get(&account.owner) {
        let result: PoolUpdate = decoder(account)?;
        Ok(result)
    } else {
        info!("Unknown DEX, skipping decoding");
        Err(anyhow!("Unknown DEX"))
    }
}
