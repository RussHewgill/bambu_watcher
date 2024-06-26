pub mod command;
pub mod message;
pub mod parse;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tokio::sync::RwLock;
use tracing::{debug, error, info, trace, warn};

use futures::StreamExt;
use rumqttc::{
    tokio_rustls::{client, rustls},
    AsyncClient, EventLoop, Incoming, MqttOptions,
};
use std::{sync::Arc, time::Duration};

use crate::{
    config::{ConfigArc, PrinterConfig},
    conn_manager::PrinterId,
};

use self::{command::Command, message::Message};

/// scary, insecure, do not allow outside of local network
#[derive(Debug)]
pub struct NoCertificateVerification {
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
    // config: PrinterConfig,
    config: Arc<RwLock<PrinterConfig>>,

    // client: paho_mqtt::AsyncClient,
    // stream: paho_mqtt::AsyncReceiver<Option<paho_mqtt::Message>>,
    client: rumqttc::AsyncClient,
    // eventloop: rumqttc::EventLoop,
    tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
    // rx: tokio::sync::broadcast::Receiver<Command>,
    topic_device_request: String,
    topic_device_report: String,
    // kill_rx: tokio::sync::oneshot::Receiver<()>,
}

impl BambuClient {
    pub async fn new_and_init(
        config: ConfigArc,
        // printer_cfg: PrinterConfig,
        printer_cfg: Arc<RwLock<PrinterConfig>>,
        tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
        kill_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<Self> {
        if config.logged_in() {
            Self::_new_and_init_cloud(config, printer_cfg, tx, kill_rx).await
        } else {
            Self::_new_and_init_lan(printer_cfg, tx, kill_rx).await
        }
    }

    async fn _new_and_init_cloud(
        config: ConfigArc,
        // printer_cfg: Arc<PrinterConfig>,
        // printer_cfg: &PrinterConfig,
        printer_cfg: Arc<RwLock<PrinterConfig>>,
        tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
        kill_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<Self> {
        debug!("init cloud mqtt listener");
        let client_id = format!("bambu-watcher-{}", nanoid::nanoid!(8));

        let (username, password) = {
            let db = config.auth.read().await;
            db.get_cloud_mqtt_creds()?
        };

        const CLOUD_HOST: &'static str = "us.mqtt.bambulab.com";

        let mut mqttoptions = rumqttc::MqttOptions::new(client_id, CLOUD_HOST, 8883);
        /// XXX: does this matter?
        mqttoptions.set_keep_alive(Duration::from_secs(15));
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
        // mqttoptions.set_clean_session(true);

        debug!("connecting, printer = {}", &printer_cfg.read().await.name);
        let (mut client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
        debug!("connected, printer = {}", &printer_cfg.read().await.name);

        let mut out = Self {
            config: printer_cfg.clone(),
            topic_device_request: format!("device/{}/request", &printer_cfg.read().await.serial),
            topic_device_report: format!("device/{}/report", &printer_cfg.read().await.serial),
            client,
            // eventloop,
            // stream,
            tx,
            // rx,
            // kill_rx,
        };

        out.init(eventloop, kill_rx).await?;

        Ok(out)
    }

    async fn _new_and_init_lan(
        // printer_cfg: Arc<PrinterConfig>,
        // printer_cfg: &PrinterConfig,
        printer_cfg: Arc<RwLock<PrinterConfig>>,
        tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
        kill_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<Self> {
        debug!("init lan mqtt listener");
        let client_id = format!("bambu-watcher-{}", nanoid::nanoid!(8));

        let printer = printer_cfg.read().await;

        // let host = printer.host.as_ref().context("missing host")?;
        if printer.host.is_empty() {
            bail!("missing host");
        }

        let mut mqttoptions = MqttOptions::new(client_id, &printer.host, 8883);
        mqttoptions.set_keep_alive(Duration::from_secs(5));
        mqttoptions.set_credentials("bblp", &printer.access_code);

        let client_config = rumqttc::tokio_rustls::rustls::ClientConfig::builder()
            // .with_root_certificates(root_cert_store)
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertificateVerification {
                serial: (*printer.serial).clone(),
            }))
            .with_no_client_auth();

        // let transport = rumqttc::Transport::tls_with_config(rumqttc::TlsConfiguration::Native);
        let transport = rumqttc::Transport::tls_with_config(rumqttc::TlsConfiguration::Rustls(
            Arc::new(client_config),
        ));

        mqttoptions.set_transport(transport);
        // mqttoptions.set_clean_session(true);

        debug!("connecting, printer = {}", &printer.name);
        let (mut client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
        debug!("connected, printer = {}", &printer.name);

        let mut out = Self {
            config: printer_cfg.clone(),
            topic_device_request: format!("device/{}/request", &printer.serial),
            topic_device_report: format!("device/{}/report", &printer.serial),
            client,
            // eventloop,
            // stream,
            tx,
            // kill_rx,
            // rx,
        };

        out.init(eventloop, kill_rx).await?;

        Ok(out)
    }

    pub async fn init(
        &mut self,
        eventloop: EventLoop,
        mut kill_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Result<()> {
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
                // kill_rx,
            );
            loop {
                tokio::select! {
                    _ = &mut kill_rx => {
                        debug!("Listener task got kill command");
                        break;
                    }
                    event = listener.poll_eventloop() => {
                        if let Err(e) = event {
                            error!("Error in listener: {:?}", e);
                            listener
                                .tx
                                .send((
                                    listener.printer_cfg.read().await.serial.clone(),
                                    Message::Disconnected,
                                ))
                                .unwrap();
                        }
                        listener.eventloop.clean();
                        debug!("Reconnecting...");
                    }
                }
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
    // printer_cfg: PrinterConfig,
    printer_cfg: Arc<RwLock<PrinterConfig>>,
    client: rumqttc::AsyncClient,
    eventloop: rumqttc::EventLoop,
    tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
    topic_device_report: String,
    topic_device_request: String,
    // kill_rx: tokio::sync::oneshot::Receiver<()>,
}

impl ClientListener {
    pub fn new(
        // printer_cfg: PrinterConfig,
        printer_cfg: Arc<RwLock<PrinterConfig>>,
        client: rumqttc::AsyncClient,
        eventloop: rumqttc::EventLoop,
        tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
        topic_device_report: String,
        topic_device_request: String,
        // kill_rx: tokio::sync::oneshot::Receiver<()>,
    ) -> Self {
        Self {
            printer_cfg,
            client,
            eventloop,
            tx,
            topic_device_report,
            topic_device_request,
            // kill_rx,
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
                    self.tx
                        .send((self.printer_cfg.read().await.serial.clone(), msg))?;
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

pub async fn debug_get_printer_report(printer: PrinterConfig) -> Result<()> {
    let client_id = format!("bambu-watcher-{}", nanoid::nanoid!(8));

    // let mut mqttoptions = MqttOptions::new(client_id, &printer.host.context("missing_host")?, 8883);
    let mut mqttoptions = MqttOptions::new(client_id, &printer.host, 8883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    mqttoptions.set_credentials("bblp", &printer.access_code);

    let client_config = rumqttc::tokio_rustls::rustls::ClientConfig::builder()
        // .with_root_certificates(root_cert_store)
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoCertificateVerification {
            serial: (*printer.serial).clone(),
        }))
        .with_no_client_auth();

    // let transport = rumqttc::Transport::tls_with_config(rumqttc::TlsConfiguration::Native);
    let transport = rumqttc::Transport::tls_with_config(rumqttc::TlsConfiguration::Rustls(
        Arc::new(client_config),
    ));

    mqttoptions.set_transport(transport);
    // mqttoptions.set_clean_session(true);

    debug!("connecting, printer = {}", &printer.name);
    let (mut client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    debug!("connected, printer = {}", &printer.name);

    let topic_device_request = format!("device/{}/request", &printer.serial);
    let topic_device_report = format!("device/{}/report", &printer.serial);

    use rumqttc::Event;

    loop {
        match eventloop.poll().await? {
            Event::Outgoing(event) => {}
            Event::Incoming(Incoming::PingResp) => {}
            Event::Incoming(Incoming::ConnAck(c)) => {
                debug!("got ConnAck: {:?}", c.code);
                if c.code == rumqttc::ConnectReturnCode::Success {
                    // debug!("Connected to MQTT");
                    client
                        .subscribe(&topic_device_report, rumqttc::QoS::AtMostOnce)
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
                    // self.send_pushall().await?;
                    let command = Command::PushAll;
                    let payload = command.get_payload();

                    let qos = rumqttc::QoS::AtMostOnce;
                    client
                        .publish(&topic_device_request, qos, false, payload)
                        .await?;
                    debug!("sent");
                    // debug!("sending get version");
                    // self.send_get_version().await?;
                    // debug!("sent");
                }
            }
            Event::Incoming(Incoming::Publish(p)) => {
                // debug!("incoming publish");

                let payload = &p.payload;

                let parsed_message = serde_json::from_slice::<serde_json::Value>(&payload)?;

                debug!("incoming publish: {:#?}", parsed_message);

                let s = serde_json::to_string_pretty(&parsed_message).unwrap();
                // std::fs::write("printer_report.json", s)?;
                let mut file = std::fs::OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open("printer_reports.json")
                    .unwrap();

                use std::io::Write;

                if let Err(e) = writeln!(&mut file, "{},", s) {
                    eprintln!("Couldn't write to file: {}", e);
                }

                // panic!()

                // let msg = parse::parse_message(&p);
                // match msg {
                //     Message::Report(report) => {
                //         debug!("incoming report: {:#?}", report);
                //         let s = serde_json::to_string_pretty(report).unwrap();
                //         std::fs::write("printer_report.json", s)?;
                //     }
                //     _ => {
                //         debug!("incoming publish: {:#?}", msg);
                //     }
                // }
                // debug!("incoming publish: {:#?}", msg);
                // self.tx
                //     .send((self.printer_cfg.read().await.serial.clone(), msg))?;
            }
            Event::Incoming(event) => {
                debug!("incoming other event: {:?}", event);
            }
        }
    }

    // Ok(())
}
