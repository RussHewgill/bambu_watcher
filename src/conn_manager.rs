use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

// use bambulab::{Client as BambuClient, Message};
use crate::{
    config::ConfigArc,
    mqtt::{
        command::Command,
        message::{Message, PrintData},
        BambuClient,
    },
    status::PrinterType,
};
use dashmap::DashMap;
// use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    config::{Config, PrinterConfig},
    status::{PrinterState, PrinterStatus},
};

/// The serial number of a printer
// pub type PrinterId = String;
pub type PrinterId = Arc<String>;

/// messages from PrinterConnManager to UI
#[derive(Debug, Clone)]
pub enum PrinterConnMsg {
    Empty,
    /// The current status of a printer
    StatusReport(PrinterId, PrintData),
    LoggedIn,
}

/// messages from UI to PrinterConnManager
#[derive(Debug)]
pub enum PrinterConnCmd {
    AddPrinter(PrinterConfig),
    /// get the status of a printer
    ReportStatus(PrinterId),
    ReportInfo(PrinterId),
    Login(String, String),
    Logout,
}

pub struct PrinterConnManager {
    // config: Config,
    config: ConfigArc,
    printers: HashMap<PrinterId, BambuClient>,
    printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,
    cmd_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>,
    cmd_rx: tokio::sync::mpsc::UnboundedReceiver<PrinterConnCmd>,
    // msg_tx: tokio::sync::watch::Sender<PrinterConnMsg>,
    msg_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnMsg>,
    ctx: egui::Context,
    // win_handle: std::num::NonZeroIsize,
    // alert_tx: tokio::sync::mpsc::Sender<(String, String)>,
    tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
    rx: tokio::sync::mpsc::UnboundedReceiver<(PrinterId, Message)>,
}

/// new, start listeners
impl PrinterConnManager {
    pub fn new(
        config: ConfigArc,
        printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,
        cmd_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnCmd>,
        cmd_rx: tokio::sync::mpsc::UnboundedReceiver<PrinterConnCmd>,
        // msg_tx: tokio::sync::watch::Sender<PrinterConnMsg>,
        msg_tx: tokio::sync::mpsc::UnboundedSender<PrinterConnMsg>,
        ctx: egui::Context,
        // win_handle: std::num::NonZeroIsize,
        // alert_tx: tokio::sync::mpsc::Sender<(String, String)>,
    ) -> Self {
        // let channel_size = if cfg!(debug_assertions) { 1 } else { 50 };
        // let (tx, mut rx) = tokio::sync::mpsc::channel::<(PrinterId, Message)>(channel_size);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(PrinterId, Message)>();
        Self {
            config,
            printers: HashMap::new(),
            printer_states,
            cmd_tx,
            cmd_rx,
            msg_tx,
            ctx,
            // win_handle,
            // alert_tx,
            tx,
            rx,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        for printer in self.config.printers_async().await.iter() {
            // let client = Self::start_printer_listener(self.tx.clone(), printer).await?;
            // self.printers.insert(printer.serial.clone(), client);
            self.add_printer(printer.clone(), true).await?;
        }

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
                    if let Some(printer) = self.config.get_printer_async(&id).await {
                        self.handle_printer_msg(printer, printer_msg).await?;
                    }
                }
            }
        }

        // Ok(())
    }

    async fn add_printer(&mut self, printer: Arc<PrinterConfig>, from_cfg: bool) -> Result<()> {
        if !from_cfg {
            // self.config.add_printer(printer.unwrap_or_clone()));
            self.config.add_printer(printer.clone());
        }

        let client = Self::start_printer_listener(self.tx.clone(), printer.clone()).await?;
        self.printers.insert(printer.serial.clone(), client);

        Ok(())
    }

    async fn start_printer_listener(
        msg_tx: tokio::sync::mpsc::UnboundedSender<(PrinterId, Message)>,
        printer: Arc<PrinterConfig>,
    ) -> Result<BambuClient> {
        let mut client = crate::mqtt::BambuClient::new_and_init(printer, msg_tx).await?;
        Ok(client)
    }
}

/// handle messages, commands
impl PrinterConnManager {
    async fn handle_printer_msg(
        &mut self,
        printer: Arc<PrinterConfig>,
        msg: Message,
    ) -> Result<()> {
        match msg {
            Message::Print(report) => {
                // debug!("got print report");

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
                    if entry.state == PrinterState::Finished {
                        let _ = notify_rust::Notification::new()
                            .summary(&format!("Print Complete on {}", printer.name))
                            .body(&format!(
                                "{}",
                                entry
                                    .current_file
                                    .as_ref()
                                    .unwrap_or(&"Unknown File".to_string())
                            ))
                            // .icon("thunderbird")
                            .appname("Bambu Watcher")
                            .timeout(0)
                            .show();
                    }

                    /// either print just started, or app was just started
                    if entry.state == PrinterState::Printing && entry.subtask_id.is_some() {
                        entry.current_task_thumbnail_url = None;
                    }
                }

                /// logged in and printing, but no thumbnail
                if self.config.logged_in_async().await
                    && entry.state == PrinterState::Printing
                    && entry.subtask_id.is_some()
                    && entry.current_task_thumbnail_url.is_none()
                {
                    let config2 = self.config.clone();
                    let printer2 = printer.clone();
                    let printer_states2 = self.printer_states.clone();
                    let task_id = entry.subtask_id.as_ref().unwrap().clone();
                    // warn!("skipping fetch thumnail");
                    warn!("spawning fetch thumnail");
                    tokio::spawn(async {
                        fetch_printer_task_thumbnail(config2, printer2, printer_states2, task_id)
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
                        .get_printer_async(&printer.serial)
                        .await
                        .context("printer not found")?
                        .name
                        .clone();

                    let _ = notify_rust::Notification::new()
                        .summary(&format!("Printer Error: {}", name))
                        .body(&format!(
                            "Printer error: {:?}\n\nError: {:?}",
                            &printer.name, error
                        ))
                        // .icon("thunderbird")
                        .appname("Bambu Watcher")
                        .timeout(0)
                        .show();
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
                debug!("got printer info for printer: {:?}", &printer.name);

                let mut entry = self
                    .printer_states
                    .entry(printer.serial.clone())
                    .or_default();

                for module in info.info.module.iter() {
                    // debug!("module {:?} = {:?}", module.name, module.project_name);

                    // let mut module = module.clone();
                    // module.sn = "redacted".to_string();
                    // debug!("module {:?} = {:?}", module.name, module);

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
            Message::Unknown(unknown) => match unknown {
                Some(unknown) => warn!("unknown message: {}", unknown),
                _ => warn!("unknown message: None"),
            },
            Message::Connecting => debug!("printer connecting: {:?}", &printer.name),
            Message::Connected => {
                info!("printer connected: {:?}", &printer.name);
                let client = self
                    .printers
                    .get(&printer.serial)
                    .with_context(|| format!("printer not found: {:?}", &printer.name))?;
                if let Err(e) = client.publish(Command::PushAll).await {
                    error!("error publishing status: {:?}", e);
                }
                let mut entry = self
                    .printer_states
                    .entry(printer.serial.clone())
                    .or_default();
                entry.reset();
                self.ctx.request_repaint();
            }
            Message::Reconnecting => warn!("printer reconnecting: {:?}", &printer.name),
            Message::Disconnected => {
                error!("printer disconnected: {:?}", &printer.name);

                let mut entry = self
                    .printer_states
                    .entry(printer.serial.clone())
                    .or_default();
                entry.state = PrinterState::Disconnected;
                self.ctx.request_repaint();
            }
        }
        Ok(())
    }

    async fn handle_command(&mut self, cmd: PrinterConnCmd) -> Result<()> {
        match cmd {
            PrinterConnCmd::AddPrinter(printer) => {
                self.add_printer(Arc::new(printer), false).await?;
                // unimplemented!()
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
                self.config.config.write().await.logged_in = false;
                if let Err(e) = self.config.auth.write().await.clear_token() {
                    error!("error clearing token: {:?}", e);
                }
            }
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

    config.config.write().await.logged_in = true;

    // let token = crate::cloud::get_token(&username, &pass).await?;
    // config.set_token(token);
    tx.send(PrinterConnMsg::LoggedIn)?;
    Ok(())
}

async fn fetch_printer_task_thumbnail(
    config: ConfigArc,
    id: Arc<PrinterConfig>,
    printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,
    task_id: String,
) {
    debug!("fetch_printer_task_thumbnail: {:?}", task_id);
    if let Ok(Some(token)) = config.get_token_async().await {
        debug!("got token");
        if let Ok(info) = crate::cloud::get_subtask_info(&token, &task_id).await {
            debug!("got subtask info");
            let url = info.context.plates[0].thumbnail.url.clone();
            if let Some(mut entry) = printer_states.get_mut(&id.serial) {
                entry.current_task_thumbnail_url = Some(url);
            }
        }
    }

    // if let Some(mut entry) = printer_states.get_mut(&id.serial) {
    //     entry.current_task_thumbnail_url = Some(std::env::var("TEST_IMG").unwrap());
    // }
}
