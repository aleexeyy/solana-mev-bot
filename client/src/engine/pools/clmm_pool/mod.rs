use solana_sdk::pubkey::Pubkey;

use crate::{engine::pools::dex_pool::{DexPool, PoolType, SwapQuoteError, SwapQuoteInput, SwapQuoteResult}, shred_decoders::interfaces::SwapConstraint};
pub mod clmm_error;
pub mod math;

#[derive(Clone, Debug)]
pub struct TickArray {
    pub start_tick: i32,
    pub ticks: Vec<TickState>,
}

#[derive(Clone, Copy, Debug)]
pub struct TickState {
    pub liquidity_net: i128,
    pub liquidity_gross: u128,
    pub initialized: bool,
}


pub struct SwapState {
    // the amount remaining to be swapped in/out of the input/output asset
    amount_specified_remaining: u64,
    // the amount already swapped out/in of the output/input asset
    amount_calculated: u64,
    // current sqrt(price)
    sqrt_price_x96: u128,
    // the tick associated with the current price
    tick: i32,
    // the current liquidity in range
    liquidity: u128,
    // accumulated swap fees
    swap_fee: u64,
}


pub struct StepComputations {
    // the price at the beginning of the step
    sqrt_price_start_x96: u128,
    // the next tick to swap to from the current tick in the swap direction
    tick_next: i32,
    // whether tickNext is initialized or not
    initialized: bool,
    // sqrt(price) for the next tick (1/0)
    sqrt_price_next_x96: u128,
    // how much is being swapped in this step
    amount_in: u64,
    // how much is being swapped out
    amount_out: u64,
    // how much fee is being paid in
    fee_amount: u64,
}

#[derive(Clone, Debug)]
pub struct ClmmPool {
    //Pool parameters
    pub address: Pubkey,
    pub token_a: Pubkey,
    pub token_b: Pubkey,

    pub fee_bps: u16,
    pub tick_spacing: u16,

    //Dex parameters

    pub min_tick: i32,
    pub max_tick: i32,
    pub min_sqrt_ratio: u128,
    pub max_sqrt_ratio: u128,

    // Dynamic parameters
    pub current_price_x64: u128,
    pub liquidity: u128,
    pub current_tick: i32,

    pub tick_arrays: Vec<TickArray>,

    pub last_slot_updated: u64,
}

impl ClmmPool {
    fn virtual_reserves(&self) -> Option<(u128, u128)> {
        if !self.can_execute() {
            return None;
        }

        let sqrt_price_x64 = self.current_price_x64;
        let liquidity = self.liquidity;

        let reserve_a = (math::U256::from(liquidity) * math::U256::from(math::fixed_point_64::Q64))
            / math::U256::from(sqrt_price_x64);
        let reserve_b = (math::U256::from(liquidity) * math::U256::from(sqrt_price_x64))
            / math::U256::from(math::fixed_point_64::Q64);

        if reserve_a > math::U256::from(u128::MAX) || reserve_b > math::U256::from(u128::MAX) {
            return None;
        }

        Some((reserve_a.as_u128(), reserve_b.as_u128()))
    }
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

    #[inline]
    fn get_reserves(&self) -> (u128, u128) {
        self.virtual_reserves().unwrap_or((0, 0))
    }

    #[inline]
    fn can_execute(&self) -> bool {
        self.liquidity > 0
            && self.tick_spacing > 0
            && self.fee_bps < 10_000
            && self.current_tick >= math::tick_math::MIN_TICK
            && self.current_tick <= math::tick_math::MAX_TICK
            && self.current_price_x64 >= math::tick_math::MIN_SQRT_PRICE_X64
            && self.current_price_x64 < math::tick_math::MAX_SQRT_PRICE_X64
    }

    #[inline]
    fn pool_type(&self) -> PoolType {
        PoolType::CLMM
    }
    #[inline]
    fn last_slot_updated(&self) -> u64 {
        self.last_slot_updated
    }

    fn get_swap_quote(&self, input: SwapQuoteInput) -> Result<SwapQuoteResult, SwapQuoteError> {

        if !self.can_execute() {
            return Err(SwapQuoteError::PoolNotExecutable);
        }

        if input.amount_specified == 0 {
            return Err(SwapQuoteError::ZeroAmount);
        }

        if self.liquidity == 0 {
            return Err(SwapQuoteError::InsufficientLiquidity)
        }

        let sqrt_price_limit_x64 = match input.limit {
            SwapConstraint::SqrtPriceLimit(limit) => limit,
            _ => {
                if input.a_to_b {
                    self.min_sqrt_ratio + 1
                } else {
                    self.max_sqrt_ratio - 1
                }
            }
        };



        if input.a_to_b {
            if (sqrt_price_limit_x64 >= self.current_price_x64) || (sqrt_price_limit_x64 <= self.min_sqrt_ratio) {
                return Err(SwapQuoteError::SqrtPriceOutOfBounds)
            } else if (sqrt_price_limit_x64 <= self.current_price_x64) || (sqrt_price_limit_x64 >= self.max_sqrt_ratio) {
                return Err(SwapQuoteError::SqrtPriceOutOfBounds)
            }
        }
        todo!("CLMM quoting not implemented yet")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pool() -> ClmmPool {
        ClmmPool {
            address: Pubkey::new_from_array([3u8; 32]),
            token_a: Pubkey::new_from_array([1u8; 32]),
            token_b: Pubkey::new_from_array([2u8; 32]),
            fee_bps: 30,
            current_price_x64: math::fixed_point_64::Q64,
            liquidity: 1_000_000,
            current_tick: 0,
            tick_spacing: 1,
            last_slot_updated: 0,
            min_tick: -10,
            max_tick: 10,
            min_sqrt_ratio: 0,
            max_sqrt_ratio: 1000,
            tick_arrays: vec![],
        }
    }

    #[test]
    fn test_can_execute_happy_path() {
        let pool = create_test_pool();
        assert!(pool.can_execute());
    }

    #[test]
    fn test_can_execute_rejects_invalid_state() {
        let mut pool = create_test_pool();

        pool.liquidity = 0;
        assert!(!pool.can_execute());

        pool = create_test_pool();
        pool.fee_bps = 10_000;
        assert!(!pool.can_execute());

        pool = create_test_pool();
        pool.tick_spacing = 0;
        assert!(!pool.can_execute());

        pool = create_test_pool();
        pool.current_price_x64 = 0;
        assert!(!pool.can_execute());

        pool = create_test_pool();
        pool.current_tick = math::tick_math::MAX_TICK + 1;
        assert!(!pool.can_execute());
    }

    #[test]
    fn test_get_reserves_virtual_at_price_one() {
        let pool = create_test_pool();
        assert_eq!(pool.get_reserves(), (1_000_000, 1_000_000));
    }
}
