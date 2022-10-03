mod devices;
mod services;
mod storage;
mod utils;
use async_executor::LocalExecutor;
use base58::ToBase58;
use embedded_svc::wifi::{
    ClientConfiguration, ClientConnectionStatus, ClientIpStatus, ClientStatus, Configuration,
    Status, Wifi,
};
use esp_idf_svc::{netif::EspNetifStack, sysloop::EspSysLoopStack, wifi::EspWifi};
use futures_lite::future::or;

use std::{collections::BTreeSet, sync::Arc, time::Duration};

use crate::{
    devices::{Button, Handle, Lcd1602, Light, Lock, LogicPin, Rfid},
    services::TransportSocket,
    utils::{sleep, Connection, InternalConnection},
};

fn main() -> anyhow::Result<()> {
    esp_idf_sys::link_patches();

    let hal = esp_idf_hal::peripherals::Peripherals::take().unwrap();
    let sysloop = Arc::new(EspSysLoopStack::new().unwrap());
    let netif = Arc::new(EspNetifStack::new().unwrap());

    let mut wifi = EspWifi::new(netif, sysloop, storage::take()).unwrap();
    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: "Nokia".into(),
        password: "12346789".into(),
        ..Default::default()
    }))
    .unwrap();

    wifi.wait_status_with_timeout(Duration::from_secs(20), |status| !status.is_transitional())
        .map_err(|e| anyhow::anyhow!("Unexpected Wifi status: {:?}", e))?;

    let status = wifi.get_status();

    if let Status(
        ClientStatus::Started(ClientConnectionStatus::Connected(ClientIpStatus::Done(ip_settings))),
        _,
    ) = status
    {
        println!("Wifi connected");
        println!("{:?}", ip_settings);
    } else {
        println!("Unexpected Wifi status: {:?}", status);
    }

    let mqtt = services::MqttService::new();

    #[cfg(all(feature = "motion_light"))]
    let (light, motion) = {
        let light3 = Light::new(hal.pins.gpio3.into_input_output().unwrap());
        let motion = devices::MotionSensor::new(hal.pins.gpio6.into_input().unwrap());
        (light3, motion)
    };

    #[cfg(feature = "lock")]
    let lock = {
        let rfid = Rfid::new(
            hal.spi2,
            hal.pins.gpio4.into_output().unwrap(),
            hal.pins.gpio5.into_output().unwrap(),
            hal.pins.gpio1.into_input().unwrap(),
            hal.pins.gpio2.into_output().unwrap(),
        );

        let lcd = Lcd1602::new(hal.i2c0, hal.pins.gpio7, hal.pins.gpio8);

        let lock = Lock::new(
            rfid,
            lcd,
            LogicPin::new(hal.pins.gpio3.into_input_output().unwrap()),
        );
        lock
    };

    #[cfg(feature = "fan")]
    let fan = devices::Fan::new(
        hal.pins.gpio3.into_input_output().unwrap(),
        hal.pins.gpio4.into_input_output().unwrap(),
        hal.pins.gpio5.into_input().unwrap(),
    );

    #[cfg(feature = "motion")]
    let motion = devices::MotionSensor::new(hal.pins.gpio6.into_input().unwrap());

    #[cfg(feature = "dht_sensor")]
    let dht = devices::Dht11Device::new(
        hal.pins.gpio7.into_input_output_od().unwrap(),
        hal.pins.gpio6.into_input().unwrap(),
    );

    #[cfg(feature = "light")]
    let light6 = Light::new(hal.pins.gpio3.into_input_output().unwrap());

    #[cfg(feature = "button")]
    let button8 = Button::new(hal.pins.gpio8.into_input().unwrap(), false);

    let name = storage::public_key()?.to_base58();
    let short = &name[..6];
    println!("{}", short);

    let mqtt_handler = async {
        let channel = mqtt.get_channel(short).await.unwrap();

        #[cfg(all(feature = "motion_light"))]
        light.connection_handle(channel).await.unwrap();

        #[cfg(feature = "fan")]
        fan.connection_handle(channel).await.unwrap();

        #[cfg(feature = "motion")]
        motion.connection_handle(channel.clone()).await.unwrap();

        #[cfg(feature = "dht_sensor")]
        dht.connection_handle(channel).await.unwrap();

        #[cfg(feature = "lock")]
        lock.connection_handle(channel).await.unwrap();

        #[cfg(feature = "light")]
        light6.connection_handle(channel).await.unwrap();

        #[cfg(feature = "button")]
        button8.connection_handle(channel).await.unwrap();
    };

    #[cfg(all(feature = "motion_light"))]
    let task = async {
        let (c1, c2) = InternalConnection::pair();
        or(light.connection_handle(c1), motion.connection_handle(c2)).await;
    };

    let connection_handle = || async {
        loop {
            services::advertise().await.unwrap();
            let channel = services::next_channel().await;
            println!("New con");
            if let Ok(channel) = TransportSocket::handshake(channel).await {
                #[cfg(all(feature = "motion_light"))]
                light.connection_handle(channel).await.unwrap();

                #[cfg(feature = "motion")]
                motion.connection_handle(channel.clone()).await.unwrap();

                #[cfg(feature = "fan")]
                fan.connection_handle(channel).await.unwrap();

                #[cfg(feature = "dht_sensor")]
                dht.connection_handle(channel).await.unwrap();

                #[cfg(feature = "lock")]
                lock.connection_handle(channel).await;

                #[cfg(feature = "light")]
                light6.connection_handle(channel).await;

                #[cfg(feature = "button")]
                button8.connection_handle(channel).await;
            }
        }
    };

    let free_heap_monitor = async {
        //loop {
        //    let size = unsafe { esp_idf_sys::esp_get_free_heap_size() };

        //    println!("free heap: {size}");
        //    sleep(Duration::from_secs(3)).await
        //}
    };

    let ex = LocalExecutor::new();
    ex.spawn(connection_handle()).detach();
    ex.spawn(connection_handle()).detach();
    ex.spawn(connection_handle()).detach();
    ex.spawn(connection_handle()).detach();
    ex.spawn(connection_handle()).detach();
    ex.spawn(connection_handle()).detach();
    ex.spawn(connection_handle()).detach();
    ex.spawn(mqtt_handler).detach();

    #[cfg(all(feature = "motion_light"))]
    ex.spawn(task).detach();

    ex.spawn(free_heap_monitor).detach();
    loop {
        for _ in 0..2 {
            ex.try_tick();
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}
