use super::big_num::U256;

pub fn most_significant_bit(x: U256) -> Option<u8> {
    if x.is_zero() {
        return None;
    }

    let leading = x.leading_zeros();
    debug_assert!(leading < 256);
    Some((255u32 - leading) as u8)
}

pub fn least_significant_bit(x: U256) -> Option<u8> {
    if x.is_zero() {
        return None;
    }

    let trailing = x.trailing_zeros();
    debug_assert!(trailing < 256);
    Some(trailing as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------- most_significant_bit tests -------------------------

    #[test]
    fn msb_errors_on_zero() {
        let x = U256::zero();
        let res = most_significant_bit(x);
        assert!(res.is_none());
    }

    #[test]
    fn msb_of_power_of_two() {
        // 1 << 7  = bit 7
        let x = U256::from(1u128 << 7);
        let res = most_significant_bit(x).unwrap();
        assert_eq!(res, 7);
    }

    #[test]
    fn msb_of_multiple_bits() {
        // binary: 1001_0100 (MSB = bit 7)
        let x = U256::from(0b1001_0100u128);
        let res = most_significant_bit(x).unwrap();
        assert_eq!(res, 7);
    }

    #[test]
    fn msb_of_max_u256() {
        // U256::MAX has MSB = 255
        let x = U256::max_value();
        let res = most_significant_bit(x).unwrap();
        assert_eq!(res, 255);
    }

    // ------------------------- least_significant_bit tests -------------------------

    #[test]
    fn lsb_errors_on_zero() {
        let x = U256::zero();
        let res = least_significant_bit(x);
        assert!(res.is_none());
    }

    #[test]
    fn lsb_of_power_of_two() {
        // 1 << 12 => LSB = 12
        let x = U256::from(1u128 << 12);
        let res = least_significant_bit(x).unwrap();
        assert_eq!(res, 12);
    }

    #[test]
    fn lsb_of_multiple_bits() {
        // binary: 1011001000 -> LSB is position 3 (0-based)
        let x = U256::from(0b1011001000u128);
        let res = least_significant_bit(x).unwrap();
        assert_eq!(res, 3);
    }

    #[test]
    fn lsb_of_max_u256() {
        // U256::MAX ends with ...1111, so LSB = 0
        let x = U256::max_value();
        let res = least_significant_bit(x).unwrap();
        assert_eq!(res, 0);
    }
}
