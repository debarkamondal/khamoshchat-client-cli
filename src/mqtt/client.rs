//! rumqttc async MQTT client wrapper.

use super::{InboundMessage, OutboundMessage};
use anyhow::Result;
use rumqttc::{AsyncClient, Event, MqttOptions, Outgoing, Packet, QoS};
use std::cell::RefCell;
use std::sync::Arc;
use tokio::sync::mpsc;

const QOS: QoS = QoS::AtLeastOnce;

pub struct MqttClient {
    client: AsyncClient,
    /// RefCell holds the guard across await points — safe here because the
    /// guard is released and re-acquired per iteration.
    event_loop: Arc<RefCell<rumqttc::EventLoop>>,
    inbound_tx: mpsc::Sender<InboundMessage>,
}

impl MqttClient {
    /// Connect to the MQTT broker.
    pub async fn connect(
        client_id: &str,
        broker_uri: &str,
        username: &str,
        password: &str,
        inbound_tx: mpsc::Sender<InboundMessage>,
    ) -> Result<Self> {
        let mut opts = MqttOptions::new(client_id, broker_uri, 8883);
        opts.set_credentials(username, password);
        opts.set_keep_alive(std::time::Duration::from_secs(30));

        let (client, event_loop) = AsyncClient::new(opts, 100);
        Ok(Self {
            client,
            event_loop: Arc::new(RefCell::new(event_loop)),
            inbound_tx,
        })
    }

    /// Subscribe to messages from a given sender.
    pub async fn subscribe(&self, them: &str, us: &str) -> Result<()> {
        let topic = format!("/khamoshchat/{them}/{us}");
        self.client.subscribe(&topic, QOS).await?;
        tracing::info!("Subscribed to {topic}");
        Ok(())
    }

    /// Publish a message to a recipient.
    pub async fn publish(&self, msg: OutboundMessage) -> Result<()> {
        let topic = format!("/khamoshchat/{}/{}/", msg.recipient, msg.sender);
        self.client
            .publish(&topic, QOS, false, &msg.payload[..])
            .await?;
        tracing::debug!("Published {} bytes to {topic}", msg.payload.len());
        Ok(())
    }

    /// Drive the event loop forever. Call inside a tokio task.
    pub async fn run(&self) -> Result<()> {
        loop {
            // Acquire guard, call poll, release guard before await
            let event = {
                let mut el = self.event_loop.borrow_mut();
                el.poll().await
            };

            match event.map_err(|e| anyhow::anyhow!("mqtt error: {e}"))? {
                Event::Incoming(Packet::Publish(p)) => {
                    let topic = p.topic.clone();
                    let parts: Vec<_> = topic.split('/').collect();
                    if parts.len() >= 4 && parts[0].is_empty() && parts[1] == "khamoshchat" {
                        let sender = parts[2].to_string();
                        let inbound = InboundMessage {
                            sender,
                            payload: p.payload.to_vec(),
                        };
                        let _ = self.inbound_tx.send(inbound).await;
                    }
                }
                Event::Incoming(Packet::PingResp) => { /* keepalive ack */ }
                Event::Outgoing(Outgoing::PingReq) => { /* outgoing ping */ }
                _ => {}
            }
        }
    }
}
