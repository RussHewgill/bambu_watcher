use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::{anyhow, bail, ensure, Context, Result};
use dashmap::DashMap;
use tracing::{debug, error, info, trace, warn};

use bambulab::{Client as BambuClient, Message};
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    config::{Config, PrinterConfig},
    status::{PrinterStatus, PrinterStatusReport},
};

/// The serial number of a printer
pub type PrinterId = String;

/// messages from PrinterConnManager to UI
#[derive(Debug, Clone)]
pub enum PrinterConnMsg {
    /// The current status of a printer
    StatusReport(PrinterId, PrinterStatusReport),
}

/// messages from UI to PrinterConnManager
#[derive(Debug, Clone)]
pub enum PrinterConnCmd {
    /// get the status of a printer
    ReportStatus(PrinterId),
}

pub struct PrinterConnManager {
    config: Config,
    printers: HashMap<PrinterId, BambuClient>,
    printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,
    cmd_rx: Receiver<PrinterConnCmd>,
    msg_tx: Sender<PrinterConnMsg>,
}

impl PrinterConnManager {
    pub fn new(
        config: Config,
        printer_states: Arc<DashMap<PrinterId, PrinterStatus>>,
        cmd_rx: Receiver<PrinterConnCmd>,
        msg_tx: Sender<PrinterConnMsg>,
    ) -> Self {
        Self {
            config,
            printers: HashMap::new(),
            printer_states,
            cmd_rx,
            msg_tx,
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
            Message::Print(print) => {
                let status = PrinterStatusReport::from_print_data(&print.print);
                self.msg_tx
                    .send(PrinterConnMsg::StatusReport(id, status))
                    .await
                    .unwrap();
            }
            Message::Info(info) => debug!("printer info: {:?}", info),
            Message::System(system) => debug!("printer system: {:?}", system),
            Message::Unknown(unknown) => error!("unknown message: {:?}", unknown),
            Message::Connecting => debug!("printer connecting: {:?}", id),
            Message::Connected => {
                info!("printer connected: {:?}", id);
                let client = self
                    .printers
                    .get(&id)
                    .with_context(|| format!("printer not found: {:?}", id))?;
                client.publish(bambulab::Command::PushAll).await.unwrap();
            }
            Message::Reconnecting => warn!("printer reconnecting: {:?}", id),
            Message::Disconnected => error!("printer disconnected: {:?}", id),
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
                client.publish(bambulab::Command::PushAll).await.unwrap();
            }
        }
        Ok(())
    }

    async fn start_printer_listener(
        msg_tx: Sender<(PrinterId, Message)>,
        printer: &PrinterConfig,
    ) -> Result<BambuClient> {
        let (tx, mut rx) = tokio::sync::broadcast::channel::<Message>(25);
        let mut client =
            bambulab::Client::new(&printer.host, &printer.access_code, &printer.serial, tx);
        let client_clone = client.clone();
        tokio::spawn(async move {
            client.run().await.unwrap();
        });
        let serial = printer.serial.clone();

        /// get a message from the printer, add the ID, and forward to the conn manager
        tokio::spawn(async move {
            loop {
                let message = rx.recv().await.unwrap();
                msg_tx.send((serial.clone(), message)).await.unwrap();
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
