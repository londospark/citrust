pub type Key128 = [u8; 16];

pub const KEY_X_2C: u128 = u128::from_be_bytes([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
]);
pub const KEY_X_25: u128 = u128::from_be_bytes([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
]);
pub const KEY_X_18: u128 = u128::from_be_bytes([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
]);
pub const KEY_X_1B: u128 = u128::from_be_bytes([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
]);

pub const KEY_X_2C_DEV: u128 = u128::from_be_bytes([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
]);
pub const KEY_X_25_DEV: u128 = u128::from_be_bytes([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
]);
pub const KEY_X_18_DEV: u128 = u128::from_be_bytes([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
]);
pub const KEY_X_1B_DEV: u128 = u128::from_be_bytes([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
]);

pub const CONSTANT: u128 = u128::from_be_bytes([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoMethod {
    Original,
    Key7x,
    Key93,
    Key96,
}

impl CryptoMethod {
    pub fn from_flag(flag: u8) -> Option<CryptoMethod> {
        match flag {
            0x00 => Some(CryptoMethod::Original),
            0x01 => Some(CryptoMethod::Key7x),
            0x0A => Some(CryptoMethod::Key93),
            0x0B => Some(CryptoMethod::Key96),
            _ => None,
        }
    }
}

pub fn key_x_for_method(method: CryptoMethod) -> u128 {
    match method {
        CryptoMethod::Original => KEY_X_2C,
        CryptoMethod::Key7x => KEY_X_25,
        CryptoMethod::Key93 => KEY_X_18,
        CryptoMethod::Key96 => KEY_X_1B,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_x_for_method_returns_correct_keys() {
        assert_eq!(key_x_for_method(CryptoMethod::Original), KEY_X_2C);
        assert_eq!(key_x_for_method(CryptoMethod::Key7x), KEY_X_25);
        assert_eq!(key_x_for_method(CryptoMethod::Key93), KEY_X_18);
        assert_eq!(key_x_for_method(CryptoMethod::Key96), KEY_X_1B);
    }

    #[test]
    fn test_all_key_constants_are_nonzero() {
        assert_ne!(KEY_X_2C, 0);
        assert_ne!(KEY_X_25, 0);
        assert_ne!(KEY_X_18, 0);
        assert_ne!(KEY_X_1B, 0);
        assert_ne!(KEY_X_2C_DEV, 0);
        assert_ne!(KEY_X_25_DEV, 0);
        assert_ne!(KEY_X_18_DEV, 0);
        assert_ne!(KEY_X_1B_DEV, 0);
        assert_ne!(CONSTANT, 0);
    }

    #[test]
    fn test_crypto_method_from_flag_maps_correctly() {
        assert_eq!(CryptoMethod::from_flag(0x00), Some(CryptoMethod::Original));
        assert_eq!(CryptoMethod::from_flag(0x01), Some(CryptoMethod::Key7x));
        assert_eq!(CryptoMethod::from_flag(0x0A), Some(CryptoMethod::Key93));
        assert_eq!(CryptoMethod::from_flag(0x0B), Some(CryptoMethod::Key96));
        assert_eq!(CryptoMethod::from_flag(0xFF), None);
        assert_eq!(CryptoMethod::from_flag(0x02), None);
    }
}
