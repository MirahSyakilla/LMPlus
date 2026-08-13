use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use base64::Engine;
use hmac::Mac;
use sha2::Digest;

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    hex::decode(hex).map_err(|e| format!("Invalid hex key: {}", e))
}

#[tauri::command]
pub fn encrypt_aes256_cbc(data: String, key_hex: String) -> Result<String, String> {
    let key = hex_to_bytes(&key_hex)?;
    if key.len() != 32 {
        return Err("Key must be 32 bytes (64 hex chars)".to_string());
    }

    let mut iv = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut iv);

    let cipher = Aes256CbcEnc::new(key.as_slice().into(), &iv.into());
    let mut buffer = data.into_bytes();
    let data_len = buffer.len();
    buffer.resize(data_len + 16, 0);

    let ct = cipher
        .encrypt_padded_mut::<Pkcs7>(&mut buffer, data_len)
        .map_err(|e| format!("Encryption failed: {:?}", e))?;

    let mut combined = Vec::with_capacity(16 + ct.len());
    combined.extend_from_slice(&iv);
    combined.extend_from_slice(ct);

    Ok(base64::engine::general_purpose::STANDARD.encode(&combined))
}

#[tauri::command]
pub fn decrypt_aes256_cbc(data_b64: String, key_hex: String) -> Result<String, String> {
    let key = hex_to_bytes(&key_hex)?;
    if key.len() != 32 {
        return Err("Key must be 32 bytes (64 hex chars)".to_string());
    }

    let combined = base64::engine::general_purpose::STANDARD
        .decode(&data_b64)
        .map_err(|e| format!("Base64 decode failed: {}", e))?;

    if combined.len() < 16 {
        return Err("Data too short".to_string());
    }

    let iv = &combined[..16];
    let ciphertext = &combined[16..];

    let cipher = Aes256CbcDec::new(key.as_slice().into(), iv.into());
    let mut buffer = ciphertext.to_vec();
    let pt = cipher
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .map_err(|e| format!("Decryption failed: {:?}", e))?;

    String::from_utf8(pt.to_vec()).map_err(|e| format!("UTF-8 decode failed: {}", e))
}

#[tauri::command]
pub fn hmac_sha256(data: String, key_hex: String) -> Result<String, String> {
    let key = hex_to_bytes(&key_hex)?;
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(&key)
        .map_err(|e| format!("HMAC init failed: {}", e))?;
    mac.update(data.as_bytes());
    let result = mac.finalize();
    Ok(base64::engine::general_purpose::STANDARD.encode(result.into_bytes()))
}

#[tauri::command]
pub fn sha256_hex(data: String) -> String {
    let mut hasher = sha2::Sha256::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key_hex() -> String {
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string()
    }

    #[test]
    fn aes_roundtrip() {
        let key = test_key_hex();
        let plaintext = "hello lmplus license key 1234567890";
        let enc = encrypt_aes256_cbc(plaintext.to_string(), key.clone()).unwrap();
        let dec = decrypt_aes256_cbc(enc, key).unwrap();
        assert_eq!(dec, plaintext);
    }

    #[test]
    fn aes_roundtrip_empty_aligned_lengths() {
        let key = test_key_hex();
        for s in ["", "a", "exactly16bytes!!", "exactly32byteslongstring!!!!!!!!"] {
            let enc = encrypt_aes256_cbc(s.to_string(), key.clone()).unwrap();
            let dec = decrypt_aes256_cbc(enc, key.clone()).unwrap();
            assert_eq!(dec, s);
        }
    }

    #[test]
    fn aes_rejects_bad_key_length() {
        let err = encrypt_aes256_cbc("data".into(), "abcd".into()).unwrap_err();
        assert!(err.contains("32 bytes"));
    }

    #[test]
    fn aes_ciphertext_has_iv_prefix() {
        let key = test_key_hex();
        let enc = encrypt_aes256_cbc("test".into(), key).unwrap();
        let raw = base64::engine::general_purpose::STANDARD.decode(&enc).unwrap();
        assert!(raw.len() >= 16 + 16);
        assert_eq!(raw.len() % 16, 0);
    }

    #[test]
    fn aes_random_iv_per_call() {
        let key = test_key_hex();
        let a = encrypt_aes256_cbc("same".into(), key.clone()).unwrap();
        let b = encrypt_aes256_cbc("same".into(), key).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn hmac_sha256_matches_rfc4231_vector() {
        // RFC 4231 test case 1
        let key_hex = hex::encode([0x0bu8; 20]);
        let out = hmac_sha256("Hi There".into(), key_hex).unwrap();
        let raw = base64::engine::general_purpose::STANDARD.decode(&out).unwrap();
        assert_eq!(
            hex::encode(raw),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );
    }

    #[test]
    fn sha256_hex_known_vector() {
        assert_eq!(
            sha256_hex("abc".to_string()),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex("".to_string()),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
