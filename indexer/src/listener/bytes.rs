use bigdecimal::BigDecimal;
use byte_slice_cast::AsByteSlice;
use num_bigint::{BigInt, Sign};

/// Converts a byte slice to a `BigDecimal`
pub fn bytes_to_bigdecimal<T>(byte_slice: impl AsByteSlice<T>) -> BigDecimal {
    let bigint = BigInt::from_bytes_le(Sign::Plus, byte_slice.as_byte_slice());
    BigDecimal::from(bigint)
}

/// Converts a `BigDecimal` to a byte slice
pub fn bigdecimal_to_bytes<'a, T>(bigdecimal: BigDecimal) -> T
where
    T: From<[u8; 8]>,
{
    let (bigint, _) = bigdecimal.as_bigint_and_exponent();
    let (_, bytes) = bigint.to_bytes_le();
    let mut array = [0u8; 8];
    array[..bytes.len()].copy_from_slice(&bytes);
    T::from(array)
}
