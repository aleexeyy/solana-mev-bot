use solana_sdk::pubkey::Pubkey;

use super::dex_pool::{DexPool, PoolType};
use crate::{engine::pools::dex_pool::{SwapQuoteError, SwapQuoteInput, SwapQuoteResult}, shred_decoders::interfaces::SwapConstraint};

#[derive(Clone, Debug)]
pub struct CpmmPool {
    pub address: Pubkey,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub fee_bps: u16,
    pub last_slot_updated: u64,
}

impl CpmmPool {

    #[inline]
    fn get_swap_quote_exact_input(
        &self,
        input: SwapQuoteInput,
    ) -> Result<SwapQuoteResult, SwapQuoteError> {
        if !self.can_execute() {
            return Err(SwapQuoteError::PoolNotExecutable);
        }

        if input.amount_specified == 0 {
            return Err(SwapQuoteError::ZeroAmount);
        }

        let (reserve_in_u64, reserve_out_u64) = if input.a_to_b {
            (self.reserve_a, self.reserve_b)
        } else {
            (self.reserve_b, self.reserve_a)
        };

        let reserve_in = reserve_in_u64 as u128;
        let reserve_out = reserve_out_u64 as u128;

        let fee_denominator: u128 = 10_000;
        let fee_bps: u128 = self.fee_bps as u128;
        let fee_multiplier = fee_denominator - fee_bps;

        let amount_in = input.amount_specified as u128;
        let amount_in_with_fee = amount_in * fee_multiplier;

        let numerator = amount_in_with_fee * reserve_out;
        let denominator = reserve_in * fee_denominator + amount_in_with_fee;

        let amount_out = numerator / denominator;

        if amount_out == 0 || amount_out >= reserve_out {
            return Err(SwapQuoteError::InsufficientLiquidity);
        }

        if let SwapConstraint::TokenAmountLimit(min_out) = input.limit {
            if amount_out < min_out as u128 {
                return Err(SwapQuoteError::ConstraintViolated);
            }
        }

        let new_reserve_in = reserve_in + amount_in;
        let new_reserve_out = reserve_out - amount_out;

        if new_reserve_in > u64::MAX as u128 {
            return Err(SwapQuoteError::Overflow);
        }

        Ok(SwapQuoteResult {
            amount_in: input.amount_specified,
            amount_out: amount_out as u64,
            new_reserve_in: new_reserve_in as u64,
            new_reserve_out: new_reserve_out as u64,
        })
    }

    #[inline]
    fn get_swap_quote_exact_output(
        &self,
        input: SwapQuoteInput,
    ) -> Result<SwapQuoteResult, SwapQuoteError> {
        if !self.can_execute() {
            return Err(SwapQuoteError::PoolNotExecutable);
        }

        if input.amount_specified == 0 {
            return Err(SwapQuoteError::ZeroAmount);
        }

        let (reserve_in_u64, reserve_out_u64) = if input.a_to_b {
            (self.reserve_a, self.reserve_b)
        } else {
            (self.reserve_b, self.reserve_a)
        };

        let reserve_in = reserve_in_u64 as u128;
        let reserve_out = reserve_out_u64 as u128;

        if input.amount_specified as u128 >= reserve_out {
            return Err(SwapQuoteError::InsufficientLiquidity);
        }

        let fee_denominator: u128 = 10_000;
        let fee_bps: u128 = self.fee_bps as u128;
        let fee_multiplier = fee_denominator - fee_bps;

        let amount_out = input.amount_specified as u128;

        let numerator = reserve_in
            .checked_mul(amount_out)
            .ok_or(SwapQuoteError::Overflow)?
            .checked_mul(fee_denominator)
            .ok_or(SwapQuoteError::Overflow)?;

        let denominator = (reserve_out - amount_out)
            .checked_mul(fee_multiplier)
            .ok_or(SwapQuoteError::Overflow)?;

        let mut amount_in = numerator / denominator;
        if numerator % denominator != 0 {
            amount_in += 1; // round up
        }

        if amount_in == 0 || amount_in > u64::MAX as u128 {
            return Err(SwapQuoteError::Overflow);
        }

        if let SwapConstraint::TokenAmountLimit(max_in) = input.limit {
            if amount_in > max_in as u128 {
                return Err(SwapQuoteError::ConstraintViolated);
            }
        }

        let new_reserve_in = reserve_in + amount_in;
        let new_reserve_out = reserve_out - amount_out;

        if new_reserve_in > u64::MAX as u128 {
            return Err(SwapQuoteError::Overflow);
        }

        Ok(SwapQuoteResult {
            amount_in: amount_in as u64,
            amount_out: input.amount_specified,
            new_reserve_in: new_reserve_in as u64,
            new_reserve_out: new_reserve_out as u64,
        })
    }
    
}

impl DexPool for CpmmPool {
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
    fn get_swap_quote(&self, input: SwapQuoteInput) -> Result<SwapQuoteResult, SwapQuoteError> {
        if input.is_exact_input {
            self.get_swap_quote_exact_input(input)
        } else {
            self.get_swap_quote_exact_output(input)
        }
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
    use super::*;
    use crate::engine::pools::dex_pool::DexPool;

    fn create_test_pool() -> CpmmPool {
        let token_a = Pubkey::new_from_array([1u8; 32]);
        let token_b = Pubkey::new_from_array([2u8; 32]);

        let pool = CpmmPool {
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

    fn create_empty_test_pool() -> CpmmPool {
        let token_a = Pubkey::new_from_array([4u8; 32]);
        let token_b = Pubkey::new_from_array([5u8; 32]);

        let pool = CpmmPool {
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

        let input = SwapQuoteInput {
            slot: 0,
            amount_specified: 1_000_000,
            limit: SwapConstraint::TokenAmountLimit(0),
            a_to_b: true,
            is_exact_input: true,
        };

        let new_state = pool.get_swap_quote(input).unwrap();

        assert_eq!(new_state.amount_out, 498_003);
        assert_eq!(new_state.new_reserve_in, 1_001_000_000);
        assert_eq!(new_state.new_reserve_out, 499_501_997);
    }

    #[test]
    fn test_get_swap_quote_happy_path_not_direct() {
        let pool = create_test_pool();

        let input = SwapQuoteInput {
            slot: 0,
            amount_specified: 1_000_000,
            limit: SwapConstraint::TokenAmountLimit(0),
            a_to_b: false,
            is_exact_input: true,
        };

        let new_state = pool.get_swap_quote(input).unwrap();

        assert_eq!(new_state.amount_out, 1_990_031);
        assert_eq!(new_state.new_reserve_in, 501_000_000);
        assert_eq!(new_state.new_reserve_out, 998_009_969);
    }

    #[test]
    fn test_get_swap_quote_zero_amount_in() {
        let pool = create_test_pool();

        let input = SwapQuoteInput {
            slot: 0,
            amount_specified: 0,
            limit: SwapConstraint::TokenAmountLimit(0),
            a_to_b: true,
            is_exact_input: true,
        };

        let new_state = pool.get_swap_quote(input);

        assert!(matches!(new_state, Err(SwapQuoteError::ZeroAmount)));
    }

    #[test]
    fn test_get_swap_quote_zero_reserve() {
        let mut pool = create_test_pool();

        pool.reserve_a = 0;

        let input = SwapQuoteInput {
            slot: 0,
            amount_specified: 1_000_000,
            limit: SwapConstraint::TokenAmountLimit(0),
            a_to_b: false,
            is_exact_input: true,
        };

        let new_state = pool.get_swap_quote(input);

        assert!(matches!(new_state, Err(SwapQuoteError::PoolNotExecutable)));
    }

    #[test]
    fn test_get_swap_quote_too_large_amount_in() {
        let pool = create_test_pool();

        let input = SwapQuoteInput {
            slot: 0,
            amount_specified: u64::MAX,
            limit: SwapConstraint::TokenAmountLimit(0),
            a_to_b: false,
            is_exact_input: true,
        };

        let new_state = pool.get_swap_quote(input);

        assert!(matches!(new_state, Err(SwapQuoteError::Overflow)));
    }
}