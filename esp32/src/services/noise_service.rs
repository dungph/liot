use std::cell::RefCell;
use std::cell::RefMut;
use std::rc::Rc;
use std::time::Duration;

use anyhow::Result;
use base58::ToBase58;
use serde::Deserialize;
use serde::Serialize;
use snow::HandshakeState;
use snow::TransportState;

use crate::storage;
use crate::utils::sleep;
use crate::utils::Connection;

pub struct Handshake(Box<HandshakeState>);

impl std::ops::DerefMut for Handshake {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for Handshake {
    type Target = Box<HandshakeState>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
pub struct Transport(Box<TransportState>);

impl std::ops::DerefMut for Transport {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl std::ops::Deref for Transport {
    type Target = Box<TransportState>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TransportMsg<'a> {
    data: &'a [u8],
    nonce: u64,
}

impl Handshake {
    pub fn new(init: bool) -> Result<Self> {
        let pkey = storage::private_key()?;
        let builder = snow::Builder::new("Noise_IX_25519_ChaChaPoly_BLAKE2s".parse().unwrap())
            .local_private_key(&pkey);

        let handshake = if init {
            Box::new(builder.build_initiator().unwrap())
        } else {
            Box::new(builder.build_responder().unwrap())
        };
        Ok(Self(handshake))
    }
    pub fn init() -> Result<Self> {
        Self::new(true)
    }
    pub fn resp() -> Result<Self> {
        Self::new(false)
    }
    pub fn read_message(&mut self, buf: &[u8]) -> Result<Vec<u8>> {
        let mut out = [0u8; 224];
        let len = self.0.read_message(buf, &mut out)?;
        Ok(out[..len].to_vec())
    }
    pub fn write_message(&mut self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut out = [0u8; 224];
        let len = self.0.write_message(payload, &mut out)?;
        Ok(out[..len].to_vec())
    }
    pub fn remote_static(&self) -> Option<[u8; 32]> {
        self.0.get_remote_static().map(|v| v.try_into().unwrap())
    }
    pub fn to_transport(self) -> Transport {
        let transport = self.0.into_transport_mode().unwrap();
        Transport(Box::new(transport))
    }
}

impl Transport {
    pub fn new(transport: TransportState) -> Self {
        Transport(Box::new(transport))
    }
    pub fn read_message(&mut self, buf: &[u8]) -> Result<Vec<u8>> {
        let msg: TransportMsg = postcard::from_bytes(buf)?;
        if msg.nonce >= self.0.receiving_nonce() {
            self.0.set_receiving_nonce(msg.nonce)
        }

        let mut out = [0u8; 224];
        let len = self.0.read_message(msg.data, &mut out)?;
        Ok(out[..len].to_vec())
    }
    pub fn write_message(&mut self, payload: &[u8]) -> Result<Vec<u8>> {
        let mut out = [0u8; 224];
        let nonce = self.0.sending_nonce();
        let len = self.0.write_message(payload, &mut out)?;
        let msg = TransportMsg {
            nonce,
            data: &out[..len],
        };
        Ok(postcard::to_allocvec(&msg)?)
    }
    pub fn remote_static(&self) -> [u8; 32] {
        self.0
            .get_remote_static()
            .map(|v| v.try_into().unwrap())
            .unwrap()
    }
}
//#[derive(Serialize, Deserialize, Debug)]
//enum NoiseFrame {
//    HandshakeRequest,
//    Handshake1(Vec<u8>),
//    Handshake2(Vec<u8>),
//    Payload(u64, Vec<u8>),
//}
#[derive(Serialize, Deserialize, Debug)]
struct TransportFrame {
    nonce: u64,
    data: Vec<u8>,
}

#[derive(Clone)]
pub struct TransportSocket<Socket> {
    noise: Rc<RefCell<Transport>>,
    socket: Socket,
}

impl<Socket> TransportSocket<Socket>
where
    Socket: Connection,
{
    pub async fn handshake(con: Socket) -> Result<Self> {
        let handshake = if con.is_init() {
            let mut handshake = Handshake::init()?;
            let out = handshake.write_message(&[])?;
            con.send(&out).await?;
            handshake.read_message(&con.recv().await?)?;
            handshake
        } else {
            let mut handshake = Handshake::resp()?;
            handshake.read_message(&con.recv().await?)?;
            let out = handshake.write_message(&[])?;
            con.send(&out).await?;
            handshake
        };
        Ok(Self {
            noise: Rc::new(RefCell::new(handshake.to_transport())),
            socket: con,
        })
    }
    async fn get_noise(&self) -> RefMut<Transport> {
        loop {
            if let Ok(noise) = self.noise.try_borrow_mut() {
                break noise;
            } else {
                sleep(Duration::from_millis(10)).await
            }
        }
    }
    pub async fn remote_static(&self) -> [u8; 32] {
        self.get_noise().await.remote_static()
    }
}

#[async_trait::async_trait(?Send)]
impl<Socket: Connection> Connection for TransportSocket<Socket> {
    async fn remote_id(&self) -> Vec<u8> {
        let full = self.get_noise().await.remote_static().to_base58();
        full[..6].as_bytes().to_vec()
    }
    async fn send(&self, data: &[u8]) -> Result<()> {
        let len = 224.min(data.len());
        let data = &data[..len];
        let out = self.get_noise().await.write_message(&data[..len])?;
        self.socket.send(&out).await?;
        Ok(())
    }
    async fn recv(&self) -> Result<Vec<u8>> {
        let data = self.socket.recv().await?;
        let mut noise = self.get_noise().await;
        let out = noise.read_message(&data)?;
        Ok(out)
    }

    fn is_init(&self) -> bool {
        self.socket.is_init()
    }
}
