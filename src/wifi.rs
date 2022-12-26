use crate::data_schema::{DataSchema, DetailDataSchema, Schema};
use crate::espnow;
use crate::storage::{StorageEntry, StorageService};
use anyhow::Result;
use base58::ToBase58;
use embedded_svc::wifi::{
    AccessPointConfiguration, AuthMethod, ClientConfiguration, Configuration, Wifi,
};
use esp_idf_hal::modem::Modem;
use esp_idf_svc::{eventloop::EspSystemEventLoop, wifi::EspWifi};
use futures_lite::future::or;
use serde_json::Value;
use std::{cell::RefCell, collections::BTreeMap, net::Ipv4Addr, rc::Rc, time::Duration};

#[derive(Clone)]
pub struct WifiService<'a> {
    wifi: Rc<RefCell<EspWifi<'a>>>,
    ssid_config: StorageEntry,
    password_config: StorageEntry,
    connect_config: StorageEntry,
    status_ip: StorageEntry,
}

impl<'a> WifiService<'a> {
    pub fn new(modem: Modem, storage: &StorageService) -> anyhow::Result<Self> {
        let wifi = EspWifi::new(modem, EspSystemEventLoop::take()?, None)?;
        storage.get_or_init("wifi_config_ssid", || Value::String("example".to_string()));
        storage.get_or_init("wifi_config_password", || {
            Value::String("example".to_string())
        });
        storage.get_or_init("wifi_config_connect", || Value::Bool(false));
        let this = Self {
            wifi: Rc::new(RefCell::new(wifi)),
            ssid_config: storage.entry("wifi_config_ssid"),
            password_config: storage.entry("wifi_config_password"),
            connect_config: storage.entry("wifi_config_connect"),
            status_ip: storage.entry("wifi_status_ip"),
        };
        this.enable_ap()?;
        this.start()?;
        Ok(this)
    }

    pub fn get_connected(&self) -> Result<Option<String>> {
        match self.wifi.as_ref().borrow().get_configuration()? {
            Configuration::Client(sta) => Ok(Some(sta.ssid.to_string())),
            Configuration::Mixed(sta, _) => Ok(Some(sta.ssid.to_string())),
            _ => Ok(None),
        }
    }
    pub fn get_ip(&self) -> Result<Ipv4Addr> {
        Ok(self.wifi.borrow().sta_netif().get_ip_info()?.ip)
    }
    pub fn start(&self) -> Result<()> {
        self.wifi.as_ref().borrow_mut().start()?;
        Ok(())
    }
    pub fn is_started(&self) -> Result<bool> {
        Ok(self.wifi.borrow().is_started()?)
    }
    pub async fn wait_start(&self) -> Result<()> {
        while !self.is_started()? {
            futures_timer::Delay::new(Duration::from_millis(100)).await;
        }
        Ok(())
    }

    fn default_ap_conf() -> AccessPointConfiguration {
        AccessPointConfiguration {
            ssid: "ESP32".into(),
            ssid_hidden: true,
            auth_method: AuthMethod::None,
            max_connections: 0,
            channel: 1,
            ..Default::default()
        }
    }
    fn public_ap_conf() -> AccessPointConfiguration {
        AccessPointConfiguration {
            ssid: format!("ESP32-{}", espnow::get_mac().to_base58())
                .as_str()
                .into(),
            ssid_hidden: false,
            auth_method: AuthMethod::None,
            max_connections: 5,
            channel: 1,
            ..Default::default()
        }
    }
    pub fn enable_ap(&self) -> Result<()> {
        let mut wifi = self.wifi.as_ref().borrow_mut();
        let conf = wifi.get_configuration()?;
        let conf = match conf {
            Configuration::None | Configuration::AccessPoint(_) => {
                Configuration::AccessPoint(Self::public_ap_conf())
            }
            Configuration::Client(sta_conf) | Configuration::Mixed(sta_conf, _) => {
                Configuration::Mixed(sta_conf, Self::public_ap_conf())
            }
        };
        wifi.set_configuration(&conf)?;
        Ok(())
    }
    pub fn disable_ap(&self) -> Result<()> {
        let mut wifi = self.wifi.as_ref().borrow_mut();
        let conf = wifi.get_configuration()?;
        let conf = match conf {
            Configuration::None | Configuration::AccessPoint(_) => {
                Configuration::AccessPoint(Self::default_ap_conf())
            }
            Configuration::Client(sta_conf) | Configuration::Mixed(sta_conf, _) => {
                Configuration::Mixed(sta_conf, Self::default_ap_conf())
            }
        };
        wifi.set_configuration(&conf)?;
        Ok(())
    }
    pub async fn connect(&self, ssid: &str, pwd: &str) -> Result<()> {
        {
            let mut wifi = self.wifi.borrow_mut();
            let conf = wifi.get_configuration()?;
            let sta_conf = ClientConfiguration {
                ssid: ssid.into(),
                password: pwd.into(),
                auth_method: if pwd.is_empty() {
                    AuthMethod::None
                } else {
                    AuthMethod::WPA2Personal
                },
                ..Default::default()
            };
            let conf = match conf {
                Configuration::None | Configuration::Client(_) => Configuration::Client(sta_conf),
                Configuration::AccessPoint(ap_conf) | Configuration::Mixed(_, ap_conf) => {
                    Configuration::Mixed(sta_conf, ap_conf)
                }
            };
            wifi.set_configuration(&conf)?;
            wifi.start()?;
        }
        self.wait_start().await?;
        self.wifi.as_ref().borrow_mut().connect()?;
        Ok(())
    }

    pub fn disconnect(&self) -> Result<()> {
        self.wifi.borrow_mut().disconnect()?;
        Ok(())
    }
    pub fn is_connected(&self) -> Result<bool> {
        Ok(self.wifi.borrow_mut().is_connected()?)
    }

    pub async fn wait_connect(&self) -> Result<()> {
        loop {
            if self.is_connected()? && self.get_ip()? != Ipv4Addr::new(0, 0, 0, 0) {
                break Ok(());
            }
            futures_timer::Delay::new(Duration::from_millis(100)).await
        }
    }
    pub fn active_interface(&self) -> u32 {
        esp_idf_sys::esp_interface_t_ESP_IF_WIFI_AP
    }
    pub async fn run_handle(&self) {
        if let Some(true) = self.connect_config.get().as_bool() {
            futures_timer::Delay::new(Duration::from_millis(500)).await;
            self.connect(
                self.ssid_config.get().as_str().unwrap(),
                self.password_config.get().as_str().unwrap(),
            )
            .await
            .unwrap();
        }
        let future1 = async {
            loop {
                if let Some(true) = self.connect_config.wait_new().await.as_bool() {
                    self.connect(
                        self.ssid_config.get().as_str().unwrap(),
                        self.password_config.get().as_str().unwrap(),
                    )
                    .await
                    .unwrap();
                } else {
                    self.disconnect().ok();
                }
            }
        };
        let future4 = async {
            loop {
                futures_timer::Delay::new(Duration::from_millis(5000)).await;
                let ip = self.get_ip().unwrap_or(Ipv4Addr::new(0, 0, 0, 0));
                self.status_ip
                    .set(serde_json::Value::String(ip.to_string()));
            }
        };
        or(future1, future4).await
    }
}
impl<'a> Schema for WifiService<'a> {
    fn get_schema(&self) -> DataSchema {
        let wifi = DataSchema {
            id: self.ssid_config.get_key().to_string(),
            title: Some(String::from("SSID")),
            detail: DetailDataSchema::String,
            ..Default::default()
        };
        let pwd = DataSchema {
            id: self.password_config.get_key().to_string(),
            title: Some(String::from("Password")),
            detail: DetailDataSchema::String,
            ..Default::default()
        };
        let connected = DataSchema {
            id: self.status_ip.get_key().to_string(),
            title: Some(String::from("Connected socket")),
            detail: DetailDataSchema::String,
            ..Default::default()
        };
        let connect = DataSchema {
            id: self.connect_config.get_key().to_string(),
            title: Some(String::from("Connect")),
            detail: DetailDataSchema::Bool,
            ..Default::default()
        };
        let mut map = BTreeMap::new();
        map.insert(self.ssid_config.get_key().to_string(), wifi);
        map.insert(self.password_config.get_key().to_string(), pwd);
        map.insert(self.connect_config.get_key().to_string(), connect);
        map.insert(self.status_ip.get_key().to_string(), connected);
        DataSchema {
            id: String::from("wifi"),
            title: Some(String::from("Wifi configure")),
            detail: DetailDataSchema::Object { properties: map },
            ..Default::default()
        }
    }
}
