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
pub mod auth;
pub mod config;
pub mod conn_manager;
// pub mod ftp;
pub mod logging;
pub mod mqtt;
pub mod status;
// pub mod tray;
pub mod cloud;
pub mod ui;
// pub mod mqtt_types;

use anyhow::{anyhow, bail, ensure, Context, Result};
use config::ConfigArc;
use parking_lot::RwLock;
use tracing::{debug, error, info, trace, warn};

use futures::StreamExt;
// use rumqttc::{Client, MqttOptions, QoS};
use dashmap::DashMap;
use rumqttc::tokio_rustls::rustls;
use std::{collections::HashMap, env, sync::Arc, time::Duration};

// use bambulab::{Command, Message};

use crate::{
    conn_manager::{PrinterConnCmd, PrinterConnManager, PrinterConnMsg, PrinterId},
    status::PrinterStatus,
};

/// config test
#[cfg(feature = "nope")]
fn main() -> Result<()> {
    dotenv::dotenv()?;
    logging::init_logs();

    // let path = "config.yaml";
    // let path = "config_test.yaml";

    // let printer0 = config::PrinterConfig {
    //     name: "bambu".to_string(),
    //     host: env::var("BAMBU_IP")?,
    //     access_code: env::var("BAMBU_ACCESS_CODE")?,
    //     serial: Arc::new(env::var("BAMBU_IDENT")?),
    // };

    // let config = config::ConfigFile {
    //     printers: vec![printer0],
    // };

    // serde_yaml::to_writer(std::fs::File::create(path)?, &config)?;

    // let config: config::ConfigFile = serde_yaml::from_reader(std::fs::File::open(path)?)?;

    // let config = config::Config::read_from_file(path)?;

    // debug!("config = {:#?}", config);

    // let path = "example.json";
    let path = "example2.json";
    // let path = "example3.json";

    // let msg: bambulab::Message = serde_json::from_reader(std::fs::File::open(path)?)?;
    let msg: mqtt::message::Message = serde_json::from_reader(std::fs::File::open(path)?)?;

    debug!("msg = {:#?}", msg);

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

/// cloud test
#[cfg(feature = "nope")]
// #[tokio::main]
fn main() -> Result<()> {
    dotenvy::dotenv()?;
    logging::init_logs();

    let username = env::var("CLOUD_USERNAME")?;
    let password = env::var("CLOUD_PASSWORD")?;

    // let username = "test_user";
    // let password = "test_pass";

    // debug!("username = {}", username);
    // debug!("password = {}", password);

    #[cfg(feature = "nope")]
    {
        let url = "https://bambulab.com/api/sign-in/form";

        let mut map = HashMap::new();
        map.insert("account", username);
        map.insert("password", password);
        // map.insert("apiError", "".to_string());

        // let client = reqwest::blocking::Client::new();
        let client = reqwest::blocking::ClientBuilder::new()
            .use_rustls_tls()
            // .use_na
            // .use_rustls_tls()
            .build()?;

        // let req = client.post(url).json(&map);

        // let json = serde_json::to_string(&map)?;
        // debug!("json = {}", json);
        let req = client
            .post(url)
            .header("content-type", "application/json")
            // .header("content-length", format!("{}", json.len()))
            // .body(json);
            .json(&map);

        // let req = req.build()?;

        // debug!("req = {:#?}", req);

        // let body = req.body().unwrap().as_bytes().unwrap();
        // let body = std::str::from_utf8(body).unwrap();

        // debug!("body = {}", body);

        let res = req.send()?;

        debug!("res = {:#?}", res);

        if !res.status().is_success() {
            debug!("failure");
            panic!();
        } else {
            debug!("success");
        }

        // debug!("headers = {:#?}", res.headers());

        // let cookies = res.headers().get_all("set-cookie");

        // let mut token = None;
        // let mut refresh_token = None;

        // for cookie in cookies.iter() {
        //     let cookie = cookie::Cookie::parse(cookie.to_str()?).unwrap();

        //     if cookie.name() == "token" {
        //         debug!("expires = {:?}", cookie.expires());
        //         token = Some(auth::Token::from_cookie(&cookie)?);
        //     } else if cookie.name() == "refreshToken" {
        //         refresh_token = Some(auth::Token::from_cookie(&cookie)?);
        //     }
        // }

        // let token = token.unwrap();
        // debug!("token = {:#?}", token.get_token());

        // let refresh_token = refresh_token.unwrap();

        // let set_cookie = res.headers().get("set-cookie").unwrap().to_str()?;
        // let set_cookie = cookie::Cookie::parse(set_cookie).unwrap();

        // debug!("cookie = {:#?}", set_cookie);

        // let set_cookie = res.headers().get("set-cookie").unwrap().to_str()?;
        // let set_cookie = cookie::Cookie::parse(set_cookie).unwrap();

        // let token = auth::Token::from_cookie(&cloud_cookie)?;

        // debug!("token = {:#?}", token.get_token());

        //
    }

    #[cfg(feature = "nope")]
    {
        let cloud_cookie = env::var("CLOUD_COOKIE")?;
        let cloud_cookie = cookie::Cookie::parse(cloud_cookie).unwrap();
        // debug!("cookie = {:#?}", cloud_cookie);
        // debug!("token = {}", cloud_cookie.value());
        // debug!("expires = {:?}", cloud_cookie.expires());

        let token = auth::Token::from_cookie(&cloud_cookie)?;

        let _ = cloud::get_machines_list(&token)?;

        // let
    }

    #[cfg(feature = "nope")]
    {
        debug!("making auth file");
        let mut db = auth::AuthDb::read_or_create("auth.db")?;

        // let inner = db.get_inner()?;

        // db.set_credentials(&username, &password)?;
        // debug!("set credentials");

        debug!("reading auth file");
        let mut db = auth::AuthDb::read_or_create("auth.db")?;

        debug!("logging in");

        db.login_and_get_token(&username, &password)?;

        // debug!("getting auth");
        // let creds = db.get_auth()?;

        // debug!("creds = {:#?}", creds);

        // debug!("getting token");
        // let token = db.get_token()?;

        // debug!("token = {:#?}", token);
    }

    debug!("reading auth file");
    let mut db = auth::AuthDb::read_or_create("auth.db")?;

    // db.login_and_get_token(&username, &password)?;

    // let token = db.get_token()?.unwrap();
    // debug!("token = {:?}", token.get_token());

    // let _ = cloud::get_machines_list(&token)?;
    // let _ = cloud::get_printer_status(&token)?;
    // let _ = cloud::get_project_list(&token)?;

    // let _ = cloud::get_subtask_info(&token, "157720277")?;

    // let s = std::fs::read_to_string("example4.json")?;

    // let json: cloud::cloud_types::MainStruct = serde_json::from_str(&s)?;

    // debug!("json = {:#?}", json);

    // "H"
    // "157442542"
    // "C"
    // "157720277"

    // let json = cloud::get_response(&token, "/v1/user-service/my/tasks")?;
    // let json = cloud::get_response(&token, "/v1/iot-service/api/user/project/157720277")?;
    // let json = cloud::get_response(&token, "/v1/iot-service/api/user/task/157720277")?;
    // debug!("json {:#?}", json);

    Ok(())
}

/// streaming test
#[cfg(feature = "nope")]
// #[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;
    logging::init_logs();

    let (config, auth) = match config::Config::read_from_file("config.yaml") {
        Ok(config) => config,
        Err(e) => {
            warn!("error reading config: {:?}", e);
            panic!("error reading config: {:?}", e);
        }
    };
    let config = ConfigArc::new(config, auth);

    Ok(())
}

/// MARK: Main:
///     fan speeds
///     AMS status
///     graphs
/// threads:
///     main egui thread
///     tokio thread, listens for messages from the printer
// #[cfg(feature = "nope")]
fn main() -> eframe::Result<()> {
    let _ = dotenvy::dotenv();
    logging::init_logs();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([850.0, 750.0])
            .with_min_inner_size([550.0, 400.0]),
        ..Default::default()
    };

    let (config, auth) = match config::Config::read_from_file("config.yaml") {
        Ok((config, auth)) => (config, auth),
        Err(e) => {
            warn!("error reading config: {:?}", e);
            panic!("error reading config: {:?}", e);
        }
    };

    let config = ConfigArc::new(config, auth);
    let config2 = config.clone();

    // let mut _tray_icon = std::rc::Rc::new(std::cell::RefCell::new(None));
    // let tray_c = _tray_icon.clone();

    let channel_size = if cfg!(debug_assertions) { 1 } else { 50 };

    // let (msg_tx, mut msg_rx) = tokio::sync::watch::channel::<PrinterConnMsg>(PrinterConnMsg::Empty);
    // let (msg_tx, mut msg_rx) = tokio::sync::mpsc::channel::<PrinterConnMsg>(channel_size);
    // let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<PrinterConnCmd>(channel_size);
    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::unbounded_channel::<PrinterConnMsg>();
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<PrinterConnCmd>();

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
            // status.gcode_state = Some(status::GcodeState::Running);
            status.print_percent = Some(50);
            status.layer_num = Some(50);
            status.total_layer_num = Some(100);

            status.cooling_fan_speed = Some(100);
            status.aux_fan_speed = Some(70);
            status.chamber_fan_speed = Some(80);

            // status.ams = Some(status::AmsStatus {
            //     units: vec![
            //         status::AmsUnit {
            //             id: 0,
            //             humidity: 0,
            //             temp: 0.,
            //             slots: [
            //                 Some(status::AmsSlot {
            //                     material: "PLA".to_string(),
            //                     k: 0.03,
            //                     color: egui::Color32::RED,
            //                 }),
            //                 None,
            //                 None,
            //                 None,
            //             ],
            //         },
            //         status::AmsUnit {
            //             id: 1,
            //             humidity: 0,
            //             temp: 0.,
            //             slots: [
            //                 Some(status::AmsSlot {
            //                     material: "PLA".to_string(),
            //                     k: 0.03,
            //                     color: egui::Color32::GREEN,
            //                 }),
            //                 None,
            //                 None,
            //                 None,
            //             ],
            //         },
            //     ],
            //     current_tray: Some(status::AmsCurrentSlot::Tray {
            //         ams_id: 0,
            //         tray_id: 0,
            //     }),
            // });

            let serial = config.printers()[0].serial.clone();
            printer_states.insert(serial, status);
        }

        // #[cfg(feature = "nope")]
        {
            let mut status = PrinterStatus::default();
            status.temp_nozzle = Some(200.0);
            status.temp_tgt_nozzle = Some(200.0);
            status.state = status::PrinterState::Idle;
            // status.eta = Some(chrono::Local::now() + chrono::Duration::minutes(10));

            let serial = config.printers()[1].serial.clone();
            printer_states.insert(serial, status);
        }
    }

    let cmd_tx2 = cmd_tx.clone();

    // #[cfg(feature = "nope")]
    /// tokio thread
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let ctx = ctx_rx.await.unwrap();
            let mut manager =
                PrinterConnManager::new(config2, printer_states2, cmd_tx2, cmd_rx, msg_tx, ctx);
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

            Box::new(ui::ui_types::App::new(
                // tray_c,
                printer_states,
                config,
                cc,
                // alert_tx,
                cmd_tx,
            ))
        }),
    )

    //
}

/// rumqttc test
#[cfg(feature = "nope")]
fn main() -> Result<()> {
    dotenv::dotenv()?;
    logging::init_logs();

    use rumqttc::{Client, MqttOptions, QoS};

    let host = env::var("BAMBU_IP")?;
    let access_code = env::var("BAMBU_ACCESS_CODE")?;
    let serial = env::var("BAMBU_IDENT")?;

    let client_id = "bambu_watcher";

    let mut mqttoptions = MqttOptions::new(client_id, host, 8883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    mqttoptions.set_credentials("bblp", &access_code);

    let mut root_cert_store = rumqttc::tokio_rustls::rustls::RootCertStore::empty();
    // root_cert_store.add_parsable_certificates(
    //     rustls_native_certs::load_native_certs().expect("could not load platform certs"),
    // );
    let mut cert_file = std::io::BufReader::new(std::fs::File::open("certs/root.pem")?);
    let certs = rustls_pemfile::certs(&mut cert_file).flatten();
    root_cert_store.add_parsable_certificates(certs);

    let client_config = rumqttc::tokio_rustls::rustls::ClientConfig::builder()
        // .with_root_certificates(root_cert_store)
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoCertificateVerification))
        .with_no_client_auth();

    // let transport = rumqttc::Transport::tls_with_config(rumqttc::TlsConfiguration::Native);
    let transport = rumqttc::Transport::tls_with_config(rumqttc::TlsConfiguration::Rustls(
        Arc::new(client_config),
    ));

    mqttoptions.set_transport(transport);

    debug!("connecting");
    let (mut client, mut connection) = Client::new(mqttoptions, 10);
    debug!("connected");

    // // client.subscribe(topic, QoS::AtMostOnce).unwrap();

    for (i, notification) in connection.iter().enumerate() {
        println!("Notification = {:?}", notification);
        break;
    }

    Ok(())
}

/// paho-mqtt test
#[cfg(feature = "nope")]
// #[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv()?;
    logging::init_logs();

    let host = env::var("BAMBU_IP")?;
    let access_code = env::var("BAMBU_ACCESS_CODE")?;
    let serial = env::var("BAMBU_IDENT")?;

    let client_id = "bambu_watcher";

    let create_opts = paho_mqtt::CreateOptionsBuilder::new()
        .server_uri(&host)
        .client_id(client_id)
        .max_buffered_messages(25)
        .finalize();

    let mut client = paho_mqtt::AsyncClient::new(create_opts).expect("Failed to create client");
    let stream = conn_manager.get_stream(25);

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
        .password(&access_code)
        .finalize();

    debug!("connecting");
    conn_manager.connect(conn_opts).await?;
    debug!("connected");

    Ok(())
}

/// client test
// #[tokio::main]
#[cfg(feature = "nope")]
async fn main() -> Result<()> {
    use crate::mqtt::{command::Command, message::Message};

    dotenv::dotenv()?;
    logging::init_logs();

    // let config: config::Config =
    //     match serde_yaml::from_reader(std::fs::File::open("config.yaml").unwrap()) {
    //         Ok(config) => config,
    //         Err(e) => {
    //             warn!("error reading config: {:?}", e);
    //             panic!("error reading config: {:?}", e);
    //         }
    //     };

    let host = env::var("BAMBU_IP")?;
    let access_code = env::var("BAMBU_ACCESS_CODE")?;
    let serial = env::var("BAMBU_IDENT")?;

    let printer_config = config::PrinterConfig {
        name: "Calvin".to_string(),
        host,
        access_code,
        serial,
    };
    let config = config::Config {
        printers: vec![printer_config],
    };

    let printer_states: Arc<DashMap<PrinterId, PrinterStatus>> = Arc::new(DashMap::new());

    let (msg_tx, mut msg_rx) = tokio::sync::watch::channel::<PrinterConnMsg>(PrinterConnMsg::Empty);
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel::<PrinterConnCmd>(2);

    let ctx = egui::Context::default();
    let mut conn_manager =
        conn_manager::PrinterConnManager::new(config, printer_states, cmd_rx, msg_tx, ctx);

    debug!("running");
    conn_manager.run().await?;
    debug!("done");

    #[cfg(feature = "nope")]
    {
        let (msg_tx, mut msg_rx) =
            tokio::sync::mpsc::channel::<(PrinterId, mqtt::message::Message)>(50);
        // let (cmd_tx, cmd_rx) = tokio::sync::broadcast::channel::<mqtt::command::Command>(50);

        // let mut client = mqtt::Client::new(&config.printers[0], tx);
        // let mut client = mqtt::Client::new(&config, msg_tx, cmd_rx);
        let mut client = mqtt::BambuClient::new(&config, msg_tx).await?;

        debug!("running");

        // client.publish(Command::PushAll).await?;
        // debug!("published");

        loop {
            let message = msg_rx.recv().await.unwrap();
            debug!("received: {:#?}", message);
        }
    }

    Ok(())
}

/// working?
// #[tokio::main]
#[cfg(feature = "nope")]
async fn main() -> Result<()> {
    use bambulab::{Command, Message};

    dotenv::dotenv()?;
    logging::init_logs();

    let host = env::var("BAMBU_IP")?;
    let access_code = env::var("BAMBU_ACCESS_CODE")?;
    let serial = env::var("BAMBU_IDENT")?;

    let (tx, mut rx) = tokio::sync::broadcast::channel::<Message>(50);

    let mut client = bambulab::Client::new(host, access_code, serial, tx);
    let mut client_clone = conn_manager.clone();

    tokio::try_join!(
        tokio::spawn(async move {
            conn_manager.run().await.unwrap();
        }),
        tokio::spawn(async move {
            loop {
                let message = rx.recv().await.unwrap();
                // debug!("received: {:#?}", message);
                match message {
                    Message::Connected => {
                        client_clone.publish(Command::PushAll).await.unwrap();
                    }
                    Message::Print(_) => {
                        debug!("got print report");
                    }
                    _ => {
                        debug!("got message: {:#?}", message);
                    }
                }
            }
        }),
    )?;

    // let mut client = crate::client::Client::new(host, access_code, serial);

    // debug!("running");
    // client.run().await.unwrap();

    Ok(())
}
