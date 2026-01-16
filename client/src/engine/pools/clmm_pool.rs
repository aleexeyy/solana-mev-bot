use solana_sdk::pubkey::Pubkey;

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
