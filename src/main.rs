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
use controller::Controller;
use device::pwm_device::PWMDevice;
use esp_idf_hal::peripherals::Peripherals;
use espnow::EspNowService;
use std::time::Duration;

pub fn run() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();
    let p = Peripherals::take().unwrap();
    let storage = storage::StorageService::new()?;
    let wifi = wifi::WifiService::new(p.modem, &storage)?;
    let http = http_service::HttpServe::new(&wifi)?;
    let espnow = EspNowService::new(&wifi)?;

    //let device = SensorDevice::new(p.pins.gpio9);
    let device = PWMDevice::new(
        "dev",
        p.ledc.timer0,
        p.ledc.channel0,
        p.pins.gpio3,
        storage.clone(),
    );
    let controller = Controller::new(
        "dev",
        wifi.clone(),
        &storage,
        device.clone(),
        espnow.clone(),
    );

    let ex = LocalExecutor::new();
    ex.spawn(device.run_handle()).detach();
    ex.spawn(http.run(&controller, &storage)).detach();
    ex.spawn(espnow.reactor()).detach();
    ex.spawn(storage.periodic_store(Duration::from_secs(5)))
        .detach();
    //ex.spawn(wifi.connect("Nokia", "12346789")).detach();
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
