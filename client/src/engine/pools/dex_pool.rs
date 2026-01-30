use solana_sdk::pubkey::Pubkey;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoolType {
    Amm,
    Clmm,
}

pub struct SwapQuoteResult {
    pub amount_out: u64,
    pub new_reserve_in: u64,
    pub new_reserve_out: u64,
}

pub struct QuoteSwapContext {}

pub trait DexPool {
    fn address(&self) -> &Pubkey;

    fn token_mint_a(&self) -> &Pubkey;

    fn token_mint_b(&self) -> &Pubkey;

    fn get_reserves(&self) -> (u128, u128);

    fn get_swap_quote(&self, amount_in: u64, a_to_b: bool) -> Option<SwapQuoteResult>;

    fn can_execute(&self) -> bool;

    fn pool_type(&self) -> PoolType;

    fn last_slot_updated(&self) -> u64;
}
