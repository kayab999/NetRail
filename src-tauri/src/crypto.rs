use fernet::Fernet;
use std::env;

pub fn encryption_active() -> bool {
    get_key_material().is_some()
}

pub fn ensure_encryption_key() -> bool {
    get_key_material().is_some() || create_key().is_ok()
}

fn get_key_material() -> Option<String> {
    if let Ok(key) = env::var("NETRAIL_DB_KEY") {
        if !key.is_empty() {
            return Some(key);
        }
    }
    let entry = keyring::Entry::new("netrail", "db-key").ok()?;
    entry.get_password().ok()
}

fn create_key() -> Result<String, keyring::Error> {
    let key = Fernet::generate_key();
    let entry = keyring::Entry::new("netrail", "db-key")?;
    entry.set_password(&key)?;
    Ok(key)
}

pub fn encrypt_text(value: &str, use_encryption: bool) -> Vec<u8> {
    if !use_encryption || value.is_empty() {
        return value.as_bytes().to_vec();
    }
    let Some(key) = get_key_material() else {
        return value.as_bytes().to_vec();
    };
    let Some(fernet) = Fernet::new(&key) else {
        return value.as_bytes().to_vec();
    };
    fernet.encrypt(value.as_bytes()).into_bytes()
}

pub const DECRYPTION_FAILED_MARKER: &str = "[DECRYPTION_FAILED]";

const FERNET_PREFIX: &str = "gAAAAA";

pub fn decrypt_text(blob: &[u8], use_encryption: bool) -> String {
    if blob.is_empty() {
        return String::new();
    }
    let token = String::from_utf8_lossy(blob);
    // Every Fernet token is base64 and starts with version byte 0x80, which
    // encodes to the literal prefix "gAAAAA". Blobs without it are plaintext
    // rows passed through untouched; blobs with it are encrypted rows and
    // surface a marker instead of base64 garbage when they cannot be opened.
    if !token.starts_with(FERNET_PREFIX) {
        return token.into_owned();
    }
    if !use_encryption {
        return DECRYPTION_FAILED_MARKER.to_string();
    }
    let Some(key) = get_key_material() else {
        return DECRYPTION_FAILED_MARKER.to_string();
    };
    let Some(fernet) = Fernet::new(&key) else {
        return DECRYPTION_FAILED_MARKER.to_string();
    };
    fernet
        .decrypt(&token)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_else(|_| DECRYPTION_FAILED_MARKER.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn roundtrip_matches_python_fernet_format() {
        let key = Fernet::generate_key();
        std::env::set_var("NETRAIL_DB_KEY", &key);
        let encrypted = encrypt_text("battery regulations EU", true);
        let token = String::from_utf8_lossy(&encrypted);
        assert!(token.starts_with("gAAAAA"));
        let decrypted = decrypt_text(&encrypted, true);
        assert_eq!(decrypted, "battery regulations EU");
        std::env::remove_var("NETRAIL_DB_KEY");
    }

    #[test]
    #[serial_test::serial]
    fn wrong_key_surfaces_marker_not_garbage() {
        let key = Fernet::generate_key();
        std::env::set_var("NETRAIL_DB_KEY", &key);
        let encrypted = encrypt_text("battery regulations EU", true);
        let other_key = Fernet::generate_key();
        std::env::set_var("NETRAIL_DB_KEY", &other_key);
        let decrypted = decrypt_text(&encrypted, true);
        assert_eq!(decrypted, DECRYPTION_FAILED_MARKER);
        std::env::remove_var("NETRAIL_DB_KEY");
    }

    #[test]
    #[serial_test::serial]
    fn missing_key_surfaces_marker() {
        let key = Fernet::generate_key();
        std::env::set_var("NETRAIL_DB_KEY", &key);
        let encrypted = encrypt_text("battery regulations EU", true);
        std::env::remove_var("NETRAIL_DB_KEY");
        let decrypted = decrypt_text(&encrypted, true);
        assert_eq!(decrypted, DECRYPTION_FAILED_MARKER);
    }

    #[test]
    #[serial_test::serial]
    fn corrupt_token_surfaces_marker() {
        std::env::set_var("NETRAIL_DB_KEY", Fernet::generate_key());
        let token = b"gAAAAAnot-a-valid-token";
        assert_eq!(decrypt_text(token, true), DECRYPTION_FAILED_MARKER);
        std::env::remove_var("NETRAIL_DB_KEY");
    }

    #[test]
    #[serial_test::serial]
    fn legacy_plaintext_passes_through() {
        std::env::set_var("NETRAIL_DB_KEY", Fernet::generate_key());
        let plain = b"pre-encryption era row";
        assert_eq!(decrypt_text(plain, true), "pre-encryption era row");
        std::env::remove_var("NETRAIL_DB_KEY");
    }

    #[test]
    #[serial_test::serial]
    fn empty_and_plaintext_mode() {
        std::env::set_var("NETRAIL_DB_KEY", Fernet::generate_key());
        assert_eq!(decrypt_text(b"", true), "");
        let encrypted = encrypt_text("battery regulations EU", true);
        assert_eq!(decrypt_text(&encrypted, false), DECRYPTION_FAILED_MARKER);
        std::env::remove_var("NETRAIL_DB_KEY");
    }
}