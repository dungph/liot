use std::time::Duration;

use async_channel::{bounded, Sender};
use esp_idf_hal::{
    gpio::{Gpio7, Gpio8, Unknown},
    i2c::I2C0,
    units::Hertz,
};

use crate::utils::sleep;

pub struct Lcd1602 {
    tx: Sender<String>,
}

impl Lcd1602 {
    pub fn new(i2c: I2C0, sda: Gpio7<Unknown>, scl: Gpio8<Unknown>) -> Self {
        let (tx, rx) = bounded::<String>(10);
        std::thread::spawn(move || {
            let mut i2c = esp_idf_hal::i2c::Master::new(
                i2c,
                esp_idf_hal::i2c::MasterPins { sda, scl },
                esp_idf_hal::i2c::config::MasterConfig {
                    baudrate: Hertz(400_000),
                    timeout: None,
                    sda_pullup_enabled: false,
                    scl_pullup_enabled: false,
                },
            )
            .unwrap();

            let mut delay = esp_idf_hal::delay::FreeRtos;
            let mut lcd = lcd_lcm1602_i2c::Lcd::new(&mut i2c, &mut delay)
                .address(0x27)
                .cursor_on(true)
                .rows(2)
                .init()
                .unwrap();
            lcd.clear().unwrap();
            lcd.backlight(lcd_lcm1602_i2c::Backlight::On).unwrap();
            lcd.write_str("Hello").unwrap();

            loop {
                let mut print: String;
                loop {
                    if let Ok(s) = rx.try_recv() {
                        print = s;
                        while let Ok(s) = rx.try_recv() {
                            print = s;
                        }
                        break;
                    } else {
                        std::thread::sleep(Duration::from_millis(100));
                    }
                }
                lcd.clear().unwrap();
                if let Some((l1, l2)) = print.split_once('\n') {
                    lcd.write_str(l1).ok();
                    lcd.set_cursor(1, 0).ok();
                    lcd.write_str(l2).ok();
                } else {
                    lcd.write_str(&print).unwrap();
                }
            }
        });
        Self { tx }
    }

    pub async fn print(&self, s: &str) {
        self.tx.send(s.to_string()).await.ok();
    }
}
