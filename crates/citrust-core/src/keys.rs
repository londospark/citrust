pub type Key128 = [u8; 16];

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

#[cfg(test)]
mod tests {
    use super::*;

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
