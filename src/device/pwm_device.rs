use crate::data_schema::Schema;
use crate::{
    data_schema::{DataSchema, DetailDataSchema},
    storage::{StorageEntry, StorageService},
};
use esp_idf_hal::{
    gpio::OutputPin,
    ledc::{config::TimerConfig, LedcChannel, LedcDriver, LedcTimer, LedcTimerDriver, Resolution},
    peripheral::Peripheral,
    units::Hertz,
};
use futures_lite::future::or;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[derive(Clone)]
pub struct PWMDevice<'a> {
    dev: Rc<RefCell<esp_idf_hal::ledc::LedcDriver<'a>>>,
    store_val: Rc<RefCell<u32>>,
    min: u32,
    max: u32,
    title: StorageEntry,
    duty: StorageEntry,
    state: StorageEntry,
}

impl<'a> PWMDevice<'a> {
    pub fn new<T: LedcTimer, C: LedcChannel>(
        name: &str,
        timer: impl Peripheral<P = T> + 'a,
        channel: impl Peripheral<P = C> + 'a,
        pin: impl Peripheral<P = impl OutputPin> + 'a,
        storage: StorageService,
    ) -> Self {
        let timer_config = TimerConfig::new()
            .frequency(Hertz(5000))
            .resolution(Resolution::Bits10);
        let timer = LedcTimerDriver::new(timer, &timer_config).unwrap();
        let channel = LedcDriver::new(channel, timer, pin, &timer_config).unwrap();
        let max = channel.get_max_duty();

        Self {
            min: 0,
            max,
            store_val: Rc::new(RefCell::new(channel.get_duty())),
            dev: Rc::new(RefCell::new(channel)),
            title: storage.entry(&format!("{name}_title")),
            duty: storage.entry(&format!("{name}_duty")),
            state: storage.entry(&format!("{name}_state")),
        }
    }
    pub async fn run_handle(&self) {
        let future1 = async {
            loop {
                if let Some(new) = self.duty.wait_new().await.as_u64() {
                    if new >= self.min.into() && new <= self.max.into() {
                        if new != self.min as u64 {
                            *self.store_val.borrow_mut() = self.dev.borrow().get_duty();
                        }
                        self.dev.borrow_mut().set_duty(new as u32).ok();
                    }
                }
            }
        };
        let future2 = async {
            loop {
                if let Some(new) = self.state.wait_new().await.as_bool() {
                    if new {
                        let current = *self.store_val.borrow();
                        if current == 0 {
                            self.dev.borrow_mut().set_duty(self.max).unwrap();
                        } else {
                            self.dev.borrow_mut().set_duty(current).unwrap();
                        }
                    } else {
                        self.dev.borrow_mut().set_duty(0).unwrap();
                    }
                }
            }
        };

        or(future1, future2).await
    }
}

impl<'a> Schema for PWMDevice<'a> {
    fn get_schema(&self) -> BTreeMap<String, DataSchema> {
        let onoff_field = DataSchema {
            id: self.state.get_key().to_string(),
            title: Some(String::from("Trạng thái")),
            detail: DetailDataSchema::String,
            ..Default::default()
        };
        let level_field = DataSchema {
            id: self.duty.get_key().to_string(),
            title: Some(String::from("Trạng thái")),
            detail: DetailDataSchema::String,
            ..Default::default()
        };
        let mut properties = BTreeMap::new();
        properties.insert(onoff_field.id.to_owned(), onoff_field);
        properties.insert(level_field.id.to_owned(), level_field);
        properties
    }
}
