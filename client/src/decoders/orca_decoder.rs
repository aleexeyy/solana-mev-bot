use anyhow::{Result, anyhow};
use solana_sdk::account::Account;
use tracing::error;

use crate::bootstrap::pool_schema::PoolUpdate;

pub fn decode_orca_account(account: &Account) -> Result<PoolUpdate> {
    if account.data.len() != 653 {
        return Err(anyhow!("Account data has wrong length"));
    }

    let data = &account.data;
    let discriminator: [u8; 8] = data[0..8].try_into()?;

    if discriminator != [63, 149, 209, 12, 225, 128, 99, 9] {
        error!("Discriminator: {:?}", discriminator);
        return Err(anyhow!("Wrong Discriminator Found"));
    }
    // let config = Pubkey::new_from_array(data[8..40].try_into()?);
    // let bump: u8 = data[40];
    // let tick_spacing: [u8; 2] = [data[41], data[42]];
    //let fee_tier_index: [u8; 2] = [data[43], data[44]];
    //let fee_rate : [u8; 2] = [data[45], data[46]];

    //possible to do with unsafe in the future
    let liquidity: u128 = u128::from_le_bytes(data[49..65].try_into()?);
    let sqrt_price: u128 = u128::from_le_bytes(data[65..81].try_into()?);
    let current_tick_index: i32 = i32::from_le_bytes([data[81], data[82], data[83], data[84]]);
    //idea for a test: having account data and trying to decode it using this function and assert_eq it to Whilrpool decoder
    Ok(PoolUpdate {
        new_liquidity: liquidity,
        new_sqrt_price: sqrt_price,
        new_current_tick_index: current_tick_index,
    })
}
