use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use rand::rngs::OsRng;
use serde::{Deserialize, Deserializer, Serializer};
use sha2::Digest;

const LEGACY_FIXED_NONCE: &[u8; 12] = b"antigravsalt";
const ENCRYPTED_PREFIX: &str = "ag_enc_";
const ENCRYPTED_V2_PREFIX: &str = "ag_enc_v2_";

fn get_encryption_key() -> Result<[u8; 32], String> {
    encryption_key_from_device_id(machine_uid::get().map_err(|_| {
        "Machine identity unavailable; refusing to use a shared encryption key".to_string()
    }))
}

fn encryption_key_from_device_id(device_id: Result<String, String>) -> Result<[u8; 32], String> {
    let device_id = device_id?;
    if device_id.trim().is_empty() {
        return Err(
            "Machine identity is empty; refusing to use a shared encryption key".to_string(),
        );
    }
    Ok(sha2::Sha256::digest(device_id.as_bytes()).into())
}

pub fn serialize_password<S>(password: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if password.starts_with(ENCRYPTED_PREFIX) || password.starts_with(ENCRYPTED_V2_PREFIX) {
        return serializer.serialize_str(password);
    }

    let encrypted = encrypt_string(password).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(&encrypted)
}

pub fn deserialize_password<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    if raw.is_empty() {
        return Ok(raw);
    }

    if raw.starts_with(ENCRYPTED_V2_PREFIX) {
        match decrypt_string_v2(&raw[ENCRYPTED_V2_PREFIX.len()..]) {
            Ok(plaintext) => Ok(plaintext),
            Err(_) => {
                tracing::warn!(
                    "password decryption failed, key likely changed (machine-id differs from when it was encrypted)"
                );
                Ok(raw)
            }
        }
    } else if raw.starts_with(ENCRYPTED_PREFIX) {
        match decrypt_legacy(&raw[ENCRYPTED_PREFIX.len()..]) {
            Ok(plaintext) => Ok(plaintext),
            Err(_) => {
                tracing::warn!(
                    "password decryption failed, key likely changed (machine-id differs from when it was encrypted)"
                );
                Ok(raw)
            }
        }
    } else {
        match decrypt_legacy(&raw) {
            Ok(plaintext) => Ok(plaintext),
            Err(_) => Ok(raw),
        }
    }
}

pub fn encrypt_string(password: &str) -> Result<String, String> {
    let key = get_encryption_key()?;
    let cipher = Aes256Gcm::new(&key.into());

    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    let ciphertext = cipher
        .encrypt(&nonce, password.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    let encoded_nonce = general_purpose::STANDARD_NO_PAD.encode(nonce);
    let encoded_ciphertext = general_purpose::STANDARD_NO_PAD.encode(ciphertext);
    Ok(format!(
        "{}{}.{}",
        ENCRYPTED_V2_PREFIX, encoded_nonce, encoded_ciphertext
    ))
}

fn decrypt_legacy(encrypted_base64: &str) -> Result<String, String> {
    decrypt_legacy_with_key(encrypted_base64, get_encryption_key()?)
}

fn decrypt_legacy_with_key(encrypted_base64: &str, key: [u8; 32]) -> Result<String, String> {
    let cipher = Aes256Gcm::new(&key.into());
    let nonce = Nonce::from_slice(LEGACY_FIXED_NONCE);

    let ciphertext = general_purpose::STANDARD
        .decode(encrypted_base64)
        .map_err(|e| format!("Base64 decode failed: {}", e))?;

    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| format!("Decryption failed: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| format!("UTF-8 conversion failed: {}", e))
}

fn decrypt_string_v2(encrypted: &str) -> Result<String, String> {
    let (nonce_base64, ciphertext_base64) = encrypted
        .split_once('.')
        .ok_or_else(|| "Invalid encrypted payload".to_string())?;

    let nonce_bytes = general_purpose::STANDARD_NO_PAD
        .decode(nonce_base64)
        .map_err(|e| format!("Nonce decode failed: {}", e))?;
    if nonce_bytes.len() != 12 {
        return Err("Invalid nonce length".to_string());
    }

    let ciphertext = general_purpose::STANDARD_NO_PAD
        .decode(ciphertext_base64)
        .map_err(|e| format!("Ciphertext decode failed: {}", e))?;

    let key = get_encryption_key()?;
    let cipher = Aes256Gcm::new(&key.into());
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
        .map_err(|e| format!("Decryption failed: {}", e))?;

    String::from_utf8(plaintext).map_err(|e| format!("UTF-8 conversion failed: {}", e))
}

pub fn decrypt_string(encrypted: &str) -> Result<String, String> {
    if encrypted.starts_with(ENCRYPTED_V2_PREFIX) {
        decrypt_string_v2(&encrypted[ENCRYPTED_V2_PREFIX.len()..])
    } else if encrypted.starts_with(ENCRYPTED_PREFIX) {
        decrypt_legacy(&encrypted[ENCRYPTED_PREFIX.len()..])
    } else {
        decrypt_legacy(encrypted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_password() -> String {
        general_purpose::STANDARD_NO_PAD.encode(Aes256Gcm::generate_nonce(&mut OsRng))
    }

    #[test]
    fn test_machine_identity_failure_has_no_fallback_key() {
        assert!(encryption_key_from_device_id(Err("unavailable".to_string())).is_err());
        assert!(encryption_key_from_device_id(Ok("  ".to_string())).is_err());
    }

    #[test]
    fn test_encrypt_decrypt_cycle() {
        let password = random_password();
        let encrypted = encrypt_string(&password).unwrap();

        assert!(encrypted.starts_with(ENCRYPTED_V2_PREFIX));
        assert_ne!(password, encrypted);

        let decrypted = decrypt_string(&encrypted).unwrap();
        assert_eq!(password, decrypted);
    }

    #[test]
    fn test_encrypt_uses_unique_nonce() {
        let password = random_password();
        let encrypted_a = encrypt_string(&password).unwrap();
        let encrypted_b = encrypt_string(&password).unwrap();

        assert_ne!(encrypted_a, encrypted_b);
        assert_eq!(decrypt_string(&encrypted_a).unwrap(), password);
        assert_eq!(decrypt_string(&encrypted_b).unwrap(), password);
    }

    #[test]
    fn test_tampered_ciphertext_is_rejected() {
        let encrypted = encrypt_string(&random_password()).unwrap();
        let payload = encrypted.strip_prefix(ENCRYPTED_V2_PREFIX).unwrap();
        let (nonce, ciphertext) = payload.split_once('.').unwrap();
        let mut ciphertext = general_purpose::STANDARD_NO_PAD.decode(ciphertext).unwrap();
        ciphertext[0] ^= 1;
        let tampered = format!(
            "{}{}.{}",
            ENCRYPTED_V2_PREFIX,
            nonce,
            general_purpose::STANDARD_NO_PAD.encode(ciphertext)
        );
        assert!(decrypt_string(&tampered).is_err());
    }

    #[test]
    fn test_legacy_compatibility() {
        #[derive(Deserialize)]
        struct HistoricalPassword {
            device_id: String,
            ciphertext: String,
            plaintext: String,
        }

        // Public, independently produced historical test vector. Its device ID
        // is test data, never a fallback identity or an application secret.
        let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/legacy-password.json");
        let fixture: HistoricalPassword =
            serde_json::from_slice(&std::fs::read(fixture_path).unwrap()).unwrap();
        let key = encryption_key_from_device_id(Ok(fixture.device_id)).unwrap();
        let decrypted = decrypt_legacy_with_key(&fixture.ciphertext, key).unwrap();
        assert_eq!(fixture.plaintext, decrypted);
    }

    #[derive(Deserialize)]
    struct PasswordContainer {
        #[serde(deserialize_with = "deserialize_password")]
        password: String,
    }

    #[test]
    fn test_deserialize_password_plaintext() {
        let json = r#"{"password": "plain_password_123"}"#;
        let parsed: PasswordContainer = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.password, "plain_password_123");
    }

    #[test]
    fn test_deserialize_password_valid_encrypted() {
        let password = random_password();
        let encrypted = encrypt_string(&password).unwrap();
        let json = format!(r#"{{"password": "{}"}}"#, encrypted);
        let parsed: PasswordContainer = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.password, password);
    }

    #[test]
    fn test_deserialize_password_failed_decryption_returns_raw() {
        // Corrupted v2 payload: should return raw string as fallback
        let corrupted_v2 = "ag_enc_v2_invalidnonce.invalidciphertext";
        let json_v2 = format!(r#"{{"password": "{}"}}"#, corrupted_v2);
        let parsed_v2: PasswordContainer = serde_json::from_str(&json_v2).unwrap();
        assert_eq!(parsed_v2.password, corrupted_v2);

        // Corrupted legacy payload with ag_enc_ prefix: should return raw string as fallback
        let corrupted_legacy = "ag_enc_invalidbase64content==";
        let json_legacy = format!(r#"{{"password": "{}"}}"#, corrupted_legacy);
        let parsed_legacy: PasswordContainer = serde_json::from_str(&json_legacy).unwrap();
        assert_eq!(parsed_legacy.password, corrupted_legacy);
    }
}
