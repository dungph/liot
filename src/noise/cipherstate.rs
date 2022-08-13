use chacha20poly1305::aead::AeadInPlace;
use chacha20poly1305::{ChaCha20Poly1305, KeyInit};

use super::Error;

pub const TAG_LEN: usize = 16;
#[derive(Clone)]
pub struct CipherState {
    c: ChaCha20Poly1305,
    pub(super) n: u64,
}

impl CipherState {
    pub(crate) fn new(k: [u8; 32]) -> Self {
        Self {
            c: ChaCha20Poly1305::new(&k.into()),
            n: 0,
        }
    }
    pub(crate) fn set_nonce(&mut self, nonce: u64) {
        self.n = nonce
    }
    pub(crate) fn encrypt_with_ad(
        &mut self,
        ad: &[u8],
        plaintext: &[u8],
        ciphertext: &mut [u8],
    ) -> Result<usize, Error> {
        let len = plaintext.len() + TAG_LEN;
        if ciphertext.len() < len {
            return Err(Error::Input);
        }

        let (ciphertext, rest) = ciphertext.split_at_mut(plaintext.len());
        let (ciphertext_mac, _) = rest.split_at_mut(TAG_LEN);

        ciphertext.copy_from_slice(plaintext);

        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..].copy_from_slice(&self.n.to_le_bytes());

        let tag = self
            .c
            .encrypt_in_place_detached(&nonce_bytes.into(), ad, ciphertext)
            .unwrap();

        self.n += 1;
        ciphertext_mac.copy_from_slice(&tag);
        Ok(len)
    }
    pub(crate) fn decrypt_with_ad(
        &mut self,
        ad: &[u8],
        ciphertext: &[u8],
        plaintext: &mut [u8],
    ) -> Result<usize, Error> {
        if ciphertext.len() < TAG_LEN {
            return Err(Error::Input);
        }
        let len = ciphertext.len() - TAG_LEN;
        if plaintext.len() < len {
            return Err(Error::Input);
        }

        let (ciphertext, ciphertext_mac) = ciphertext.split_at(len);
        let (plaintext, _) = plaintext.split_at_mut(len);

        plaintext.copy_from_slice(ciphertext);

        let mut nonce_bytes = [0u8; 12];
        nonce_bytes[4..].copy_from_slice(&self.n.to_le_bytes());

        self.c
            .decrypt_in_place_detached(&nonce_bytes.into(), ad, plaintext, ciphertext_mac.into())
            .map_err(|_| Error::Decrypt)?;

        self.n += 1;
        Ok(len)
    }
}
