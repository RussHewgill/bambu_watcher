#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(dead_code)]
#![allow(unused_doc_comments)]
#![allow(unused_labels)]
#![allow(unexpected_cfgs)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// pub mod app;
// pub mod app_types;
pub mod alert;
pub mod client;
pub mod config;
pub mod ftp;
pub mod icons;
pub mod logging;
pub mod status;
pub mod tray;
pub mod ui;
pub mod ui_types;
// pub mod mqtt_types;

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use futures::StreamExt;
// use rumqttc::{Client, MqttOptions, QoS};
use dashmap::DashMap;
use std::{env, sync::Arc, time::Duration};

use bambulab::{Command, Message};

use crate::{
    client::{PrinterConnCmd, PrinterConnManager, PrinterConnMsg, PrinterId},
    status::PrinterStatus,
};

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

/// FTP test
#[cfg(feature = "nope")]
fn main() {
    dotenv::dotenv().unwrap();
    logging::init_logs();

    let printer_cfg = config::PrinterConfig {
        name: "bambu".to_string(),
        host: env::var("BAMBU_IP").unwrap(),
        access_code: env::var("BAMBU_ACCESS_CODE").unwrap(),
        serial: env::var("BAMBU_IDENT").unwrap(),
    };

    crate::ftp::get_gcode_thumbnail(
        &printer_cfg,
        "/cache/AMS Purging Strips 2 Colors Modified.3mf",
    )
    .unwrap();

    #[cfg(feature = "nope")]
    {
        use suppaftp::native_tls::{TlsConnector, TlsStream};
        use suppaftp::{FtpStream, NativeTlsConnector, NativeTlsFtpStream};

        let port = 990;

        let addr = format!("{}:{}", env::var("BAMBU_IP").unwrap(), port);
        debug!("addr = {}", addr);

        /// explicit doesn't work for some reason
        debug!("connecting");
        let mut ftp_stream =
            NativeTlsFtpStream::connect_secure_implicit(&addr, ctx, &env::var("BAMBU_IP").unwrap())
                .unwrap();

        // let mut ftp_stream = FtpStream::connect(&addr).unwrap_or_else(|err| panic!("{}", err));
        debug!("connected to server");
        assert!(ftp_stream
            .login("bblp", &env::var("BAMBU_ACCESS_CODE").unwrap())
            .is_ok());

        debug!("listing");
        if let Ok(list) = ftp_stream.list(None) {
            for item in list {
                println!("{}", item);
            }
        }

        debug!("done");

        // Disconnect from server
        assert!(ftp_stream.quit().is_ok());
    }

    //
}

/// MARK: TODO:
///     fan speeds
///     AMS status
///     graphs
/// threads:
///     main egui thread
///     tokio thread, listens for messages from the printer
// #[cfg(feature = "nope")]
fn main() -> eframe::Result<()> {
    // dotenv::dotenv().unwrap();
    logging::init_logs();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([850.0, 750.0])
            .with_min_inner_size([850.0, 750.0]),
        ..Default::default()
    };

    static VISIBLE: std::sync::Mutex<bool> = std::sync::Mutex::new(true);

    let config: config::Config =
        serde_yaml::from_reader(std::fs::File::open("config.yaml").unwrap()).unwrap();
    let config2 = config.clone();

    let mut _tray_icon = std::rc::Rc::new(std::cell::RefCell::new(None));
    let tray_c = _tray_icon.clone();

    let (msg_tx, mut msg_rx) = tokio::sync::watch::channel::<PrinterConnMsg>(PrinterConnMsg::Empty);
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<PrinterConnCmd>(2);

    let printer_states: Arc<DashMap<PrinterId, PrinterStatus>> = Arc::new(DashMap::new());
    let printer_states2 = printer_states.clone();

    let (ctx_tx, ctx_rx) = tokio::sync::oneshot::channel::<egui::Context>();

    /// debug printer state
    #[cfg(feature = "nope")]
    {
        warn!("adding debug printer state");

        {
            let mut status = PrinterStatus::default();
            status.temp_nozzle = Some(200.0);
            status.temp_tgt_nozzle = Some(200.0);
            status.temp_bed = Some(60.0);
            status.temp_tgt_bed = Some(60.0);

            status.state = status::PrinterState::Printing;
            status.eta = Some(chrono::Local::now() + chrono::Duration::minutes(10));
            status.current_file = Some("test.gcode".to_string());
            status.gcode_state = Some(status::GcodeState::Running);
            status.print_percent = Some(50);
            status.layer_num = Some(50);
            status.total_layer_num = Some(100);

            status.cooling_fan_speed = Some(100);
            status.aux_fan_speed = Some(70);
            status.chamber_fan_speed = Some(80);

            status.ams = Some(status::AmsStatus {
                units: vec![status::AmsUnit {
                    id: 0,
                    humidity: 0,
                    temp: 0,
                    slots: [
                        Some(status::AmsSlot {
                            material: "PLA".to_string(),
                            k: 0.03,
                            color: egui::Color32::RED,
                        }),
                        None,
                        None,
                        None,
                    ],
                }],
                current_tray: Some(status::AmsCurrentSlot::Tray {
                    ams_id: 0,
                    tray_id: 0,
                }),
            });

            let serial = config.printers[0].serial.clone();
            printer_states.insert(serial, status);
        }

        #[cfg(feature = "nope")]
        {
            let mut status = PrinterStatus::default();
            status.temp_nozzle = Some(200.0);
            status.temp_tgt_nozzle = Some(200.0);
            status.state = status::PrinterState::Idle;
            // status.eta = Some(chrono::Local::now() + chrono::Duration::minutes(10));

            let serial = config.printers[1].serial.clone();
            printer_states.insert(serial, status);
        }
    }

    // let (handle_tx, handle_rx) = tokio::sync::oneshot::channel::<std::num::NonZeroIsize>();
    // let (alert_tx, mut alert_rx) = tokio::sync::mpsc::channel::<(String, String)>(2);

    // #[cfg(feature = "nope")]
    /// tokio thread
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let ctx = ctx_rx.await.unwrap();
            let mut manager =
                PrinterConnManager::new(config2, printer_states2, cmd_rx, msg_tx, ctx);
            // PrinterConnManager::new(config2, printer_states2, cmd_rx, msg_tx, ctx, alert_tx);

            debug!("running PrinterConnManager");
            manager.run().await.unwrap();
        });
    });

    eframe::run_native(
        "Bambu Watcher",
        native_options,
        Box::new(move |cc| {
            // let winit::raw_window_handle::RawWindowHandle::Win32(handle) =
            //     winit::raw_window_handle::HasWindowHandle::window_handle(&cc)
            //         .unwrap()
            //         .as_raw()
            // else {
            //     panic!("Unsupported platform");
            // };

            // std::thread::spawn(move || {
            //     std::thread::sleep(std::time::Duration::from_secs(5));
            //     debug!("spawning");
            //     crate::alert::alert_message(handle.hwnd, "test alert", "test message", false);
            // });

            let context = cc.egui_ctx.clone();

            ctx_tx.send(context.clone()).unwrap();
            // handle_tx.send(handle.hwnd).unwrap();

            // tray-icon crate
            // https://docs.rs/tray-icon/0.12.0/tray_icon/struct.TrayIconEvent.html#method.set_event_handler
            #[cfg(feature = "nope")]
            tray_icon::TrayIconEvent::set_event_handler(Some(
                move |event: tray_icon::TrayIconEvent| {
                    // println!("TrayIconEvent: {:?}", event);
                    if event.click_type != tray_icon::ClickType::Left {
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
                            let _ = windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow(
                                window_handle,
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

            #[cfg(feature = "nope")]
            {
                /// Icon by https://www.flaticon.com/authors/freepik
                let icon = crate::tray::load_icon(&"icon.png");

                tray_c.borrow_mut().replace(
                    tray_icon::TrayIconBuilder::new()
                        // .with_menu(Box::new(menu))
                        .with_menu(Box::new(tray_icon::menu::Menu::new()))
                        .with_tooltip("Bambu Watcher")
                        .with_icon(icon)
                        // .with_title("x")
                        .build()
                        .unwrap(),
                );
            }

            // let tray_icon = tray_icon::TrayIconBuilder::new()
            //     .build()
            //     .unwrap();

            egui_extras::install_image_loaders(&cc.egui_ctx);

            Box::new(ui_types::App::new(
                tray_c,
                printer_states,
                config,
                cc,
                // alert_tx,
            ))
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

                // if matches!(message, Message::Print(_)) {
                //     // let status = message.get_printer_status().unwrap();
                //     // debug!("status = {:#?}", status);
                //     // debug!("done");
                //     break;
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
