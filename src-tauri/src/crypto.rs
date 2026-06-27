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

pub fn decrypt_text(blob: &[u8], use_encryption: bool) -> String {
    if blob.is_empty() {
        return String::new();
    }
    if !use_encryption {
        return String::from_utf8_lossy(blob).into_owned();
    }
    let Some(key) = get_key_material() else {
        return String::from_utf8_lossy(blob).into_owned();
    };
    let Some(fernet) = Fernet::new(&key) else {
        return String::from_utf8_lossy(blob).into_owned();
    };
    let token = String::from_utf8_lossy(blob);
    fernet
        .decrypt(&token)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_else(|_| String::from_utf8_lossy(blob).into_owned())
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
}