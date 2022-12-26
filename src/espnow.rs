use anyhow::Result;
use async_channel::{bounded, Receiver, Sender};
use async_mutex::Mutex;
use dashmap::DashMap;
use esp_idf_svc::espnow::{EspNow, BROADCAST};
use esp_idf_sys::esp_wifi_get_mac;
use serde::{de::DeserializeOwned, Serialize};
use std::{borrow::Borrow, rc::Rc};

use crate::{controller::Connection, wifi::WifiService};

pub fn get_mac() -> [u8; 6] {
    let mut mac = [0u8; 6];
    unsafe {
        esp_wifi_get_mac(0, &mut mac as *mut u8);
    }
    mac
}

type Incoming = Receiver<([u8; 6], Receiver<Vec<u8>>)>;
type IncomingTx = Sender<([u8; 6], Receiver<Vec<u8>>)>;
#[derive(Clone)]
pub struct EspNowService {
    espnow: Rc<EspNow>,
    incoming: Incoming,
    incoming_tx: IncomingTx,
    handlers: DashMap<[u8; 6], Sender<Vec<u8>>>,
    raw_rx: Receiver<([u8; 6], Vec<u8>)>,
}

impl EspNowService {
    pub fn new(wifi: &WifiService) -> anyhow::Result<Self> {
        let interface = wifi.active_interface();
        let espnow = EspNow::take()?;
        espnow.add_peer(esp_idf_sys::esp_now_peer_info {
            peer_addr: BROADCAST,
            ifidx: interface,
            ..Default::default()
        })?;
        let (incoming_tx, incoming) = bounded(10);
        let (raw_tx, raw_rx) = bounded(10);

        espnow.register_recv_cb(move |addr, data| {
            let addr: [u8; 6] = addr.try_into().unwrap();
            raw_tx.try_send((addr, data.to_vec())).ok();
        })?;
        Ok(Self {
            espnow: Rc::new(espnow),
            incoming,
            incoming_tx,
            raw_rx,
            handlers: DashMap::new(),
        })
    }
    pub async fn reactor_tick(&self) {
        static RUN_LOCK: Mutex<()> = Mutex::new(());
        if RUN_LOCK.try_lock().is_some() {
            if let Ok((addr, data)) = self.raw_rx.recv().await {
                if !self.espnow.peer_exists(addr).unwrap() {
                    self.espnow
                        .add_peer(esp_idf_sys::esp_now_peer_info {
                            peer_addr: addr,
                            channel: 0,
                            ifidx: 1,
                            ..Default::default()
                        })
                        .unwrap();
                    let (tx, rx) = bounded(10);
                    self.handlers.insert(addr, tx.clone());
                    self.incoming_tx.send((addr, rx)).await.unwrap();
                }
                self.handlers.retain(|_k, s| !s.is_closed());
                if let Some(sender) = self.handlers.get(&addr) {
                    sender.send(data).await.ok();
                }
            }
        }
    }
    pub async fn run_handle(&self) {
        while let Ok((addr, data)) = self.raw_rx.recv().await {
            if !self.espnow.peer_exists(addr).unwrap() {
                self.espnow
                    .add_peer(esp_idf_sys::esp_now_peer_info {
                        peer_addr: addr,
                        channel: 0,
                        ifidx: 1,
                        ..Default::default()
                    })
                    .unwrap();
                let (tx, rx) = bounded(10);
                self.handlers.insert(addr, tx.clone());
                self.incoming_tx.send((addr, rx)).await.unwrap();
            }
            self.handlers.retain(|_k, s| !s.is_closed());
            if let Some(sender) = self.handlers.get(&addr) {
                sender.send(data).await.ok();
            }
        }
    }
    pub fn send(&self, addr: [u8; 6], data: &[u8]) -> Result<()> {
        self.espnow.as_ref().borrow().send(addr, data)?;
        Ok(())
    }
    pub async fn find_peer(&self) {}
    pub fn advertise(&self) -> Result<()> {
        self.send(BROADCAST, &postcard::to_allocvec(&(None as Option<()>))?)
    }
    pub async fn next_channel(&self) -> EspNowChannel {
        let (addr, rx) = self.incoming.recv().await.unwrap();
        println!("new channel");
        EspNowChannel {
            espnow: self.clone(),
            addr,
            rx,
        }
    }
}

#[derive(Clone)]
pub struct EspNowChannel {
    espnow: EspNowService,
    addr: [u8; 6],
    rx: Receiver<Vec<u8>>,
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
        self.send(data)?;
        Ok(())
    }
    async fn recv(&self) -> anyhow::Result<Vec<u8>> {
        self.recv().await
    }
}

impl EspNowChannel {
    pub fn is_initializer(&self) -> bool {
        get_mac() > self.addr
    }
    pub fn send(&self, data: &[u8]) -> Result<()> {
        self.espnow
            .send(self.addr, &postcard::to_allocvec(&Some(data))?)?;
        Ok(())
    }
    pub async fn recv(&self) -> Result<Vec<u8>> {
        loop {
            let recv = self.rx.recv().await?;
            if let Some(vec) = postcard::from_bytes(&recv)? {
                break Ok(vec);
            }
        }
    }
    pub fn send_json(&self, data: &impl Serialize) -> Result<()> {
        let vec = serde_json::to_vec(data)?;
        self.send(&vec)?;
        Ok(())
    }
    pub async fn recv_json<T: DeserializeOwned>(&self) -> Result<T> {
        let vec = self.recv().await?;
        let dat = serde_json::from_slice(&vec)?;
        Ok(dat)
    }
    pub fn addr(&self) -> [u8; 6] {
        self.addr
    }
}
