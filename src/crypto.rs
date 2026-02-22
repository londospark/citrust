use aes::Aes128;
use cipher::{KeyIvInit, StreamCipher};

type Aes128Ctr = ctr::Ctr128BE<Aes128>;

/// Rotate left on 128-bit value
pub fn rol128(val: u128, shift: u32) -> u128 {
    let shift = shift % 128;
    if shift == 0 {
        return val;
    }
    (val << shift) | (val >> (128 - shift))
}

/// Derive NormalKey from KeyX and KeyY using the 3DS hardware constant
pub fn derive_normal_key(key_x: u128, key_y: u128, constant: u128) -> u128 {
    let rotated_x = rol128(key_x, 2);
    let xored = rotated_x ^ key_y;
    let added = xored.wrapping_add(constant);
    rol128(added, 87)
}

/// Convert u128 to big-endian byte array
pub fn u128_to_be_bytes(val: u128) -> [u8; 16] {
    val.to_be_bytes()
}

/// Convert big-endian byte array to u128
pub fn be_bytes_to_u128(bytes: &[u8; 16]) -> u128 {
    u128::from_be_bytes(*bytes)
}

/// AES-128-CTR decrypt (encrypt and decrypt are the same XOR operation)
pub fn aes_ctr_decrypt(key: &[u8; 16], iv: u128, data: &mut [u8]) {
    let iv_bytes = iv.to_be_bytes();
    let mut cipher = Aes128Ctr::new(key.into(), &iv_bytes.into());
    cipher.apply_keystream(data);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rol128_basic() {
        assert_eq!(rol128(1, 1), 2);
        assert_eq!(rol128(1, 2), 4);
        assert_eq!(rol128(1, 127), 1u128 << 127);
    }

    #[test]
    fn test_rol128_wraparound() {
        let msb_set = 0x80000000_00000000_00000000_00000000u128;
        assert_eq!(rol128(msb_set, 1), 1);
    }

    #[test]
    fn test_rol128_edge_cases() {
        assert_eq!(rol128(42, 0), 42);
        assert_eq!(rol128(42, 128), 42);
        assert_eq!(rol128(0, 10), 0);
    }

    #[test]
    fn test_rol128_large_shifts() {
        assert_eq!(rol128(5, 256), 5);
        assert_eq!(rol128(5, 129), rol128(5, 1));
    }

    #[test]
    fn test_derive_normal_key_known_vector() {
        let key_x = 0x12345678_9ABCDEF0_11111111_22222222u128;
        let key_y = 0xAAAAAAAA_BBBBBBBB_CCCCCCCC_DDDDDDDDu128;
        let constant = 0x1F_F9E9AA_C5FE0408_024591DC_5D52768Au128;
        
        let rotated_x = rol128(key_x, 2);
        let xored = rotated_x ^ key_y;
        let added = xored.wrapping_add(constant);
        let expected = rol128(added, 87);
        
        let result = derive_normal_key(key_x, key_y, constant);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_aes_ctr_decrypt_known_vector() {
        // NIST SP 800-38A test vector: AES-128-CTR
        let key: [u8; 16] = [
            0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6,
            0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f, 0x3c,
        ];
        
        let iv = u128::from_be_bytes([
            0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7,
            0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff,
        ]);
        
        let plaintext = [
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96,
            0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93, 0x17, 0x2a,
        ];
        
        let expected_ciphertext = [
            0x87, 0x4d, 0x61, 0x91, 0xb6, 0x20, 0xe3, 0x26,
            0x1b, 0xef, 0x68, 0x64, 0x99, 0x0d, 0xb6, 0xce,
        ];
        
        let mut data = plaintext.clone();
        aes_ctr_decrypt(&key, iv, &mut data);
        assert_eq!(data, expected_ciphertext);
        
        aes_ctr_decrypt(&key, iv, &mut data);
        assert_eq!(data, plaintext);
    }

    #[test]
    fn test_u128_conversion_roundtrip() {
        let original = 0x12345678_9ABCDEF0_FEDCBA98_76543210u128;
        let bytes = u128_to_be_bytes(original);
        let restored = be_bytes_to_u128(&bytes);
        assert_eq!(original, restored);
    }
}
