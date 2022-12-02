use std::time::Duration;

use esp_idf_hal::{
    gpio::{Input, InputPin, PinDriver},
    peripheral::Peripheral,
};

use super::Device;

pub struct SensorDevice<'a, T: InputPin> {
    device: PinDriver<'a, T, Input>,
}

impl<'a, T: InputPin> SensorDevice<'a, T> {
    pub fn new(pin: impl Peripheral<P = T> + 'a) -> Self {
        let device = PinDriver::input(pin).unwrap();
        Self { device }
    }

    fn bool_field_id(&self) -> String {
        String::from("Trạng Thái")
    }

    fn get_state(&self) -> bool {
        self.device.is_high()
    }
}
#[async_trait::async_trait(?Send)]
impl<'a, T: InputPin> Device for SensorDevice<'a, T> {
    async fn wait_change(&self) {
        let state = self.get_state();
        while state == self.get_state() {
            futures_timer::Delay::new(Duration::from_millis(20)).await;
        }
    }
    fn get_data_schema(&self) -> crate::utils::Data {
        Data {
            id: self.bool_field_id(),
            title: String::from("Trạng Thái"),
            description: None,
            value: DataValue::Bool {
                value: self.device.is_high(),
            },
            read_only: false,
            write_only: false,
            unit: Some("K".to_string()),
            one_of: Some(Vec::new()),
            format: Some("Format".to_string()),
            ..Default::default()
        }
    }

    fn set_data(&self, _value: &serde_json::Value) {}
}
