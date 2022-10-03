use std::{
    cell::{RefCell, RefMut},
    fmt::Debug,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use embedded_hal::digital::blocking::{InputPin, OutputPin};

use futures_lite::future::or;
use serde_json::{from_slice, to_vec};

use crate::{
    storage,
    utils::sleep,
    utils::{Connection, DeviceData, Message},
};

use super::Handle;

pub struct LogicPin<P> {
    pin: RefCell<P>,
}

impl<P> LogicPin<P> {
    pub fn new(pin: P) -> Self {
        Self {
            pin: RefCell::new(pin),
        }
    }
    async fn inner(&self) -> RefMut<P> {
        loop {
            if let Ok(d) = self.pin.try_borrow_mut() {
                break d;
            } else {
                sleep(Duration::from_millis(10)).await
            }
        }
    }
}

impl<P: OutputPin> LogicPin<P> {
    pub async fn set_state(&self, state: bool) {
        self.inner().await.set_state(state.into()).unwrap();
    }
    pub async fn set_high(&self) {
        self.set_state(true).await
    }
    pub async fn set_low(&self) {
        self.set_state(false).await
    }
}
impl<P: InputPin> LogicPin<P> {
    pub async fn is_low(&self) -> bool {
        self.inner().await.is_low().unwrap()
    }
    pub async fn is_high(&self) -> bool {
        self.inner().await.is_high().unwrap()
    }
    pub async fn get_state(&self) -> bool {
        self.is_high().await
    }
    pub async fn wait_low(&self) -> Duration {
        let begin = Instant::now();
        while self.is_high().await {
            sleep(Duration::from_millis(10)).await;
        }
        Instant::now() - begin
    }
    pub async fn wait_high(&self) -> Duration {
        let begin = Instant::now();
        while self.is_low().await {
            sleep(Duration::from_millis(10)).await;
        }
        Instant::now() - begin
    }
    pub async fn wait_change(&self) -> bool {
        let state = self.get_state().await;
        while state == self.get_state().await {
            sleep(Duration::from_millis(10)).await;
        }
        !state
    }
}
