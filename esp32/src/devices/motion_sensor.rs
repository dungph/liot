use embedded_hal::digital::blocking::InputPin;

use crate::utils::DeviceData;

use super::{Handle, LogicPin};

pub struct MotionSensor<P> {
    pin: LogicPin<P>,
}

impl<P> std::ops::Deref for MotionSensor<P> {
    type Target = LogicPin<P>;

    fn deref(&self) -> &Self::Target {
        &self.pin
    }
}

impl<P: InputPin> MotionSensor<P> {
    pub fn new(pin: P) -> Self {
        let pin = LogicPin::new(pin);
        Self { pin }
    }
}

#[async_trait::async_trait(?Send)]
impl<P: InputPin> Handle for MotionSensor<P> {
    async fn wait_new_state(&self) -> DeviceData {
        let state = self.wait_change().await;
        DeviceData::Motion { state }
    }
}
