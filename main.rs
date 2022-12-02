fn init() {
    use std::ffi::CString;

    use esp_idf_svc::nvs::EspDefaultNvsPartition;
    use esp_idf_sys::{esp_vfs_spiffs_conf_t, esp_vfs_spiffs_register};

    esp_idf_sys::link_patches();
    EspDefaultNvsPartition::take().unwrap();
    unsafe {
        esp_vfs_spiffs_register(&esp_vfs_spiffs_conf_t {
            base_path: CString::new("").unwrap().as_c_str().as_ptr(),
            partition_label: std::ptr::null(),
            max_files: 100,
            format_if_mount_failed: true,
        });
    }
}

fn main() {
    init();
    run::main().unwrap();
}

mod run {
    use anyhow::anyhow;
    use async_executor::LocalExecutor;
    use dht11::{Dht11, Measurement};
    use embedded_svc::{
        http::Headers,
        io::{Read, Write},
        wifi::{AccessPointConfiguration, AuthMethod, ClientConfiguration},
    };
    use esp_idf_hal::{
        adc::{Atten11dB, ADC1},
        gpio::PinDriver,
        ledc::{config::TimerConfig, LedcDriver, LedcTimerDriver, Resolution},
        peripherals::Peripherals,
        units::Hertz,
    };
    use esp_idf_svc::{
        eventloop::EspSystemEventLoop,
        http::server::{Configuration, EspHttpServer},
        wifi::EspWifi,
    };
    use std::{task::Context, time::Duration};

    use serde::{Deserialize, Serialize};
    use waker_fn::waker_fn;

    pub fn main() -> anyhow::Result<()> {
        let ex = LocalExecutor::new();
        let p = Peripherals::take().unwrap();
        let mut wifi = EspWifi::new(p.modem, EspSystemEventLoop::take().unwrap(), None).unwrap();
        wifi.set_configuration(&embedded_svc::wifi::Configuration::Mixed(
            ClientConfiguration {
                ssid: "Nokia".into(),
                auth_method: AuthMethod::WPA,
                password: "12346789".into(),
                ..Default::default()
            },
            AccessPointConfiguration {
                ssid: "esp32".into(),
                auth_method: AuthMethod::None,
                ..Default::default()
            },
        ))
        .unwrap();
        wifi.start().unwrap();
        //wifi.connect().unwrap();

        let (tx, rx) = async_channel::bounded(10);
        let mut server = EspHttpServer::new(&Configuration::default()).unwrap();
        server
            .fn_handler("/", embedded_svc::http::Method::Get, |req| {
                Ok(req
                    .into_ok_response()?
                    .write_all(include_bytes!("index.html"))?)
            })
            .unwrap();
        server.fn_handler("/wifi_info", embedded_svc::http::Method::Post, |mut req| {
            let len = req
                .content_len()
                .ok_or_else(|| anyhow!("expected content len"))?;
            let mut dat = vec![0; len as usize];
            req.read(&mut dat)?;

            #[derive(Serialize, Deserialize, Debug)]
            struct WifiInfo {
                ssid: String,
                pwd: String,
            }
            let wifi_info: WifiInfo = serde_qs::from_bytes(&dat)?;
            dbg!(wifi_info);

            req.into_ok_response()?;
            //req.into_response(200, None, &[("content-length", "0")])?
            //    .write_all(b"")?;
            //let mut res = req.into_ok_response().unwrap();
            //res.
            //res.write_all(b"")?;

            Ok(())
        })?;
        server.fn_handler("/led", embedded_svc::http::Method::Post, move |mut req| {
            let len = req
                .content_len()
                .ok_or_else(|| anyhow!("expected content len"))?;
            let mut dat = vec![0; len as usize];
            req.read(&mut dat)?;

            #[derive(Serialize, Deserialize, Debug)]
            struct Brightness {
                value: u32,
            }
            let Brightness { value } = serde_qs::from_bytes(&dat)?;
            tx.try_send(value).ok();

            Ok(req
                .into_ok_response()?
                .write_all(include_bytes!("index.html"))?)
        })?;

        let adc_config = esp_idf_hal::adc::config::Config::default();
        let mut adc_driver = esp_idf_hal::adc::AdcDriver::new(p.adc1, &adc_config).unwrap();

        let mut adc_light_channel: esp_idf_hal::adc::AdcChannelDriver<
            esp_idf_hal::gpio::Gpio1,
            Atten11dB<ADC1>,
        > = esp_idf_hal::adc::AdcChannelDriver::new(p.pins.gpio1).unwrap();

        let mut adc_air_channel: esp_idf_hal::adc::AdcChannelDriver<
            esp_idf_hal::gpio::Gpio0,
            Atten11dB<ADC1>,
        > = esp_idf_hal::adc::AdcChannelDriver::new(p.pins.gpio0).unwrap();

        let pin6 = PinDriver::input_output_od(p.pins.gpio6).unwrap();
        let mut delay = esp_idf_hal::delay::Ets;
        let mut dht11 = Dht11::new(pin6);

        ex.spawn(async move {
            loop {
                let dat = adc_driver.read(&mut adc_light_channel).unwrap();
                let dat2 = adc_driver.read(&mut adc_air_channel).unwrap();
                let Measurement {
                    temperature,
                    humidity,
                } = loop {
                    let measure = dht11.perform_measurement(&mut delay);
                    if let Ok(measure) = measure {
                        break measure;
                    }
                };
                println!("Ánh sáng: {dat}");
                println!("Không khí: {dat3}");
                println!("Nhiệt độ: {temperature}");
                println!("Độ am: {humidity}");
                futures_timer::Delay::new(Duration::from_millis(1000)).await;

                //let duty = rx.recv().await.unwrap_or(0) % (max + 1);
                //channel.set_duty(duty).unwrap();
                //file.write_all(&postcard::to_vec(&duty).unwrap());
                //rx.recv().await;
                //channel.set_duty(1).unwrap();
                //rx.recv().await;
                //channel.set_duty(max).unwrap();

                //channel.set_duty(0).unwrap();
                //futures_timer::Delay::new(Duration::from_millis(300)).await;
                //channel.set_duty(max).unwrap();
                //futures_timer::Delay::new(Duration::from_millis(300)).await;

                //for i in 0..10 {
                //    channel.set_duty(i * step).unwrap();
                //    futures_timer::Delay::new(Duration::from_millis(500)).await;
                //}
            }
        })
        .detach();

        let this = std::thread::current();
        let waker = waker_fn(move || {
            this.unpark();
        });
        let mut cx = Context::from_waker(&waker);
        loop {
            use futures_lite::FutureExt;
            while ex.try_tick() {}

            let fut = ex.tick();
            futures_lite::pin!(fut);

            match fut.poll(&mut cx) {
                std::task::Poll::Ready(_) => (),
                std::task::Poll::Pending => std::thread::park(),
            }
        }
    }
}
