use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicI8, AtomicU8};
use std::time::Duration;
use std::{fmt::Debug, sync::atomic::AtomicBool};

use embedded_hal::digital::blocking::{InputPin, OutputPin};
use futures_lite::future::or;
use serde_json::{from_slice, to_vec};

use crate::storage;
use crate::utils::{sleep, Connection, DeviceData, Message};

use super::{Button, Handle, Light, LogicPin};

pub struct Fan<P1, P2: InputPin + OutputPin, P3: InputPin> {
    pin: LogicPin<P1>,
    auto_control: AtomicBool,
    threshold_temp: AtomicI8,
    current_temp: AtomicI8,
    button: Button<P3>,
    light: Light<P2>,
}

impl<P1: InputPin + OutputPin, P2: InputPin + OutputPin, P3: InputPin> Fan<P1, P2, P3> {
    pub fn new(pin: P1, light: P2, button: P3) -> Self {
        let pin = LogicPin::new(pin);
        Self {
            pin,
            auto_control: AtomicBool::new(false),
            threshold_temp: AtomicI8::new(storage::get("fan_threshold_temp").unwrap().unwrap_or(0)),
            current_temp: AtomicI8::new(0),
            light: Light::new(light),
            button: Button::new(button, false),
        }
    }

    pub async fn toggle(&self) {
        let state = self.pin.get_state().await;
        self.pin.set_state(!state).await;
    }
}

#[async_trait::async_trait(?Send)]
impl<P1: InputPin + OutputPin, P2: InputPin + OutputPin, P3: InputPin> Handle for Fan<P1, P2, P3> {
    async fn wait_new_state(&self) -> DeviceData {
        let task1 = async {
            DeviceData::Fan {
                state: Some(self.pin.wait_change().await),
                threshold_temp: Some(self.threshold_temp.load(Relaxed)),
                auto_control: Some(self.auto_control.load(Relaxed)),
                light_state: Some(self.light.get_state().await),
            }
        };
        let task2 = async {
            sleep(Duration::from_secs(10)).await;
            DeviceData::Fan {
                state: Some(self.pin.get_state().await),
                threshold_temp: Some(self.threshold_temp.load(Relaxed)),
                auto_control: Some(self.auto_control.load(Relaxed)),
                light_state: Some(self.light.get_state().await),
            }
        };
        let task3 = async {
            loop {
                if self.auto_control.load(Relaxed) {
                    if self.current_temp.load(Relaxed) > self.threshold_temp.load(Relaxed) {
                        self.pin.set_high().await;
                    } else {
                        self.pin.set_low().await;
                    }
                }
                sleep(Duration::from_millis(200)).await
            }
        };
        let task4 = async {
            let auto_control = self.auto_control.load(Relaxed);
            let threshold = self.threshold_temp.load(Relaxed);
            loop {
                if self.auto_control.load(Relaxed) != auto_control
                    || self.threshold_temp.load(Relaxed) != threshold
                {
                    break DeviceData::Fan {
                        state: Some(self.pin.get_state().await),
                        threshold_temp: Some(self.threshold_temp.load(Relaxed)),
                        auto_control: Some(self.auto_control.load(Relaxed)),
                        light_state: Some(self.light.get_state().await),
                    };
                }
                sleep(Duration::from_millis(100)).await
            }
        };
        let task5 = async {
            self.light.wait_change().await;
            DeviceData::Fan {
                state: Some(self.pin.get_state().await),
                threshold_temp: Some(self.threshold_temp.load(Relaxed)),
                auto_control: Some(self.auto_control.load(Relaxed)),
                light_state: Some(self.light.get_state().await),
            }
        };
        let task6 = async {
            self.button.wait_press().await;
            self.light.toggle().await;
            DeviceData::Fan {
                state: Some(self.pin.get_state().await),
                threshold_temp: Some(self.threshold_temp.load(Relaxed)),
                auto_control: Some(self.auto_control.load(Relaxed)),
                light_state: Some(self.light.get_state().await),
            }
        };
        futures_micro::or!(task1, task2, task3, task4, task5, task6).await
    }
    async fn handle_msg_control(&self, data: DeviceData) {
        if let DeviceData::Fan {
            state,
            threshold_temp,
            auto_control,
            light_state,
        } = data
        {
            if let Some(state) = state {
                self.pin.set_state(state).await;
                self.auto_control.store(false, Relaxed);
            }
            if let Some(threshold) = threshold_temp {
                storage::set("fan_threshold_temp", threshold).unwrap();
                self.threshold_temp.store(threshold, Relaxed);
            }
            if let Some(auto_control) = auto_control {
                self.auto_control.store(auto_control, Relaxed);
            }
            if let Some(light_state) = light_state {
                self.light.set_state(light_state).await
            }
        }
    }
    async fn handle_msg_update(&self, data: DeviceData) {
        match dbg!(data) {
            DeviceData::Switch { state: _ } => self.toggle().await,
            DeviceData::Environment {
                temperature,
                humidity: _,
                dark: _,
            } => {
                let temp = temperature as i8;
                self.current_temp.store(temp, Relaxed);
            }
            _ => (),
        }
    }
}
