use std::{env, time::Duration};

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use futures::StreamExt;
use tracing_subscriber::field::debug;

#[derive(Clone)]
pub struct Client {
    pub host: String,
    pub access_code: String,
    pub serial: String,

    client: paho_mqtt::AsyncClient,
    stream: paho_mqtt::AsyncReceiver<Option<paho_mqtt::Message>>,
}

impl Client {
    /// Creates a new Bambu printer MQTT client.
    ///
    /// # Panics
    ///
    /// Panics if the MQTT client cannot be created.
    pub fn new<S: Into<String>>(ip: S, access_code: S, serial: S) -> Self {
        let host = format!("mqtts://{}:8883", ip.into());
        debug!("host = {}", host);
        let access_code = access_code.into();
        debug!("access_code = {}", access_code);
        let serial = serial.into();
        debug!("serial = {}", serial);

        let client_id = "bambu_watcher";

        // let create_opts = paho_mqtt::CreateOptionsBuilder::new()
        let create_opts = paho_mqtt::CreateOptionsBuilder::new_v3()
            .server_uri(&host)
            .client_id(client_id)
            .max_buffered_messages(25)
            .finalize();

        let mut client = paho_mqtt::AsyncClient::new(create_opts).expect("Failed to create client");
        let stream = client.get_stream(25);

        Self {
            host,
            access_code,
            serial,
            // topic_device_request: format!("device/{}/request", &serial),
            // topic_device_report: format!("device/{}/report", &serial),
            client,
            stream,
            // tx,
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
            // self.tx.send(parse_message(&msg))?;

            let msg: crate::mqtt_types::Message = serde_json::from_slice(msg.payload())?;

            debug!("got message: {:#?}", msg);
            panic!();
        } else {
            warn!("Lost connection.");
            // self.tx.send(Message::Disconnected)?;

            while (self.client.reconnect().await).is_err() {
                tokio::time::sleep(Duration::from_secs(1)).await;
                debug!("attempting to reconnect");
                // self.tx.send(Message::Reconnecting)?;
            }

            // self.tx.send(Message::Connected)?;
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
            .password(&self.access_code)
            .finalize();

        debug!("connecting");
        // self.tx.send(Message::Connecting)?;
        self.client.connect(conn_opts).await?;
        // self.tx.send(Message::Connected)?;
        debug!("connected");

        Ok(())
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
        // self.subscibe_to_device_report();
        let topic = format!("device/{}/report", env::var("BAMBU_IDENT")?);
        debug!("subscribing to topic: {}", topic);
        self.client.subscribe(topic, paho_mqtt::QOS_0);

        loop {
            Self::poll(self).await?;
        }
    }
}
