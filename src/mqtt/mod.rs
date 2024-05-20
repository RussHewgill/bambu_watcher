pub mod command;
pub mod message;
pub mod parse;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use futures::StreamExt;
use rumqttc::{
    tokio_rustls::{client, rustls},
    AsyncClient, EventLoop, Incoming, MqttOptions,
};
use std::{sync::Arc, time::Duration};
use tracing_subscriber::field::debug;

use crate::{
    config::{ConfigArc, PrinterConfig},
    conn_manager::PrinterId,
};

use self::{command::Command, message::Message};

/// scary, insecure, do not allow outside of local network
#[derive(Debug)]
struct NoCertificateVerification {
    pub serial: String,
}

/// TODO: maybe at least check the serial is correct?
impl rumqttc::tokio_rustls::rustls::client::danger::ServerCertVerifier
    for NoCertificateVerification
{
    fn verify_server_cert(
        &self,
        end_entity: &rumqttc::tokio_rustls::rustls::pki_types::CertificateDer<'_>,
        intermediates: &[rumqttc::tokio_rustls::rustls::pki_types::CertificateDer<'_>],
        server_name: &rumqttc::tokio_rustls::rustls::pki_types::ServerName<'_>,
        ocsp_response: &[u8],
        now: rumqttc::tokio_rustls::rustls::pki_types::UnixTime,
    ) -> std::prelude::v1::Result<
        rumqttc::tokio_rustls::rustls::client::danger::ServerCertVerified,
        rumqttc::tokio_rustls::rustls::Error,
    > {
        // debug!("end_entity: {:?}", end_entity);
        // debug!("server_name: {:?}", server_name);
        // debug!("ocsp_response: {:?}", ocsp_response);
        Ok(rumqttc::tokio_rustls::rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        unimplemented!()
    }

    fn supported_verify_schemes(&self) -> Vec<rumqttc::tokio_rustls::rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA1,
            rustls::SignatureScheme::ECDSA_SHA1_Legacy,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::ED448,
        ]
    }
}

pub struct BambuClient {
    config: PrinterConfig,

    // client: paho_mqtt::AsyncClient,
    // stream: paho_mqtt::AsyncReceiver<Option<paho_mqtt::Message>>,
    client: rumqttc::AsyncClient,
    // eventloop: rumqttc::EventLoop,
    tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
    // rx: tokio::sync::broadcast::Receiver<Command>,
    topic_device_request: String,
    topic_device_report: String,
}

impl BambuClient {
    pub async fn new_and_init(
        config: ConfigArc,
        printer_cfg: PrinterConfig,
        tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
    ) -> Result<Self> {
        if config.logged_in() {
            Self::_new_and_init_cloud(config, &printer_cfg, tx).await
        } else {
            Self::_new_and_init_lan(&printer_cfg, tx).await
        }
    }

    async fn _new_and_init_cloud(
        config: ConfigArc,
        // printer_cfg: Arc<PrinterConfig>,
        printer_cfg: &PrinterConfig,
        tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
    ) -> Result<Self> {
        let client_id = format!("bambu-watcher-{}", nanoid::nanoid!(8));

        let (username, password) = {
            let db = config.auth.read().await;
            db.get_cloud_mqtt_creds()?
        };

        const CLOUD_HOST: &'static str = "us.mqtt.bambulab.com";

        let mut mqttoptions = rumqttc::MqttOptions::new(client_id, CLOUD_HOST, 8883);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        mqttoptions.set_credentials(&username, &password);

        let mut root_cert_store = rustls::RootCertStore::empty();
        root_cert_store.add_parsable_certificates(
            rustls_native_certs::load_native_certs().expect("could not load platform certs"),
        );

        let client_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_cert_store)
            .with_no_client_auth();

        let transport = rumqttc::Transport::tls_with_config(rumqttc::TlsConfiguration::Rustls(
            Arc::new(client_config),
        ));

        mqttoptions.set_transport(transport);
        mqttoptions.set_clean_session(true);

        debug!("connecting, printer = {}", &printer_cfg.name);
        let (mut client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
        debug!("connected, printer = {}", &printer_cfg.name);

        let mut out = Self {
            config: printer_cfg.clone(),
            topic_device_request: format!("device/{}/request", &printer_cfg.serial),
            topic_device_report: format!("device/{}/report", &printer_cfg.serial),
            client,
            // eventloop,
            // stream,
            tx,
            // rx,
        };

        out.init(eventloop).await?;

        Ok(out)
    }

    async fn _new_and_init_lan(
        // printer_cfg: Arc<PrinterConfig>,
        printer_cfg: &PrinterConfig,
        tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
    ) -> Result<Self> {
        let client_id = format!("bambu-watcher-{}", nanoid::nanoid!(8));

        let mut mqttoptions = MqttOptions::new(client_id, &printer_cfg.host, 8883);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        mqttoptions.set_credentials("bblp", &printer_cfg.access_code);

        let client_config = rumqttc::tokio_rustls::rustls::ClientConfig::builder()
            // .with_root_certificates(root_cert_store)
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertificateVerification {
                serial: (*printer_cfg.serial).clone(),
            }))
            .with_no_client_auth();

        // let transport = rumqttc::Transport::tls_with_config(rumqttc::TlsConfiguration::Native);
        let transport = rumqttc::Transport::tls_with_config(rumqttc::TlsConfiguration::Rustls(
            Arc::new(client_config),
        ));

        mqttoptions.set_transport(transport);
        mqttoptions.set_clean_session(true);

        debug!("connecting, printer = {}", &printer_cfg.name);
        let (mut client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
        debug!("connected, printer = {}", &printer_cfg.name);

        let mut out = Self {
            config: printer_cfg.clone(),
            topic_device_request: format!("device/{}/request", &printer_cfg.serial),
            topic_device_report: format!("device/{}/report", &printer_cfg.serial),
            client,
            // eventloop,
            // stream,
            tx,
            // rx,
        };

        out.init(eventloop).await?;

        Ok(out)
    }

    pub async fn init(&mut self, eventloop: EventLoop) -> Result<()> {
        let config2 = self.config.clone();
        let client2 = self.client.clone();
        let tx2 = self.tx.clone();
        let topic_report = self.topic_device_report.clone();
        let topic_request = self.topic_device_request.clone();
        tokio::task::spawn(async move {
            let mut listener = ClientListener::new(
                config2,
                client2,
                eventloop,
                tx2,
                topic_report,
                topic_request,
            );
            loop {
                if let Err(e) = listener.poll_eventloop().await {
                    error!("Error in listener: {:?}", e);
                    listener
                        .tx
                        .send((listener.printer_cfg.serial.clone(), Message::Disconnected))
                        .unwrap();
                }
                listener.eventloop.clean();
                debug!("Reconnecting...");
            }
        });

        // self.client
        //     .subscribe(&self.topic_device_report, rumqttc::QoS::AtMostOnce)
        //     .await?;

        Ok(())
    }

    pub async fn publish(&self, command: Command) -> Result<()> {
        let payload = command.get_payload();

        let qos = rumqttc::QoS::AtMostOnce;
        self.client
            .publish(&self.topic_device_request, qos, false, payload)
            .await?;

        Ok(())
    }
}

struct ClientListener {
    printer_cfg: PrinterConfig,
    client: rumqttc::AsyncClient,
    eventloop: rumqttc::EventLoop,
    tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
    topic_device_report: String,
    topic_device_request: String,
}

impl ClientListener {
    pub fn new(
        printer_cfg: PrinterConfig,
        client: rumqttc::AsyncClient,
        eventloop: rumqttc::EventLoop,
        tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
        topic_device_report: String,
        topic_device_request: String,
    ) -> Self {
        Self {
            printer_cfg,
            client,
            eventloop,
            tx,
            topic_device_report,
            topic_device_request,
        }
    }

    /// MARK: main event handler
    async fn poll_eventloop(&mut self) -> Result<()> {
        use rumqttc::Event;
        loop {
            let event = match self.eventloop.poll().await {
                Ok(event) => event,
                Err(e) => {
                    error!("Error in eventloop: {:?}", e);
                    continue;
                }
            };
            match event {
                Event::Outgoing(event) => {
                    // debug!("outgoing event: {:?}", event);
                }
                Event::Incoming(Incoming::PingResp) => {}
                Event::Incoming(Incoming::ConnAck(c)) => {
                    debug!("got ConnAck: {:?}", c.code);
                    if c.code == rumqttc::ConnectReturnCode::Success {
                        // debug!("Connected to MQTT");
                        self.client
                            .subscribe(&self.topic_device_report, rumqttc::QoS::AtMostOnce)
                            .await?;
                        debug!("sent subscribe to topic");
                        // self.send_pushall().await?;
                    } else {
                        error!("Failed to connect to MQTT: {:?}", c.code);
                    }
                }
                Event::Incoming(Incoming::SubAck(s)) => {
                    debug!("got SubAck");
                    if s.return_codes
                        .iter()
                        .any(|&r| r == rumqttc::SubscribeReasonCode::Failure)
                    {
                        error!("Failed to subscribe to topic");
                    } else {
                        debug!("sending pushall");
                        self.send_pushall().await?;
                        debug!("sent");
                        // debug!("sending get version");
                        // self.send_get_version().await?;
                        // debug!("sent");
                    }
                }
                Event::Incoming(Incoming::Publish(p)) => {
                    // debug!("incoming publish");
                    let msg = parse::parse_message(&p);
                    // debug!("incoming publish: {:?}", msg);
                    self.tx.send((self.printer_cfg.serial.clone(), msg))?;
                }
                Event::Incoming(event) => {
                    debug!("incoming other event: {:?}", event);
                }
            }
        }
    }

    async fn send_get_version(&mut self) -> Result<()> {
        let payload = Command::GetVersion.get_payload();

        self.client
            .publish(
                &self.topic_device_request,
                rumqttc::QoS::AtMostOnce,
                false,
                payload,
            )
            .await?;

        Ok(())
    }

    async fn send_pushall(&mut self) -> Result<()> {
        let command = Command::PushAll;
        let payload = command.get_payload();

        let qos = rumqttc::QoS::AtMostOnce;
        self.client
            .publish(&self.topic_device_request, qos, false, payload)
            .await?;

        Ok(())
    }
}

#[cfg(feature = "nope")]
mod paho {

    #[derive(Clone)]
    pub struct Client {
        config: PrinterConfig,

        client: paho_mqtt::AsyncClient,
        stream: paho_mqtt::AsyncReceiver<Option<paho_mqtt::Message>>,

        tx: tokio::sync::broadcast::Sender<Message>,

        topic_device_request: String,
        topic_device_report: String,
    }

    impl BambuClient {
        /// Creates a new Bambu printer MQTT client.
        ///
        /// # Panics
        ///
        /// Panics if the MQTT client cannot be created.
        pub fn new(
            printer_cfg: &PrinterConfig,
            tx: tokio::sync::broadcast::Sender<Message>,
        ) -> Self {
            let client_id = format!("bambu-watcher-{}", nanoid::nanoid!(8));

            let create_opts = paho_mqtt::CreateOptionsBuilder::new()
                .server_uri(&printer_cfg.host)
                .client_id(client_id)
                .max_buffered_messages(25)
                .finalize();

            let mut client =
                paho_mqtt::AsyncClient::new(create_opts).expect("Failed to create client");
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

            let msg =
                paho_mqtt::Message::new(&self.topic_device_request, payload, paho_mqtt::QOS_0);
            self.client.publish(msg).await?;

            Ok(())
        }
    }
}
