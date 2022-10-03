use std::{
    cell::{RefCell, RefMut},
    rc::Rc,
    sync::Arc,
    time::Duration,
};

use async_channel::{bounded, Receiver, Sender};
use dashmap::DashMap;
use embedded_svc::mqtt::client::{Client, Event, Message, Publish, QoS};
use esp_idf_svc::mqtt::client::{EspMqttClient, MqttClientConfiguration};

use crate::{utils::sleep, utils::Connection};

pub struct MqttService {
    mqtt: Rc<RefCell<EspMqttClient>>,
    subscribers: Arc<DashMap<String, Sender<Vec<u8>>>>,
}
#[derive(Clone)]
pub struct MqttChannel {
    base_topic: String,
    mqtt: Rc<RefCell<EspMqttClient>>,
    receiver: Receiver<Vec<u8>>,
}
impl MqttChannel {
    async fn get_mqtt(&self) -> RefMut<EspMqttClient> {
        loop {
            if let Ok(mqtt) = self.mqtt.try_borrow_mut() {
                break mqtt;
            } else {
                sleep(Duration::from_millis(10)).await;
            }
        }
    }
}
#[async_trait::async_trait(?Send)]
impl Connection for MqttChannel {
    fn is_init(&self) -> bool {
        true
    }
    async fn remote_id(&self) -> Vec<u8> {
        b"mqtt".to_vec()
    }
    async fn send(&self, data: &[u8]) -> anyhow::Result<()> {
        let topic = format!("{}/pub", self.base_topic);
        self.get_mqtt()
            .await
            .publish(&topic, QoS::ExactlyOnce, false, data)?;
        Ok(())
    }
    async fn recv(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.receiver.recv().await?)
    }
}

impl MqttService {
    pub fn new() -> Self {
        let subscribers: Arc<DashMap<String, Sender<Vec<u8>>>> = Arc::new(DashMap::new());

        let sub_clone = subscribers.clone();
        let mqtt = EspMqttClient::new(
            "mqtt://broker.emqx.io:1883",
            &MqttClientConfiguration::default(),
            move |event| {
                if let Ok(Event::Received(msg)) = event {
                    if let Some(sender) = sub_clone.get(msg.topic().unwrap_or_default()) {
                        sender.value().try_send(msg.data().to_vec()).ok();
                    }
                }
            },
        )
        .unwrap();

        MqttService {
            mqtt: Rc::new(RefCell::new(mqtt)),
            subscribers,
        }
    }

    async fn get_mqtt(&self) -> RefMut<EspMqttClient> {
        loop {
            if let Ok(mqtt) = self.mqtt.try_borrow_mut() {
                break mqtt;
            } else {
                sleep(Duration::from_millis(10)).await;
            }
        }
    }
    pub async fn subscribe(&self, topic: &str) -> anyhow::Result<Receiver<Vec<u8>>> {
        let (tx, rx) = bounded(10);
        self.get_mqtt().await.subscribe(topic, QoS::ExactlyOnce)?;
        self.subscribers.insert(topic.to_string(), tx);
        Ok(rx)
    }

    pub async fn publish(&self, topic: &str, data: &[u8]) -> anyhow::Result<()> {
        self.get_mqtt()
            .await
            .publish(topic, QoS::ExactlyOnce, false, data)
            .ok();
        Ok(())
    }

    pub async fn get_channel(&self, short: &str) -> anyhow::Result<MqttChannel> {
        let topic = format!("{}/sub", short);
        let receiver = self.subscribe(&topic).await?;

        Ok(MqttChannel {
            base_topic: short.to_string(),
            mqtt: self.mqtt.clone(),
            receiver,
        })
    }
}
