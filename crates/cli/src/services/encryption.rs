use aes_gcm::{
    aead::{Aead, AeadCore, Error, Key, KeyInit, OsRng},
    Aes256Gcm,
};

pub struct Encryption;

impl Encryption {
    pub fn generate_key() -> Key<Aes256Gcm> {
        Aes256Gcm::generate_key(OsRng)
    }

    pub fn encrypt(encryption_key: &[u8], data: &[u8]) -> Result<Vec<u8>, Error> {
        let key = Key::<Aes256Gcm>::from_slice(encryption_key);

        let cipher = Aes256Gcm::new(key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, data)?;

        let mut result = vec![nonce.len() as u8];
        let mut nonce_vec = nonce.to_vec();
        nonce_vec.extend(ciphertext);
        result.extend(nonce_vec);

        Ok(result)
    }

    pub fn decrypt(encryption_key: &[u8], data: &[u8]) -> Result<Vec<u8>, Error> {
        let key = Key::<Aes256Gcm>::from_slice(encryption_key);
        let cipher = Aes256Gcm::new(key);

        let nonce_length: usize = (*data.first().ok_or(Error)?).into();
        let nonce = &data[1..=nonce_length];
        let ciphertext = &data[nonce_length + 1..];

        let result = cipher.decrypt(nonce.into(), ciphertext)?;

        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_encryption() {
        let encryption_key = Encryption::generate_key().to_vec();
        let data = b"hello world";
        let ciphertext = Encryption::encrypt(&encryption_key, data).unwrap();
        let result = Encryption::decrypt(&encryption_key, &ciphertext).unwrap();

        assert_eq!(result.as_slice(), data.as_slice());
    }
}
