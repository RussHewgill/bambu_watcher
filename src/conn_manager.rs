use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;

// use bambulab::{Client as BambuClient, Message};
use crate::{
    cloud::{errors::ErrorMap, streaming::StreamCmd},
    config::ConfigArc,
    mqtt::{
        command::Command,
        message::{Message, PrintData},
        BambuClient,
    },
    status::{bambu::PrinterStatus, PrinterType},
    ui::ui_types::{NewPrinterEntry, ProjectsList},
};
use dashmap::DashMap;
// use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    config::{Config, PrinterConfig},
    status::PrinterState,
};

/// The serial number of a printer
// pub type PrinterId = String;
pub type PrinterId = Arc<String>;

/// messages from PrinterConnManager to UI
#[derive(Debug)]
pub enum PrinterConnMsg {
    /// The current status of a printer
    StatusReport(PrinterId, PrintData),
    LoggedIn,
    SyncedProjects(crate::ui::ui_types::ProjectsList),
    SyncedPrinters,
}

/// messages from UI to PrinterConnManager
#[derive(Debug)]
pub enum PrinterConnCmd {
    // Crash,
    SyncPrinters,
    AddPrinter(PrinterConfig),
    RemovePrinter(PrinterId),
    UpdatePrinterConfig(PrinterId, NewPrinterEntry),
    SetPrinterCloud(PrinterId, bool),

    SyncProjects,

    /// get the status of a printer
    ReportStatus(PrinterId),
    ReportInfo(PrinterId),

    Login(String, String),
    Logout,

    /// Sent with QoS of 1 for higher priority.
    Pause,
    /// Sent with QoS of 1 for higher priority.
    Stop,
    /// Sent with QoS of 1 for higher priority.
    Resume,
    SetChamberLight(bool),
    /// 1 = silent
    /// 2 = standard
    /// 3 = sport
    /// 4 = ludicrous
    ChangeSpeed(u8),

    GCodeLine(String),
    Calibration,

    UnloadFilament,

    /// tray_id
    ChangeFilament(i64),
    ChangeAMSFilamentSetting {
        ams_id: i64,
        tray_id: i64,
        tray_color: [u8; 3],
        nozzle_temp_min: i64,
        nozzle_temp_max: i64,
        tray_type: String,
    },
}

pub struct PrinterConnManager {
    config: ConfigArc,
    printers: HashMap<PrinterId, BambuClient>,
    printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,
    cmd_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>,
    cmd_rx: tokio::sync::mpsc::UnboundedReceiver<PrinterConnCmd>,
    // msg_tx: tokio::sync::watch::Sender<PrinterConnMsg>,
    msg_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnMsg>,
    ctx: egui::Context,
    // alert_tx: tokio::sync::mpsc::Sender<(String, String)>,
    tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
    rx: tokio::sync::mpsc::UnboundedReceiver<(PrinterId, Message)>,
    kill_chans: HashMap<PrinterId, tokio::sync::oneshot::Sender<()>>,
    stream_cmd_tx: tokio::sync::mpsc::UnboundedSender<StreamCmd>,
    graphs: crate::ui::plotting::Graphs,
    error_map: ErrorMap,

    num_none_msgs: u32,
}

/// new, start listeners
impl PrinterConnManager {
    pub async fn new(
        config: ConfigArc,
        printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,
        cmd_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>,
        cmd_rx: tokio::sync::mpsc::UnboundedReceiver<PrinterConnCmd>,
        // msg_tx: tokio::sync::watch::Sender<PrinterConnMsg>,
        msg_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnMsg>,
        ctx: egui::Context,
        graphs: crate::ui::plotting::Graphs,
        stream_cmd_tx: tokio::sync::mpsc::UnboundedSender<StreamCmd>,
        // win_handle: std::num::NonZeroIsize,
        // alert_tx: tokio::sync::mpsc::Sender<(String, String)>,
    ) -> Self {
        // let channel_size = if cfg!(debug_assertions) { 1 } else { 50 };
        // let (tx, mut rx) = tokio::sync::mpsc::channel::<(PrinterId, Message)>(channel_size);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(PrinterId, Message)>();

        /// fetch error codes
        let error_map = ErrorMap::read_or_fetch().await.unwrap_or_default();

        Self {
            config,
            printers: HashMap::new(),
            printer_states,
            cmd_tx,
            cmd_rx,
            msg_tx,
            ctx,
            tx,
            rx,
            kill_chans: HashMap::new(),
            stream_cmd_tx,
            graphs,
            error_map,
            num_none_msgs: 0,
        }
    }

    pub async fn init(&mut self) -> Result<()> {
        for printer in self.config.printers() {
            // let client = Self::start_printer_listener(self.tx.clone(), printer).await?;
            // self.printers.insert(printer.serial.clone(), client);
            self.add_printer(printer.clone(), true).await?;
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                Some(cmd) = self.cmd_rx.recv() => {
                    match cmd {
                        PrinterConnCmd::Login(_, _) => debug!("got cmd = Login"),
                        _ => debug!("got cmd = {:?}", cmd),
                    }
                    self.handle_command(cmd).await?;
                }
                Some((id, printer_msg)) = self.rx.recv() => {
                    // debug!("got printer_msg, id = {:?} = {:?}", id, printer_msg);
                    // if let Some(printer) = self.config.get_printer(&id) {
                    // }
                    self.handle_printer_msg(id, printer_msg).await?;
                    // panic!("TODO: handle printer message");
                }
            }
        }

        // Ok(())
    }

    async fn add_printer(
        &mut self,
        printer: Arc<RwLock<PrinterConfig>>,
        from_cfg: bool,
    ) -> Result<()> {
        if !from_cfg {
            // self.config.add_printer(printer.unwrap_or_clone()));
            self.config.add_printer(printer.clone()).await;
        }

        let (kill_tx, kill_rx) = tokio::sync::oneshot::channel::<()>();

        let id = printer.read().await.serial.clone();
        if self.kill_chans.contains_key(&id) {
            bail!("printer already exists: {:?}", id);
        }
        self.kill_chans.insert(id, kill_tx);

        // let client = Self::start_printer_listener(
        //     self.config.clone(),
        //     self.tx.clone(),
        //     printer.clone(),
        //     kill_rx,
        // )
        // .await?;

        let mut client = crate::mqtt::BambuClient::new_and_init(
            self.config.clone(),
            printer.clone(),
            self.tx.clone(),
            kill_rx,
        )
        .await?;

        self.printers
            .insert(printer.read().await.serial.clone(), client);

        Ok(())
    }

    // async fn start_printer_listener(
    //     config: ConfigArc,
    //     msg_tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
    //     // printer: PrinterConfig,
    //     printer: Arc<RwLock<PrinterConfig>>,
    //     kill_rx: tokio::sync::oneshot::Receiver<()>,
    // ) -> Result<BambuClient> {
    //     let mut client =
    //         crate::mqtt::BambuClient::new_and_init(config, printer, msg_tx, kill_rx).await?;
    //     Ok(client)
    // }
}

/// handle messages, commands
impl PrinterConnManager {
    async fn handle_printer_msg(
        &mut self,
        // printer: Arc<PrinterConfig>,
        id: PrinterId,
        msg: Message,
    ) -> Result<()> {
        let Some(printer) = self.config.get_printer(&id) else {
            bail!("printer not found: {:?}", id);
        };

        if !matches!(msg, Message::Unknown(None)) {
            if self.num_none_msgs > 0 {
                self.num_none_msgs -= 1;
            }
        }

        match msg {
            Message::Unknown(unknown) => match unknown {
                Some(unknown) => warn!("unknown message: {:?}", unknown),
                _ => {
                    self.num_none_msgs += 1;
                    if self.num_none_msgs > 2 {
                        warn!("too many unknown messages");
                        bail!("too many unknown messages");
                    }
                    trace!("unknown message: None");
                }
            },
            Message::Print(report) => {
                // debug!("got print report");

                let printer = printer.read().await;

                self.graphs.update_printer(&printer.serial, &report.print);

                let mut entry = self
                    .printer_states
                    .entry(printer.serial.clone())
                    .or_default();

                let prev_state = entry.state.clone();
                let prev_error = entry.is_error();

                entry.update(&printer, &report.print)?;

                if prev_state != entry.state {
                    info!("printer state changed: {:?}", entry.state);

                    /// print just finished, send notification
                    if prev_state != PrinterState::Disconnected
                        && entry.state == PrinterState::Finished
                    {
                        warn!("sent finish notification");
                        crate::alert::alert_print_complete(
                            &printer.name,
                            entry
                                .current_file
                                .as_ref()
                                .unwrap_or(&"Unknown File".to_string()),
                        )
                    }

                    /// either print just started, or app was just started
                    if entry.state == PrinterState::Printing && entry.subtask_id.is_some() {
                        entry.current_task_thumbnail_url = None;
                    }
                }

                /// logged in and printing, but no thumbnail
                if self.config.logged_in()
                    && entry.state == PrinterState::Printing
                    && entry.subtask_id.is_some()
                    && entry.current_task_thumbnail_url.is_none()
                {
                    let config2 = self.config.clone();
                    // let printer2 = printer.clone();
                    let serial = printer.serial.clone();
                    let printer_states2 = self.printer_states.clone();
                    let task_id = entry.subtask_id.as_ref().unwrap().clone();
                    // warn!("skipping fetch thumnail");
                    warn!("spawning fetch thumnail");
                    tokio::spawn(async {
                        fetch_printer_task_thumbnail(config2, serial, printer_states2, task_id)
                            .await;
                    });
                    warn!("spawned fetch thumnail");
                    //
                }

                if !prev_error && entry.is_error() {
                    warn!("printer error: {:?}", &printer.name);

                    let error = report
                        .print
                        .print_error
                        .clone()
                        .context("no error found?")?;
                    let name = self
                        .config
                        .get_printer(&printer.serial)
                        .context("printer not found")?
                        .read()
                        .await
                        .name
                        .clone();

                    let error = self
                        .error_map
                        .get_error(error as u64)
                        .unwrap_or("Unknown Error");

                    crate::alert::alert_printer_error(&printer.name, error);
                }

                self.ctx.request_repaint();

                if let Err(e) = self.msg_tx.send(PrinterConnMsg::StatusReport(
                    printer.serial.clone(),
                    report.print,
                )) {
                    error!("error sending status report: {:?}", e);
                }

                if entry.printer_type.is_none() {
                    self.cmd_tx
                        .send(PrinterConnCmd::ReportInfo(printer.serial.clone()))?;
                }

                // .await
            }
            Message::Info(info) => {
                // debug!("printer info for {:?}: {:?}", &printer.name, info);
                debug!(
                    "got printer info for printer: {:?}",
                    &printer.read().await.name
                );

                let mut entry = self
                    .printer_states
                    .entry(printer.read().await.serial.clone())
                    .or_default();

                entry.printer_type = Some(crate::utils::get_printer_type(&info.info));

                #[cfg(feature = "nope")]
                for module in info.info.module.iter() {
                    // debug!("module {:?} = {:?}", module.name, module.project_name);

                    // let mut module = module.clone();
                    // module.sn = "redacted".to_string();
                    // debug!("module {:?} = {:?}", module.name, module);

                    #[cfg(feature = "nope")]
                    if module.name == "mc" {
                        // debug!("project_name = {:?}", module.project_name);
                        match module.project_name.as_ref() {
                            None => entry.printer_type = Some(PrinterType::X1),
                            Some(s) => match s.as_str() {
                                "P1" => {
                                    if entry.chamber_fan_speed.is_some() {
                                        entry.printer_type = Some(PrinterType::P1S);
                                    } else {
                                        entry.printer_type = Some(PrinterType::P1P);
                                    }
                                }
                                "N2S" => entry.printer_type = Some(PrinterType::A1),
                                "N1" => entry.printer_type = Some(PrinterType::A1m),
                                _ => {
                                    warn!("unknown printer type: {:?}", s);
                                    entry.printer_type = Some(PrinterType::Unknown);
                                }
                            },
                        }
                        debug!("set printer type: {:?}", entry.printer_type);
                    }
                }
                // entry.printer_type

                //
            }
            Message::System(system) => debug!("printer system: {:?}", system),
            Message::Connecting => debug!("printer connecting: {:?}", &printer.read().await.name),
            Message::Connected => {
                let name = &printer.read().await.name;
                info!("printer connected: {:?}", &name);

                let client = self
                    .printers
                    .get(&printer.read().await.serial)
                    .with_context(|| format!("printer not found: {:?}", &name))?;
                if let Err(e) = client.publish(Command::PushAll).await {
                    error!("error publishing status: {:?}", e);
                }
                let mut entry = self
                    .printer_states
                    .entry(printer.read().await.serial.clone())
                    .or_default();
                entry.reset();
                self.ctx.request_repaint();
            }
            Message::Reconnecting => {
                warn!("printer reconnecting: {:?}", &printer.read().await.name)
            }
            Message::Disconnected => {
                error!("printer disconnected: {:?}", &printer.read().await.name);

                let mut entry = self
                    .printer_states
                    .entry(printer.read().await.serial.clone())
                    .or_default();
                entry.state = PrinterState::Disconnected;
                self.ctx.request_repaint();
            }
        }
        Ok(())
    }

    async fn handle_command(&mut self, cmd: PrinterConnCmd) -> Result<()> {
        match cmd {
            // PrinterConnCmd::Crash => {
            //     bail!("crash");
            // }
            PrinterConnCmd::AddPrinter(printer) => {
                self.add_printer(Arc::new(RwLock::new(printer)), false)
                    .await?;
                // unimplemented!()
            }
            PrinterConnCmd::SyncPrinters => {
                let ctx2 = self.ctx.clone();
                let config2 = self.config.clone();
                let msg_tx2 = self.msg_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = sync_printers(ctx2, config2, msg_tx2).await {
                        error!("error syncing printers: {:?}", e);
                    }
                });
            }
            PrinterConnCmd::SetPrinterCloud(id, cloud) => {
                debug!("set printer cloud: {:?}", cloud);

                {
                    // let mut cfg = self.config.config.write().await;
                    // if let Some(printer) = cfg.printer_mut(&id) {
                    //     // printer.cloud = cloud;
                    // }
                    error!("TODO: set printer cloud");
                }

                //
            }
            PrinterConnCmd::SyncProjects => {
                let config2 = self.config.clone();
                let msg_tx2 = self.msg_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = sync_projects(config2, msg_tx2).await {
                        error!("error syncing projects: {:?}", e);
                    }
                });
            }
            PrinterConnCmd::ReportInfo(id) => {
                let client = self
                    .printers
                    .get(&id)
                    .with_context(|| format!("printer not found: {:?}", id))?;
                if let Err(e) = client.publish(Command::GetVersion).await {
                    error!("error publishing status: {:?}", e);
                }
            }
            PrinterConnCmd::ReportStatus(id) => {
                let client = self
                    .printers
                    .get(&id)
                    .with_context(|| format!("printer not found: {:?}", id))?;
                if let Err(e) = client.publish(Command::PushAll).await {
                    error!("error publishing status: {:?}", e);
                }
            }
            PrinterConnCmd::Login(username, password) => {
                // self.get_token(username, pass).await?;
                let tx2 = self.msg_tx.clone();
                let config2 = self.config.clone();

                tokio::spawn(async move {
                    if let Err(e) = login(tx2, config2, username, password).await {
                        error!("error getting token: {:?}", e);
                    }
                });

                #[cfg(feature = "nope")]
                tokio::spawn(async move {
                    // if let Err(e) = login(tx2, auth, username, password).await {
                    //     error!("error getting token: {:?}", e);
                    // }
                    // login(tx2, auth, username, password).await.unwrap();
                    // let t = auth.write().get_token();
                    // debug!("got token: {:?}", t);
                    // tx2.send(PrinterConnMsg::LoggedIn).unwrap();

                    let mut auth2 = auth.write();

                    auth2
                        .login_and_get_token(&username2, &password2)
                        .await
                        .unwrap();

                    // auth.write()
                    //     .login_and_get_token(&username2, &password2)
                    //     .await
                    //     .unwrap();
                    // if let Err(e) = auth.write().login_and_get_token(&username, &password).await {
                    //     error!("error fetching token: {:?}", e);
                    // };
                });
            }
            PrinterConnCmd::Logout => {
                // self.config.config.write().await.logged_in = false;
                self.config.set_logged_in(false);
                if let Err(e) = self.config.auth.write().await.clear_token() {
                    error!("error clearing token: {:?}", e);
                }
            }

            PrinterConnCmd::RemovePrinter(_) => todo!(),
            PrinterConnCmd::UpdatePrinterConfig(id, cfg) => {
                self.config.update_printer(&id, &cfg).await;
                if !cfg.host.is_empty() {
                    self.stream_cmd_tx.send(StreamCmd::RestartStream(id))?;
                } else {
                    self.stream_cmd_tx.send(StreamCmd::StopStream(id))?;
                }
            }
            PrinterConnCmd::Pause => todo!(),
            PrinterConnCmd::Stop => todo!(),
            PrinterConnCmd::Resume => todo!(),
            PrinterConnCmd::SetChamberLight(_) => todo!(),
            PrinterConnCmd::ChangeSpeed(_) => todo!(),
            PrinterConnCmd::GCodeLine(_) => todo!(),
            PrinterConnCmd::Calibration => todo!(),
            PrinterConnCmd::UnloadFilament => todo!(),
            PrinterConnCmd::ChangeFilament(_) => todo!(),
            PrinterConnCmd::ChangeAMSFilamentSetting {
                ams_id,
                tray_id,
                tray_color,
                nozzle_temp_min,
                nozzle_temp_max,
                tray_type,
            } => todo!(),
        }
        Ok(())
    }
}

async fn login(
    tx: tokio::sync::mpsc::UnboundedSender<PrinterConnMsg>,
    // auth: Arc<tokio::sync::RwLock<crate::auth::AuthDb>>,
    config: ConfigArc,
    username: String,
    password: String,
) -> Result<()> {
    // = config.fetch_new_token(&username, &password).await {
    if let Err(e) = config
        .auth
        .write()
        .await
        .login_and_get_token(&username, &password)
        .await
    {
        error!("error fetching token: {:?}", e);
        return Err(e);
    };

    // config.config.write().await.logged_in = true;
    config.set_logged_in(true);

    // let token = crate::cloud::get_token(&username, &pass).await?;
    // config.set_token(token);
    tx.send(PrinterConnMsg::LoggedIn)?;
    Ok(())
}

async fn fetch_printer_task_thumbnail(
    config: ConfigArc,
    id: PrinterId,
    printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,
    task_id: String,
) {
    debug!("fetch_printer_task_thumbnail: {:?}", task_id);
    if let Ok(Some(token)) = config.get_token_async().await {
        // debug!("got token");
        if let Ok(info) = crate::cloud::get_subtask_info(&token, &task_id).await {
            // debug!("got subtask info");
            let url = info.context.plates[0].thumbnail.url.clone();
            if let Some(mut entry) = printer_states.get_mut(&id) {
                entry.current_task_thumbnail_url = Some(url);
            }

            // let pick_picture = info.context.plates[0].pick_picture.url.clone();
            // let top_picture = info.context.plates[0].top_picture.url.clone();

            // debug!("pick_picture = {:?}", pick_picture);
            // debug!("top_picture = {:?}", top_picture);
        }
    }

    // if let Some(mut entry) = printer_states.get_mut(&id.serial) {
    //     entry.current_task_thumbnail_url = Some(std::env::var("TEST_IMG").unwrap());
    // }
}

async fn sync_projects(
    config: ConfigArc,
    msg_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnMsg>,
) -> Result<()> {
    let Some(token) = config.get_token_async().await? else {
        bail!("no token found");
    };

    /// projects
    #[cfg(feature = "nope")]
    {
        let projects = crate::cloud::get_project_list(&token).await?;

        let mut project_list = vec![];

        warn!("skipping all but first 3 projects");
        let projects = &projects[..3];

        for project in projects {
            let Ok(project) = crate::cloud::get_project_info(&token, &project.project_id).await
            else {
                continue;
            };

            let project = crate::cloud::projects::ProjectData::from_json(project)?;

            project_list.push(project);
        }
    }

    let task_list = crate::cloud::get_task_list(&token, None, None, Some(40)).await?;

    debug!(
        "got task list, total = {}: {:?}",
        task_list.total,
        task_list.hits.len()
    );

    let task_list = task_list
        .hits
        .into_iter()
        .map(|t| crate::cloud::projects::TaskData::from_json(&t))
        .collect();

    let project_list = ProjectsList::new(task_list);
    msg_tx.send(PrinterConnMsg::SyncedProjects(project_list))?;
    Ok(())
}

async fn sync_printers(
    ctx: egui::Context,
    config: ConfigArc,
    msg_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnMsg>,
) -> Result<()> {
    let Some(token) = config.get_token_async().await? else {
        bail!("no token found");
    };

    debug!("syncing printers");
    let devices = crate::cloud::get_printer_list(&token).await?;
    debug!("got printer list");

    for device in devices {
        let id = Arc::new(device.dev_id.clone());
        debug!("adding id");
        if config.get_printer(&id).is_some() {
            debug!("skipping");
            continue;
        }

        let printer = PrinterConfig::from_device(id.clone(), &device);

        let printer = Arc::new(RwLock::new(printer));

        config.add_printer(printer).await;
        debug!("added");
    }

    ctx.request_repaint();
    Ok(())
}
