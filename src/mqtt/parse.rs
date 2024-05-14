use super::message::Message;

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
