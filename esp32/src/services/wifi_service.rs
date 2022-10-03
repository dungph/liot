use std::{
    cell::{RefCell, RefMut},
    collections::BTreeMap,
    convert::TryInto,
    rc::Rc,
    time::Duration,
};

use async_channel::{bounded, Receiver, Sender};
use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_sys::{esp_wifi_get_mac, EspError};

use postcard::to_allocvec;
use serde::{Deserialize, Serialize};

use crate::{storage, timer::sleep};

#[derive(Serialize, Deserialize, Debug)]
pub enum EspNowPacketData {
    Hello,
    Init(Vec<u8>),
    Resp(Vec<u8>),
    Data(u64, Vec<u8>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EspNowPacket {
    pub addr: [u8; 6],
    pub data: EspNowPacketData,
}

type TxRxChannel<T> = (Sender<T>, Receiver<T>);
pub struct WifiService<'a> {
    wifi: RefCell<WifiDriver<'a, WifiModem>>,
}

impl<'a> WifiService<'a> {
    pub fn new(ssid: &str, pwd: &str) -> Self {
        let wifi = {
            let modem = unsafe { WifiModem::new() };
            let sysloop = EspSystemEventLoop::take().unwrap();
            let nvs = storage::take();
            let mut wifi = WifiDriver::new(modem, sysloop, Some(nvs)).unwrap();

            wifi.set_configuration(&Configuration::Client(ClientConfiguration {
                ssid: ssid.into(),
                auth_method: if pwd.is_empty() {
                    AuthMethod::None
                } else {
                    AuthMethod::WPA2Personal
                },
                password: pwd.into(),
                ..Default::default()
            }))
            .unwrap();
            wifi.start().unwrap();
            wifi
        };
        Self {
            wifi: RefCell::new(wifi),
        }
    }
    async fn get_wifi(&self) -> RefMut<WifiDriver<'a, WifiModem>> {
        loop {
            if let Ok(wifi) = self.wifi.try_borrow_mut() {
                return wifi;
            } else {
                sleep(Duration::from_millis(50)).await
            }
        }
    }
    pub async fn set_sta(&self, ssid: heapless::String<32>, password: heapless::String<64>) {
        let sta = ClientConfiguration {
            ssid,
            auth_method: if password.is_empty() {
                AuthMethod::None
            } else {
                AuthMethod::WPA2Personal
            },
            password,
            ..Default::default()
        };
        let mut wifi = self.get_wifi().await;
        wifi.set_configuration(&Configuration::Client(sta)).unwrap();
        wifi.start().unwrap();
    }
    pub async fn connect(&self) -> Result<(), EspError> {
        self.get_wifi().await.connect()
    }
    pub async fn is_connected(&self) -> Result<bool, EspError> {
        self.get_wifi().await.is_connected()
    }
    pub async fn disconnect(&self) -> Result<(), EspError> {
        self.get_wifi().await.disconnect()
    }
}
