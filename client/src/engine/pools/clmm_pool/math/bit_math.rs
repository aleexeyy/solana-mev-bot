use super::big_num::U1024;

/// Returns the index (0–255) of the most significant set bit in a `U256`,
/// or `MathError::ZeroValue` if the input is zero.
///
/// Useful for fast order‑of‑magnitude / log2‑like operations on large integers.
pub fn most_significant_bit(x: U1024) -> Option<u16> {
    if x.is_zero() {
        None
    } else {
        Some(u16::try_from(x.leading_zeros()).unwrap())
    }
}

/// Returns the index (0–255) of the least significant set bit in a `U256`,
/// or `MathError::ZeroValue` if the input is zero.
///
/// This is typically used when scanning bitmaps from the right to find
/// the first initialized position.
pub fn least_significant_bit(x: U1024) -> Option<u16> {
    if x.is_zero() {
        None
    } else {
        Some(u16::try_from(x.trailing_zeros()).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------- most_significant_bit tests -------------------------

    #[test]
    fn msb_errors_on_zero() {
        let x = U1024::zero();
        let res = most_significant_bit(x);
        assert!(res.is_none());
    }

    #[test]
    fn msb_of_power_of_two() {
        // 1 << 7  = bit 7
        let x = U1024::from(1u128 << 7);
        let res = most_significant_bit(x).unwrap();
        assert_eq!(res, 7);
    }

    #[test]
    fn msb_of_multiple_bits() {
        // binary: 1001_0100 (MSB = bit 7)
        let x = U1024::from(0b1001_0100u128);
        let res = most_significant_bit(x).unwrap();
        assert_eq!(res, 7);
    }

    #[test]
    fn msb_of_max_u256() {
        // U256::MAX has MSB = 255
        let x = U1024::max_value();
        let res = most_significant_bit(x).unwrap();
        assert_eq!(res, 255);
    }

    // ------------------------- least_significant_bit tests -------------------------

    #[test]
    fn lsb_errors_on_zero() {
        let x = U1024::zero();
        let res = least_significant_bit(x);
        assert!(res.is_none());
    }

    #[test]
    fn lsb_of_power_of_two() {
        // 1 << 12 => LSB = 12
        let x = U1024::from(1u128 << 12);
        let res = least_significant_bit(x).unwrap();
        assert_eq!(res, 12);
    }

    #[test]
    fn lsb_of_multiple_bits() {
        // binary: 1011001000 -> LSB is position 3 (0-based)
        let x = U1024::from(0b1011001000u128);
        let res = least_significant_bit(x).unwrap();
        assert_eq!(res, 3);
    }

    #[test]
    fn lsb_of_max_u256() {
        // U256::MAX ends with ...1111, so LSB = 0
        let x = U1024::max_value();
        let res = least_significant_bit(x).unwrap();
        assert_eq!(res, 0);
    }
}
