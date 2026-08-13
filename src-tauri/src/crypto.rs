use aes::cipher::{BlockEncryptMut, KeyIvInit};
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
    let padding_len = 16 - (buffer.len() % 16);
    buffer.resize(buffer.len() + padding_len, padding_len as u8);

    let ciphertext = cipher.encrypt_padded_vec_mut(&buffer)
        .map_err(|e| format!("Encryption failed: {}", e))?;

    let mut combined = Vec::with_capacity(16 + ciphertext.len());
    combined.extend_from_slice(&iv);
    combined.extend_from_slice(&ciphertext);

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
    let plaintext = cipher.decrypt_padded_vec_mut(&mut buffer)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| format!("UTF-8 decode failed: {}", e))
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