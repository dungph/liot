use std::cell::RefCell;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicU32, AtomicU64};
use std::time::{Duration, Instant};
use std::{fmt::Debug, sync::atomic::AtomicBool};

use embedded_hal::digital::blocking::{InputPin, OutputPin};
use futures_lite::future::or;
use serde_json::{from_slice, to_vec};

use crate::storage;
use crate::utils::{sleep, Connection, DeviceData, Message};

use super::{Handle, LogicPin, BEGIN};

pub struct Light<P> {
    pin: LogicPin<P>,
    motion_detect: AtomicBool,
    environment_dark: AtomicBool,
    auto_control: AtomicBool,
}

impl<P> std::ops::Deref for Light<P> {
    type Target = LogicPin<P>;

    fn deref(&self) -> &Self::Target {
        &self.pin
    }
}
impl<P: InputPin + OutputPin> Light<P> {
    pub fn new(pin: P) -> Self {
        let pin = LogicPin::new(pin);
        Self {
            pin,
            motion_detect: AtomicBool::new(true),
            environment_dark: AtomicBool::new(true),
            auto_control: AtomicBool::new(true),
        }
    }

    pub async fn toggle(&self) {
        let state = self.get_state().await;
        self.set_state(!state).await;
    }
}

#[async_trait::async_trait(?Send)]
impl<P: InputPin + OutputPin> Handle for Light<P> {
    async fn wait_new_state(&self) -> DeviceData {
        let task1 = async {
            let state = self.wait_change().await;
            DeviceData::Light {
                state: Some(state),
                auto_control: Some(self.auto_control.load(Relaxed)),
            }
        };
        let task2 = async {
            sleep(Duration::from_secs(10)).await;
            let state = self.get_state().await;
            DeviceData::Light {
                state: Some(state),
                auto_control: Some(self.auto_control.load(Relaxed)),
            }
        };
        let task3 = async {
            loop {
                if self.auto_control.load(Relaxed) {
                    if self.environment_dark.load(Relaxed) && self.motion_detect.load(Relaxed) {
                        self.set_high().await
                    } else {
                        self.set_low().await
                    }
                }
                sleep(Duration::from_secs(1)).await;
            }
        };

        let task4 = async {
            let auto_control = self.auto_control.load(Relaxed);
            loop {
                sleep(Duration::from_millis(500)).await;
                if self.auto_control.load(Relaxed) != auto_control {
                    let state = self.get_state().await;
                    break DeviceData::Light {
                        state: Some(state),
                        auto_control: Some(!auto_control),
                    };
                }
            }
        };
        futures_micro::or!(task1, task2, task3, task4).await
    }
    async fn handle_msg_control(&self, data: DeviceData) {
        dbg!(&data);
        if let DeviceData::Light {
            state,
            auto_control,
        } = data
        {
            if let Some(state) = state {
                self.set_state(state).await;
                self.auto_control.store(false, Relaxed);
            }
            if let Some(auto_control) = auto_control {
                self.auto_control.store(auto_control, Relaxed);
            }
        }
    }
    async fn handle_msg_update(&self, data: DeviceData) {
        match dbg!(data) {
            DeviceData::Switch { state: _ } => {
                self.auto_control.store(false, Relaxed);
                self.toggle().await
            }
            DeviceData::Motion { state } => {
                self.motion_detect.store(state, Relaxed);
            }
            DeviceData::Environment {
                temperature: _,
                humidity: _,
                dark,
            } => self.environment_dark.store(dark, Relaxed),
            _ => (),
        }
    }
}
