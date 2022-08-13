use blake2::{Blake2s256, Digest};

use super::{cipherstate::CipherState, Error, BLOCKLEN, HASHLEN, KEYLEN};

#[derive(Clone)]
pub(super) struct SymmetricState {
    ck: [u8; HASHLEN],
    h: [u8; HASHLEN],
    cipher: CipherState,
    has_key: bool,
}

impl SymmetricState {
    pub(crate) fn new(name: &[u8]) -> Self {
        let h = if name.len() < HASHLEN {
            let mut h = [0; HASHLEN];
            h[..name.len()].copy_from_slice(name);
            h
        } else {
            hash(name)
        };
        Self {
            ck: h,
            h,
            cipher: CipherState::new([0u8; 32]),
            has_key: false,
        }
    }
    pub(crate) fn mix_key(&mut self, input_key_material: &[u8]) {
        let temp_k;
        [self.ck, temp_k] = hkdf(self.ck.as_slice(), input_key_material);
        self.cipher = CipherState::new(temp_k);
        self.has_key = true;
    }
    pub(crate) fn mix_hash(&mut self, data: &[u8]) {
        self.h = hash_with_context(self.h.as_slice(), data)
    }
    pub(crate) fn encrypt_and_hash(
        &mut self,
        payload: &[u8],
        message: &mut [u8],
    ) -> Result<usize, Error> {
        let len = if self.has_key {
            self.cipher.encrypt_with_ad(&self.h, payload, message)?
        } else {
            if message.len() < payload.len() {
                return Err(Error::Input);
            }
            let (message, _) = message.split_at_mut(payload.len());
            message.copy_from_slice(payload);
            payload.len()
        };
        self.mix_hash(&message[..len]);
        Ok(len)
    }
    pub(crate) fn decrypt_and_hash(
        &mut self,
        message: &[u8],
        payload: &mut [u8],
    ) -> Result<usize, Error> {
        let len = if self.has_key {
            self.cipher.decrypt_with_ad(&self.h, message, payload)?
        } else {
            let (payload, _) = payload.split_at_mut(message.len());
            payload.copy_from_slice(message);
            message.len()
        };
        self.mix_hash(message);
        Ok(len)
    }

    pub(crate) fn split(self) -> (CipherState, CipherState) {
        let [k1, k2] = hkdf(self.ck.as_slice(), &[]);
        (CipherState::new(k1), CipherState::new(k2))
    }
}

pub(crate) fn hash(data: &[u8]) -> [u8; HASHLEN] {
    let mut context = Blake2s256::new();
    context.update(data);
    context.finalize().into()
}

pub(crate) fn hash_with_context(con: &[u8], data: &[u8]) -> [u8; HASHLEN] {
    let mut context = Blake2s256::new();
    context.update(con);
    context.update(data);
    context.finalize().into()
}

pub(crate) fn hmac(key: &[u8], data: &[u8], out: &mut [u8]) {
    let mut context = Blake2s256::new();
    let mut ipad = [0x36_u8; BLOCKLEN];
    let mut opad = [0x5c_u8; BLOCKLEN];
    for count in 0..key.len() {
        ipad[count] ^= key[count];
        opad[count] ^= key[count];
    }
    context.update(ipad.as_slice());
    context.update(data);
    let inner_output = context.finalize_reset();

    context.update(opad.as_slice());
    context.update(inner_output.as_slice());
    out.copy_from_slice(context.finalize().as_slice());
}

pub(crate) fn hkdf<const N: usize>(
    chaining_key: &[u8],
    input_key_material: &[u8],
) -> [[u8; HASHLEN]; N] {
    if N < 1 || 3 < N {
        panic!()
    }
    let mut out = [[0; HASHLEN]; N];
    let mut temp_key = [0; KEYLEN];
    hmac(chaining_key, input_key_material, &mut temp_key);
    hmac(&temp_key, &[1u8], &mut out[0]);
    for i in 1..N {
        let mut in2 = [0_u8; HASHLEN + 1];
        in2[..HASHLEN].copy_from_slice(&out[i - 1][..HASHLEN]);
        in2[HASHLEN] = i as u8 + 1;
        hmac(&temp_key, in2.as_slice(), &mut out[i]);
    }
    out
}
