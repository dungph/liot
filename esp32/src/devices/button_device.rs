use std::{
    fmt::Debug,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use base58::ToBase58;
use embedded_hal::digital::blocking::{InputPin, OutputPin};
use serde_json::{from_slice, to_vec};

use crate::{
    storage,
    utils::{Connection, DeviceData, Message},
};

use super::{Handle, LogicPin};

pub struct Button<P: InputPin> {
    inner: LogicPin<P>,
    state: AtomicBool,
    nh: bool,
}

impl<P: InputPin> std::ops::Deref for Button<P> {
    type Target = LogicPin<P>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[async_trait::async_trait(?Send)]
impl<P: InputPin> Handle for Button<P> {
    async fn wait_new_state(&self) -> DeviceData {
        let state = self.wait_press().await;
        DeviceData::Switch { state }
    }
}
impl<P: InputPin> Button<P> {
    pub fn new(pin: P, nh: bool) -> Self {
        Self {
            inner: LogicPin::new(pin),
            state: AtomicBool::new(false),
            nh,
        }
    }

    pub async fn press_duration(&self) -> Duration {
        if self.nh {
            self.inner.wait_low().await;
            self.inner.wait_high().await
        } else {
            self.inner.wait_high().await;
            self.inner.wait_low().await
        }
    }

    pub async fn get_state(&self) -> bool {
        self.state.load(Ordering::Relaxed)
    }

    pub async fn wait_press(&self) -> bool {
        let current = self.get_state().await;

        loop {
            if (self.press_duration().await) < Duration::from_secs(1) {
                self.state
                    .compare_exchange(current, !current, Ordering::Relaxed, Ordering::Relaxed)
                    .ok();
                break !current;
            }
        }
    }

    pub async fn wait_long_press(&self) {
        loop {
            if (self.press_duration().await) > Duration::from_secs(3) {
                break;
            }
        }
    }
}
