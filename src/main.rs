#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_doc_comments)]
#![allow(unused_labels)]
#![allow(unexpected_cfgs)]

pub mod client;
pub mod logging;
pub mod mqtt_types;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use futures::StreamExt;
// use rumqttc::{Client, MqttOptions, QoS};
use std::{env, time::Duration};

#[tokio::main]
// #[cfg(feature = "nope")]
async fn main() -> Result<()> {
    dotenv::dotenv()?;

    logging::init_logs();

    let host = env::var("BAMBU_IP")?;
    let access_code = env::var("BAMBU_ACCESS_CODE")?;
    let serial = env::var("BAMBU_IDENT")?;

    let mut client = crate::client::Client::new(host, access_code, serial);

    debug!("running");
    // client.run().await.unwrap();

    tokio::try_join!(
        tokio::spawn(async move {
            client.run().await.unwrap();
            debug!("finished running");
        }),
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5)).await;
                debug!("idling");
            }
        }),
    )?;

    Ok(())
}

/// working?
// #[tokio::main]
#[cfg(feature = "nope")]
async fn main() -> Result<()> {
    dotenv::dotenv()?;

    logging::init_logs();

    use bambulab::{Command, Message};

    let host = env::var("BAMBU_IP")?;
    let access_code = env::var("BAMBU_ACCESS_CODE")?;
    let serial = env::var("BAMBU_IDENT")?;

    let (tx, mut rx) = tokio::sync::broadcast::channel::<Message>(25);

    let mut client = bambulab::Client::new(host, access_code, serial, tx);
    let mut client_clone = client.clone();

    tokio::try_join!(
        tokio::spawn(async move {
            client.run().await.unwrap();
        }),
        tokio::spawn(async move {
            loop {
                let message = rx.recv().await.unwrap();
                println!("received: {message:?}");

                // if message == Message::Connected {
                //     client_clone.publish(Command::PushAll).await.unwrap();
                // }
            }
        }),
    )?;

    // let mut client = crate::client::Client::new(host, access_code, serial);

    // debug!("running");
    // client.run().await.unwrap();

    Ok(())
}

#[cfg(feature = "nope")]
// #[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv()?;

    logging::init_logs();

    let addr = format!("mqtts://{}:8883", env::var("BAMBU_IP")?);
    debug!("addr = {}", addr);

    let client_id = "bambu_watcher";

    let create_opts = paho_mqtt::CreateOptionsBuilder::new()
        .server_uri(&addr)
        .client_id(client_id)
        .max_buffered_messages(25)
        .finalize();

    debug!("creating client");
    let mut cli = paho_mqtt::AsyncClient::new(create_opts).expect("Failed to create client");
    let mut stream = cli.get_stream(25);

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
        .password(env::var("BAMBU_ACCESS_CODE")?)
        .finalize();

    debug!("connecting");
    cli.connect(conn_opts).await?;
    debug!("connected");

    debug!("subscribing");
    let topic = format!("/device/{}/report", env::var("BAMBU_IDENT")?);
    debug!("topic = {}", topic);
    cli.subscribe(topic, paho_mqtt::QOS_0);
    debug!("subscribed");

    debug!("Waiting for messages...");

    while let Some(msg_opt) = stream.next().await {
        if let Some(msg) = msg_opt {
            println!("{}", msg);
        } else {
            println!("Lost connection.");
            break;
        }
    }

    Ok(())
}

#[cfg(feature = "nope")]
fn main() -> Result<()> {
    dotenv::dotenv()?;

    logging::init_logs();

    // let mut mqttoptions = MqttOptions::new("bambu_watcher", env::var("BAMBU_IP")?, 8883);

    let addr = format!("{}", env::var("BAMBU_IP")?);
    debug!("addr = {}", addr);

    let mut mqttoptions = MqttOptions::new("bambu_watcher", addr, 8883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    mqttoptions.set_credentials("bblp", env::var("BAMBU_ACCESS_CODE")?);

    debug!("broker = {:?}", mqttoptions.broker_address());

    // let tlsconfig = rumqttc::TlsConfiguration::Rustls();

    // mqttoptions.set_transport(rumqttc::Transport::Tls(tlsconfig));
    // mqttoptions.set_transport(rumqttc::Transport::tls_with_default_config());
    // mqttoptions.set_transport(rumqttc::Transport::tcp());

    // // Use rustls-native-certs to load root certificates from the operating system.
    // let mut root_cert_store = rumqttc::tokio_rustls::rustls::RootCertStore::empty();
    // root_cert_store.add_parsable_certificates(rustls_native_certs::load_native_certs().expect("could not load platform certs"));

    // let client_config = rumqttc::tokio_rustls::rustls::ClientConfig::builder()
    //     .with_root_certificates(root_cert_store)
    //     .with_no_client_auth();

    // mqttoptions.set_transport(rumqttc::Transport::tls_with_config(client_config.into()));

    let (mut client, mut connection) = Client::new(mqttoptions, 10);

    // let topic = format!("/device/{}/report", env::var("BAMBU_IDENT")?);
    // debug!("topic = {}", topic);
    // client.subscribe(topic, QoS::AtMostOnce).unwrap();

    for (i, notification) in connection.iter().enumerate() {
        println!("Notification = {:?}", notification);
        break;
    }

    Ok(())
}
