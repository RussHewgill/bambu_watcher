use super::message::Message;

pub(crate) fn parse_message(message: &rumqttc::mqttbytes::v4::Publish) -> Message {
    let payload = &message.payload;

    if let Ok(parsed_message) = serde_json::from_slice::<Message>(&payload) {
        parsed_message
    } else {
        if payload.len() == 0 {
            return Message::Unknown(None);
        }
        if let Ok(message_str) = String::from_utf8(payload.to_vec()) {
            if &message_str != "{}" {
                return Message::Unknown(Some(message_str));
            }
        }
        Message::Unknown(None)
    }
}

#[cfg(feature = "nope")]
pub(crate) fn parse_message(message: &paho_mqtt::Message) -> Message {
    let payload = message.payload();

    if let Ok(parsed_message) = serde_json::from_slice::<Message>(payload) {
        parsed_message
    } else {
        if let Ok(message_str) = String::from_utf8(payload.to_vec()) {
            return Message::Unknown(Some(message_str));
        }
        Message::Unknown(None)
    }
}
