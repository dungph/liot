use std::{
    cell::{RefCell, RefMut},
    fmt::Debug,
    ops::DerefMut,
};

use anyhow::Result;
use async_channel::{bounded, Receiver, Sender};
use base58::ToBase58;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use embedded_svc::{
    timer::asynch::{OnceTimer, TimerService},
    utils::{asynch::signal::AtomicSignal, asyncify::timer::AsyncTimerService},
};
use esp_idf_svc::timer::EspTaskTimerService;
use once_cell::sync::Lazy;
use std::time::Duration;

static SERVICE: Lazy<AsyncTimerService<EspTaskTimerService, AtomicSignal<()>>> =
    Lazy::new(|| AsyncTimerService::new(EspTaskTimerService::new().unwrap()));

pub async fn sleep(dur: Duration) {
    SERVICE.clone().timer().unwrap().after(dur).unwrap().await;
}
#[async_trait::async_trait(?Send)]
pub trait Connection {
    fn is_init(&self) -> bool;
    async fn remote_id(&self) -> Vec<u8>;
    async fn send(&self, data: &[u8]) -> Result<()>;
    async fn recv(&self) -> Result<Vec<u8>>;
    async fn send_postcard(&self, data: impl Serialize) -> Result<()> {
        let data = postcard::to_allocvec(&data)?;
        self.send(&data).await
    }
    async fn recv_postcard<T: DeserializeOwned>(&self) -> Result<T> {
        let data = self.recv().await?;
        let out = postcard::from_bytes(&data)?;
        Ok(out)
    }
    async fn send_json(&self, data: impl Serialize) -> Result<()> {
        let data = serde_json::to_vec(&data)?;
        self.send(&data).await
    }
    async fn recv_json<T: DeserializeOwned>(&self) -> Result<T> {
        let data = self.recv().await?;
        let out = serde_json::from_slice(&data)?;
        Ok(out)
    }
}

pub struct InternalConnection {
    init: bool,
    tx: Sender<Vec<u8>>,
    rx: Receiver<Vec<u8>>,
}

impl InternalConnection {
    pub fn pair() -> (InternalConnection, InternalConnection) {
        let (tx1, rx2) = bounded(10);
        let (tx2, rx1) = bounded(10);
        (
            InternalConnection {
                init: true,
                tx: tx1,
                rx: rx1,
            },
            InternalConnection {
                init: false,
                tx: tx2,
                rx: rx2,
            },
        )
    }
}

#[async_trait::async_trait(?Send)]
impl Connection for InternalConnection {
    fn is_init(&self) -> bool {
        self.init
    }

    async fn remote_id(&self) -> Vec<u8> {
        b"internal".to_vec()
    }

    async fn send(&self, data: &[u8]) -> Result<()> {
        self.tx.send(data.to_vec()).await?;
        Ok(())
    }
    async fn recv(&self) -> Result<Vec<u8>> {
        let out = self.rx.recv().await?;
        Ok(out)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Message {
    AddController(String),
    RemoveController(String),
    AddSubscriber(String),
    RemoveSubscriber(String),
    AddManager(String),
    RemoveManager(String),

    Update(DeviceData),
    Control(DeviceData),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum DeviceData {
    Light {
        state: Option<bool>,
        auto_control: Option<bool>,
    },
    Switch {
        state: bool,
    },
    Lock {
        unlock: Option<bool>,
        add_rfid: Option<bool>,
        clear_rfid: Option<bool>,
    },
    Environment {
        temperature: f32,
        humidity: f32,
        dark: bool,
    },
    Motion {
        state: bool,
    },
    Fan {
        state: Option<bool>,
        threshold_temp: Option<i8>,
        auto_control: Option<bool>,
        light_state: Option<bool>,
    },
}

//#[async_trait::async_trait(?Send)]
//trait Lock<T> {
//    type Out: DerefMut<Target = T>;
//    async fn lock(&self) -> Self::Out;
//}
//
//#[async_trait::async_trait(?Send)]
//impl<'a, T> Lock<T> for RefCell<T> {
//    type Out = RefMut<'a, T>;
//    async fn lock(&'a self) -> Self::Out {
//        loop {
//            if let Ok(val) = self.try_borrow_mut() {
//                break val;
//            }
//            sleep(Duration::from_millis(50)).await
//        }
//    }
//}
