use crate::data_schema::Schema;
use crate::data_schema::ThingSchema;
use crate::espnow::{self, EspNowService};
use crate::storage::{StorageEntry, StorageService};
use crate::wifi::WifiService;
use base58::ToBase58;
use serde_json::Value;
use std::{collections::BTreeMap, time::Duration};

#[async_trait::async_trait(?Send)]
pub trait Connection {
    fn is_init(&self) -> bool;
    async fn remote_id(&self) -> Vec<u8>;
    async fn remote_name(&self) -> String {
        self.remote_id().await.to_base58()
    }
    async fn send(&self, data: &[u8]) -> anyhow::Result<()>;
    async fn recv(&self) -> anyhow::Result<Vec<u8>>;
}

pub struct Controller<'a> {
    id: String,
    wifi: WifiService<'a>,
    devices: Vec<Box<dyn Schema>>,
    espnow: EspNowService,
    title: StorageEntry,
    //external_data: RefCell<BTreeMap<String, Value>>,
}

impl<'a> Controller<'a> {
    pub fn new(
        name: &str,
        wifi: WifiService<'a>,
        storage: &StorageService,
        device: Vec<Box<dyn Schema>>,
        espnow: EspNowService,
    ) -> Self {
        Self {
            id: name.to_string(),
            wifi,
            devices: device,
            espnow,
            title: storage.entry("thing_title"),
            //external_data: RefCell::new(BTreeMap::new()),
        }
    }

    pub fn get_schema(&self) -> ThingSchema {
        let mut properties = BTreeMap::new();
        for device in &self.devices {
            let device_schema = device.get_schema();
            properties.insert(device_schema.id.clone(), device_schema);
        }
        let wifi_schema = self.wifi.get_schema();
        properties.insert(wifi_schema.id.clone(), wifi_schema);
        //let mut setting_properties = BTreeMap::new();

        //setting_properties.extend(self.wifi.get_schema());
        //properties.insert(
        //    String::from("setting"),
        //    DataSchema {
        //        id: self.id.clone(),
        //        title: Some(String::from("Setting")),
        //        detail: crate::data_schema::DetailDataSchema::Object {
        //            properties: setting_properties,
        //        },
        //        ..Default::default()
        //    },
        //);

        //for device in &self.devices {
        //    properties.extend(device.get_schema());
        //}

        ThingSchema {
            id: espnow::get_mac().to_base58(),
            title: self
                .title
                .get_or_init(|| Value::String(String::from("Title")))
                .as_str()
                .map(|v| v.to_string()),
            properties,
            ..Default::default()
        }
    }

    //pub async fn handle_setting(&self, value: &Value) {}
    //pub fn get_status(&self) -> DataSchema {}
    //pub fn get_setting(&self) -> DataSchema {
    //    let mut map = BTreeMap::new();
    //    if self.wifi.is_connected().unwrap_or(false) {
    //    } else {
    //        self.get_wifi_setting(&mut map);
    //    }
    //    Data {
    //        id: String::from("ssid"),
    //        title: String::from("Ten WIFI"),
    //        description: None,
    //        value: DataValue::Object { value: map },
    //        ..Default::default()
    //    }
    //}
    //pub fn thing_data(&self, data: BTreeMap<String, Data>) -> ThingData {
    //    ThingData {
    //        id: self.id.clone(),
    //        title: self.title.borrow().as_string().unwrap().clone(),
    //        data,
    //        external: self.external_data.borrow().clone(),
    //    }
    //}
    //pub fn get_data(&self) -> ThingData {
    //    let mut map = BTreeMap::new();
    //    let dev_data = self.device.get_data_schema();
    //    if let Some(obj) = dev_data.value.as_object() {
    //        for (key, value) in obj {
    //            map.insert(key.clone(), value.clone());
    //        }
    //    } else {
    //        map.insert(dev_data.id.to_owned(), dev_data.clone());
    //    }
    //    map.insert(String::from("setting"), self.get_setting());
    //    map.insert(String::from("status"), self.get_status());
    //    self.thing_data(map)
    //}
    //pub fn set_other_val(&self, dev: &str, data: &Value) {
    //    if let Some(dev_data) = self.external_data.borrow_mut().get_mut(dev) {
    //        merge_value(dev_data, data);
    //    }
    //}
    pub async fn run_handle(&self) {
        //let task1 = async {
        //    let dat = self.get_data();
        //    self.http_serve.set_data(dat).await;
        //    loop {
        //        self.wait_changed().await;
        //        let dat = self.get_data();
        //        self.http_serve.set_data(dat).await;
        //    }
        //};
        //let task2 = async {
        //    loop {
        //        let dat = self.http_serve.get_data().await;
        //        self.handle_setting(&dat["data"]).await;
        //        self.device.set_data(&dat["data"]);
        //        self.notify_change.notify(MAX);
        //    }
        //};
        //let task3 = async {
        //    loop {
        //        let channel = self.espnow.next_channel().await;
        //        let task1 = async {
        //            while let Ok(msg) = dbg!(channel.recv_json::<Value>().await) {
        //                self.set_other_val(&channel.remote_name().await, &msg);
        //            }
        //        };
        //        let task2 = async {
        //            loop {
        //                self.wait_changed().await;
        //                let data = self.device.get_data();
        //                let thing_data = json!({ "value": data });
        //                if let Ok(()) = channel.send_json(&dbg!(thing_data)) {
        //                } else {
        //                    break;
        //                }
        //            }
        //        };
        //        or(task1, task2).await;
        //    }
        //};
        let task4 = async {
            loop {
                self.espnow.advertise().unwrap();
                futures_timer::Delay::new(Duration::from_secs(3)).await
            }
        };
        task4.await;
        //zip(zip(task1, task2), zip(task3, task4)).await;
    }
}
