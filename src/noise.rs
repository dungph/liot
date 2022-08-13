pub mod cipherstate;
pub mod handshake;
pub mod symmetric;
pub mod transport;

pub use handshake::HandshakeXX;
pub use transport::{NoiseRead, NoiseWrite, Transport};

const DHKEYLEN: usize = 32;
const TAGLEN: usize = 16;
const KEYLEN: usize = 32;
const HASHLEN: usize = 32;
const BLOCKLEN: usize = 64;

#[derive(Debug)]
pub enum Error {
    Input,
    Decrypt,
    NotMyTurn,
    NeedUpgrade,
}
