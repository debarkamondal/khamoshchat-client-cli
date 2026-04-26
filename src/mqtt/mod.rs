//! MQTT transport using rumqttc.
//!
//! Topic structure mirrors mobile:
//!   Publish: `/khamoshchat/{recipient}/{sender}`
//!   Subscribe: `/khamoshchat/{sender}/{recipient}`

mod client;


use rumqttc::QoS;

const QOS: QoS = QoS::AtLeastOnce;

/// Outbound envelope the caller builds and passes to `MqttClient::publish`.
#[derive(Debug)]
pub struct OutboundMessage {
    pub recipient: String,
    pub sender: String,
    pub payload: Vec<u8>,
}

/// Inbound message delivered to the consumer callback.
#[derive(Debug)]
pub struct InboundMessage {
    pub sender: String,
    pub payload: Vec<u8>,
}
