use crate::bootstrap::pool_schema::PoolUpdate;
use anyhow::{Result, anyhow};
use solana_sdk::account::Account;
use tracing::{error};

pub fn decode_raydium_account(account: &Account) -> Result<PoolUpdate> {
    if account.data.len() != 1544 {
        return Err(anyhow!("Account data has wrong length"));
    }

    let data = &account.data;
    let descriminator: [u8; 8] = data[0..8].try_into()?;

    if descriminator != [247, 237, 227, 245, 215, 195, 222, 70] {
        error!("Descriminator: {:?}", descriminator);
        return Err(anyhow!("Wrong Descriminator Found"));
    }

    //let bump: u8 = data[8];

    let liquidty: u128 = u128::from_le_bytes(data[237..253].try_into()?);
    let sqrt_price: u128 = u128::from_le_bytes(data[253..269].try_into()?);
    let current_tick_index : i32 = i32::from_le_bytes([data[269], data[270], data[271], data[272]]);

    Ok(PoolUpdate { new_liquidity: liquidty, new_sqrt_price: sqrt_price, new_current_tick_index: current_tick_index })
}