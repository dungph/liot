use std::{cell::RefCell, collections::BTreeMap, rc::Rc, time::Duration};

use anyhow::Result;
use embedded_svc::storage::RawStorage;
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
use event_listener::Event;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone)]
pub struct StorageService {
    default_nvs: EspDefaultNvsPartition,
    storage: Rc<RefCell<EspNvs<NvsDefault>>>,
    map: Rc<RefCell<BTreeMap<String, DataValue>>>,
    notify: Rc<Event>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct DataValue {
    value: Value,
    writers: Vec<String>,
    readers: Vec<String>,
}

impl StorageService {
    pub fn new() -> Result<Self> {
        let default_nvs = EspDefaultNvsPartition::take()?;
        let storage = Rc::new(RefCell::new(EspNvs::new(
            default_nvs.clone(),
            "storage",
            true,
        )?));

        let mut buf = vec![0; 20480];

        let map = match storage
            .as_ref()
            .borrow_mut()
            .get_raw("data", &mut buf)?
            .map(serde_json::from_slice)
            .transpose()?
        {
            Some(map) => map,
            None => BTreeMap::new(),
        };

        Ok(Self {
            default_nvs,
            storage,
            map: Rc::new(RefCell::new(map)),
            notify: Rc::new(Event::new()),
        })
    }
    pub fn default_nvs(&self) -> EspDefaultNvsPartition {
        self.default_nvs.clone()
    }
    pub fn get(&self, key: &str) -> Value {
        self.get_all(key).value
    }
    pub fn get_or_init(&self, key: &str, get_value: impl Fn() -> Value) -> Value {
        if self.get(key).is_null() {
            self.set(key, get_value());
        }
        self.get(key)
    }
    pub fn get_all(&self, key: &str) -> DataValue {
        self.map.borrow_mut().get(key).cloned().unwrap_or_default()
    }

    pub fn set(&self, key: &str, value: Value) {
        self.map
            .borrow_mut()
            .entry(String::from(key))
            .or_default()
            .value = value;
        self.notify.notify(usize::MAX);
    }

    pub async fn wait_new(&self, key: &str) -> Value {
        let cur = self.get(key);
        loop {
            let new = self.get(key);
            if new == cur {
                self.notify.listen().await;
            } else {
                println!("{new}");
                return new;
            }
        }
    }
    pub fn entry(&self, key: &str) -> StorageEntry {
        StorageEntry {
            storage: self.clone(),
            key: key.to_string(),
        }
    }
    pub async fn periodic_store(&self, duration: Duration) {
        loop {
            futures_timer::Delay::new(duration).await;
            let vec = serde_json::to_vec(&*self.map.borrow()).unwrap();
            self.storage.borrow_mut().set_raw("data", &vec).unwrap();
        }
    }
}

#[derive(Clone)]
pub struct StorageEntry {
    storage: StorageService,
    key: String,
}

impl StorageEntry {
    pub async fn wait_new(&self) -> Value {
        self.storage.wait_new(&self.key).await
    }
    pub fn get_value(&self) -> Value {
        self.storage.get(&self.key)
    }
    pub fn set_value(&self, value: Value) {
        self.storage.set(&self.key, value);
    }
    pub fn get_key(&self) -> &str {
        self.key.as_str()
    }
    pub fn get_or_init(&self, get_value: impl Fn() -> Value) -> Value {
        self.storage.get_or_init(&self.key, get_value)
    }
}
