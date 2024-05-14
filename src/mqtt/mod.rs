pub mod command;
pub mod message;
pub mod parse;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use futures::StreamExt;
use std::time::Duration;

use crate::config::PrinterConfig;

use self::{command::Command, message::Message};

#[derive(Clone)]
pub struct Client {
    config: PrinterConfig,

    client: paho_mqtt::AsyncClient,
    stream: paho_mqtt::AsyncReceiver<Option<paho_mqtt::Message>>,

    tx: tokio::sync::broadcast::Sender<Message>,

    topic_device_request: String,
    topic_device_report: String,
}

impl Client {
    /// Creates a new Bambu printer MQTT client.
    ///
    /// # Panics
    ///
    /// Panics if the MQTT client cannot be created.
    pub fn new(printer_cfg: &PrinterConfig, tx: tokio::sync::broadcast::Sender<Message>) -> Self {
        let client_id = format!("bambu_watcher_{}", nanoid::nanoid!(8));

        let create_opts = paho_mqtt::CreateOptionsBuilder::new()
            .server_uri(&printer_cfg.host)
            .client_id(client_id)
            .max_buffered_messages(25)
            .finalize();

        let mut client = paho_mqtt::AsyncClient::new(create_opts).expect("Failed to create client");
        let stream = client.get_stream(25);

        Self {
            config: printer_cfg.clone(),
            topic_device_request: format!("device/{}/request", &printer_cfg.serial),
            topic_device_report: format!("device/{}/report", &printer_cfg.serial),
            client,
            stream,
            tx,
        }
    }

    /// Polls for a message from the MQTT event loop.
    /// You need to poll periodically to receive messages
    /// and to keep the connection alive.
    /// This function also handles reconnects.
    ///
    /// **NOTE** Don't block this while iterating
    ///
    /// # Errors
    ///
    /// Returns an error if there was a problem polling for a message or parsing the event.
    async fn poll(&mut self) -> Result<()> {
        let msg_opt = self.stream.next().await.flatten();

        if let Some(msg) = msg_opt {
            self.tx.send(self::parse::parse_message(&msg))?;
        } else {
            // A "None" means we were disconnected. Try to reconnect...
            self.tx.send(Message::Disconnected)?;

            while (self.client.reconnect().await).is_err() {
                tokio::time::sleep(Duration::from_secs(1)).await;
                self.tx.send(Message::Reconnecting)?;
            }

            self.tx.send(Message::Connected)?;
        }

        Ok(())
    }

    async fn connect(&self) -> Result<()> {
        let ssl_opts = paho_mqtt::SslOptionsBuilder::new()
            .disable_default_trust_store(true)
            .enable_server_cert_auth(false)
            .verify(false)
            .finalize();

        let conn_opts = paho_mqtt::ConnectOptionsBuilder::new()
            .ssl_options(ssl_opts)
            .keep_alive_interval(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(3))
            .user_name("bblp")
            .password(&self.config.access_code)
            .finalize();

        self.tx.send(Message::Connecting)?;
        self.client.connect(conn_opts).await?;
        self.tx.send(Message::Connected)?;

        Ok(())
    }

    fn subscibe_to_device_report(&self) {
        self.client
            .subscribe(&self.topic_device_report, paho_mqtt::QOS_0);
    }

    /// Runs the Bambu MQTT client.
    /// You should run this in a tokio task.
    ///
    /// # Errors
    ///
    /// Returns an error if there was a problem connecting to the MQTT broker
    /// or subscribing to the device report topic.
    pub async fn run(&mut self) -> Result<()> {
        self.connect().await?;
        self.subscibe_to_device_report();

        loop {
            Self::poll(self).await?;
        }
    }

    /// Publishes a command to the Bambu MQTT broker.
    ///
    /// # Errors
    ///
    /// Returns an error if there was a problem publishing the command.
    pub async fn publish(&self, command: Command) -> Result<()> {
        let payload = command.get_payload();

        let msg = paho_mqtt::Message::new(&self.topic_device_request, payload, paho_mqtt::QOS_0);
        self.client.publish(msg).await?;

        Ok(())
    }
}
