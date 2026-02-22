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
