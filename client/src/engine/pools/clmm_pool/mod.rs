use solana_sdk::pubkey::Pubkey;
use crate::engine::pools::dex_pool::DexPool;
pub mod math;
pub mod clmm_error;

#[derive(Clone, Debug)]
pub struct ClmmPool {
    pub address: Pubkey,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub current_price_x64: u128,
    pub liquidity: u128,
    pub tick_current: i32,
    pub fee_bps: u16,
}


impl DexPool for ClmmPool {
    #[inline]
    fn address(&self) -> &Pubkey {
        &self.address
    }

    #[inline]
    fn token_mint_a(&self) -> &Pubkey {
        &self.token_a
    }

    #[inline]
    fn token_mint_b(&self) -> &Pubkey {
        &self.token_b
    }
}