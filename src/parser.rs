
use primitive_types::{U256};

// Decimal system string to U256
pub fn parse(s: &str) -> Result<U256,String> {
    let mut ret = U256::zero();
    for (_, &item) in s.as_bytes().iter().enumerate() {
        if item < 48 || item > 57 {
            return Err("Invalid value".to_string());
        }
        let (r , _ )= ret.overflowing_mul(U256::from(10u64));
        let value = item - b'0';
        ret = r + value;
    }
    Ok(ret)
}