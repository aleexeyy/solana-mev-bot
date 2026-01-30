use solana_sdk::pubkey::Pubkey;

use crate::shred_decoders::interfaces::SwapConstraint;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoolType {
    CPMM,
    CLMM,
    DLMM,
}

#[derive(Debug)]
pub struct SwapQuoteResult {
    pub amount_in: u64,
    pub amount_out: u64,
    pub new_reserve_in: u64,
    pub new_reserve_out: u64,
}


#[derive(Debug)]
pub struct SwapQuoteInput {
    pub(crate) slot: u64,
    pub(crate) amount_specified: u64,
    pub(crate) limit: SwapConstraint,
    pub(crate) a_to_b: bool,
    pub(crate) is_exact_input: bool,
}

#[derive(Debug)]
pub enum SwapQuoteError {
    PoolNotExecutable,
    ZeroAmount,
    InsufficientLiquidity,
    ConstraintViolated,
    Overflow,
    SqrtPriceOutOfBounds,
}


pub trait DexPool {
    fn address(&self) -> &Pubkey;

    fn token_mint_a(&self) -> &Pubkey;

    fn token_mint_b(&self) -> &Pubkey;

    fn get_reserves(&self) -> (u128, u128);


    // TODO: change option to result
    fn get_swap_quote(&self, input: SwapQuoteInput) -> Result<SwapQuoteResult, SwapQuoteError>;

    fn can_execute(&self) -> bool;

    fn pool_type(&self) -> PoolType;

    fn last_slot_updated(&self) -> u64;
}
