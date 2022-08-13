use super::{cipherstate::CipherState, Error, DHKEYLEN};

pub struct Transport {
    rs: [u8; DHKEYLEN],
    send: CipherState,
    recv: CipherState,
}

pub struct NoiseRead {
    recv: CipherState,
    rs: [u8; 32],
}

pub struct NoiseWrite {
    send: CipherState,
    rs: [u8; 32],
}

impl Transport {
    pub fn new(rs: [u8; DHKEYLEN], send: CipherState, recv: CipherState) -> Self {
        Self { rs, send, recv }
    }

    pub fn remote_key(&self) -> [u8; 32] {
        self.rs
    }
    pub fn set_receive_nonce(&mut self, nonce: u64) {
        self.recv.set_nonce(nonce)
    }
    pub fn send_nonce(&self) -> u64 {
        self.send.n
    }
    pub fn recv_nonce(&self) -> u64 {
        self.recv.n
    }
    pub fn split(self) -> (NoiseRead, NoiseWrite) {
        (
            NoiseRead {
                recv: self.recv,
                rs: self.rs,
            },
            NoiseWrite {
                send: self.send,
                rs: self.rs,
            },
        )
    }
    pub fn read_message(&mut self, message: &[u8], payload: &mut [u8]) -> Result<usize, Error> {
        self.recv.decrypt_with_ad(&[], message, payload)
    }
    pub fn write_message(&mut self, payload: &[u8], message: &mut [u8]) -> Result<usize, Error> {
        self.send.encrypt_with_ad(&[], payload, message)
    }
}

impl NoiseRead {
    pub fn remote_key(&self) -> [u8; 32] {
        self.rs
    }

    pub fn read_message(&mut self, message: &[u8], payload: &mut [u8]) -> Result<usize, Error> {
        self.recv.decrypt_with_ad(&[], message, payload)
    }
}

impl NoiseWrite {
    pub fn remote_key(&self) -> [u8; 32] {
        self.rs
    }
    pub fn write_message(&mut self, payload: &[u8], message: &mut [u8]) -> Result<usize, Error> {
        self.send.encrypt_with_ad(&[], payload, message)
    }
}
