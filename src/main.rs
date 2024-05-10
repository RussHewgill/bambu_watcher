#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_doc_comments)]
#![allow(unused_labels)]
#![allow(unexpected_cfgs)]

pub mod app;
pub mod app_types;
pub mod client;
pub mod config;
pub mod logging;
pub mod status;
pub mod ui;
pub mod ui_types;
// pub mod mqtt_types;

use anyhow::{anyhow, bail, ensure, Context, Result};
use app_types::AppEvent;
use client::{PrinterConnCmd, PrinterConnManager, PrinterConnMsg};
use tracing::{debug, error, info, trace, warn};

use futures::StreamExt;
// use rumqttc::{Client, MqttOptions, QoS};
use dashmap::DashMap;
use std::{env, sync::Arc, time::Duration};

use bambulab::{Command, Message};

use crate::{client::PrinterId, status::PrinterStatus};

/// config test
#[cfg(feature = "nope")]
fn main() -> Result<()> {
    dotenv::dotenv()?;
    logging::init_logs();

    let path = "config.yaml";

    // let printer0 = config::PrinterConfig {
    //     name: "bambu".to_string(),
    //     host: env::var("BAMBU_IP")?,
    //     access_code: env::var("BAMBU_ACCESS_CODE")?,
    //     serial: env::var("BAMBU_IDENT")?,
    // };

    // let config = config::Configs { printers: vec![printer0] };

    // serde_yaml::to_writer(std::fs::File::create(path)?, &config)?;

    let config: config::Config = serde_yaml::from_reader(std::fs::File::open(path)?)?;

    debug!("config = {:#?}", config);

    Ok(())
}

/// threads:
///     main egui thread
///     tokio thread, listens for messages from the printer
fn main() -> eframe::Result<()> {
    // dotenv::dotenv().unwrap();
    logging::init_logs();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    static VISIBLE: std::sync::Mutex<bool> = std::sync::Mutex::new(true);

    let config: config::Config =
        serde_yaml::from_reader(std::fs::File::open("config.yaml").unwrap()).unwrap();

    let mut _tray_icon = std::rc::Rc::new(std::cell::RefCell::new(None));
    let tray_c = _tray_icon.clone();

    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::channel::<PrinterConnMsg>(25);
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<PrinterConnCmd>(25);

    let printer_states: Arc<DashMap<PrinterId, PrinterStatus>> = Arc::new(DashMap::new());
    let printer_states2 = printer_states.clone();

    /// debug printer state
    {
        warn!("adding debug printer state");

        for printer in config.printers.iter() {
            printer_states.insert(printer.serial.clone(), PrinterStatus::default());
        }
    }

    #[cfg(feature = "nope")]
    /// tokio thread
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut manager = PrinterConnManager::new(config, printer_states2, cmd_rx, msg_tx);

            debug!("running PrinterConnManager");
            manager.run().await.unwrap();
        });
    });

    eframe::run_native(
        "Bambu Watcher",
        native_options,
        Box::new(move |cc| {
            let winit::raw_window_handle::RawWindowHandle::Win32(handle) =
                winit::raw_window_handle::HasWindowHandle::window_handle(&cc)
                    .unwrap()
                    .as_raw()
            else {
                panic!("Unsupported platform");
            };

            let context = cc.egui_ctx.clone();

            // tray-icon crate
            // https://docs.rs/tray-icon/0.12.0/tray_icon/struct.TrayIconEvent.html#method.set_event_handler
            tray_icon::TrayIconEvent::set_event_handler(Some(
                move |event: tray_icon::TrayIconEvent| {
                    // println!("TrayIconEvent: {:?}", event);
                    if event.click_type != tray_icon::ClickType::Double {
                        return;
                    }

                    // Just a static Mutex<bool>
                    let mut visible = VISIBLE.lock().unwrap();

                    if *visible {
                        debug!("hiding window");
                        let window_handle = windows::Win32::Foundation::HWND(handle.hwnd.into());
                        let hide = windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
                        unsafe {
                            let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(
                                window_handle,
                                hide,
                            );
                        }
                        *visible = false;
                    } else {
                        debug!("showing window");
                        let window_handle = windows::Win32::Foundation::HWND(handle.hwnd.into());
                        // You can show the window in all sorts of ways:
                        // https://learn.microsoft.com/en-gb/windows/win32/api/winuser/nf-winuser-showwindow
                        let show = windows::Win32::UI::WindowsAndMessaging::SW_SHOWDEFAULT;
                        unsafe {
                            let _ = windows::Win32::UI::WindowsAndMessaging::ShowWindow(
                                window_handle,
                                show,
                            );
                        }
                        *visible = true;
                    }
                },
            ));

            /// Icon by https://www.flaticon.com/authors/freepik
            let icon = crate::app::load_icon(&"icon.png");

            {
                tray_c.borrow_mut().replace(
                    tray_icon::TrayIconBuilder::new()
                        // .with_menu(Box::new(menu))
                        .with_menu(Box::new(tray_icon::menu::Menu::new()))
                        .with_tooltip("winit - awesome windowing lib")
                        .with_icon(icon)
                        .with_title("x")
                        .build()
                        .unwrap(),
                );
            }

            // let tray_icon = tray_icon::TrayIconBuilder::new()
            //     .build()
            //     .unwrap();

            egui_extras::install_image_loaders(&cc.egui_ctx);

            Box::new(ui_types::App::new(tray_c, printer_states, config, cc))
        }),
    )

    //
}

#[cfg(feature = "nope")]
fn main() -> Result<()> {
    dotenv::dotenv()?;
    logging::init_logs();

    let event_loop = winit::event_loop::EventLoop::<AppEvent>::with_user_event().build()?;

    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::channel::<PrinterConnMsg>(25);
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<PrinterConnCmd>(25);

    let config = serde_yaml::from_reader(std::fs::File::open("config.yaml").unwrap()).unwrap();

    let mut state = app_types::State::new(&config, cmd_tx);

    let proxy = event_loop.create_proxy();
    /// update timer thread
    std::thread::spawn(move || loop {
        proxy.send_event(AppEvent::Timer).unwrap();
        std::thread::sleep(Duration::from_millis(1000));
    });

    let proxy = event_loop.create_proxy();
    /// event listener thread
    std::thread::spawn(move || {
        loop {
            if let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
                // println!("tray event: {:?}", event);
                proxy.send_event(AppEvent::TrayEvent(event)).unwrap();
            }

            if let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
                // println!("menu event: {:?}", event);
                proxy.send_event(AppEvent::MenuEvent(event)).unwrap();
            }

            if let Ok(msg) = msg_rx.try_recv() {
                // println!("msg: {:?}", msg);
                proxy.send_event(AppEvent::ConnMsg(msg)).unwrap();
            }
        }
    });

    #[cfg(feature = "nope")]
    /// tokio thread
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let mut manager = PrinterConnManager::new(config, cmd_rx, msg_tx);

            debug!("running PrinterConnManager");
            manager.run().await.unwrap();
        });
    });

    event_loop.run_app(&mut state)?;

    Ok(())
}

#[cfg(feature = "nope")]
fn main() -> Result<()> {
    dotenv::dotenv()?;

    logging::init_logs();

    let event_loop = winit::event_loop::EventLoop::builder().build()?;

    let (tx, rx) = tokio::sync::broadcast::channel::<Message>(25);

    let mut state = app::State::new(rx);

    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let host = env::var("BAMBU_IP").unwrap();
            let access_code = env::var("BAMBU_ACCESS_CODE").unwrap();
            let serial = env::var("BAMBU_IDENT").unwrap();

            start_printer_listener(tx, &host, &access_code, &serial)
                .await
                .unwrap();
        });
    });

    event_loop.run_app(&mut state)?;

    Ok(())
}

/// working?
// #[tokio::main]
#[cfg(feature = "nope")]
async fn main() -> Result<()> {
    dotenv::dotenv()?;
    logging::init_logs();

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
                debug!("received: {:#?}", message);

                if message == Message::Connected {
                    client_clone.publish(Command::PushAll).await.unwrap();
                }
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
