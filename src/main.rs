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
// pub mod ui2;
// pub mod ui3;
pub mod utils;
// pub mod mqtt_types;

use anyhow::{anyhow, bail, ensure, Context, Result};
use cloud::streaming::{StreamCmd, WebcamTexture};
use config::ConfigArc;
use parking_lot::RwLock;
use tracing::{debug, error, info, trace, warn};

use futures::StreamExt;
// use rumqttc::{Client, MqttOptions, QoS};
use dashmap::DashMap;
use rumqttc::tokio_rustls::rustls;
use std::{collections::HashMap, env, sync::Arc, time::Duration, usize};

// use bambulab::{Command, Message};

use crate::{
    conn_manager::{PrinterConnCmd, PrinterConnManager, PrinterConnMsg, PrinterId},
    status::PrinterStatus,
};

/// iced test
#[cfg(feature = "nope")]
fn main() -> iced::Result {
    let _ = dotenvy::dotenv();
    logging::init_logs();

    std::panic::set_hook(Box::new(|panic_info| {
        use std::io::Write;
        eprintln!("{}", panic_info);
        let mut file = std::fs::File::create("crash_log.log").unwrap();
        write!(file, "{}", panic_info).unwrap();
    }));

    let (config, auth) = match config::Config::read_from_file("config.yaml") {
        Ok((config, auth)) => (config, auth),
        Err(e) => {
            warn!("error reading config: {:?}", e);
            panic!("error reading config: {:?}", e);
        }
    };
    let config = ConfigArc::new(config, auth);
    let config2 = config.clone();

    let (msg_tx, mut msg_rx) = tokio::sync::mpsc::unbounded_channel::<PrinterConnMsg>();
    let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel::<PrinterConnCmd>();

    let (img_tx, img_rx) = tokio::sync::watch::channel::<Vec<u8>>(vec![]);

    let printer_states: Arc<DashMap<PrinterId, PrinterStatus>> = Arc::new(DashMap::new());
    let printer_states2 = printer_states.clone();

    /// debug printer state
    // #[cfg(feature = "nope")]
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

            let serial = config.printers()[0].blocking_read().serial.clone();
            printer_states.insert(serial, status);
        }

        // #[cfg(feature = "nope")]
        {
            let mut status = PrinterStatus::default();
            status.temp_nozzle = Some(200.0);
            status.temp_tgt_nozzle = Some(200.0);
            status.state = status::PrinterState::Idle;
            // status.eta = Some(chrono::Local::now() + chrono::Duration::minutes(10));

            let serial = config.printers()[1].blocking_read().serial.clone();
            printer_states.insert(serial, status);
        }
    }

    let flags = ui3::ui_types::AppFlags {
        printer_states,
        config,
        cmd_tx,
        msg_rx,
    };

    // let settings = iced::Settings {
    //     flags,
    // };

    let mut settings = iced::Settings::with_flags(flags);
    settings.id = Some("Bambu Watcher".to_string());
    settings.window = iced::window::Settings {
        size: [850., 750.].into(),
        min_size: Some([550., 400.].into()),
        ..Default::default()
    };

    use iced::Application;
    crate::ui3::ui_types::App::run(settings)

    // ui3::ui3_main().unwrap();
}

/// cloud test
#[cfg(feature = "nope")]
// #[tokio::main]
async fn main() -> Result<()> {
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

    /// cloud test
    #[cfg(feature = "nope")]
    {
        debug!("reading auth file");
        let mut db = auth::AuthDb::read_or_create()?;

        let (username, password) = db.get_cloud_mqtt_creds()?;

        let client_id = format!("bambu-watcher-{}", nanoid::nanoid!(8));

        let host = "us.mqtt.bambulab.com";

        let mut mqttoptions = rumqttc::MqttOptions::new(client_id, host, 8883);
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

        debug!("connecting");
        let (mut client, mut eventloop) = rumqttc::AsyncClient::new(mqttoptions, 10);
        debug!("connected");

        let serial = env::var("BAMBU_IDENT")?;
        let topic_device_request = format!("device/{}/request", &serial);
        let topic_device_report = format!("device/{}/report", &serial);

        let (tx, mut rx) = tokio::sync::broadcast::channel::<mqtt::message::Message>(50);

        use rumqttc::{Event, Incoming};

        let client2 = client.clone();
        loop {
            debug!("looping");
            match eventloop.poll().await.unwrap() {
                Event::Outgoing(event) => {
                    // debug!("outgoing event: {:?}", event);
                }
                Event::Incoming(Incoming::PingResp) => {}
                Event::Incoming(Incoming::ConnAck(c)) => {
                    debug!("got ConnAck: {:?}", c.code);
                    if c.code == rumqttc::ConnectReturnCode::Success {
                        // debug!("Connected to MQTT");
                        client2
                            .subscribe(&topic_device_report, rumqttc::QoS::AtMostOnce)
                            .await
                            .unwrap();
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
                        client2
                            .publish(
                                &topic_device_request,
                                rumqttc::QoS::AtMostOnce,
                                false,
                                mqtt::command::Command::PushAll.get_payload(),
                            )
                            .await
                            .unwrap();
                        debug!("sent");
                        // debug!("sending get version");
                        // self.send_get_version().await?;
                        // debug!("sent");
                    }
                }
                Event::Incoming(Incoming::Publish(p)) => {
                    // debug!("incoming publish");
                    let msg = mqtt::parse::parse_message(&p);
                    debug!("incoming publish: {:?}", msg);
                    // self.tx.send((self.printer_cfg.serial.clone(), msg))?;
                }
                Event::Incoming(event) => {
                    debug!("incoming other event: {:?}", event);
                }
            }
        }
    }

    debug!("reading auth file");
    let mut db = auth::AuthDb::read_or_create()?;

    // // // db.login_and_get_token(&username, &password).await?;

    let token = db.get_token()?.unwrap();

    // let projects = cloud::get_project_list(&token).await?;
    // let s = serde_json::to_string_pretty(&projects)?;
    // std::fs::write("projects.json", s)?;

    // let tasks = cloud::get_task_list(&token).await?;
    // debug!("tasks = {:#?}", tasks);

    // let project_id = "82911955";
    // let project_id = "79930702";

    /// projects list:
    ///     "project_id": 81857163
    /// task_list:
    ///     "id": 82911955
    ///     "profileId": 79930702
    /// project_data:
    ///     "project_id": "81753675",
    /// printer_report:
    ///     "task_id": "161481157",
    ///     "subtask_id": "161481158",
    /// none work with get_subtask_info
    /// projects_list project_id works with get_project_info
    /// printer_reprt task_id works with get_subtask_info
    ///
    // let project_id = "81857163";
    // let project_id = "82911955";
    // let project_id = "79930702";
    // let project_id = "161481157";
    // let project_id = "161481158";
    let project_id = "81753675";
    // let project_id = "82195512";

    // let s = cloud::get_printer_list(&token).await?;

    // debug!("s = {:#?}", s);

    // // let s = cloud::get_subtask_info(&token, project_id).await?;
    // let s = cloud::get_project_info(&token, project_id).await?;

    // // debug!("s = {:#?}", s);

    // let s = serde_json::to_string_pretty(&s)?;
    // std::fs::write("project_data.json", s)?;

    // let s: serde_json::Value = cloud::get_response(&token, "/v1/user-service/my/messages").await?;

    // let printer = config::PrinterConfig {
    //     name: "bambu".to_string(),
    //     host: env::var("BAMBU_IP")?,
    //     access_code: env::var("BAMBU_ACCESS_CODE")?,
    //     serial: Arc::new(env::var("BAMBU_IDENT")?),
    //     color: [0; 3],
    // };
    // crate::mqtt::debug_get_printer_report(printer).await?;

    // let mut file = std::fs::File::open("projects.json")?;
    // let s = std::fs::read_to_string("projects.json")?;
    // let projects: cloud::projects::Root = serde_json::from_str(&s)?;

    // // for project in projects.projects {
    // //     debug!("project = {:#?}", project);
    // // }
    // let s = std::fs::read_to_string("project_data.json")?;

    // let p: serde_json::Value = serde_json::from_str(&s)?;
    // let p: cloud::projects::ProjectDataJson = serde_json::from_str(&s)?;
    // let p = cloud::projects::ProjectData::from_json(p)?;

    // debug!("p = {:#?}", p);

    // let t = "2024-05-22 04:16:34";

    // let t = chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S").unwrap();

    // let s = std::fs::read_to_string("task_list.json")?;
    // let s: cloud::projects::TasksInfo = serde_json::from_str(&s)?;

    // let t = &s.hits[0];

    // let t = cloud::projects::TaskData::from_json(t);

    // debug!("t = {:#?}", t);

    /// 117473284 - fail to retract
    let e = 117473284;

    // let errors = cloud::errors::fetch_error_codes().await?;

    // let s = format!("{:x}", e);
    // debug!("s = {}", s);

    // let errors = cloud::errors::ErrorMap::read_or_fetch().await?;

    // let e = errors.get_error(e).unwrap();

    // debug!("error = {}", e);

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

    std::panic::set_hook(Box::new(|panic_info| {
        use std::io::Write;
        eprintln!("{}", panic_info);
        let mut file = std::fs::File::create("crash_log.log").unwrap();
        write!(file, "{}", panic_info).unwrap();
    }));

    // let icon: egui::IconData = {
    //     let icon = include_bytes!("../icon.png");
    //     let icon = image::load_from_memory(icon).unwrap();
    //     let icon = egui::IconData {
    //         rgba: icon.to_rgba8().into_raw(),
    //         width: icon.width(),
    //         height: icon.height(),
    //     };
    //     icon
    // };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            // .with_icon(icon)
            .with_inner_size([850.0, 750.0])
            .with_min_inner_size([550.0, 400.0]),
        ..Default::default()
    };

    // if true {
    //     return crate::ui::error_message::run_error_app("Test Error".to_string());
    // }

    let (config, auth) = match config::Config::read_from_file("config.yaml") {
        Ok((config, auth)) => (config, auth),
        Err(e) => {
            warn!("error reading config: {:?}", e);
            // panic!("error reading config: {:?}", e);
            // (config::Config::empty(), auth::AuthDb::empty())
            // return crate::ui::error_message::run_error_app(e.to_string());
            return crate::ui::error_message::run_error_app(e.to_string());
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

    let (img_tx, img_rx) = tokio::sync::watch::channel::<Vec<u8>>(vec![]);

    let (stream_cmd_tx, stream_cmd_rx) = tokio::sync::mpsc::unbounded_channel::<StreamCmd>();
    let stream_cmd_tx2 = stream_cmd_tx.clone();

    let printer_states: Arc<DashMap<PrinterId, PrinterStatus>> = Arc::new(DashMap::new());
    let printer_states2 = printer_states.clone();

    // let (ctx_tx, ctx_rx) = tokio::sync::oneshot::channel::<egui::Context>();
    let (ctx_tx, ctx_rx) = tokio::sync::oneshot::channel::<egui::Context>();
    // let (ctx_tx, ctx_rx) = tokio::sync::oneshot::channel::<(egui::Context, DashMap<PrinterId, WebcamTexture>)>();

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

            let serial = config.printers()[0].blocking_read().serial.clone();
            printer_states.insert(serial, status);
        }

        // #[cfg(feature = "nope")]
        {
            let mut status = PrinterStatus::default();
            status.temp_nozzle = Some(200.0);
            status.temp_tgt_nozzle = Some(200.0);
            status.state = status::PrinterState::Idle;
            // status.eta = Some(chrono::Local::now() + chrono::Duration::minutes(10));

            let serial = config.printers()[1].blocking_read().serial.clone();
            printer_states.insert(serial, status);
        }
    }

    let cmd_tx2 = cmd_tx.clone();

    let graphs = ui::plotting::Graphs::new();
    // let graphs = {
    //     warn!("using debug graph data");
    //     let id0 = config.printer_ids()[0].clone();
    //     let id1 = config.printer_ids()[1].clone();
    //     ui::plotting::Graphs::debug_new(id0, id1)
    // };
    let graphs2 = graphs.clone();

    let handles: Arc<DashMap<PrinterId, WebcamTexture>> = Arc::new(DashMap::new());
    let handles2 = handles.clone();

    // #[cfg(feature = "nope")]
    /// tokio thread
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async move {
            let ctx = ctx_rx.await.unwrap();
            let ctx2 = ctx.clone();

            let config3 = config2.clone();
            tokio::task::spawn(async move {
                let mut manager = crate::cloud::streaming::StreamManager::new(
                    config3.clone(),
                    handles2,
                    stream_cmd_rx,
                    ctx2,
                );

                if let Err(e) = manager.run().await {
                    error!("stream manager error: {:?}", e);
                }
            });

            // /// spawn image streamers
            // for printer in config2.printer_ids() {
            //     let config3 = config2.clone();

            //     let handle = handles.get(&printer).unwrap().clone();
            //     tokio::task::spawn(async move {
            //         if let Ok(mut streamer) =
            //             crate::cloud::streaming::JpegStreamViewer::new(config3, printer, handle)
            //                 .await
            //         {
            //             if let Err(e) = streamer.run().await {
            //                 error!("streamer error: {:?}", e);
            //             }
            //         }
            //     });
            // }

            let mut manager = PrinterConnManager::new(
                config2,
                printer_states2,
                cmd_tx2,
                cmd_rx,
                msg_tx,
                ctx,
                graphs2,
                stream_cmd_tx2,
            )
            .await;
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

            // let mut handles: DashMap<PrinterId, WebcamTexture> = DashMap::new();
            // for printer in config.printer_ids() {
            //     let image = egui::ColorImage::new([80, 80], egui::Color32::from_gray(220));
            //     let handle = cc.egui_ctx.load_texture(
            //         format!("{}_texture", &printer),
            //         image,
            //         Default::default(),
            //     );
            //     handles.insert(printer.clone(), WebcamTexture::new(false, handle.clone()));
            //     // handles.insert(printer.clone(), (true, handle.clone()));
            // }

            ctx_tx
                // .send((context.clone(), handles.clone()))
                .send(context.clone())
                .ok()
                .context("sending context")
                .unwrap();

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
                msg_rx,
                stream_cmd_tx,
                handles,
                graphs,
            ))
        }),
    )

    //
}

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

/// streaming test
#[cfg(feature = "nope")]
// #[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;
    logging::init_logs();

    let host = env::var("BAMBU_IP")?;
    let serial = env::var("BAMBU_IDENT")?;
    let access_code = env::var("BAMBU_ACCESS_CODE")?;

    let mut root_cert_store = rustls::RootCertStore::empty();
    root_cert_store.add_parsable_certificates(
        rustls_native_certs::load_native_certs().expect("could not load platform certs"),
    );

    let client_config = rustls::ClientConfig::builder()
        // .with_root_certificates(root_cert_store)
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(mqtt::NoCertificateVerification {
            serial: serial.clone(),
        }))
        .with_no_client_auth();

    let connector = rumqttc::tokio_rustls::TlsConnector::from(Arc::new(client_config));

    let addr = format!("{}:6000", host);

    debug!("Connecting to {}", addr);
    let stream = tokio::net::TcpStream::connect(addr).await?;
    debug!("Connected");

    let domain = rustls::pki_types::ServerName::try_from(host).unwrap();
    let mut tls_stream = connector.connect(domain, stream).await?;
    debug!("TLS handshake completed");

    let auth_data = {
        use byteorder::{LittleEndian, WriteBytesExt};

        let username = "bblp";

        let mut auth_data = vec![];
        auth_data.write_u32::<LittleEndian>(0x40).unwrap();
        auth_data.write_u32::<LittleEndian>(0x3000).unwrap();
        auth_data.write_u32::<LittleEndian>(0).unwrap();
        auth_data.write_u32::<LittleEndian>(0).unwrap();

        for &b in username.as_bytes() {
            auth_data.push(b);
        }
        for _ in 0..(32 - username.len()) {
            auth_data.push(0);
        }

        for &b in access_code.as_bytes() {
            auth_data.push(b);
        }
        for _ in 0..(32 - access_code.len()) {
            auth_data.push(0);
        }
        auth_data
    };

    let jpeg_start = vec![0xff, 0xd8, 0xff, 0xe0];
    let jpeg_end = vec![0xff, 0xd9];

    /// 4096 is the max we'll get even if we increase this.
    const READ_CHUNK_SIZE: usize = 4096;

    debug!("writing auth data");
    /// Payload format for each image is:
    /// 16 byte header:
    ///   Bytes 0:3   = little endian payload size for the jpeg image (does not include this header).
    ///   Bytes 4:7   = 0x00000000
    ///   Bytes 8:11  = 0x00000001
    ///   Bytes 12:15 = 0x00000000
    /// These first 16 bytes are always delivered by themselves.
    ///
    /// Bytes 16:19                       = jpeg_start magic bytes
    /// Bytes 20:payload_size-2           = jpeg image bytes
    /// Bytes payload_size-2:payload_size = jpeg_end magic bytes
    ///
    /// Further attempts to receive data will get SSLWantReadError until a new image is ready (1-2 seconds later)
    tokio::io::AsyncWriteExt::write_all(&mut tls_stream, &auth_data).await?;

    debug!("getting socket status");
    let status = tls_stream.get_ref().0.take_error()?;
    debug!("status = {:?}", status);

    let mut buf = [0u8; READ_CHUNK_SIZE];

    let mut payload_size = 0;

    let mut img: Vec<u8> = vec![];

    let mut got_header = false;

    use tokio::io::AsyncReadExt;
    loop {
        buf.fill(0);
        tls_stream.get_ref().0.readable().await?;
        let n = tls_stream.read(&mut buf).await?;

        if got_header {
            // debug!("extending image by {}", n);
            img.extend_from_slice(&buf[..n]);

            if img.len() > payload_size {
                warn!(
                    "unexpected image payload received: {} > {}",
                    img.len(),
                    payload_size,
                );
                break;
            } else if img.len() == payload_size {
                if &img[0..4] != &jpeg_start {
                    warn!("missing jpeg start bytes");
                    break;
                } else if &img[payload_size - 2..payload_size - 0] != &jpeg_end {
                    warn!("missing jpeg end bytes");
                    break;
                }

                debug!("got image");
                /// use image crate to write jpeg to file
                let mut f = std::fs::File::create("test.jpg")?;
                std::io::Write::write_all(&mut f, &img)?;

                break;
            }
        } else if n == 16 {
            debug!("got header");
            // img.extend_from_slice(&buf);

            // payload_size = int.from_bytes(dr[0:3], byteorder='little')
            // payload_size = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
            payload_size =
                <byteorder::LittleEndian as byteorder::ByteOrder>::read_u32(&buf[0..4]) as usize;

            debug!("payload_size = {}", payload_size);
            got_header = true;
        }

        if n == 0 {
            debug!("wrong access code");
            break;
        }
    }

    Ok(())
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
