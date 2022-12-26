pub mod controller;
pub mod data_schema;
pub mod device;
pub mod espnow;
pub mod http_service;
pub mod storage;
pub mod utils;
pub mod wifi;

use crate::utils::run_ex;
use async_executor::LocalExecutor;
use base58::ToBase58;
use controller::Controller;
use device::pwm_device::PWMDevice;
use esp_idf_hal::peripherals::Peripherals;
use espnow::{get_mac, EspNowService};
use std::time::Duration;

pub fn run() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    let p = Peripherals::take().unwrap();
    let storage = storage::StorageService::new()?;
    let wifi = wifi::WifiService::new(p.modem, &storage)?;
    let http = http_service::HttpServe::new(&wifi)?;
    let espnow = EspNowService::new(&wifi)?;

    //let device = SensorDevice::new(p.pins.gpio9);
    let module1 = PWMDevice::new(
        "module-1",
        p.ledc.timer0,
        p.ledc.channel0,
        p.pins.gpio3,
        storage.clone(),
    );
    let module2 = PWMDevice::new(
        "module-2",
        p.ledc.timer1,
        p.ledc.channel1,
        p.pins.gpio4,
        storage.clone(),
    );
    let controller = Controller::new(
        &get_mac().to_base58(),
        wifi.clone(),
        &storage,
        vec![Box::new(module2.clone()), Box::new(module1.clone())],
        espnow.clone(),
    );

    let ex = LocalExecutor::new();
    ex.spawn(wifi.run_handle()).detach();
    ex.spawn(module1.run_handle()).detach();
    ex.spawn(module2.run_handle()).detach();
    ex.spawn(http.run(&controller, &storage)).detach();
    ex.spawn(espnow.run_handle()).detach();
    ex.spawn(storage.periodic_store(Duration::from_secs(5)))
        .detach();
    ex.spawn(controller.run_handle()).detach();
    run_ex(ex);
}
pub fn main() {
    std::thread::Builder::new()
        .stack_size(40000)
        .name("task_main".to_string())
        .spawn(|| {
            run().ok();
        })
        .unwrap()
        .join()
        .unwrap();
}
