use std::ops::{Add, Div, Mul, Sub};

use solana_sdk::pubkey::Pubkey;

use super::dex_pool::{DexPool, PoolType};
use crate::engine::pools::dex_pool::SwapQuoteResult;

#[derive(Clone, Debug)]
pub struct AmmPool {
    pub address: Pubkey,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub fee_bps: u16,
    pub last_slot_updated: u64,
}

impl DexPool for AmmPool {
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
        (self.reserve_a as u128, self.reserve_b as u128)
    }

    #[inline]
    fn last_slot_updated(&self) -> u64 {
        self.last_slot_updated
    }

    #[inline]
    fn get_swap_quote(&self, amount_in: u64, a_to_b: bool) -> Option<SwapQuoteResult> {
        if amount_in == 0 || !self.can_execute() {
            return None;
        }

        let (reserve_in_u64, reserve_out_u64) = if a_to_b {
            (self.reserve_a, self.reserve_b)
        } else {
            (self.reserve_b, self.reserve_a)
        };

        let fee_denominator: u128 = 10_000;
        let fee_bps_u128: u128 = self.fee_bps as u128;

        let amount_in_u128 = amount_in as u128;
        let fee_multiplier = fee_denominator.sub(fee_bps_u128);
        let amount_in_with_fee = amount_in_u128.mul(fee_multiplier);

        let reserve_in_u128 = reserve_in_u64 as u128;
        let reserve_out_u128 = reserve_out_u64 as u128;

        let numerator = amount_in_with_fee.mul(reserve_out_u128);

        let denominator = reserve_in_u128.mul(fee_denominator).add(amount_in_with_fee);

        let amount_out_u128 = numerator.div(denominator);

        if amount_out_u128 == 0u128 || amount_out_u128 >= reserve_out_u128 {
            return None;
        }

        let new_reserve_out = reserve_out_u128.sub(amount_out_u128);
        let new_reserve_in = reserve_in_u128.add(amount_in_u128);

        if new_reserve_in > u64::MAX as u128 {
            return None;
        }

        return Some(SwapQuoteResult {
            amount_out: amount_out_u128 as u64,
            new_reserve_in: new_reserve_in as u64,
            new_reserve_out: new_reserve_out as u64,
        });
    }

    #[inline]
    fn can_execute(&self) -> bool {
        self.reserve_a > 0u64 && self.reserve_b > 0u64 && self.fee_bps < 10_000u16
    }

    #[inline]
    fn pool_type(&self) -> PoolType {
        PoolType::CPMM
    }
}

#[cfg(test)]
mod tests {
    use std::u64;

    use super::*;
    use crate::engine::pools::dex_pool::DexPool;

    fn create_test_pool() -> AmmPool {
        let token_a = Pubkey::new_from_array([1u8; 32]);
        let token_b = Pubkey::new_from_array([2u8; 32]);

        let pool = AmmPool {
            address: Pubkey::new_from_array([3u8; 32]),
            token_a,
            token_b,
            reserve_a: 1_000_000_000,
            reserve_b: 500_000_000,
            fee_bps: 30,
            last_slot_updated: 0,
        };
        return pool;
    }

    fn create_empty_test_pool() -> AmmPool {
        let token_a = Pubkey::new_from_array([4u8; 32]);
        let token_b = Pubkey::new_from_array([5u8; 32]);

        let pool = AmmPool {
            address: Pubkey::new_from_array([6u8; 32]),
            token_a,
            token_b,
            reserve_a: 0,
            reserve_b: 0,
            fee_bps: 30,
            last_slot_updated: 0,
        };
        return pool;
    }

    #[test]
    fn test_can_exute() {
        let pool = create_test_pool();

        assert!(pool.can_execute());

        let empty_pool = create_empty_test_pool();

        assert!(!empty_pool.can_execute());
    }

    #[test]
    fn test_get_swap_quote_happy_path_direct() {
        let pool = create_test_pool();

        let amount_in = 1_000_000;

        let a_to_b = true;

        let new_state = pool.get_swap_quote(amount_in, a_to_b).unwrap();

        assert_eq!(new_state.amount_out, 498_003);
        assert_eq!(new_state.new_reserve_in, 1_001_000_000);
        assert_eq!(new_state.new_reserve_out, 499_501_997);
    }

    #[test]
    fn test_get_swap_quote_happy_path_not_direct() {
        let pool = create_test_pool();

        let amount_in = 1_000_000;

        let a_to_b = false;

        let new_state = pool.get_swap_quote(amount_in, a_to_b).unwrap();

        assert_eq!(new_state.amount_out, 1_990_031);
        assert_eq!(new_state.new_reserve_in, 501_000_000);
        assert_eq!(new_state.new_reserve_out, 998_009_969);
    }

    #[test]
    fn test_get_swap_quote_zero_amount_in() {
        let pool = create_test_pool();

        let amount_in = 0;

        let a_to_b = true;

        let new_state = pool.get_swap_quote(amount_in, a_to_b);

        assert!(new_state.is_none());
    }

    #[test]
    fn test_get_swap_quote_zero_reserve() {
        let mut pool = create_test_pool();

        pool.reserve_a = 0;

        let amount_in = 1_000_000;

        let a_to_b = false;

        let new_state = pool.get_swap_quote(amount_in, a_to_b);

        assert!(new_state.is_none());
    }

    #[test]
    fn test_get_swap_quote_too_large_amount_in() {
        let pool = create_test_pool();

        let amount_in = u64::MAX;

        let a_to_b = false;

        let new_state = pool.get_swap_quote(amount_in, a_to_b);

        assert!(new_state.is_none());
    }
}
