use std::{
    collections::BTreeSet,
    sync::atomic::{AtomicBool, Ordering::Relaxed},
    time::Duration,
};

use base58::ToBase58;
use embedded_hal::digital::blocking::{InputPin, OutputPin};
use futures_micro::or;

use crate::{
    storage,
    utils::{sleep, Connection, DeviceData, Message},
};

use super::{Handle, Lcd1602, LogicPin, Rfid};

pub struct Lock<LockPin: OutputPin> {
    rfid: Rfid,
    lcd_print: Lcd1602,
    lock: LogicPin<LockPin>,
    unlock_request: AtomicBool,
    add_card: AtomicBool,
}

impl<LockPin> Lock<LockPin>
where
    LockPin: OutputPin + InputPin,
{
    pub fn new(rfid: Rfid, lcd_print: Lcd1602, lock: LogicPin<LockPin>) -> Self {
        Self {
            rfid,
            lcd_print,
            lock,
            add_card: AtomicBool::new(false),
            unlock_request: AtomicBool::new(false),
        }
    }
}
#[async_trait::async_trait(?Send)]
impl<LockPin> Handle for Lock<LockPin>
where
    LockPin: OutputPin + InputPin,
{
    async fn wait_new_state(&self) -> DeviceData {
        let task1 = async {
            let unlocked = self.lock.wait_change().await;

            DeviceData::Lock {
                unlock: Some(unlocked),
                add_rfid: Some(false),
                clear_rfid: Some(false),
            }
        };
        let task2 = async {
            let add_card = self.add_card.load(Relaxed);
            loop {
                sleep(Duration::from_millis(200)).await;
                if self.add_card.load(Relaxed) != add_card {
                    let unlocked = self.lock.get_state().await;
                    break DeviceData::Lock {
                        unlock: Some(unlocked),
                        add_rfid: Some(!add_card),
                        clear_rfid: Some(false),
                    };
                }
            }
        };
        let task_lock = async {
            loop {
                if self.unlock_request.load(Relaxed) {
                    self.lock.set_high().await;

                    self.lcd_print.print("Welcome").await;

                    sleep(Duration::from_millis(3000)).await;

                    self.lock.set_low().await;
                    self.unlock_request.store(false, Relaxed);
                }
                sleep(Duration::from_millis(100)).await;
            }
        };
        let clear_add_card = async {
            loop {
                if self.add_card.load(Relaxed) {
                    sleep(Duration::from_secs(10)).await;
                    if self.add_card.load(Relaxed) {
                        self.add_card.store(false, Relaxed);
                    }
                }
                sleep(Duration::from_millis(500)).await
            }
        };
        let task_receive_rfid = async {
            loop {
                let uid = self.rfid.wait_read().await;
                if storage::is_rfid(uid.as_ref()).unwrap_or(false) {
                    self.unlock_request.store(true, Relaxed);
                } else {
                    self.lcd_print.print("Invalid card").await;
                }
                if self.add_card.load(Relaxed) {
                    storage::add_rfid(&uid);
                    self.lcd_print.print("Success").await;
                    self.add_card.store(false, Relaxed);
                }
            }
        };
        or!(task1, task2, task_lock, clear_add_card, task_receive_rfid).await
    }

    async fn handle_msg_control(&self, data: DeviceData) {
        if let DeviceData::Lock {
            unlock,
            add_rfid,
            clear_rfid,
        } = data
        {
            if let Some(state) = unlock {
                self.unlock_request.store(state, Relaxed);
            }
            if let Some(true) = add_rfid {
                self.add_card.store(true, Relaxed);
            }
            if let Some(true) = clear_rfid {
                storage::set_rfid_list(BTreeSet::new());
            }
        }
    }
    async fn handle_msg_update(&self, data: DeviceData) {
        if let DeviceData::Environment {
            temperature,
            humidity,
            dark: _,
        } = data
        {
            self.lcd_print
                .print(&format!("Tem: {}*C\nHum: {}%", temperature, humidity))
                .await;
        }
    }
}
