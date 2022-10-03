use anyhow::Result;
use async_channel::{bounded, Receiver, Sender};
use esp_idf_svc::espnow::{EspNowClient as EspNow, BROADCAST};
use esp_idf_sys::esp_wifi_get_mac;
use once_cell::sync::Lazy;
use postcard::to_allocvec;
use serde::{Deserialize, Serialize};

use std::collections::{btree_map::Entry, BTreeMap};

use crate::utils::Connection;

type Channel<T> = (Sender<T>, Receiver<T>);

static MAC: Lazy<[u8; 6]> = Lazy::new(|| {
    let mut mac = [0u8; 6];
    unsafe {
        esp_wifi_get_mac(0, &mut mac as *mut u8);
    }
    mac
});
static INCOMING: Lazy<Channel<([u8; 6], Receiver<EspNowPacket>)>> = Lazy::new(|| bounded(10));
static ESPNOW: Lazy<EspNow> = Lazy::new(|| {
    let espnow = EspNow::new().unwrap();
    espnow
        .add_peer(esp_idf_sys::esp_now_peer_info {
            peer_addr: BROADCAST,
            ifidx: 0,
            ..Default::default()
        })
        .unwrap();
    let mut handlers: BTreeMap<[u8; 6], Sender<EspNowPacket>> = BTreeMap::new();
    espnow
        .register_recv_cb(move |addr, data| {
            let addr = addr.try_into().unwrap();
            if let Ok(packet) = postcard::from_bytes::<EspNowPacket>(data) {
                handlers.retain(|_, v| !v.is_closed());
                match handlers.entry(addr) {
                    Entry::Vacant(e) => {
                        let (tx, rx) = bounded(10);
                        e.insert(tx).try_send(packet).ok();
                        INCOMING.0.try_send((addr, rx)).ok();
                    }
                    Entry::Occupied(e) => {
                        e.get().try_send(packet).ok();
                    }
                }
            }
        })
        .unwrap();
    espnow
});
fn espnow_send(addr: [u8; 6], data: EspNowPacketData) -> Result<()> {
    Ok(ESPNOW.send(BROADCAST, &to_allocvec(&EspNowPacket { addr, data })?)?)
}
pub async fn advertise() -> Result<()> {
    espnow_send(BROADCAST, EspNowPacketData::Hello)
}
pub async fn next_channel() -> EspNowChannel {
    let (addr, rx) = INCOMING.1.recv().await.unwrap();

    EspNowChannel { addr, rx }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum EspNowPacketData {
    Hello,
    Data(Vec<u8>),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct EspNowPacket {
    pub addr: [u8; 6],
    pub data: EspNowPacketData,
}
#[derive(Clone)]
pub struct EspNowChannel {
    addr: [u8; 6],
    rx: Receiver<EspNowPacket>,
}
#[async_trait::async_trait(?Send)]
impl Connection for EspNowChannel {
    fn is_init(&self) -> bool {
        self.is_initializer()
    }
    async fn remote_id(&self) -> Vec<u8> {
        self.addr().to_vec()
    }
    async fn send(&self, data: &[u8]) -> anyhow::Result<()> {
        self.send(data).await
    }
    async fn recv(&self) -> anyhow::Result<Vec<u8>> {
        self.recv().await
    }
}

impl EspNowChannel {
    pub fn is_initializer(&self) -> bool {
        *MAC > self.addr
    }
    fn send_msg(&self, data: Vec<u8>) -> Result<()> {
        ESPNOW.send(
            BROADCAST,
            &to_allocvec(&EspNowPacket {
                addr: self.addr,
                data: EspNowPacketData::Data(data),
            })?,
        )?;
        Ok(())
    }
    pub async fn recv(&self) -> Result<Vec<u8>> {
        loop {
            let packet = self.rx.recv().await?;
            match (packet.addr, packet.data) {
                (mac, EspNowPacketData::Hello) if mac == BROADCAST => {
                    ESPNOW
                        .send(
                            BROADCAST,
                            &to_allocvec(&EspNowPacket {
                                addr: self.addr,
                                data: EspNowPacketData::Hello,
                            })
                            .unwrap(),
                        )
                        .unwrap();
                }

                (mac, EspNowPacketData::Data(vec)) if mac == *MAC => break Ok(vec),
                _ => {}
            }
        }
    }
    pub async fn send(&self, data: &[u8]) -> Result<()> {
        self.send_msg(data.to_vec())
    }
    pub fn addr(&self) -> [u8; 6] {
        self.addr
    }
}
