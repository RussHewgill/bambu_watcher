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
};
use dashmap::DashMap;
// use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    config::{Config, PrinterConfig},
    status::{PrinterState, PrinterStatus},
};

/// The serial number of a printer
pub type PrinterId = String;

/// messages from PrinterConnManager to UI
#[derive(Debug, Clone)]
pub enum PrinterConnMsg {
    Empty,
    /// The current status of a printer
    StatusReport(PrinterId, PrintData),
}

/// messages from UI to PrinterConnManager
#[derive(Debug, Clone)]
pub enum PrinterConnCmd {
    AddPrinter(PrinterConfig),
    /// get the status of a printer
    ReportStatus(PrinterId),
}

pub struct PrinterConnManager {
    // config: Config,
    config: ConfigArc,
    printers: HashMap<PrinterId, BambuClient>,
    printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,
    cmd_rx: tokio::sync::mpsc::Receiver<PrinterConnCmd>,
    msg_tx: tokio::sync::watch::Sender<PrinterConnMsg>,
    ctx: egui::Context,
    // win_handle: std::num::NonZeroIsize,
    // alert_tx: tokio::sync::mpsc::Sender<(String, String)>,
    tx: tokio::sync::mpsc::Sender<(PrinterId, Message)>,
    rx: tokio::sync::mpsc::Receiver<(PrinterId, Message)>,
}

impl PrinterConnManager {
    pub fn new(
        config: ConfigArc,
        printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,
        cmd_rx: tokio::sync::mpsc::Receiver<PrinterConnCmd>,
        msg_tx: tokio::sync::watch::Sender<PrinterConnMsg>,
        ctx: egui::Context,
        // win_handle: std::num::NonZeroIsize,
        // alert_tx: tokio::sync::mpsc::Sender<(String, String)>,
    ) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(PrinterId, Message)>(25);
        Self {
            config,
            printers: HashMap::new(),
            printer_states,
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
        for printer in self.config.printers().iter() {
            // let client = Self::start_printer_listener(self.tx.clone(), printer).await?;
            // self.printers.insert(printer.serial.clone(), client);
            self.add_printer(printer.clone(), true).await?;
        }

        loop {
            tokio::select! {
                Some(cmd) = self.cmd_rx.recv() => {
                    debug!("got cmd = {:?}", cmd);
                    self.handle_command(cmd).await?;
                }
                Some((id, printer_msg)) = self.rx.recv() => {
                    // debug!("got printer_msg, id = {:?} = {:?}", id, printer_msg);
                    self.handle_printer_msg(id, printer_msg).await?;
                }
            }
        }

        // Ok(())
    }

    async fn add_printer(&mut self, printer: PrinterConfig, from_cfg: bool) -> Result<()> {
        if !from_cfg {
            self.config.add_printer(printer.clone());
        }

        let client = Self::start_printer_listener(self.tx.clone(), &printer).await?;
        self.printers.insert(printer.serial.clone(), client);

        Ok(())
    }

    async fn start_printer_listener(
        msg_tx: tokio::sync::mpsc::Sender<(PrinterId, Message)>,
        printer: &PrinterConfig,
    ) -> Result<BambuClient> {
        let mut client = crate::mqtt::BambuClient::new_and_init(&printer, msg_tx).await?;
        Ok(client)
    }
}

impl PrinterConnManager {
    async fn handle_printer_msg(&mut self, id: PrinterId, msg: Message) -> Result<()> {
        match msg {
            Message::Print(report) => {
                // debug!("got print report");
                // debug!("got print report = {:?}", report.print);
                // debug!("gcode_state = {:?}", report.print.gcode_state);
                // if report.print.spd_lvl.is_some() {
                // }
                // let report = PrinterStatusReport::from_print_data(&print.print);

                let mut entry = self.printer_states.entry(id.clone()).or_default();

                let prev_error = entry.is_error();

                entry.update(&report.print)?;

                // debug!("is_error: {:?}", entry.is_error());

                if !prev_error && entry.is_error() {
                    warn!("printer error: {:?}", id);

                    let error = report
                        .print
                        .print_error
                        .clone()
                        .context("no error found?")?;
                    let name = self
                        .config
                        .get_printer(&id)
                        .context("printer not found")?
                        .name
                        .clone();

                    let _ = notify_rust::Notification::new()
                        .summary(&format!("Printer Error: {}", name))
                        .body(&format!("Printer error: {:?}\n\nError: {:?}", id, error))
                        // .icon("thunderbird")
                        .appname("Bambu Watcher")
                        .timeout(0)
                        .show();
                }

                // let handle = self.win_handle.clone();
                // let id2 = id.clone();
                // std::thread::spawn(move || {
                //     crate::alert::alert_message(
                //         handle,
                //         "Print Error",
                //         "Printer error",
                //         // true,
                //         false,
                //     );
                // });

                self.ctx.request_repaint();

                if let Err(e) = self
                    .msg_tx
                    .send(PrinterConnMsg::StatusReport(id, report.print))
                {
                    error!("error sending status report: {:?}", e);
                }
                // .await
            }
            Message::Info(info) => debug!("printer info: {:?}", info),
            Message::System(system) => debug!("printer system: {:?}", system),
            Message::Unknown(unknown) => match unknown {
                Some(unknown) => warn!("unknown message: {}", unknown),
                _ => warn!("unknown message: None"),
            },
            Message::Connecting => debug!("printer connecting: {:?}", id),
            Message::Connected => {
                info!("printer connected: {:?}", id);
                let client = self
                    .printers
                    .get(&id)
                    .with_context(|| format!("printer not found: {:?}", id))?;
                if let Err(e) = client.publish(Command::PushAll).await {
                    error!("error publishing status: {:?}", e);
                }
                let mut entry = self.printer_states.entry(id.clone()).or_default();
                entry.reset();
                self.ctx.request_repaint();
            }
            Message::Reconnecting => warn!("printer reconnecting: {:?}", id),
            Message::Disconnected => {
                error!("printer disconnected: {:?}", id);

                let mut entry = self.printer_states.entry(id.clone()).or_default();
                entry.state = PrinterState::Disconnected;
                self.ctx.request_repaint();
            }
        }
        Ok(())
    }

    async fn handle_command(&mut self, cmd: PrinterConnCmd) -> Result<()> {
        match cmd {
            PrinterConnCmd::AddPrinter(printer) => {
                self.add_printer(printer, false).await?;
                // unimplemented!()
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
        }
        Ok(())
    }
}

/// old
#[cfg(feature = "nope")]
impl PrinterConnManager {
    pub fn new(
        config: Config,
        printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,
        cmd_rx: tokio::sync::mpsc::Receiver<PrinterConnCmd>,
        msg_tx: tokio::sync::watch::Sender<PrinterConnMsg>,
        ctx: egui::Context,
        // win_handle: std::num::NonZeroIsize,
        // alert_tx: tokio::sync::mpsc::Sender<(String, String)>,
    ) -> Self {
        Self {
            config,
            printers: HashMap::new(),
            printer_states,
            cmd_rx,
            msg_tx,
            ctx,
            // win_handle,
            // alert_tx,
        }
    }

    /// init:
    ///     - create a channel to get events from all printers
    ///     - start a listener for each printer
    ///     - save that printer's cloned client to the printers hashmap
    /// loop:
    ///     - wait for a message from either the UI or a printer
    pub async fn run(&mut self) -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<(PrinterId, Message)>(25);

        for printer in self.config.printers.iter() {
            let client = Self::start_printer_listener(tx.clone(), printer).await?;
            self.printers.insert(printer.serial.clone(), client);
        }

        loop {
            tokio::select! {
                Some(cmd) = self.cmd_rx.recv() => {
                    debug!("got cmd = {:?}", cmd);
                    self.handle_command(cmd).await?;
                }
                Some((id, printer_msg)) = rx.recv() => {
                    // debug!("got printer_msg, id = {:?} = {:?}", id, printer_msg);
                    self.handle_printer_msg(id, printer_msg).await?;
                }
            }
        }

        // Ok(())
    }

    async fn handle_printer_msg(&mut self, id: PrinterId, msg: Message) -> Result<()> {
        match msg {
            Message::Print(report) => {
                debug!("got print report");
                // if report.print.spd_lvl.is_some() {
                // }
                // let report = PrinterStatusReport::from_print_data(&print.print);

                let mut entry = self.printer_states.entry(id.clone()).or_default();

                let prev_error = entry.is_error();

                entry.update(&report.print)?;

                // debug!("is_error: {:?}", entry.is_error());

                if !prev_error && entry.is_error() {
                    warn!("printer error: {:?}", id);

                    let error = report
                        .print
                        .print_error
                        .clone()
                        .context("no error found?")?;
                    let name = self
                        .config
                        .printers
                        .iter()
                        .find(|p| p.serial == id)
                        .context("printer not found")?
                        .name
                        .clone();

                    let _ = notify_rust::Notification::new()
                        .summary(&format!("Printer Error: {}", name))
                        .body(&format!("Printer error: {:?}\n\nError: {:?}", id, error))
                        // .icon("thunderbird")
                        .appname("Bambu Watcher")
                        .timeout(0)
                        .show();
                }

                // let handle = self.win_handle.clone();
                // let id2 = id.clone();
                // std::thread::spawn(move || {
                //     crate::alert::alert_message(
                //         handle,
                //         "Print Error",
                //         "Printer error",
                //         // true,
                //         false,
                //     );
                // });

                self.ctx.request_repaint();

                if let Err(e) = self
                    .msg_tx
                    .send(PrinterConnMsg::StatusReport(id, report.print))
                {
                    error!("error sending status report: {:?}", e);
                }
                // .await
            }
            Message::Info(info) => debug!("printer info: {:?}", info),
            Message::System(system) => debug!("printer system: {:?}", system),
            Message::Unknown(unknown) => match unknown {
                Some(unknown) => warn!("unknown message: {}", unknown),
                _ => warn!("unknown message: None"),
            },
            Message::Connecting => debug!("printer connecting: {:?}", id),
            Message::Connected => {
                info!("printer connected: {:?}", id);
                let client = self
                    .printers
                    .get(&id)
                    .with_context(|| format!("printer not found: {:?}", id))?;
                if let Err(e) = client.publish(Command::PushAll).await {
                    error!("error publishing status: {:?}", e);
                }
                let mut entry = self.printer_states.entry(id.clone()).or_default();
                entry.reset();
                self.ctx.request_repaint();
            }
            Message::Reconnecting => warn!("printer reconnecting: {:?}", id),
            Message::Disconnected => {
                error!("printer disconnected: {:?}", id);

                let mut entry = self.printer_states.entry(id.clone()).or_default();
                entry.state = PrinterState::Disconnected;
                self.ctx.request_repaint();
            }
        }
        Ok(())
    }

    async fn handle_command(&mut self, cmd: PrinterConnCmd) -> Result<()> {
        match cmd {
            PrinterConnCmd::ReportStatus(id) => {
                let client = self
                    .printers
                    .get(&id)
                    .with_context(|| format!("printer not found: {:?}", id))?;
                if let Err(e) = client.publish(Command::PushAll).await {
                    error!("error publishing status: {:?}", e);
                }
            }
        }
        Ok(())
    }

    async fn start_printer_listener(
        msg_tx: tokio::sync::mpsc::Sender<(PrinterId, Message)>,
        printer: &PrinterConfig,
    ) -> Result<BambuClient> {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<Message>(25);
        let mut client =
            // bambulab::Client::new(&printer.host, &printer.access_code, &printer.serial, tx);
            crate::mqtt::BambuClient::new(&printer, tx);
        let client_clone = client.clone();
        tokio::spawn(async move {
            if let Err(e) = client.run().await {
                error!("error running client: {:?}", e);
            }
        });
        let serial = printer.serial.clone();

        /// get a message from the printer, add the ID, and forward to the conn manager
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(message) => {
                        if let Err(e) = msg_tx.send((serial.clone(), message)).await {
                            error!("error sending message: {:?}", e);
                        }
                    }
                    Err(e) => {
                        error!("error receiving message: {:?}", e);
                        break;
                    }
                }
            }
        });
        Ok(client_clone)
    }
}

#[cfg(feature = "nope")]
mod old {

    pub struct PrinterConnManager {
        // pub printers: HashMap<String, PrinterConn>,
        pub printers_chans: HashMap<String, Sender<PrinterConnCmd>>,
        pub cmd_rx: Receiver<PrinterConnCmd>,
    }

    pub struct PrinterConn {
        msg_rx: tokio::sync::broadcast::Receiver<Message>,
    }

    impl PrinterConnManager {
        pub fn new() -> Self {
            Self { printers: vec![] }
        }

        pub async fn run(&mut self) -> Result<()> {
            Ok(())
        }
    }

    #[cfg(feature = "nope")]
    async fn start_printer_listener(
        tx: tokio::sync::broadcast::Sender<Message>,
        host: &str,
        access_code: &str,
        serial: &str,
    ) -> Result<()> {
        let mut client = bambulab::Client::new(host, access_code, serial, tx);
        client.run().await.unwrap();
        Ok(())
    }
}
