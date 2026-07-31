// API-key protection. Windows: DPAPI (CryptProtectData, user-scoped).
// Other platforms (dev): pass-through so keys stay usable on macOS/Linux builds.
//
// Stored form: "dpapi:<base64>". Plaintext entries without the prefix are
// legacy configs and decrypt as-is; the next save re-encrypts them.

#[cfg(windows)]
mod imp {
    use base64::{engine::general_purpose::STANDARD as B64, Engine};
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    const PREFIX: &str = "dpapi:";

    pub fn encrypt(plain: &str) -> String {
        if plain.is_empty() {
            return String::new();
        }
        let bytes = plain.as_bytes();
        let mut in_blob = CRYPT_INTEGER_BLOB {
            cbData: bytes.len() as u32,
            pbData: bytes.as_ptr() as *mut u8,
        };
        let mut out = CRYPT_INTEGER_BLOB::default();
        let result = unsafe {
            CryptProtectData(
                &in_blob,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out,
            )
        };
        if result.is_err() {
            return plain.to_string();
        }
        let cipher = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec();
        unsafe {
            let _ = LocalFree(Some(HLOCAL(out.pbData.cast())));
        }
        format!("{PREFIX}{}", B64.encode(&cipher))
    }

    pub fn decrypt(stored: &str) -> Option<String> {
        let b64 = stored.strip_prefix(PREFIX)?;
        let cipher = B64.decode(b64).ok()?;
        let mut in_blob = CRYPT_INTEGER_BLOB {
            cbData: cipher.len() as u32,
            pbData: cipher.as_ptr() as *mut u8,
        };
        let mut out = CRYPT_INTEGER_BLOB::default();
        let result = unsafe {
            CryptUnprotectData(&in_blob, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out)
        };
        if result.is_err() {
            return None;
        }
        let plain = unsafe { std::slice::from_raw_parts(out.pbData, out.cbData as usize) }.to_vec();
        unsafe {
            let _ = LocalFree(Some(HLOCAL(out.pbData.cast())));
        }
        String::from_utf8(plain).ok()
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn encrypt(plain: &str) -> String {
        plain.to_string()
    }

    pub fn decrypt(stored: &str) -> Option<String> {
        Some(stored.to_string())
    }
}

pub use imp::{decrypt, encrypt};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let plain = "sk-test-123";
        let stored = encrypt(plain);
        assert_eq!(decrypt(&stored).as_deref(), Some(plain));
    }

    #[test]
    fn empty_stays_empty() {
        assert_eq!(encrypt(""), "");
        assert_eq!(decrypt(""), Some(String::new()));
    }
}
