mod button_device;
mod dht11_device;
mod fan_device;
mod lcd_device;
mod light_device;
mod lock_device;
mod logic_device;
mod motion_sensor;
mod rfid_device;

use std::time::Instant;

pub use button_device::Button;
pub use dht11_device::Dht11Device;
pub use fan_device::Fan;
pub use lcd_device::Lcd1602;
pub use light_device::Light;
pub use lock_device::Lock;
pub use logic_device::LogicPin;
pub use motion_sensor::MotionSensor;
use once_cell::sync::Lazy;
pub use rfid_device::Rfid;

use serde_json::{from_slice, to_vec};

use crate::{
    storage,
    utils::{Connection, DeviceData, Message},
};
pub static BEGIN: Lazy<Instant> = Lazy::new(Instant::now);
#[async_trait::async_trait(?Send)]
pub trait Handle {
    async fn wait_new_state(&self) -> DeviceData;
    async fn handle_msg_control(&self, _data: DeviceData) {}
    async fn handle_msg_update(&self, _data: DeviceData) {}
    async fn connection_handle(&self, con: impl Connection) -> anyhow::Result<()> {
        println!("run handle");
        let peer_id = &con.remote_id().await;

        let task_update = async {
            loop {
                let state = self.wait_new_state().await;
                if storage::is_subscriber(peer_id)? {
                    con.send(&to_vec(&Message::Update(state))?).await?;
                }
            }
        };

        let task_actuator = async {
            while let Ok(bytes) = con.recv().await {
                match from_slice(&bytes) {
                    Ok(Message::Control(data)) => {
                        if storage::is_controller(peer_id)? {
                            self.handle_msg_control(data).await;
                        }
                    }
                    Ok(Message::Update(data)) => {
                        if storage::is_controller(peer_id)? {
                            self.handle_msg_update(data).await;
                        }
                    }
                    Ok(Message::RemoveManager(id)) => {
                        if storage::is_manager(peer_id)? {
                            storage::rm_manager(id.trim().as_bytes())?
                        }
                    }
                    Ok(Message::RemoveController(id)) => {
                        if storage::is_manager(peer_id)? {
                            storage::rm_controller(id.trim().as_bytes())?
                        }
                    }
                    Ok(Message::RemoveSubscriber(id)) => {
                        if storage::is_manager(peer_id)? {
                            storage::rm_subscriber(id.trim().as_bytes())?
                        }
                    }
                    Ok(Message::AddManager(id)) => {
                        if storage::is_manager(peer_id)? {
                            storage::add_manager(id.trim().as_bytes())?
                        }
                    }
                    Ok(Message::AddController(id)) => {
                        if storage::is_manager(peer_id)? {
                            storage::add_controller(id.trim().as_bytes())?
                        }
                    }
                    Ok(Message::AddSubscriber(id)) => {
                        if storage::is_manager(peer_id)? {
                            storage::add_subscriber(id.trim().as_bytes())?
                        }
                    }
                    _ => (),
                }
            }
            Ok(())
        };
        futures_micro::or!(task_update, task_actuator).await
    }
}
