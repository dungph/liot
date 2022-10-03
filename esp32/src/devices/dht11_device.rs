use std::{cell::RefCell, fmt::Debug, time::Duration};

use anyhow::anyhow;
use dht11::{Dht11, Measurement};
use embedded_hal::digital::blocking::InputPin as InputPin2;
use embedded_hal_02::digital::v2::{InputPin, OutputPin};
use esp_idf_sys::EspError;
use futures_micro::or;
use serde_json::from_slice;

use crate::{
    storage,
    utils::{sleep, Connection, DeviceData, Message},
};

use super::{Handle, LogicPin};
pub struct Dht11Device<P: InputPin + OutputPin, P2: InputPin2> {
    dht: RefCell<Dht11<P>>,
    light: LogicPin<P2>,
}

#[async_trait::async_trait(?Send)]
impl<E: Debug, P: InputPin<Error = E> + OutputPin<Error = E>, P2: InputPin2> Handle
    for Dht11Device<P, P2>
{
    async fn wait_new_state(&self) -> DeviceData {
        let mut measure = self.measure().await;
        or!(sleep(Duration::from_secs(10)), async {
            loop {
                let new_measure = self.measure().await;
                if new_measure.temperature != measure.temperature
                    || new_measure.humidity != measure.humidity
                {
                    measure = new_measure;
                    break;
                }
                sleep(Duration::from_secs(1)).await
            }
        })
        .await;

        let temperature = measure.temperature as f32 / 10.0;
        let humidity = measure.humidity as f32 / 10.0;
        DeviceData::Environment {
            temperature,
            humidity,
            dark: self.light.get_state().await,
        }
    }
}
impl<E: Debug, P: InputPin<Error = E> + OutputPin<Error = E>, P2: InputPin2> Dht11Device<P, P2> {
    pub fn new(pin: P, light_pin: P2) -> Self {
        let dht = RefCell::new(Dht11::new(pin));
        let light = LogicPin::new(light_pin);
        Self { dht, light }
    }

    pub async fn measure(&self) -> Measurement {
        loop {
            if let Ok(mut inner) = self.dht.try_borrow_mut() {
                let mut delay = esp_idf_hal::delay::FreeRtos;
                if let Ok(measurement) = inner.perform_measurement(&mut delay) {
                    if measurement.humidity < 1000 {
                        break measurement;
                    }
                }
            }
            sleep(Duration::from_millis(100)).await;
        }
    }
}
