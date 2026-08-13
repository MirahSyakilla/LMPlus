use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use base64::Engine;
use hmac::Mac;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

const SERVER_PUBKEY_PEM: &str = "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA785rTbA9EYjsITD0kjnx\nYAO9+lYcH836cdn9YsByxdG6/WYJqByL4ZPSOS3nCSTTl+NGUnSnffUs1Da6R6OU\nVpBpu9KdIrfPv4RWPZk2a9pcOqDC/bqcQ1deJWpUGDLjxzrkuXhxjXigg2jroGwY\nNLrE4KpHRcsQIpPubApambBSANjVNfWMlo42dm7sJg671xuuxwPz5+CxHo3vRDDp\n0whqDhn+e/nOV4rpxOb+z/XdA2iWJ9uj+dYXVwzWjkYNrzZz8ob14zSSeqURpzDK\nQJ6oT6quWu9vrZT+H9DZ5NGg0MAZ2gnc7Un/pBnobhCqgbRtaZGZ0nrvgWGFNM2z\niwIDAQAB\n-----END PUBLIC KEY-----";

const VERIFY_URL: &str = "https://lmp.nobullypls.site/verify";

fn exe_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Failed to get exe path: {}", e))?;
    exe.parent()
        .map(|p| p.to_path_buf())
        .ok_or("Failed to get exe directory".to_string())
}

fn config_path() -> Result<PathBuf, String> {
    Ok(exe_dir()?.join("config.cfg"))
}

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    hex::decode(hex).map_err(|e| format!("Invalid hex: {}", e))
}

fn aes_encrypt(data: &[u8], key_hex: &str) -> Result<String, String> {
    let key = hex_to_bytes(key_hex)?;
    let mut iv = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut iv);

    let cipher = Aes256CbcEnc::new(key.as_slice().into(), &iv.into());
    let mut buffer = data.to_vec();
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

fn aes_decrypt(data_b64: &str, key_hex: &str) -> Result<Vec<u8>, String> {
    let key = hex_to_bytes(key_hex)?;
    let combined = base64::engine::general_purpose::STANDARD
        .decode(data_b64)
        .map_err(|e| format!("Base64 decode: {}", e))?;

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

    Ok(pt.to_vec())
}

fn read_config() -> BTreeMap<String, BTreeMap<String, String>> {
    let path = config_path().unwrap_or_default();
    let content = fs::read_to_string(&path).unwrap_or_default();
    let mut map: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut section = String::new();
    for line in content.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with(';') || t.starts_with('#') {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') {
            section = t[1..t.len() - 1].to_string();
            map.entry(section.clone()).or_default();
            continue;
        }
        if let Some(eq) = t.find('=') {
            let k = t[..eq].trim().to_string();
            let v = t[eq + 1..].trim().to_string();
            map.entry(section.clone()).or_default().insert(k, v);
        }
    }
    map
}

fn write_config(map: &BTreeMap<String, BTreeMap<String, String>>) -> Result<(), String> {
    let path = config_path()?;
    let mut content = String::new();
    for (section, keys) in map {
        content.push_str(&format!("[{}]\n", section));
        for (k, v) in keys {
            content.push_str(&format!("{}={}\n", k, v));
        }
        content.push('\n');
    }
    fs::write(&path, content).map_err(|e| format!("Failed to write config: {}", e))
}

fn compute_config_signature(
    map: &BTreeMap<String, BTreeMap<String, String>>,
    fingerprint: &str,
) -> String {
    let mut content = String::new();
    for keys in map.values() {
        for (k, v) in keys {
            if k == "config_signature" {
                continue;
            }
            content.push_str(&format!("{}={}\n", k, v));
        }
    }
    let key = hex_to_bytes(fingerprint).unwrap_or_default();
    let mut mac = hmac::Hmac::<sha2::Sha256>::new_from_slice(&key).unwrap();
    mac.update(content.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
}

fn verify_config_signature(fingerprint: &str) -> Result<bool, String> {
    let map = read_config();
    let stored_sig = map
        .get("license")
        .and_then(|l| l.get("config_signature"))
        .cloned()
        .unwrap_or_default();
    if stored_sig.is_empty() {
        return Ok(false);
    }
    let computed = compute_config_signature(&map, fingerprint);
    Ok(computed == stored_sig)
}

fn sign_config(
    map: &mut BTreeMap<String, BTreeMap<String, String>>,
    fingerprint: &str,
) {
    let sig = compute_config_signature(map, fingerprint);
    map.entry("license".to_string())
        .or_default()
        .insert("config_signature".to_string(), sig);
}

fn verify_server_signature(payload: &str, sig_b64: &str) -> Result<bool, String> {
    use rsa::pkcs8::DecodePublicKey;
    use rsa::traits::SignatureScheme;
    use rsa::Pkcs1v15Sign;

    let pub_key = rsa::RsaPublicKey::from_public_key_pem(SERVER_PUBKEY_PEM)
        .map_err(|e| format!("Failed to parse public key: {}", e))?;

    let sig = base64::engine::general_purpose::STANDARD
        .decode(sig_b64)
        .map_err(|e| format!("Failed to decode signature: {}", e))?;

    let scheme = Pkcs1v15Sign::new::<sha2::Sha256>();
    scheme
        .verify(&pub_key, payload.as_bytes(), &sig)
        .map(|_| true)
        .map_err(|e| format!("Signature verification failed: {}", e))
}

fn urlencode(s: &str) -> String {
    let mut result = String::new();
    for byte in s.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(*byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

#[tauri::command]
pub async fn verify_license(
    key: String,
    fingerprint: String,
) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let url = format!(
        "{}?k={}&d={}&v=2",
        VERIFY_URL,
        urlencode(&key),
        urlencode(&fingerprint)
    );

    let resp = client
        .get(&url)
        .header("User-Agent", "LMP-Client/2 (Rust)")
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    let status = resp.status();
    let body = resp.text().await.map_err(|e| format!("Read response: {}", e))?;

    if !status.is_success() {
        return Err(format!("Server error: {}", status));
    }

    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Parse JSON: {}", e))?;

    let rstatus = json.get("status").and_then(|v| v.as_str()).unwrap_or("");

    if rstatus == "ok" {
        let token = json.get("token").and_then(|v| v.as_str()).unwrap_or("");
        let sig = json.get("sig").and_then(|v| v.as_str()).unwrap_or("");
        let exp = json.get("exp").and_then(|v| v.as_i64()).unwrap_or(0);
        let end_date = json.get("end_date").and_then(|v| v.as_i64());

        let payload = format!("{}|{}|{}", token, exp, fingerprint);

        if !verify_server_signature(&payload, sig)? {
            return Err("Server signature verification failed".to_string());
        }

        let now = chrono::Utc::now().timestamp();
        if exp <= now {
            return Err("License token expired".to_string());
        }

        let encrypted = aes_encrypt(key.as_bytes(), &fingerprint)?;

        let mut map = read_config();
        let license = map.entry("license".to_string()).or_default();
        license.insert("ek".to_string(), encrypted);
        license.insert("exp".to_string(), exp.to_string());
        license.insert("token".to_string(), token.to_string());
        license.insert(
            "last_attempt".to_string(),
            chrono::Utc::now().to_rfc3339(),
        );
        license.insert("attempt_count".to_string(), "0".to_string());
        sign_config(&mut map, &fingerprint);
        write_config(&map)?;

        let license_info = if let Some(end_secs) = end_date {
            if end_secs <= now {
                return Err("License expired".to_string());
            }
            let dt = chrono::DateTime::from_timestamp(end_secs, 0)
                .unwrap_or_default()
                .with_timezone(&chrono::Local);
            dt.format("%Y-%m-%d %I:%M %p").to_string()
        } else {
            "Permanent".to_string()
        };

        Ok(serde_json::json!({
            "status": "ok",
            "license_info": license_info,
        }))
    } else {
        let msg = json
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");
        Err(msg.to_string())
    }
}

#[tauri::command]
pub fn load_saved_license(fingerprint: String) -> Result<serde_json::Value, String> {
    let map = read_config();
    let ek = map
        .get("license")
        .and_then(|l| l.get("ek"))
        .cloned()
        .unwrap_or_default();

    if ek.is_empty() {
        return Ok(serde_json::json!({"key": null}));
    }

    let decrypted = aes_decrypt(&ek, &fingerprint)?;
    let key = String::from_utf8(decrypted).map_err(|e| format!("UTF-8: {}", e))?;

    Ok(serde_json::json!({"key": key}))
}

#[tauri::command]
pub fn get_config_signature_status(fingerprint: String) -> Result<bool, String> {
    verify_config_signature(&fingerprint)
}
