use solana_sdk::pubkey::Pubkey;

use crate::engine::pools::dex_pool::{DexPool, PoolType, SwapQuoteResult};
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

#[derive(Clone, Debug)]
pub struct ClmmPool {
    pub address: Pubkey,
    pub token_a: Pubkey,
    pub token_b: Pubkey,

    pub fee_bps: u16,

    pub min_tick: i32,
    pub max_tick: i32,
    pub tick_spacing: u16,

    // Current state
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

    fn get_swap_quote(&self, _amount_in: u64, _a_to_b: bool) -> Option<SwapQuoteResult> {
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
