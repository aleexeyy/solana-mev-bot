//! A custom implementation of https://github.com/sdroege/rust-muldiv to support phantom overflow resistant
//! multiply-divide operations. This library uses U128 in place of u128 for u64 operations,
//! and supports U128 operations.

use super::big_num::{U128, U256, U512};

/// Trait for calculating `val * num / denom` with different rounding modes and overflow
/// protection.
///
/// Implementations of this trait have to ensure that even if the result of the multiplication does
/// not fit into the type, as long as it would fit after the division the correct result has to be
/// returned instead of `None`. `None` only should be returned if the overall result does not fit
/// into the type.
///
/// This specifically means that e.g. the `u64` implementation must, depending on the arguments, be
/// able to do 128 bit integer multiplication.
pub trait MulDiv<RHS = Self> {
    /// Output type for the methods of this trait.
    type Output;

    /// Calculates `floor(val * num / denom)`, i.e. the largest integer less than or equal to the
    /// result of the division.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use libraries::full_math::MulDiv;
    ///
    /// # fn main() {
    /// let x = 3i8.mul_div_floor(4, 2);
    /// assert_eq!(x, Some(6));
    ///
    /// let x = 5i8.mul_div_floor(2, 3);
    /// assert_eq!(x, Some(3));
    ///
    /// let x = (-5i8).mul_div_floor(2, 3);
    /// assert_eq!(x, Some(-4));
    ///
    /// let x = 3i8.mul_div_floor(3, 2);
    /// assert_eq!(x, Some(4));
    ///
    /// let x = (-3i8).mul_div_floor(3, 2);
    /// assert_eq!(x, Some(-5));
    ///
    /// let x = 127i8.mul_div_floor(4, 3);
    /// assert_eq!(x, None);
    /// # }
    /// ```
    fn mul_div_floor(self, num: RHS, denom: RHS) -> Option<Self::Output>;

    /// Calculates `ceil(val * num / denom)`, i.e. the the smallest integer greater than or equal to
    /// the result of the division.
    ///
    /// ## Example
    ///
    /// ```rust
    /// use libraries::full_math::MulDiv;
    ///
    /// # fn main() {
    /// let x = 3i8.mul_div_ceil(4, 2);
    /// assert_eq!(x, Some(6));
    ///
    /// let x = 5i8.mul_div_ceil(2, 3);
    /// assert_eq!(x, Some(4));
    ///
    /// let x = (-5i8).mul_div_ceil(2, 3);
    /// assert_eq!(x, Some(-3));
    ///
    /// let x = 3i8.mul_div_ceil(3, 2);
    /// assert_eq!(x, Some(5));
    ///
    /// let x = (-3i8).mul_div_ceil(3, 2);
    /// assert_eq!(x, Some(-4));
    ///
    /// let x = (127i8).mul_div_ceil(4, 3);
    /// assert_eq!(x, None);
    /// # }
    /// ```
    fn mul_div_ceil(self, num: RHS, denom: RHS) -> Option<Self::Output>;

    /// Return u64 not out of bounds
    fn to_underflow_u64(self) -> u64;
}

pub trait Upcast256 {
    fn as_u256(self) -> U256;
}
impl Upcast256 for U128 {
    fn as_u256(self) -> U256 {
        U256([self.0[0], self.0[1], 0, 0])
    }
}

pub trait Downcast256 {
    /// Unsafe cast to U128
    /// Bits beyond the 128th position are lost
    fn as_u128(self) -> U128;
}
impl Downcast256 for U256 {
    fn as_u128(self) -> U128 {
        U128([self.0[0], self.0[1]])
    }
}

pub trait Upcast512 {
    fn as_u512(self) -> U512;
}
impl Upcast512 for U256 {
    fn as_u512(self) -> U512 {
        U512([self.0[0], self.0[1], self.0[2], self.0[3], 0, 0, 0, 0])
    }
}

pub trait Downcast512 {
    /// Unsafe cast to U256
    /// Bits beyond the 256th position are lost
    fn as_u256(self) -> U256;
}
impl Downcast512 for U512 {
    fn as_u256(self) -> U256 {
        U256([self.0[0], self.0[1], self.0[2], self.0[3]])
    }
}

impl MulDiv for u64 {
    type Output = u64;

    fn mul_div_floor(self, num: Self, denom: Self) -> Option<Self::Output> {
        assert_ne!(denom, 0);
        let r = (U128::from(self) * U128::from(num)) / U128::from(denom);
        if r > U128::from(u64::MAX) {
            None
        } else {
            Some(r.as_u64())
        }
    }

    fn mul_div_ceil(self, num: Self, denom: Self) -> Option<Self::Output> {
        assert_ne!(denom, 0);
        let r = (U128::from(self) * U128::from(num) + U128::from(denom - 1)) / U128::from(denom);
        if r > U128::from(u64::MAX) {
            None
        } else {
            Some(r.as_u64())
        }
    }

    fn to_underflow_u64(self) -> u64 {
        self
    }
}

impl MulDiv for U128 {
    type Output = U128;

    fn mul_div_floor(self, num: Self, denom: Self) -> Option<Self::Output> {
        assert_ne!(denom, U128::default());
        let r = ((self.as_u256()) * (num.as_u256())) / (denom.as_u256());
        if r > U128::MAX.as_u256() {
            None
        } else {
            Some(r.as_u128())
        }
    }

    fn mul_div_ceil(self, num: Self, denom: Self) -> Option<Self::Output> {
        assert_ne!(denom, U128::default());
        let r = (self.as_u256() * num.as_u256() + (denom - 1).as_u256()) / denom.as_u256();
        if r > U128::MAX.as_u256() {
            None
        } else {
            Some(r.as_u128())
        }
    }

    fn to_underflow_u64(self) -> u64 {
        if self < U128::from(u64::MAX) {
            self.as_u64()
        } else {
            0
        }
    }
}

impl MulDiv for U256 {
    type Output = U256;

    fn mul_div_floor(self, num: Self, denom: Self) -> Option<Self::Output> {
        assert_ne!(denom, U256::default());
        let r = (self.as_u512() * num.as_u512()) / denom.as_u512();
        if r > U256::MAX.as_u512() {
            None
        } else {
            Some(r.as_u256())
        }
    }

    fn mul_div_ceil(self, num: Self, denom: Self) -> Option<Self::Output> {
        assert_ne!(denom, U256::default());
        let r = (self.as_u512() * num.as_u512() + (denom - 1).as_u512()) / denom.as_u512();
        if r > U256::MAX.as_u512() {
            None
        } else {
            Some(r.as_u256())
        }
    }

    fn to_underflow_u64(self) -> u64 {
        if self < U256::from(u64::MAX) {
            self.as_u64()
        } else {
            0
        }
    }
}

#[cfg(test)]
mod muldiv_u64_tests {
    use super::*;

    fn test_vectors_u64() -> Vec<u64> {
        let mut v = vec![
            0,
            1,
            2,
            3,
            7,
            10,
            63,
            64,
            65,
            127,
            128,
            129,
            u32::MAX as u64,
            u64::MAX / 2,
            u64::MAX - 1,
            u64::MAX,
        ];

        // Deterministic pseudo-random values
        let mut x = 0xdead_beef_cafe_babe_u64;
        for _ in 0..5000 {
            x ^= x << 7;
            x ^= x >> 9;
            x ^= x << 8;
            if x != 0 {
                v.push(x);
            }
        }

        v
    }

    #[test]
    fn scale_floor_u64() {
        let values = test_vectors_u64();

        for &val in &values {
            for &num in &values {
                for &den in &values {
                    if den == 0 {
                        continue;
                    }

                    let res = val.mul_div_floor(num, den);

                    let expected = (U128::from(val) * U128::from(num)) / U128::from(den);

                    if expected > U128::from(u64::MAX) {
                        assert!(
                            res.is_none(),
                            "expected overflow: val={val}, num={num}, den={den}"
                        );
                    } else {
                        assert_eq!(
                            res,
                            Some(expected.as_u64()),
                            "val={val}, num={num}, den={den}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn scale_ceil_u64() {
        let values = test_vectors_u64();

        for &val in &values {
            for &num in &values {
                for &den in &values {
                    if den == 0 {
                        continue;
                    }

                    let res = val.mul_div_ceil(num, den);

                    let prod = U128::from(val) * U128::from(num);
                    let mut expected = prod / U128::from(den);
                    let rem = prod % U128::from(den);

                    if rem != U128::default() {
                        expected += U128::from(1);
                    }

                    if expected > U128::from(u64::MAX) {
                        assert!(
                            res.is_none(),
                            "expected overflow: val={val}, num={num}, den={den}"
                        );
                    } else {
                        assert_eq!(
                            res,
                            Some(expected.as_u64()),
                            "val={val}, num={num}, den={den}"
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod muldiv_u128_tests {
    use super::*;

    fn test_vectors_u128() -> Vec<U128> {
        let mut v = vec![
            U128::from(0u128),
            U128::from(1u128),
            U128::from(2u128),
            U128::from(3u128),
            U128::from(10u128),
            U128::from(u64::MAX as u128),
            U128::from(u128::MAX / 2),
            U128::from(u128::MAX - 1),
            U128::from(u128::MAX),
        ];

        let mut x = 0x1234_5678_9abc_def0_u128;
        for _ in 0..3000 {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            if x != 0 {
                v.push(U128::from(x));
            }
        }

        v
    }

    #[test]
    fn scale_floor_u128() {
        let values = test_vectors_u128();

        for val in &values {
            for num in &values {
                for den in &values {
                    if den.is_zero() {
                        continue;
                    }

                    let res = val.mul_div_floor(*num, *den);

                    let expected = (val.as_u256() * num.as_u256()) / den.as_u256();

                    if expected > U128::MAX.as_u256() {
                        assert!(
                            res.is_none(),
                            "expected overflow: val={val:?}, num={num:?}, den={den:?}"
                        );
                    } else {
                        assert_eq!(
                            res,
                            Some(expected.as_u128()),
                            "val={val:?}, num={num:?}, den={den:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn scale_ceil_u128() {
        let values = test_vectors_u128();

        for val in &values {
            for num in &values {
                for den in &values {
                    if den.is_zero() {
                        continue;
                    }

                    let res = val.mul_div_ceil(*num, *den);

                    let prod = val.as_u256() * num.as_u256();
                    let mut expected = prod / den.as_u256();
                    let rem = prod % den.as_u256();

                    if rem != U256::default() {
                        expected += U256::from(1);
                    }

                    if expected > U128::MAX.as_u256() {
                        assert!(
                            res.is_none(),
                            "expected overflow: val={val:?}, num={num:?}, den={den:?}"
                        );
                    } else {
                        assert_eq!(
                            res,
                            Some(expected.as_u128()),
                            "val={val:?}, num={num:?}, den={den:?}"
                        );
                    }
                }
            }
        }
    }
}
