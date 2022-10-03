use std::{borrow::BorrowMut, cell::RefCell, sync::atomic::AtomicU32, time::Duration};

use async_channel::{bounded, Receiver};
use esp_idf_hal::{
    gpio::{Gpio1, Gpio2, Gpio4, Gpio5, Input, InputOutput, Output},
    spi::{Master, SPI2},
    units::Hertz,
};

pub struct Rfid {
    rx: Receiver<[u8; 4]>,
}

impl Rfid {
    pub fn new(
        spi2: SPI2,
        sclk: Gpio4<Output>,
        sdo: Gpio5<Output>,
        sdi: Gpio1<Input>,
        nss: Gpio2<Output>,
    ) -> Self {
        let (tx, rx) = bounded(1);
        let spi = esp_idf_hal::spi::Master::<SPI2, Gpio4<_>, Gpio5<_>>::new(
            spi2,
            esp_idf_hal::spi::Pins {
                sclk,
                sdo,
                sdi: Some(sdi),
                cs: None,
            },
            esp_idf_hal::spi::config::Config {
                baudrate: Hertz(100_000),
                data_mode: embedded_hal::spi::MODE_3,
                write_only: false,
                dma: esp_idf_hal::spi::Dma::Disabled,
            },
        )
        .unwrap();
        let mut rfid = mfrc522::Mfrc522::new(spi, nss).unwrap();
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(100));
            if let Ok(atqa) = rfid.reqa() {
                if let Ok(uid) = rfid.select(&atqa) {
                    let mut ret = [0u8; 4];
                    ret.copy_from_slice(&uid.as_bytes()[..4]);

                    println!("{:?}", ret);
                    tx.try_send(ret);
                }
            }
        });
        Rfid { rx }
    }
    pub async fn wait_read(&self) -> [u8; 4] {
        self.rx.recv().await.unwrap()
    }
}
